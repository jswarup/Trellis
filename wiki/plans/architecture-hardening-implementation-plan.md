# Trellis Architecture Hardening Implementation Plan

## 1. Purpose

This plan converts the validated findings from `arg.pdf` and `arg2.pdf` into an
implementation sequence for Trellis. It gives correctness and memory safety work
priority over performance tuning. Optimizations that do not yet have measurements
remain gated by benchmarks.

The plan follows the repository directives in `agents/AGENTS.md` and
`agents/FORMATTING.md`:

- Functional data structures keep private, underscore-prefixed fields.
- Public core APIs use `Arr`, `MutArr`, `Buff`, `Stash`, and `USeg` rather than
  slices, `Vec`, or hash maps.
- Container indexes and counts use `u32`.
- Project algorithms use Trellis traversal APIs rather than native integer ranges.
- New tests use `jeeves_test!` in the owning module's `_tests.rs`.
- Structural changes update `tools/trellis.natvis` where debugger inspection is useful.

## 2. Objectives and Non-Goals

### Objectives

1. Remove undefined behavior and publication races from Silo storage and stack APIs.
2. Make the Zephyr shared-memory ring lossless and define its concurrency contract.
3. Remove permanent allocations and incomplete imports from Flux/Shard JSON handling.
4. Remove avoidable per-cycle thread creation from Karst.
5. Fix confirmed Fascia leaks, lifecycle bugs, and incorrect UI behavior.
6. Establish measurements before changing Heist atomics, Rube flag processing, or
   rendering algorithms.
7. Improve module boundaries without exposing mutable state.

### Non-goals

- Do not weaken atomic ordering based only on intuition or microbenchmarks.
- Do not replace Trellis containers with standard hash maps or public vectors.
- Do not combine the safety work with broad naming or formatting rewrites.
- Do not remove robustness for unordered VCD timestamps.
- Do not introduce a new async runtime.

## 3. Delivery Strategy

Each phase should land independently and leave the tree buildable. Safety phases
precede performance work because benchmarks from undefined behavior are not useful.

| Phase | Scope | Exit condition |
|---|---|---|
| 0 | Baselines and characterization | Reproducible correctness and performance baselines recorded |
| 1 | Silo initialization and `Stk` | No uninitialized `T` is exposed or dropped; concurrency contract tested |
| 2 | Zephyr shared-memory ring | Full/empty/wrap behavior is lossless with defined ordering |
| 3 | Flux and Shard import ownership | No parser leaks; `USeg` imports correctly and rejects malformed objects |
| 4 | Karst scheduling | No OS-thread creation per simulated cycle; parity retained |
| 5 | Fascia correctness | Confirmed leaks and lifecycle defects fixed |
| 6 | Swarm, Symph, Rube, and measured performance work | Backend contracts agree; only benchmark-supported changes retained |
| 7 | Encapsulation and modularization | Stateful representation is private and subsystem boundaries are simpler |

## 4. Phase 0: Baseline and Characterization

### 4.1 Record a clean baseline

Start from a named commit and record all pre-existing worktree changes. Do not fold
unrelated edits into any implementation packet. If the dependency cache is incomplete,
run the same commands without `--offline` only in an approved network-enabled
environment and record that exception.

Run and record:

```text
cargo check --all-targets --offline
cargo clippy --offline -- -D warnings
cargo test --lib --offline
cargo run --offline -- -t
cargo run --release --offline --example rube_perf -- 4096 100
```

Record the commit, compiler version, CPU, worker count, and build profile with the
benchmark results. GPU and Renode tests remain explicitly opt-in and should be marked
as skipped when their environments are unavailable.

### 4.2 Add focused characterization tests

Before changing behavior, add tests that capture:

- Silo drop counts through `Stash` reserve, grow, pop, clear, and extraction.
- Concurrent `Stk` pushes and pops with non-`Copy`, drop-counted values.
- Zephyr ring empty, one-slot-left, full, exact-end wrap, and repeated wrap behavior.
- JSON import of string-backed enums, `USeg`, malformed objects, and repeated parses.
- Karst serial/parallel trace parity over representative cycle counts.
- Fascia menu construction, close-all cleanup, and deepest-node hit testing.

Tests that would execute existing undefined behavior must initially be written as
ignored characterization harnesses or enabled only after the replacement storage is
in place.

