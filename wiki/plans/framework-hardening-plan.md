# Framework Hardening and Optimization Plan

## Scope

This plan covers five core Trellis frameworks:

- `flux`: structured export/import and stream adapters.
- `shard`: grammar composition and text parsing.
- `heist`: DAG scheduling and work stealing.
- `crew`: virtual-machine co-simulation messaging.
- `karst`: host-to-memory-fabric simulation.

The work is ordered by dependency. Flux and Shard define data interchange; Heist is shared execution infrastructure; Crew and Karst depend on reliable queues and handshakes. Each phase must land with targeted tests before work begins on the next one.

## Principles

- Fix observable correctness defects before optimizing allocations or throughput.
- Preserve core API boundaries: Flux models structured data, Shard parses text, Heist executes jobs, and Crew/Karst model transport and state transitions.
- Use `Buff`, `Stash`, `Arr`, and `USeg` in public and hot-path APIs. Avoid new `Vec` or slice-based public APIs.
- Preserve private underscore-prefixed fields; add read-only accessors for observability instead of exposing storage.
- Make queue capacity and backpressure explicit. No producer may report success for a message that was dropped.
- Every parser-facing API must offer strict behavior. Recovery is permitted only as an explicit mode with collected diagnostics.

## Phase 0: Restore a Reliable Baseline

### Objective

Ensure the whole framework suite can be compiled and its registered tests can run before behavioral work begins.

### Tasks

1. Repair the currently reported unclosed delimiter in `src/rube/vcd_model.rs`.
2. Confirm each framework test module is correctly declared from its `mod.rs` file. In particular, reconcile `karst/mod.rs` with the existing `_tests.rs` naming convention.
3. Run the framework-specific Cove filters and record their test counts:

```text
cargo check --all-targets
cargo run -- -test Flux
cargo run -- -test Shard
cargo run -- -test Heist
cargo run -- -test Crew
cargo run -- -test Karst
```

4. Add a small test-registration smoke check if a module can compile while unintentionally omitting its component tests.

### Acceptance Criteria

- `cargo check --all-targets` passes.
- Each framework filter executes at least one intended test.
- No framework test relies on an unrelated subsystem's compile failure or panic behavior.

## Phase 1: Flux Output and Stream Correctness

### Objective

Make Flux safe to use as the canonical structured interchange layer.

### Work Package 1.1: Repair Streaming `OutStream`

`OutStream::from(W)` currently constructs a streaming writer with a zero-length `Buff`. The streaming `Write` implementation cannot advance with a zero-sized cache.

1. Introduce a private nonzero cache capacity constant, such as `K_OUTSTREAM_CACHE_BYTES`.
2. Allocate the cache in `impl From<W> for OutStream` using `Buff::WithCapacity` or `Buff::FromDispenser`.
3. Ensure `write()` handles writes larger than the cache by flushing completed cache contents and continuing from the remainder.
4. Ensure `flush()` writes only initialized bytes, resets the marker after success, and forwards `inner.flush()` errors.
5. Do not depend on `Drop` for error reporting. Keep `Drop` best-effort, but require explicit `flush()` from file-producing callers that need errors.

### Work Package 1.2: Produce Valid JSON

1. Add a private JSON string-emission helper used for both keys and `FieldExp::{Str, String}` values.
2. Escape quotation marks, reverse solidus, control characters, and the JSON short escapes (`\b`, `\f`, `\n`, `\r`, `\t`). Encode other control bytes as `\u00XX`.
3. Emit `FieldExp::Null` as the JSON literal `null`.
4. Emit non-finite floats as `null`, not as the string `"null"`.
5. Preserve the current Flux event model: `JsonOutStream` remains an `IFluxExportSink` and callers continue dispatching `FieldExp::FluxSource`.

### Work Package 1.3: Strengthen Flux Import Semantics

