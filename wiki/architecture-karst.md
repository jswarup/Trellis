# Karst Architecture

## Purpose

`karst` is Trellis's memory fabric simulation framework. It models high-radix, chiplet-based memory interconnects inspired by Kandou's Karst/Castor PMF (Parallel Memory Fabric) architecture. Karst maps hardware constructs — front-port IO chiplets (Castor-KarstFore), memory fabric crossbars (Castor-KarstHind), KarstLink interconnects, mpipe retiming FIFOs, physical DDR5 channels, and Near-Memory Compute (EPU) — across Trellis's four substrate layers: **Rube, Crew, Swarm, and Zephyr**.

## Main building blocks

- `KarstFlit` is a packed 64-bit transaction word carried across links, encoding `IsWrite`, `SrcId` (7 bits), byte `Addr` (24 bits), and payload `Data` (32 bits).
- `KarstLink` encapsulates bidirectional valid/data/ready port pairs representing KarstLink (TW3) point-to-point streaming channels.
- `KarstPipe` models the `pipeline_axi_channel` / mpipe retiming stages with configurable FIFO depth and deterministic cycle-accurate delay.
- `KarstNoc` implements the Castor-KarstHind MF-NoC crossbar switch as a Rube coroutine, performing 1 kB striped memory interleaving, local MC routing, and inter-die KarstLink packet forwarding.
- `KarstDChan` simulates a DDR5-8800 physical memory channel backed by a Swarm `ComputeBuffer`.
- `KarstVPU` encapsulates an Edge Processing Unit running Swarm compute kernels directly against a local `KarstDChan` buffer.
- `KarstHostNode` models a Castor-KarstFore front-port IO die with dual-homed Tiger-links (primary Link0 and cross-home Link1).
- `KarstFabricNode` composes one complete Castor-KarstHind die (MF-NoC, 4 DDR5 memory controllers with mpipe retimers, 4 DChans, and 4 EPUs).
- `KarstFabric` is the top-level builder orchestrating the balanced `Karst(8,8)` topology inside a Rube `Layout` and executing it via `SimEngine`.

## Hardware to Substrate Mapping

| Hardware Block (Karst) | Substrate Layer | Karst Construct | Role |
|---|---|---|---|
| Castor-KarstFore IO die | Rube / Crew | `KarstHostNode` | Host front-port transaction generation & dual-homed routing |
| Castor-KarstHind MFab die | Rube | `KarstFabricNode` | Die composite module with NoC and memory channels |
| Tiger-link (TW3) | Rube | `KarstLink` | Bidirectional streaming valid/data/ready channels |
| mpipe retimer | Rube | `KarstPipe` | `pipeline_axi_channel` synchronous FIFO delay line |
| MF-NoC Crossbar | Rube | `KarstNoc` | 10 TW ports + 4 MC ports with 1 kB memory interleaving |
| DDR5 Channel | Swarm | `KarstDChan` | Physical memory storage backed by `ComputeBuffer` |
| RISC-V EPU | Swarm | `KarstVPU` | Near-memory compute dispatch via `ComputeKernel` |
| Host VM Firmware | Zephyr / Renode | `karst_n8.resc` | N=8 virtual machine co-simulation nodes |

## Communication Flow

1. **Host Ingress**: `KarstHostNode` accepts memory read/write requests from the host or co-simulated Zephyr VM.
2. **Dual-Home Routing**: The Forwarding Engine inspects address bit 12 to determine the target KarstHind die. Local requests are routed out over primary `Link0`; cross-die requests are routed over cross-home `Link1`.
3. **KarstLink Transport**: Transactions flow over the `KarstLink` valid/ready streaming interface into the destination Castor-KarstHind die.
4. **MF-NoC Switching**: `KarstNoc` inspects address bits [11:10] to steer transactions across the 4 local memory controllers (1 kB interleaving). If a transaction arrives addressed to the peer die, it is forwarded across inter-die KarstLink ports 8 or 9.
5. **mpipe Retiming**: Transactions pass through `KarstPipe`, simulating the physical pipeline delay of the on-die AXI fabric.
6. **DDR5 Servicing**: The local Memory Controller performs read or write operations on `KarstDChan`'s underlying Swarm buffer. Read responses return via the reverse NoC and KarstLink paths to the requesting host.
7. **Near-Memory Compute**: `KarstVPU` can execute SIMT vector compute kernels directly on `KarstDChan` without moving data across external fabric links.

## Invariants

- All inter-chiplet and inter-module communications adhere strictly to credit-based valid/ready streaming contracts.
- 1 kB address striping deterministically balances host traffic across all 8 DDR5 memory channels.
- Co-simulation operates identically in serial execution and multi-threaded parallel execution (`SimEngineMode::Parallel`).