### 4.3 Add narrow benchmarks

Add allocation and latency probes for:

- `Stash` push/pop/grow and the replacement concurrent stack.
- `CrewHub::HandleRequest` status reads and one-to-many delivery.
- Karst `step_cycle` for 1, 2, and many cycles with one and two worker budgets.
- VCD parse plus `VcdDisplayModel` construction for monotonic and unordered inputs.
- Cask label layout and rasterization for ASCII and non-ASCII names.
- Dense waveform drawing at several transition-to-pixel ratios.

Use stable input fixtures and report median or trimmed mean after warmup. Do not make
performance acceptance depend on a single wall-clock sample.

## 5. Phase 1: Silo Storage and Stack Safety

This is the first implementation phase because the existing design can access or drop
uninitialized values.

### 5.1 Separate allocation capacity from initialized length

Refactor `Buff<T>` so a safe owning `Buff<T>` always represents initialized values.
Choose one of these designs during implementation review:

1. Preferred: move raw spare-capacity allocation into a private
   `RawBuff<MaybeUninit<T>>` owned by `Stash<T>`. `Buff<T>` retains its compact,
   fully-initialized fixed-size contract.
2. Alternative: add a private initialized-length member to `Buff<T>` and ensure every
   safe accessor, traversal, destructor, clone, and debugger visualizer uses that length.

The preferred design better preserves the public meaning of `Buff::Cap()` and prevents
safe code from constructing an owning buffer whose elements are uninitialized.

Required changes:

- Make raw allocation constructors private or explicitly unsafe.
- Ensure allocation failure and capacity overflow retain current behavior.
- Use `MaybeUninit<T>` for spare slots.
- Drop exactly the initialized prefix during unwinding and normal destruction.
- Move initialized elements during growth without creating duplicate ownership.
- Preserve `Buff<T>`'s initialized contiguous representation after
  `Stash::ExtractBuff`.
- Update `tools/trellis.natvis` if the inspected layout changes.

Acceptance criteria:

- Drop-count tests show every constructed value is dropped exactly once.
- Panic during construction drops only values completed before the panic.
- Miri, when available, reports no invalid reads or drops in Silo tests.
- Existing `Arr`/`MutArr` public behavior remains compatible.

### 5.2 Replace the current `Stk` protocol

Do not repair `Stk` by replacing `SwapAt` with `WriteAt` alone. The current size CAS
publishes a slot before its value is written, and `PushX` allows producers to race on
the same slot.

First decide the required concurrency model from actual call sites:

- If all uses are externally serialized or single-producer/single-consumer, encode that
  narrower contract in the type and remove unsupported lock-free claims.
- If multi-producer/multi-consumer behavior is required, implement reservation and
  publication separately, with per-slot state or a proven bounded-stack algorithm.

The replacement must provide these invariants:

1. A consumer cannot observe a slot before construction completes.
2. Only one producer owns an uninitialized slot.
3. Only one consumer moves a published value out.
4. A moved-out slot is not dropped until it has been initialized again.
5. Failed pushes leave ownership of the caller's value unchanged.
6. Atomic ordering is documented at each publication and reclamation point.

Avoid returning an unrestricted `MutArr` over concurrently managed storage. Provide
specific operations and read-only snapshots where inspection is needed.

Acceptance criteria:

- Saturation, empty-pop, wrap/reuse, and drop-count tests pass.
- Multi-producer and multi-consumer stress tests preserve every unique item exactly once.
- ThreadSanitizer or an equivalent race detector is clean when available.
- Silo documentation no longer claims a stronger concurrency model than implemented.

## 6. Phase 2: Zephyr Shared-Memory Ring Integrity

### 6.1 Define the transport contract

Document whether `ShmRingBuffer` is:

- an in-process emulator protected by `&mut self`,
- a true shared-memory single-producer/single-consumer structure, or
- a wire-format model copied to and from external shared memory.

The implementation and names must match that contract. A compiler fence is not an
inter-thread or inter-process synchronization mechanism.

### 6.2 Represent ring state without ambiguity

Use monotonic read and write sequence counters, or reserve one slot so `read == write`
means only empty. Before writing, compute available capacity from a validated snapshot
of the consumer position. Return an explicit full result without modifying storage.

For a shared SPSC implementation:

