# Data abstraction review

Date: 2026-10-18

## Scope and criteria

The initial structural scan covered 182 Rust and Python source/stub files under `src`,
`tools`, `truss`, and `examples`, including all 19 library subsystems and the
firmware support package. Review focused on owned payloads, parallel state,
queue boundaries, resource lifetimes, and existing abstraction contracts.
This is an architecture review and a tested implementation packet, not a complete
memory-safety proof of every unsafe block.

An abstraction was selected for implementation when it centralizes an actual
invariant, preserves subsystem boundaries, and adds neither heap allocation nor
dynamic dispatch to execution paths. Existing abstractions were retained where
another wrapper would add indirection without protecting an invariant.

## Implemented changes

### Kernel source ownership

`src/swarm/traits.rs`: `KernelSource` previously stored a discriminator, a string,
a byte buffer, an optional closure, and optional standard-operation metadata in
parallel. A private `KernelPayload` enum now carries exactly one source payload.
CPU metadata lives with the CPU closure. Constructors, accessors, and clone
semantics remain compatible, including empty accessors for unrelated payloads.
The debugger visualizer exposes the new payload field.

This removes invalid kind/payload combinations from the representation and avoids
carrying inactive owners. Regression coverage exercises every variant, cloning,
owned bytecode surviving its original source, and CPU operation metadata.

### Compute buffer acquisition: measured rejection

The two-buffer and three-buffer paths were initially consolidated into a private
stack-only `ComputeBufferLocks<N>`. Alternating release samples showed the
64-element vector-add median increasing from 54.59 to 75.61 ns/dispatch. Despite
substantial timing variance, that result failed the performance requirement, so
the generic helper was removed and the original specialized lock acquisition
paths restored. There is no generalized lock abstraction in the final code.

Regression coverage remains: all six three-buffer permutations, every alias pair,
unsupported backends, opposing concurrent acquisitions, and panic unwinding.
These tests exercise the retained deadlock and ownership contracts.

### Shared-memory packet slots

`src/zephyr/shm.rs`: push, pop, and skip now share a validated `USeg` slot
calculation. Offsets must be within physical capacity and aligned to packet size.
Subtraction after bounds validation replaces overflow-prone offset addition.
Unused tail bytes remain excluded; exact-end offsets wrap on the next operation.
Pop and skip share consumption accounting, including the protocol count mirror.

Oversized declared payloads now fail checksum verification instead of validating
a silently truncated prefix. Failed reads retain the packet and cursor. Invalid
skip offsets leave state unchanged; callers can repair the control block or reset
the ring. Existing public method signatures and serialized packet sizes remain
unchanged. The ring remains an exclusively borrowed in-process emulator; compiler
fences do not establish a concurrent shared-memory transport contract.

Tests cover malformed offsets including `u32::MAX`, oversized payloads, repeated
full/drain cycles at one-slot, exact-multiple, and partial-tail capacities, plus
the existing corruption recovery and long-running wrap tests.

### Link startup state

`src/karst/link.rs`: channel `Default` now delegates to `new()`. This removes a
second initialization policy that started default-constructed links with readiness
false while explicitly constructed links started ready. A regression covers both
channel and bidirectional-link equality, readiness, and idle transfer state.

## Findings across the codebase

