# Karst Architecture

## Purpose and System Role

`karst` is Trellis's memory fabric simulation framework. It models high-radix, chiplet-based memory interconnects inspired by Kandou's Karst / Castor Parallel Memory Fabric (PMF) architecture. Karst brings together high-speed point-to-point serial links (KarstLink / TW3), on-die network-on-chip crossbars (MF-NoC), retiming pipeline FIFOs (`pipeline_axi_channel` / mpipe), physical DDR5 channels, and Near-Memory Compute (EPU / VPU) into a single cycle-accurate simulation environment.

Karst unifies all four of Trellis's primary substrate layers:
1. **Rube**: Digital circuit netlists, port connections, coroutine modules (`KarstNoc`, `KarstPipe`, `KarstHostNode`), and cycle stepping via `SimEngine`.
2. **Crew**: Virtual machine endpoints, dual-homed host IO adapters, and inter-VM MMIO bus bridging.
3. **Swarm**: DDR5 memory channel storage (`ComputeBuffer`) and Near-Memory SIMT compute dispatch (`ComputeKernel`).
4. **Zephyr**: Guest operating system firmware issuing memory transactions across co-simulated host nodes.

---

## The Balanced Karst(8, 8) Topology

Karst implements a balanced **Karst(8, 8)** configuration: 8 host front-port IO chiplets communicating with 8 physical DDR5 memory channels distributed across 2 memory fabric dies.

```text
  Host 0    Host 1    Host 2    Host 3        Host 4    Host 5    Host 6    Host 7
(Castor-  (Castor-  (Castor-  (Castor-      (Castor-  (Castor-  (Castor-  (Castor-
Fore 0)   Fore 1)   Fore 2)   Fore 3)       Fore 4)   Fore 5)   Fore 6)   Fore 7)
   | \       | \       | \       | \           / |       / |       / |       / |
   |  \      |  \      |  \      |  \         /  |      /  |      /  |      /  |
   |   \     |   \     |   \     |   \       /   |     /   |     /   |     /   |
   |    \----+----+----+----+-----\---\-----/----+-----+----+----+----/----+   |
   |         |         |         |     \   /     |     |    |    |   /         |
+--+---------+---------+---------+--+ +-\-/------+-----+----+----+--+-+--------+
| TW0       TW1       TW2       TW3 | | TW4     TW5   TW6  TW7  | | TW0...TW3  |
|                                   | |                         | |            |
|       Castor-KarstHind Die 0      | |  Castor-KarstHind Die 1 | |            |
|           (MF-NoC 0)              | |       (MF-NoC 1)        | |            |
|                                   | |                         | |            |
| TW8 (Inter-Die)                   | | TW8 (Inter-Die)         | |            |
+-----------------+-----------------+ +------------+------------+ +------------+
                  |                                |
                  +====== Inter-Die KarstLink =====+
                  |     (Ports TW8 & TW9)          |
                  v                                v
         +-----------------+              +-----------------+
         | MC0 MC1 MC2 MC3 |              | MC0 MC1 MC2 MC3 |
         +--+---+---+---+--+              +--+---+---+---+--+
            |   |   |   |                    |   |   |   |
          mpipe retimers                   mpipe retimers
            |   |   |   |                    |   |   |   |
         +--+---+---+---+--+              +--+---+---+---+--+
         | DC0 DC1 DC2 DC3 |              | DC4 DC5 DC6 DC7 |
         | (DDR5 Channels) |              | (DDR5 Channels) |
         |   +   +   +   + |              |   +   +   +   + |
         | EPU0 EPU1 ...   |              | EPU4 EPU5 ...   |
         +-----------------+              +-----------------+
```

### Topology Parameters (`config.h`)

| Constant | Value | Description |
|---|---|---|
| `k_HostsPerFabric` | 8 | Total front-port IO dies (KarstFore 0..7). |
| `k_DChansPerFabric` | 8 | Total physical DDR5-8800 memory channels (DChan 0..7). |
| `k_HindDiesPerFabric` | 2 | Number of Castor-KarstHind memory fabric crossbar dies. |
| `k_McPerHind` | 4 | Memory controllers per KarstHind die. |
| `k_KarstPortsPerHind` | 10 | Total KarstLink (TW) ports per KarstHind die (8 host + 2 inter-die). |
| `k_KarstPortsPerFore` | 2 | Dual-homed KarstLink ports per KarstFore die (Link0 and Link1). |
| `k_VPUPerHind` | 4 | Near-memory Edge Processing Units (EPUs) per KarstHind die. |
| `k_LinkDepth` | 4 | Retiming FIFO stages per on-die mpipe channel. |

---

## Core Abstractions & Data Structures