- Producer writes packet bytes, then publishes the new write counter with Release.
- Consumer reads the write counter with Acquire before reading packet bytes.
- Consumer publishes the new read counter with Release.
- Producer reads the read counter with Acquire before reusing space.
- Shared metadata uses representations valid for the mapped ABI and target platforms.

Do not rely on `atomic::fence` around ordinary non-atomic offset fields.

### 6.3 Validate packets before advancing

- Validate magic, payload length, and checksum before committing the read position.
- Define corrupt-packet recovery: leave the packet pending, skip one fixed packet slot,
  or reset the ring. Test the selected policy.
- Use checked arithmetic for offsets and capacities.
- Keep payload copying bounded to `SHM_PAYLOAD_CAPACITY`.

Acceptance criteria:

- Full rings never overwrite unread packets.
- Thousands of wraps preserve sequence order and payload contents.
- Corrupt packets follow the documented recovery rule.
- Tests cover capacities that are and are not exact multiples of packet size.

## 7. Phase 3: Flux and Shard Ownership

### 7.1 Remove `Box::leak` from JSON parsing

`FieldImp::Str` cannot safely accept parser-owned temporary strings as an arbitrary
long-lived `&str`. Resolve this at the data contract rather than hiding ownership with
a leak.

Preferred direction:

- Use `FieldImp::String` for owned destinations.
- Limit `FieldImp::Str` imports to inputs whose owner outlives the destination, or reject
  them with a structured import error.
- For enum-like string imports, pass a temporary borrowed string only through a call
  whose sink consumes it synchronously and cannot store the reference.
- Express that rule in trait lifetimes rather than pointer casts.

Add repeated-parse tests with an allocation counter to prove memory use remains bounded.

### 7.2 Implement `USeg` object import without exposing fields

Keep `_First` and `_Last` private. Implement a small private import adapter that:

- accepts `_First` and `_Last` in either order,
- rejects duplicates, missing fields, unknown fields, and numeric overflow,
- constructs `USeg::New(first, last)` only after validation,
- preserves the established empty-segment representation.

Do not use raw self-pointers or publish mutation accessors solely for serialization.

### 7.3 Correct string dispatch

Ensure `Json::post_string` supplies the field variant expected by enum and record sinks.
Add round-trip tests for `KernelOp`, `PortDir`, and representative nested objects.

Acceptance criteria:

- Repeated JSON imports do not grow memory permanently.
- `USeg` round-trips for empty, singleton, normal, and maximum bounds.
- Unknown and malformed fields fail rather than silently succeeding.
- Existing Flux overflow tests continue to pass.

## 8. Phase 4: Karst Cycle Scheduling

### 8.1 Remove per-cycle thread creation

Do not spawn a scoped OS thread for each `step_cycle`. Evaluate these alternatives with
the Phase 0 benchmark:

1. Serial die stepping for single-fabric cycle simulation.
2. Persistent Heist workers for batches of cycles when dependency boundaries permit it.
3. Parallelize independent `KarstFabric` instances while retaining serial die steps.

Prefer the simplest alternative that improves measured throughput. The two dies exchange
signals every cycle, so multi-cycle batching must explicitly preserve inter-die link
dependencies.

### 8.2 Keep trace and statistics costs proportional

- Maintain running host transmit/receive totals if trace capture measurably spends time
  scanning unrelated components.
- Keep cycle tracing opt-in and bounded.
- Avoid changing observable trace ordering.

### 8.3 Consolidate VPU batch execution carefully

Replace temporary standard vectors with `Buff`/`Stash` if VPU batch allocation is
material. Reuse Heist or the established scoped executor instead of adding another
threading mechanism. Preserve channel ordering and affinity tests.

Acceptance criteria:

- Serial and configured parallel modes produce identical cycle traces.
- Backpressure, queue saturation, and fault propagation tests remain lossless.
- No thread is created by an individual `step_cycle` call.
- Measured throughput does not regress for one-worker operation.

## 9. Phase 5: Fascia Correctness and Lifecycle

### 9.1 Eliminate theme-menu leaks

Add a static-label method on `FasciaTheme` or a constant lookup table for active and
inactive labels. Remove `Box::leak` and dynamic formatting from `view_menubar`.

Acceptance criteria: repeated view construction performs no permanent label allocation.

### 9.2 Centralize tab-document cleanup

