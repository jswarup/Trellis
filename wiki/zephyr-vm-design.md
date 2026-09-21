# Zephyr VM Flavors

## Purpose

Trellis needs one Zephyr guest framework with interchangeable execution engines. A test scenario, the Crew peripheral contract, and observability must remain the same when execution moves from the current Rust library simulation to Renode and, later, to an instruction-decoding hypervisor.

The public owner is `ZephyrVm`. It owns lifecycle, configuration, runtime selection, and normalized results. Each flavor implements the execution-specific portion behind `ZephyrRuntime`.

```text
ZephyrVm
  -> ZephyrRuntime
       -> LibRuntime
       -> RenodeRuntime
       -> HypervisorRuntime

ZephyrRuntime -> Crew MMIO device -> CrewHub -> configured node links
```

## Invariants

- All flavors use the same Crew register layout at `0x5000_0000`.
- The same `CrewHub` owns node routing, queues, statistics, reset, and callback behavior.
- All flavors expose the same lifecycle: create, start, step, reset, stop.
- The scheduler controls guest progress. A flavor must not require an uncontrolled background thread to make progress.
- Runtime-specific diagnostics may be richer, but every flavor reports common state, elapsed virtual time, exit reason, and MMIO activity.
- Firmware scenarios are flavor-independent. A scenario specifies inputs and expected externally observable outputs; it does not call a particular runtime implementation.

## Shared Codebase

### Stable shared layer

The following code is shared by every flavor and must not depend on Renode or a CPU decoder:

- `crew::protocol`: register offsets, status bits, packet definitions, and MMIO access rules.
- `crew::hub` and `crew::node`: node state, routing, RX queues, counters, callbacks, reset, and topology.
- `zephyr::driver`: the host-side reference implementation of the Crew MMIO driver and register-level tests.
- `zephyr::scenario`: reusable integration scenarios, message payloads, expectations, and trace comparison.
- `zephyr::config`: machine, firmware, timing, tracing, and topology configuration.
- `zephyr::runtime`: the runtime trait, lifecycle state, step results, errors, and normalized diagnostics.

### Flavor-specific layer

| Flavor | Executes | Reuses | Owns |
| --- | --- | --- | --- |
| `LibRuntime` | Rust simulation methods | Crew hub, node, protocol, driver, scenarios | Synthetic heartbeat and direct driver calls |
| `RenodeRuntime` | A compiled Zephyr ELF in Renode | Crew hub, node, protocol, scenarios, configuration | Renode machine session, ELF load, platform description, co-simulation transport |
| `HypervisorRuntime` | The same compiled ELF via Trellis's decoder | Crew hub, node, protocol, scenarios, configuration | ELF loader, memory map, CPU state, decoder, interrupt delivery, device interception |

The compiled flavors share firmware source, Zephyr board/device-tree configuration, ELF artifact, memory map, MMIO address, and peripheral behavior. The CPU execution engine is the only intended difference.

## Runtime Contract

The first implementation should use an object-safe runtime interface so a configuration can select a flavor at runtime.

```rust
pub trait ZephyrRuntime {
    fn start(&mut self) -> Result<(), ZephyrError>;
    fn step(&mut self, budget: StepBudget) -> Result<StepResult, ZephyrError>;
    fn reset(&mut self) -> Result<(), ZephyrError>;
    fn stop(&mut self) -> Result<(), ZephyrError>;
    fn state(&self) -> ZephyrState;
    fn diagnostics(&self) -> ZephyrDiagnostics;
}
```

`StepBudget` must be explicit. It may contain an instruction limit, a virtual-time limit, or both. `StepResult` must say how much work completed and whether the guest is still runnable, waiting for an interrupt, halted, or faulted.

`ZephyrVm::send_message` and `recv_message` remain convenience helpers for the library flavor and test setup. Real guests communicate only through their compiled Crew MMIO driver.

## Configuration

Use one serializable-in-the-future Rust configuration model now. It does not require an external serialization dependency.

```rust
pub enum ZephyrFlavor {
    Lib,
    Renode,
    Hypervisor,
}

pub struct ZephyrVmConfig {
    pub flavor: ZephyrFlavor,
    pub node_id: u32,
    pub machine: ZephyrMachineConfig,
    pub execution: ZephyrExecutionConfig,
    pub observability: ZephyrObservabilityConfig,
}

pub struct ZephyrMachineConfig {
    pub crew_base_addr: u64,
    pub firmware_elf: Option<PathBuf>,
    pub board: ZephyrBoard,
    pub memory: ZephyrMemoryConfig,
}

pub struct ZephyrExecutionConfig {
    pub step_instruction_limit: u64,
    pub step_time_ns: u64,
    pub reset_on_start: bool,
    pub rx_interrupt: bool,
}
```

`ZephyrFlavor` is the primary scheme switch. The remaining parameters describe the guest and are valid independently of flavor where possible.