1. Change `FieldImp::PostU64`, `PostF64`, `PostStr`, and `PostBool` to return `bool` or `Result<(), FluxError>`.
2. Propagate failures from `IFluxImportSink::FromFieldImp` instead of discarding them.
3. Reject lossy numeric conversions by default. Permit conversion only when an explicit target type validates range and integrality.
4. Add a small `FluxError` type with expected type, actual type, and field path when a caller provides one.

### Tests

- Streaming writes to `Cursor<Vec<u8>>`, including zero-length, one-byte, exact-cache, and larger-than-cache writes.
- JSON parseability through `Shard::Json` for escaped strings, control characters, null, arrays, and nested objects.
- Non-finite float handling.
- Sink rejection propagation and numeric overflow/fractional conversion rejection.

### Acceptance Criteria

- Flux output is valid JSON for every `FieldExp` variant.
- Streaming output cannot stall on an empty cache.
- Import failures are observable by the parser or caller.

## Phase 2: Shard Parser Safety and Strictness

### Objective

Make Shard a reliable parser substrate for JSON, VCD, and future EDA artifacts.

### Work Package 2.1: Remove Unsound Thread Claims

1. Remove `unsafe impl Send` and `unsafe impl Sync` from `Parser` unless `IStream` is redesigned with explicit thread-safety bounds.
2. Keep parsers thread-confined. Parallel parsing should create one independent parser and input stream per worker.
3. Add compile-time checks or documentation that grammar actions may mutate only their own parse context.

### Work Package 2.2: Define Parse Results

1. Add `ParseError` with byte offset, expected token/category, and a concise message.
2. Add `ParseMode::{Strict, Recovering}` where recovering mode is introduced only for formats that need it.
3. Provide a top-level helper that requires full input consumption. Retain low-level partial matching for combinators and token parsing.
4. Track the farthest failure marker and expected alternatives during branch attempts, rather than returning only `None`.

### Work Package 2.3: Make JSON RFC-Compatible

1. Decode JSON string escape sequences instead of passing raw escaped text to Flux fields.
2. Reject trailing commas in arrays and objects.
3. Reject leading `+` signs, leading zeroes, malformed exponents, and non-JSON numeric forms.
4. Reject unknown fields in strict object import mode when the Flux object callback returns `false`.
5. Define duplicate-key behavior. Default recommendation: reject duplicates in strict mode; emit a diagnostic and use last-value-wins only in recovery mode.
6. Make unsupported field types and failed `Post*` calls fail the enclosing parse.

### Work Package 2.4: Harden Repetition

1. Preserve the no-infinite-loop guard for zero-width child matches.
2. In debug builds, report a grammar construction or parse error when an unbounded repetition's child succeeds without advancing.
3. Test `*`, `+`, and `?` with zero-width children, nested choices, and rollback boundaries.

### Tests

- Full-document versus token-level parse behavior.
- Invalid JSON corpus covering strings, numbers, commas, nesting, duplicate keys, and unknown fields.
- Valid JSON corpus with escape decoding and nested Flux import.
- Zero-width repetition behavior and farthest-error diagnostics.

### Acceptance Criteria

- No parser is unsafely shareable by default.
- Strict JSON accepts valid documents and rejects the invalid corpus with locations.
- Format parsers can report actionable errors without creating format-specific parser infrastructure.

## Phase 3: Heist Job Lifecycle and Scheduler Guarantees

### Objective

Prevent job graph corruption, unbounded allocation waits, and scheduler ambiguity.

### Work Package 3.1: Reset Recycled Job Slots

1. Define an internal `ResetJobSlot(jobId)` that clears `_SuccIds[jobId]`, `_SzPreds[jobId]`, and the corresponding job storage before reuse.
2. Call it at a single, documented lifecycle point: either before `ConstructJob` publishes the new job or after completed-job reclamation.
3. Ensure a job with `succId == 0` cannot retain a predecessor's successor edge.
4. Assert that a slot's predecessor count is zero when it is returned to a free list.

### Work Package 3.2: Make Capacity Exhaustion Explicit

