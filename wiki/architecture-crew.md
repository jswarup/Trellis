# Crew Architecture

## Purpose

`crew` is Trellis's virtual-machine co-simulation coordination framework. It interfaces with external VM emulators (such as Renode) over low-latency socket protocols and provides deterministic in-memory MMIO message dispatch, inter-VM packet routing, and execution telemetry.

## Main building blocks

- `CoSimAction` identifies socket protocol actions defined by Renode's `CoSimulatedPlugin` (bus reads, bus writes, handshakes, clock ticks, resets).
- `ProtocolMessage` is the fixed 24-byte packed payload exchanged between emulator sockets and the C++ co-simulation runtime.
- `REG_*` and `STATUS_*` define memory-mapped I/O (MMIO) register addresses and status flags for inter-VM communication.
- `ICrewNode` specifies the contract for a virtual machine endpoint, including ports, online state, FIFO queues, and statistics.
- `CrewNode` implements `ICrewNode` with thread-safe FIFO queues and telemetry counters.
- `ICrewHub` specifies the coordinator interface for adding nodes, launching worker threads, and observing messages.
- `CrewHub` implements `ICrewHub`, providing multi-threaded socket handling for Renode endpoints as well as direct in-memory `HandleRequest` dispatch for deterministic testing.

## Communication flow

1. Virtual machine firmware (such as Zephyr OS on x86_64) performs 32-bit MMIO reads/writes to base address `0x50000000`.
2. Renode forwards MMIO accesses across TCP sockets to `CrewHub`.
3. `CrewHub` decodes the 24-byte `ProtocolMessage` and dispatches it:
   - Writing to `REG_TX_DATA` queues data into the destination peer's RX FIFO and triggers registered message callbacks.
   - Reading `REG_RX_DATA` pops bytes from the local node's RX FIFO.
   - Reading `REG_STATUS` exposes `STATUS_TX_READY`, `STATUS_RX_READY`, and `STATUS_PEER_UP`.
   - Reading `REG_NODE_ID` returns the current machine ID (e.g. 0 or 1).
4. `CrewHub` replies to Renode with a status `Ok` and the requested register value.

## Determinism & In-Memory Execution

`CrewHub::HandleRequest` is decoupled from OS socket plumbing. This allows unit tests and embedded test harnesses to execute full dual-VM protocol exchanges deterministically in-memory without networking dependencies.

## Invariants

- `ProtocolMessage` is packed to exactly 24 bytes across all platforms.
- MMIO reads and writes update node telemetry (`_ReadsServiced`, `_WritesServiced`, `_BytesSent`, `_BytesReceived`) atomically.
- Peer routing is thread-safe and protected against queue starvation.