### 1. `KarstFlit` (64-bit Packed Transaction Flit)
All transactions flowing across KarstLinks and NoC crossbars are encoded in a single 64-bit word:

```text
 63 62                     56 55                                32 31                             0
+--+-------------------------+------------------------------------+--------------------------------+
|W |      SrcId (7 bits)     |          Addr (24 bits)            |         Data (32 bits)         |
+--+-------------------------+------------------------------------+--------------------------------+
```

- **`IsWrite` [bit 63]**: 1 for write transactions; 0 for read requests and read responses.
- **`SrcId` [bits 62:56]**: 7-bit originator ID (identifies which host 0..7 posted the request, enabling unambiguous response routing).
- **`Addr` [bits 55:32]**: 24-bit byte address in the unified memory fabric address space.
- **`Data` [bits 31:0]**: 32-bit data payload (write data or return read data).
- Functions `KarstFlit::Pack()` and `KarstFlit::Unpack()` provide zero-cost bitwise conversions.

### 2. `KarstLink`
Encapsulates a pair of unidirectional streaming links implementing a standard **valid / data / ready** handshaking contract:
- **TX Side**: `TxValid` (out), `TxData` (out), `TxReady` (in).
- **RX Side**: `RxValid` (in), `RxData` (in), `RxReady` (out).
- Handshake rule: Data is transferred on any cycle where `Valid && Ready == true`. If `Ready` is low, the sender must maintain `Valid` and keep `Data` stable.