Before redesigning all tab storage, introduce one internal close path used by menu close,
tab-bar close, close-all, and application teardown. It must:

- cancel Python consoles,
- cancel geometry loads,
- remove editor, waveform, Cask, and geometry state,
- update the active tab and status consistently.

Add a regression test proving both close-all entry points dispose the same resources.

### 9.3 Correct cursor reporting and hit testing

- Read the editor's actual cursor position after performing an action.
- Hit-test Cask nodes in reverse draw order so nested children win over containing roots.
- Add overlapping and nested-node tests.

### 9.4 Optimize rendering only after measurement

For Cask labels, preserve measured dimensions between layout and atlas planning so text
is not shaped solely to recover the same size. Validate `Shaping::Basic` against the
actual supported character set before selecting it for ASCII names.

For waveforms:

- Decimate only when transitions exceed horizontal pixel resolution.
- Preserve the value entering and leaving each pixel column.
- Preserve unknown/high-impedance representation.
- Cache waveform geometry separately from the cursor overlay.
- Invalidate for data, bounds, time range, scale, and palette changes.

For Cask 2D:

- Cache background and static box geometry using complete invalidation keys.
- Store search matches in `Buff<u32>` or `Stash<u32>`.
- Recompute matches when the query or scene changes, not during every draw.

Acceptance criteria:

- Visual snapshot or geometry-command tests cover cache invalidation.
- Dense-waveform output preserves extrema and final values.
- Theme changes update every cached color.
- UI memory remains stable during a sustained interaction test.

## 10. Phase 6: Measured Runtime Improvements

### 10.1 Heist

Safe initial changes:

- Avoid locking destination queues when the temporary queue is empty.
- Coalesce duplicate job-slot reset and construction locking.
- Replace the per-operation `_State` mutex only if state is immutable after binding;
  use `OnceLock` or direct ownership with an explicit lifecycle proof.

Atomic ordering review:

- Document the value protected by each atomic.
- Identify the publishing store and observing load.
- Change one relationship at a time.
- Run dependency, work-stealing, required-placement, coroutine, reuse, and saturation
  stress tests after every ordering change.

Do not globally replace `SeqCst` with Acquire/Release in one patch.

### 10.2 Swarm and Symph backend contracts

Treat Symph as the owner of backend-neutral shader math and operation identity, and
Swarm as the owner of host dispatch, backend selection, buffers, compilation, and
execution. Preserve the existing direction of dependency: `swarm::ops` may use
`symph::StandardOp`; Symph must not depend on Swarm host types.

- Keep `StandardOp` and `StandardOpLabel` defined once in `symph::compute`.
- Add a single backend entry-point descriptor in Swarm so WGSL, SPIR-V, PTX, and CPU
  dispatch cannot acquire different spellings for the same operation.
- Keep backend source selection in `swarm::ops`; keep the numeric kernel bodies in
  Symph or their device crate.
- Add parity tests that run every standard operation through the CPU backend and compare
  supported GPU backends with explicit numeric tolerances.
- Validate buffer usage, byte length, workgroup dimensions, and backend compatibility
  before dispatch. Return `SwarmError` rather than allowing backend-specific failures to
  escape from the abstraction.
- Measure compile caching, upload count, and repeated dispatch before changing buffer or
  pipeline caching in `swarm::viewport`.
- Keep GPU-only validation conditional. A machine without a compatible adapter must
  report the skip and still run CPU contract tests.

Acceptance criteria:

- One authoritative operation-to-label mapping serves all backends.
- CPU results match the Symph reference implementation for all standard operations.
- Enabled GPU backends match CPU results within the operation's documented tolerance.
- Invalid dispatch dimensions and buffer shapes fail before backend execution.
- No new public mutable fields are introduced in Swarm or Symph types.

### 10.3 Rube

- Move VCD time-step values out of `_CurrentValues` rather than cloning, using an
  extraction method that leaves the stash valid and reusable.
- Retain deterministic timeline sorting because unordered and repeated timestamps are
  supported by current tests.
- Prototype SWAR flag advancement behind equivalence tests covering every two-bit flag
  combination and non-multiple-of-eight tails.
- Fuse settlement edge detection only after defining whether it observes pre-advance or
  post-advance state.
- Skip non-overlapping warps in parallel drive before invoking lane evaluation.