| Subsystem | Pattern and disposition |
| --- | --- |
| Silo | `Buff`/`RawBuff`/`Stash` already separate initialized ownership from capacity. Preserve this distinction; a universal container would obscure initialization and drop invariants. Raw pointer and concurrent stack contracts require a dedicated safety review before further generalization. |
| Stalks | `SpinMutex` and guards already colocate locking with protected ownership. Compute acquisition now builds on them rather than creating a second mutex implementation. |
| Heist | `ScopedExecutor` already owns partition policy. Affinity, coroutine continuation, and work stealing have different scheduling contracts; do not combine them behind an unconstrained executor trait. |
| Flux | `FieldExp`/`FieldImp` already represent value kinds. Preserve the distinct borrowed export and mutable import lifetimes. |
| Shard | Parser marks and grammar traits provide useful boundaries. The JSON importer already owns its import stack and rejects transient string borrows; a shared parser/export owner would weaken these lifetime distinctions. |
| Fleck | `GeometryAsset` already owns normalized geometry and source bounds together. Keep external format records separate from validated rendering records. |
| Fenst | Explorer providers and layout/scene records already separate content from presentation. Additional provider traits need a second behavior contract, not merely repeated constructors. |
| Fascia | `AppState` maintains tab metadata and several per-kind resource maps separately. A future `TabDocument` payload enum can colocate lifecycle state, but must preserve cancellation, reload identity, and close-all behavior. This requires a coordinated UI API change rather than a local container replacement. |
| Swarm | Implemented source payload colocation; rejected generalized buffer locking after measurement. Keep `IComputeBackend` associated resource types; backend-specific ownership should not be forced into CPU buffers. |
| Flock | `CpuOutputPartition` already seals disjoint output ownership. Preserve it as the execution boundary rather than exposing raw output pointers. |
| Symph | Shared element operations and `StandardOp` already express CPU/shader computation contracts. Shader-compatible data layout constrains additional wrappers. |
| Drove | Host entry-point declarations and shader implementations are separate build contexts. Similar names do not justify merging their modules. |
| Rube | Simulation flags and ports are deliberately compact. VCD display changes remain a separate owned timeline; any typed transition abstraction must preserve unordered timestamp and alias behavior and be profiled before replacing compact records. |
| Karst | Packed flits, channel state, address decoding, and cycle traces already model useful boundaries. `KarstLinkChannel::default()` now delegates to `new()`, making both channel and link defaults start ready consistently. A regression checks default/new equality and idle transfer state. Transport scheduling remains unchanged. |
| Crew | Node receive queues and delivery outcomes are already bounded and explicit. Node and route tables are separately locked; a future topology snapshot could reduce repeated lookup/locking, but concurrent reconfiguration and multicast backpressure must be specified first. |
| Zephyr | Implemented packet slot validation and shared consumption. Runtime traits correctly isolate emulator/library lifecycle. Keep wire records distinct from the in-process ring owner. |
| Fresco | Expression repositories and term trees already distinguish storage from operators. A common expression trait must retain operator-specific algebraic behavior; repetition alone is insufficient. |
| Cove | Existing registration macros already unify CLI and Rust tests. New regression cases use this mechanism. |
| Python | Binding wrappers sit at the FFI boundary. Keep Python ownership/error conversion out of core geometry and compute types. |

## Verification

The initial `cargo check --all-targets --offline` passed before changes.
After implementation, all 323 library tests passed, including seven new regression
cases. Release measurement results and reproduction instructions are recorded below.

Hardware GPU and Renode tests retain their existing opt-in conditions; their
ordinary harness results do not establish hardware/emulator coverage.

### Performance measurements

The release probe measured `KernelSource` at 64 bytes before the change and
32 bytes with the enum representation on this 64-bit target. This is a storage
reduction; it does not establish a universal throughput improvement.

The rejected lock-helper experiment used six alternating baseline/current runs,
with 10,000 warmup operations and nine batches of 200,000 operations per workload.
The median of the run medians for vector-add was 54.59 versus 75.61 ns/dispatch.
Run ranges overlapped substantially (47.05–84.82 versus 62.58–89.47 ns), so this
was treated as a conservative rejection signal rather than a precise regression
estimate. The final source restores the original specialized acquisition code.
Source cloning and ring timings also varied substantially; no throughput speedup
is claimed for the final implementation.

The reusable probe is `examples/data_abstraction_perf.rs`. It measures WGSL source
cloning, 64-element vector-add and Collatz dispatch with one worker, and push/pop
of a 16-byte payload in a 1,000-byte ring. For repeatable local comparisons, run
without competing compiler jobs and repeat both revisions in alternating order:

```powershell
cargo rustc --release --offline --example data_abstraction_perf -- -C lto=off
.\target\release\examples\data_abstraction_perf.exe
```

Compiler: `rustc 1.97.0-nightly (e96c36b6f 2026-05-21)`; CPU identifier:
`Intel64 Family 6 Model 186 Stepping 2, GenuineIntel`. Library release optimization
uses the repository's `opt-level=3` and ThinLTO; the probe disables LTO at its own
link step to avoid whole-application optimization dominating probe build time.

### Final checks

All commands below completed successfully on the final implementation:

- `cargo test --lib --offline`: 323 passed, 0 failed.
- `cargo check --all-targets --offline`.
- `cargo clippy --all-targets --offline -- -D warnings`.
- `cargo build --offline`.
- Separate `cargo build --offline -p trellis --lib` and
  `cargo build --offline -p trellis --bin trellis`.
- `git diff --check`; debugger visualizer XML parsing.

Cargo still reports the existing `trellis.pdb` output-name collision between the
library and binary. Their Cargo target names were not changed by this work.