| Parameter | Lib | Renode | Hypervisor |
| --- | --- | --- | --- |
| `flavor` | Selects direct simulation | Selects Renode session | Selects decoder execution |
| `node_id` | Selects Crew node | Selects Crew node | Selects Crew node |
| `crew_base_addr` | Driver base address | Platform peripheral address | MMIO interception range |
| `firmware_elf` | Optional; normally absent | Required | Required |
| `board` | Optional behavioral profile | Required to select Renode platform | Required for CPU/memory setup |
| `memory` | Optional test limits | Must match the platform/ELF | Defines guest address spaces |
| `step_instruction_limit` | Synthetic work budget | Renode execution quantum | Decoder instruction quantum |
| `step_time_ns` | Synthetic time advance | Virtual machine time quantum | Timer/clock advance |
| `rx_interrupt` | Can be simulated | Configures peripheral IRQ | Configures interrupt injection |
| trace/log options | Driver and hub trace | Renode + hub trace | Decoder + hub trace |

Flavor-specific configuration belongs in typed optional substructures such as `RenodeConfig` and `HypervisorConfig`. Do not add a string-keyed catch-all map: invalid combinations should fail during configuration validation.

## Firmware and Peripheral Contract

The compiled firmware must be built once per board profile and used unchanged by both `RenodeRuntime` and `HypervisorRuntime`.

The board profile defines:

- CPU architecture, endianness, clock, reset vector, and exception model.
- RAM and flash ranges, including the exact ELF load address.
- Crew MMIO address and register layout.
- RX interrupt number, priority, and acknowledgement behavior when interrupts are enabled.
- UART/console routing used for test diagnostics.

The Crew peripheral specification must additionally define byte, word, dword, and qword access behavior; unaligned access policy; RX empty reads; TX backpressure; reset effects; and ordering between status reads, RX reads, and interrupt assertion.

## Renode Flavor

`RenodeRuntime` loads the board description and ELF, creates one Renode machine per VM, and bridges the Crew MMIO peripheral to `CrewHub`.

The existing `ProtocolMessage` models the Renode co-simulation packet format. The first integration should use a local process/socket transport because it matches the current protocol and provides fault isolation. A true in-process host is a later optimization and requires a supported, versioned Renode embedding boundary plus a clear ownership and shutdown model for the .NET runtime.

Each `step` issues a bounded execution request, drains co-simulation messages in order, advances the shared virtual scheduler, and returns normalized results. Renode-specific monitor output and register snapshots are attached to diagnostics rather than exposed as required public APIs.

## Hypervisor Flavor

`HypervisorRuntime` consumes the same ELF and board profile. It must provide:

- ELF segment loading and symbol metadata.
- Guest physical memory with configured permissions and reset state.
- CPU register state, instruction decoding, exception handling, and deterministic stepping.
- MMIO interception for the Crew range and invocation of the shared Crew device.
- Timer and RX interrupt delivery compatible with the board profile.
- Snapshot and restore of CPU, memory, device, and scheduler state.

This flavor must not reimplement Crew behavior. Reads and writes at the Crew MMIO range are forwarded to the same shared device contract used by the Renode bridge.

## Topology and Scheduling

`CrewHub::peer_of(id)`, which currently maps nodes with `id ^ 1`, is sufficient only for a two-node demo. Replace it with configuration-owned links before introducing more VMs:

```rust
pub struct CrewLinkConfig {
    pub source_node_id: u32,
    pub destination_node_ids: Vec<u32>,
}
```

The VM coordinator owns global virtual time. It selects runnable VMs in a documented stable order, calls each runtime with its budget, routes MMIO effects immediately in that order, and records a common trace. This is required for reproducible differential tests.

## Verification Plan

1. Preserve the existing `Zephyr::DriverApi` and `Zephyr::DualVmExchange` tests as `LibRuntime` compatibility tests.
2. Add runtime-contract tests using a fake runtime to verify lifecycle and error propagation.
3. Add the same compiled-firmware scenario for Renode and the hypervisor: boot, send a Crew payload, receive a reply, reset, and verify counters.
4. Compare normalized Crew MMIO traces and terminal state across flavors. Differences must be intentional and documented.
5. Add fault cases: missing ELF, unsupported board, invalid MMIO mapping, runtime crash/disconnect, invalid instruction, and IRQ delivery while RX is empty.

## Delivery Order

1. Extract `LibRuntime` from the current `ZephyrVm` without changing behavior.
2. Add runtime lifecycle/configuration and the coordinator, retaining the library flavor as default.
3. Generalize Crew topology and formalize peripheral semantics.
4. Build the Zephyr board and firmware artifact pipeline; implement socket-based `RenodeRuntime`.
5. Freeze Renode trace scenarios as the executable reference for the decoder.
6. Implement `HypervisorRuntime` in stages: ELF/memory, CPU stepping, Crew MMIO, timer/IRQ, then snapshots.