### 10.4 Fenst and filesystem exploration

- Implement bounded chunk reads with `File::open`, `Seek`, and `Read::take`.
- Use directory-entry type metadata where reliable, with a fallback for platforms that
  cannot supply it.
- Preserve deterministic directory/file ordering explicitly.
- Cache registered examples once if registration is immutable after startup.
- Replace quadratic sibling growth calculations only after a focused layout benchmark.

### 10.5 Fresco

Implement `VarCreate(..., reuse)` using a private sorted name index or a linear `USeg`
search appropriate to expected repository size. Reuse the existing variable attribute
index while creating a new `VarExpr` entry that refers to it. Do not return an expression
index where a variable-attribute index is required.

Add tests for repeated identifiers, mixed reuse flags, and expression serialization.

### 10.6 Python console and compute fallback

- Capture stdout and stderr separately; determine error state from stderr or the Python
  exception result, never by searching output text for `"Error:"`.
- Keep nested Python list getters for compatibility. Add flat buffer APIs only if they
  can avoid the copies implied by returning a new `Vec`.
- Document that Swarm's standard camera dispatch already prepares projection once per
  buffer. Optimize the generic Flock closure only if direct users exercise that fallback.

## 11. Phase 7: Modularization

### 11.1 Encapsulation audit

Classify each public struct before changing visibility:

- Passive transfer, trace, and statistics records may retain public fields when callers
  construct or inspect them as values and no invariant spans those fields.
- Functional structures with mutation methods or cross-field invariants must use private,
  underscore-prefixed fields and focused access methods.

Apply the second rule first to the confirmed stateful cases:

- Rube gates, latches, adders, trigger storage, fast warps, and coroutine warps.
- Karst link and channel state that can currently be mutated independently of the link
  protocol.

Do not add a one-for-one mutable accessor for every field. Add semantic operations such
as port lookup, signal update, channel observation, and link stepping. Return `Arr` or
specific values for inspection and reserve `MutArr` for APIs whose caller is meant to
replace elements without violating an invariant.

Migrate call sites in the same packet as each visibility change. Keep compatibility
methods only when an actual external caller requires them, and document their removal
path.

Acceptance criteria:

- `rg -n "pub [A-Za-z_][A-Za-z0-9_]*:"` over the affected modules has a documented
  classification for every remaining public field.
- No functional structure exposes direct mutation of invariant-bearing state.
- Record types are not burdened with boilerplate accessors solely to satisfy the audit.

### 11.2 Tab ownership

After lifecycle behavior is covered, evaluate a private `TabDocument` enum owned beside
each `TabItem`. Migration should be incremental:

1. Add the enum and move one document type.
2. Route open, update, view, close, and close-all through the same owner.
3. Move remaining types one at a time.
4. Remove old maps only when no compatibility call sites remain.

Do not expose document state fields publicly. Access state through focused methods.

### 11.3 Crew routing

Replace repeated route cloning and node lookup with a hub-owned resolved routing table.
Do not store mutual strong `Arc<CrewNode>` references inside nodes. Use hub ownership or
`Weak<CrewNode>` snapshots to avoid cycles.

Define multicast semantics before changing delivery:

- Atomic multicast requires reserving capacity for all peers before committing.
- Best-effort multicast must report partial delivery explicitly.

Copy the optional callback handle while holding its lock, release the lock, then invoke
the callback. Add competing-sender tests to cover the gap between readiness checks and
delivery.

### 11.4 API compaction

Remove duplicate snake-case forwarding methods only after checking external users and
tests. Preserve standard Rust trait method names. Make these changes per module so
breakage is reviewable.

### 11.5 Shader contracts

Move shared entry-point names and ABI constants into one lightweight module usable by
host and device crates. Keep renderer implementation details in their owning modules.
Validate names at compile time or in artifact tests.

## 12. Implementation Packets and Dependencies

Use the following packets as the unit of coding, review, and rollback. A packet begins
only after its dependency has landed and its entry tests pass.