1. Replace the unbounded `AllocJob` spin loop with `TryAllocJob` returning `Option<u16>` or `Result<u16, ScheduleError>`.
2. Propagate failure through `ConstructJob`, `PostJob`, and chore-tree posting. Preserve a convenience infallible API only if it fails fast with a clear capacity message.
3. Add scheduler statistics: high-water allocated jobs, allocation failures, queue depth, steals attempted, and steals won.

### Work Package 3.3: Reduce Contention After Correctness

1. Keep local job caches, but measure global free-list lock contention and run-queue contention.
2. Batch transfers between the global free stash and local caches using the existing `Stash` APIs.
3. Consider a work-stealing deque only after profiling identifies run-queue locking as dominant. Do not replace the current queue solely for theoretical throughput.
4. Replace `SeqCst` with Acquire/Release only after a documented happens-before proof and stress tests; retain `SeqCst` on lifecycle transitions initially.

### Tests

- Reuse a slot first with a successor and then without one; verify no stale job runs.
- Exhaust capacity using a configurable small test-only capacity; verify non-hanging failure behavior.
- Fan-out/fan-in DAG execution with one and multiple workers.
- Repeated reset and launch cycles.
- Stress test posting child jobs during execution and verify completion without lost work.

### Acceptance Criteria

- No stale successor or predecessor state survives job reuse.
- Capacity exhaustion never spins indefinitely.
- Scheduler behavior is measured before contention-oriented redesign.

## Phase 4: Crew Delivery, Routing, and Backpressure

### Objective

Give the VM co-simulation protocol explicit delivery semantics.

### Work Package 4.1: Make Receive Enqueue Observable

1. Change `CrewNode::PushRx` to return `bool` indicating whether the byte entered the receive FIFO.
2. Do not increment delivered-byte counters or invoke callbacks when enqueue fails.
3. Add rejected-byte and queue-full counters to `NodeStats` if drops remain an allowed policy.
4. Choose a transport policy: recommended default is backpressure, not drop-on-full.

### Work Package 4.2: Implement Accurate Ready Status

1. Derive `STATUS_TX_READY` from availability of at least one configured, online, routable destination with queue capacity.
2. Define multicast behavior. Either require capacity on all selected peers before accepting a write, or return per-peer delivery results; do not silently partially deliver.
3. Return `CoSimAction::Error` with a documented code for rejected writes, unknown registers, unsupported widths, and unavailable peers.
4. Validate request action and register alignment/width before mutating node state.

### Work Package 4.3: Define Route Configuration Semantics

1. Replace the current ignore-on-duplicate `AddLink` behavior with explicit `ReplaceRoute`, `AppendRoute`, and `RemoveRoute` operations, or make `AddLink` merge deterministically.
2. Validate source and destination IDs during configuration.
3. Build route lookup around the expected access pattern. Start with the current compact `Stash` representation; introduce an indexed table only after node-count profiling supports it.

### Tests

- Fill an RX FIFO, attempt delivery, and verify a producer-visible failure with no false callback or counter increment.
- Unicast, multicast, offline peer, missing peer, and full peer behavior.
- Duplicate route configuration and deterministic destination order.
- Read/write width and alignment validation.
- Concurrent node requests using `Arc<CrewHub>`.

### Acceptance Criteria

- Crew never reports successful delivery of a dropped byte.
- Protocol status and responses reflect actual routing capacity.
- Route updates have documented, tested semantics.

## Phase 5: Karst Handshake Correctness and Simulation Fidelity

### Objective

Make each valid/ready interface lossless and observable under congestion.

### Work Package 5.1: Formalize Cycle Semantics

1. Document the sampling boundary for host, NoC, pipe, memory-channel, and inter-die signals.
2. Treat a transfer as occurring only when valid and ready are true in the same sampled cycle.
3. Keep source data stable while valid is asserted and ready is false.
4. Make every receive-ready output reflect destination capacity for the upcoming cycle.