### 3. `silo::Fifo<T, Cap>`
A statically allocated, fixed-capacity circular FIFO queue in [`src/silo/fifo.h`](file:///c:/Work/Oogway/Trellis/src/silo/fifo.h). Karst hardware coroutines use `silo::Fifo` instead of `std::deque` to eliminate all heap allocations and pointer indirections during hot-path simulation cycles.

### 4. `KarstPipe` (mpipe Retiming Boundary)
Models Kandou's `pipeline_axi_channel` retiming FIFO:
- Configurable depth (`k_LinkDepth = 4`).
- Bounded synchronous storage modeled via `silo::Fifo<uint64_t, k_LinkDepth>`.
- Preserves strict FIFO ordering and applies downstream backpressure (`UpReady = !fifo.IsFull()`) when filled.

---

## Address Mapping & 1 kB Striped Interleaving

Karst applies deterministic **1 kB address striping** to balance memory bandwidth across all 8 DDR5 memory channels:

```text
 23                        13 12       11    10 9                                2 1     0
+----------------------------+--------+--------+----------------------------------+-------+
|    Higher Stripe Bits      |  Die   |  MC    |       Word in Stripe (0..255)    | Byte  |
|                            | (0..1) | (0..3) |                                  | (0..3)|
+----------------------------+--------+--------+----------------------------------+-------+
```

1. **Byte Offset (`[1:0]`)**: 4 bytes per 32-bit transaction word.
2. **Word Index within Stripe (`[9:2]`)**: 256 words = 1024 bytes (1 kB) per contiguous stripe.
3. **Memory Controller Index (`[11:10]`)**: Selects MC0, MC1, MC2, or MC3 on the target die (`(addr >> 10) & 3U`).
4. **Die Index (`[12]`)**: Selects KarstHind Die 0 or Die 1 (`(addr >> 12) & 1U`).
5. **Higher Stripe Address (`[23:13]`)**: Selects the stripe row in memory.

### Dual-Homed Forwarding Engine (FE) Routing

Each `KarstHostNode` contains a hardware Forwarding Engine that inspects address bit 12:

- **Host Nodes 0..3**:
  - Primary path (Die 0) routes out over **Link0** (connects to Die 0, ports TW0..TW3).
  - Cross-home path (Die 1) routes out over **Link1** (connects to Die 1, ports TW4..TW7).
- **Host Nodes 4..7**:
  - Primary path (Die 1) routes out over **Link0** (connects to Die 1, ports TW0..TW3).
  - Cross-home path (Die 0) routes out over **Link1** (connects to Die 0, ports TW4..TW7).

This dual-homed topology guarantees that every host can access both memory fabric dies with minimal hops, while providing path redundancy.

---

## Component Details

### 1. `KarstNoc` (MF-NoC Crossbar Switch)
The core crossbar switch running as a `rube::CoroModule`:
- **Radix**: 14 bidirectional ports:
  - Ports 0..9: KarstLink (TW) ports (10 total).
  - Ports 10..13: Local Memory Controller (MC0..MC3) ports (4 total).
- **Zero-Allocation Internal Queues**: Managed with 28 static `silo::Fifo<uint64_t, 16>` queues (`mcReqQueue[4]`, `twTxQueue[10]`, `twRxQueue[10]`, `mcRespQueue[4]`).
- **Ingress Isolation**: Samples all inputs independently into local RX FIFOs before performing routing, preventing head-of-line blocking.
- **Inter-Die Forwarding**: If a flit arrives addressed to the peer die, it is automatically forwarded out over inter-die link port 8.
- **Response Routing**: Returns read responses to the requesting host by mapping `flit._SrcId` to the appropriate TW port:
  - On Die 0: `targetTw = flit._SrcId % 8U`.
  - On Die 1: `targetTw = ((flit._SrcId >= 4) ? (flit._SrcId - 4) : (flit._SrcId + 4)) % 8U`.

### 2. `KarstDChan` (DDR5 Memory Channel)
Simulates a physical DDR5-8800 channel:
- **Storage**: Backed by a Swarm `ComputeBuffer` configured with `BufferUsage::Storage() | BufferUsage::ReadWrite()`.
- **Operations**:
  - `WriteWord(addr, data)`: Commits a 32-bit word, updates `_BytesWritten`, and increments `_WritesServiced`.
  - `ReadWord(addr)`: Reads a 32-bit word, updates `_BytesRead`, and increments `_ReadsServiced`.
  - `Fill(pattern)`: Pre-populates memory with repeating patterns for testing.
  - `Verify(expected)`: Fast verification of memory contents.

### 3. `KarstVPU` (Near-Memory Edge Processing Unit)
Models near-memory compute engines embedded adjacent to each DDR5 memory controller:
- Directly dispatches Swarm `ComputeKernel` instances (e.g. `ComputeDevice::DoubleKernel()`) over the local `KarstDChan` buffer.
- Executes SIMT compute workloads without transferring data over off-die KarstLinks or NoC crossbars, cutting memory latency and fabric congestion.
- Tracks kernel invocations via `Dispatches()`.

### 4. `KarstHostNode` (Front-Port IO Chiplet)
Simulates a Castor-KarstFore IO die:
- Features thread-safe `PostWrite(addr, data)` and `PostRead(addr)` interfaces for host software.
- Implements an internal `activeQueue` (`silo::Fifo<HostTransaction, 16>`) and dual-homing router.
- Gathers hardware telemetry: `_WritesPosted`, `_ReadsPosted`, `_TxCount`, and `_RxCount`.

### 5. `KarstFabricNode` & `KarstFabric`
- `KarstFabricNode`: Composes a single KarstHind die (1 `KarstNoc`, 4 `KarstPipe` retimers, 4 Memory Controller coroutines, 4 `KarstDChan` instances, and 4 `KarstVPU` engines).
- `KarstFabric`: Top-level builder that wires the entire `Karst(8, 8)` layout, attaches all dual-homed host links, and steps the simulation via `SimEngine::Advance(cycles)` in either Serial or Parallel work-stealing mode.

---

## Transaction Lifecycle Walkthrough

```mermaid
sequenceDiagram
    autonumber
    participant Host as HostNode 0 (Fore 0)
    participant Link as KarstLink 0 (Tiger-link)
    participant Noc as KarstNoc (Die 0)
    participant Pipe as KarstPipe (mpipe 0)
    participant MC as Memory Controller 0
    participant DChan as KarstDChan 0 (DDR5)

    Note over Host: Host 0 calls PostWrite(0x000, 0xCAFEBABE)
    Host->>Host: FE inspects bit 12 == 0 -> Route to Link0
    Host->>Link: Present KarstFlit (Valid=1, Ready=1)
    Link->>Noc: Ingress flit queued into twRxQueue[0]
    Noc->>Noc: Decode bit [11:10] == 0 -> Route to MC0
    Noc->>Pipe: Push flit into mpipe FIFO (depth=4)
    Note over Pipe: Retiming delay (k_LinkDepth cycles)
    Pipe->>MC: Dequeue flit into MC request stage
    MC->>DChan: WriteWord(0x000, 0xCAFEBABE)
    DChan-->>MC: Commit write
    Note over MC,DChan: Read requests push response flit into reverse path
```

---

## Performance & Invariants

1. **Zero Dynamic Allocation in Simulation Hot Paths**: All FIFOs in `KarstPipe`, `KarstNoc`, `KarstHostNode`, and `KarstFabricNode` use `silo::Fifo` rather than `std::deque`. No memory allocations or deallocations occur during `SimEngine::Advance()` cycles.
2. **Deterministic Interleaving**: 1 kB address striping deterministically routes transactions across the 8 physical memory channels with zero collision when accesses are uniformly distributed.
3. **Rigid Backpressure**: If any memory channel or retimer FIFO fills, backpressure propagates upstream across valid/ready handshake lines all the way to the originating host.
4. **Serial / Parallel Equivalence**: Simulation produces bit-for-bit identical results whether executed sequentially or in parallel (`KarstFabric fabric(4)` using 4 worker threads).