| Packet | Primary files | Depends on | Deliverable |
|---|---|---|---|
| P0 | owning `_tests.rs`, `examples/rube_perf.rs` | None | Baseline report and focused regression harnesses |
| P1 | `silo/buff.rs`, `silo/stash.rs`, `silo/_tests.rs`, Natvis | P0 | Initialized storage representation |
| P2 | `silo/stk.rs`, `silo/_tests.rs` | P1 | Documented and tested stack protocol |
| P3 | `zephyr/shm.rs`, `zephyr/_test/mod.rs` | P0 | Lossless ring protocol |
| P4 | `flux/fluximport.rs`, Shard JSON files, `silo/useg.rs`, tests | P1 | Leak-free imports with private `USeg` state |
| P5 | `karst/fabric.rs`, `karst/vpu.rs`, `karst/_tests.rs` | P0, P2 if Stk is used | Cycle scheduling without per-cycle threads |
| P6 | `symph/compute.rs`, `swarm/ops.rs`, backend and test files | P0 | Unified operation and backend contract |
| P7 | `rube/vcdio.rs`, `rube/trigger.rs`, `rube/engine.rs`, tests | P1, P2 | Measured Rube changes and parity proof |
| P8 | Rube/Karst functional types and their callers | P5, P7 | Private invariant-bearing state |
| P9 | Fascia state, menu, tabs, Cask and waveform files | P0 | Lifecycle fixes followed by measured rendering work |
| P10 | Crew/Fenst/Fresco/Heist files named in Phase 6 and 7 | Relevant safety packets | Independent, benchmark-supported cleanup |

For each packet, record a before measurement or failing regression, implement the
smallest change, run focused verification, run the full gates, and inspect the final
diff for unrelated formatting. If a performance packet does not show a repeatable gain,
drop it rather than widening its scope.

## 13. Verification Matrix

Every phase runs its focused tests plus:

```text
cargo check --all-targets --offline
cargo clippy --offline -- -D warnings
cargo test --lib --offline
cargo run --offline -- -t
```

Additional gates:

| Area | Required validation |
|---|---|
| Silo | Drop-count, panic-unwind, stress, Miri where supported |
| Zephyr ring | Full/wrap/corruption tests and cross-thread/process harness as applicable |
| Flux/Shard | Round-trip, malformed input, bounded-allocation test |
| Karst | Serial/parallel trace parity, saturation and fault tests, benchmark |
| Fascia | State lifecycle tests, render-command/cache invalidation tests, memory soak |
| Heist | Work stealing, placement, coroutine, reuse, saturation stress |
| Rube | Serial/parallel parity, four-state flags, VCD unordered-time tests |
| Swarm/Symph | CPU reference parity, backend contract failures, GPU parity where available |

Performance thresholds must be written into the packet after Phase 0 captures variance.
Use a threshold larger than observed run-to-run noise, report the sample count and
dispersion, and reject improvements that trade away correctness or increase steady-state
allocation without an explicit reason.

Hardware-dependent GPU and Renode checks should run in their configured CI or local
environment before release. Their absence must be reported; a skipped test is not a
hardware validation pass.

## 14. Change Packaging and Review Order

Keep patches small enough to review independently:

1. Baseline tests and benchmarks.
2. `Buff`/`Stash` initialized-storage representation.
3. `Stk` replacement and documentation.
4. Zephyr ring protocol.
5. JSON ownership and `USeg` import.
6. Karst scheduling.
7. Fascia correctness fixes.
8. Swarm/Symph backend contract consolidation.
9. Rube measured improvements.
10. Stateful Rube/Karst encapsulation.
11. Fascia measured rendering improvements.
12. Heist measured lock reduction.
13. Fenst/Fresco measured improvements.
14. Remaining modularization and API compaction.

Each review should state:

- the invariant or measured bottleneck being addressed,
- before/after behavior,
- tests and benchmarks run,
- remaining risks,
- whether any public API or binary layout changed.

## 15. Completion Criteria

The initiative is complete when:

- No safe Silo API can expose or drop uninitialized `T`.
- The stack's documented concurrency contract is proven by its implementation and tests.
- The shared-memory ring cannot overwrite unread packets and uses valid synchronization.
- JSON parsing performs no permanent string leaks and imports `USeg` correctly.
- Karst does not create an OS thread per simulated cycle.
- Fascia close-all, cursor, menu labels, and nested hit testing behave consistently.
- Retained performance changes show repeatable improvement without correctness regressions.
- Swarm backend dispatch and Symph reference operations share one tested contract.
- Core functional structures keep private state and borrowing access methods.
- The full offline build, Clippy, Rust tests, and registered test suite pass.