### Work Package 5.2: Repair Host and NoC Backpressure

1. Make host response-ready depend on remaining RX FIFO capacity, including the case of multiple simultaneous incoming links.
2. Prevent `KarstHostNode::step` from accepting more responses than it can enqueue in a cycle.
3. Ensure NoC output retirement occurs only after the receiving endpoint advertises ready.
4. Return enqueue outcomes from internal queue operations or assert failures in simulation builds rather than ignoring them.
5. Model a response's routing destination independently from request address decoding, using the flit's source identity and explicit topology rules.

### Work Package 5.3: Validate Address and Data Contracts

1. Decide whether Karst memory is host-endian or wire-little-endian; replace `to_ne_bytes` and `from_ne_bytes` with the documented choice.
2. Validate alignment and address range before a flit is accepted by the fabric.
3. Surface memory access failures as response errors rather than substituting magic data such as `0xDEADBEEF` in normal execution.
4. Add latency configuration only after zero-loss, deterministic transport is verified.

### Work Package 5.4: Observability and Throughput Work

1. Add per-link counters for accepted transfers, stalls due to ready low, and queue-full events.
2. Add per-channel queue-depth high-water marks and request/response latency histograms.
3. Use these metrics to identify actual hot paths before changing queue structures or adding parallel fabric stepping.
4. Retain fixed-size arrays for the known topology; they are preferable to heap containers here.

### Tests

- Saturate host RX, NoC ingress, MC request, MC response, and inter-die queues.
- Verify no loss, duplication, or reordering for requests and responses under backpressure.
- Verify remote-die reads and writes from every host.
- Verify word alignment, boundary addresses, and byte order.
- Run deterministic multi-cycle traces and compare transfer counters against expected values.

### Acceptance Criteria

- A valid/ready transfer is never dropped or duplicated.
- Congestion produces stalls or explicit errors, never silent data loss.
- Memory data and routing remain correct for local and remote transactions.

## Phase 6: Dependency Cleanup and Performance Review

### Objective

Restore clear subsystem direction and optimize only measured bottlenecks.

### Tasks

1. Remove the `rube` VCD re-export from `shard/mod.rs`. `rube::vcd_shard` may depend on `shard`; `shard` must not depend on `rube`.
2. Keep format-specific models and Flux sinks in their owning subsystem.
3. Review public APIs for duplicate snake_case/PascalCase compatibility pairs. Retain aliases only where an external migration need exists; otherwise converge on the project convention.
4. Benchmark these workloads before changing data structures:

```text
- Flux: 1 MiB and 64 MiB streamed output.
- Shard: valid and invalid nested JSON/VCD documents.
- Heist: independent jobs, fan-in DAGs, and nested child posting.
- Crew: sustained byte routing with bounded peers.
- Karst: local and remote read/write streams under queue pressure.
```

5. Record allocations, queue depth, throughput, latency, and contention. Change an implementation only when a measurement identifies a bottleneck and the replacement retains the established API contract.

### Acceptance Criteria

- Dependency direction is `rube -> shard/flux`, never `shard -> rube`.
- Compatibility aliases have an owner and removal criterion.
- Performance changes include before/after measurements and do not weaken correctness tests.

## Merge Sequence

Keep pull requests small and independently releasable:

1. Baseline compile/test registration repair.
2. Flux `OutStream` cache repair and JSON correctness tests.
3. Flux import error propagation.
4. Shard parser safety and strict JSON diagnostics.
5. Heist slot-reset and capacity-error behavior.
6. Crew queue result and protocol backpressure.
7. Karst handshake and memory-contract fixes.
8. Dependency cleanup and benchmark-driven optimizations.

Every pull request must run:

```text
cargo check --all-targets
cargo test
cargo run -- -test <affected-framework>
cargo clippy -- -D warnings
```

Run `cargo run -- -test` before merging a phase. Add a regression test in the affected framework's `_tests.rs` for every corrected defect.
