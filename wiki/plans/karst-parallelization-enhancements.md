# Karst parallelization: fresh assessment and enhancement plan

Assessed 2026-09-21 against commit `50d3465`. This is the current forward plan;
the [earlier plan](karst-parallelization-plan.md) remains a historical record.
Scope: Karst, Heist, Flock, Swarm, Silo, Rube, Symph, and Drove. This assessment
changes documentation only. Findings below distinguish inspected behavior from
performance hypotheses; no speedup or zero-regression claim has been measured.

## Assessment

The implementation has useful ownership and transport foundations, but it is
not yet an efficient work-stealing Karst executor. Independent fabrics can run
on scoped threads. Three standard Swarm operations have an opt-in scoped path.
Those paths neither reuse persistent workers nor run through Heist's stealing
queues. Two partition algorithms need correction before extending them.

Recommended order: repair partition and view contracts; establish serial costs
and parity; introduce reusable scoped scheduling; complete independent batches
and channel batches; evaluate two-die scheduling and GPU compute only afterward.
Rube's unsafe parallel evaluator requires its own repair and must not become
the execution foundation for Karst in its present form.

## Current capability

| Area | Observed behavior at the reviewed commit |
| --- | --- |
| Transport | Backpressure/address/fault regressions, bounded traces, link counters, and queue high-water metrics exist. |
| One fabric | `step_cycle` samples inputs, steps die 0 then die 1, records metrics, and advances time serially. |
| Independent fabrics | `AdvanceIndependent` constructs an Atelier for each call and uses `ForEachScopedMut`. |
| Swarm CPU | Default dispatch is serial. Explicit `DispatchScoped` supports Double, Collatz, and VectorAdd; PointCloud and CameraTransform remain serial. |
| Karst VPU | `Vpu::dispatch` calls ordinary `ComputeDevice::Dispatch`; scoped Swarm execution is not connected to this path. |
| Heist | Legacy queued jobs can steal. New scoped methods spawn OS threads and invoke callbacks directly, outside the maestro queues and their metrics. |
| Rube | Parallel `Drive` resets the global Atelier, clones warps, and reconstructs overlapping mutable references to the trigger bank. |
| Symph / Drove | Shared operation identities and math helpers exist. Drove exposes Double SPIR-V; the other compute artifacts return `None`. Karst has no GPU compute/readback integration. |

## Findings

Priorities: P0 is a safety contract blocker; P1 is correctness or a central
scaling blocker; P2 is optimization or missing evidence. Source links identify
the reviewed files; symbol names are the stable anchors.

| ID | Priority | Evidence and consequence | Required enhancement |
| --- | --- | --- | --- |
| F1 | P1 | [Heist](../../src/heist/atelier.rs), `ForEachScopedRange`: ceiling-sized chunks can produce `start > end`. For total=9, workers=8, the last callback receives `(14,9)`. Large totals also expose unchecked multiplication/addition. | Use bounded quotient/remainder ranges with checked invariants and no empty tasks when workers <= elements. |
| F2 | P1 | `ForEachScopedMut` and [Flock](../../src/flock/kernel.rs), `SplitForWorkers`, repeatedly split only the last partition. Eight elements/eight workers produce lengths `4,2,1,1,0,0,0,0`; 96/3 gives `48,24,24`. | Share balanced partition arithmetic and preserve global offsets. Empty work must not count as productive worker participation. |
| F3 | P1 | Scoped callbacks bypass `ExecuteLoop`, maestro queues, and steal counters. A worker mask proves callback invocation, not balanced useful work or stealing. | Separate static partition execution from queued work stealing in APIs, diagnostics, and tests. Implement scoped scheduling with more ready tasks than workers for irregular workloads. |
| F4 | P1 | [Swarm](../../src/swarm/cpu.rs), `DispatchScoped`, and [Karst](../../src/karst/fabric.rs), `AdvanceIndependent`, each call `Atelier::New`. `AtelierState::New` initializes 65,536-entry job structures plus per-maestro state, even though scoped execution bypasses them. Threads also spawn/join per call. | Reuse a caller-owned executor; validate work before allocation; normalize/cap budgets before creating maestros. Add a direct one-worker/small-work path. |
| F5 | P0 | [Silo](../../src/silo/arr.rs) still exposes safe raw-pointer `Arr::New`, generic casts constrained only by `Copy`, and `MutArr::SwapAt(&self)` alongside `Sync`. These allow invalid view construction or mutation through a shared view; move-only partition wrappers alone cannot establish general API soundness. | Audit the view surface used by workers: unsafe raw constructors, exclusive mutation, valid bit-pattern/alignment contracts, and constrained casts. Review Send/Sync and callers together. No claim that existing sealed f32/u32 calls necessarily trigger every defect. |
| F6 | P0 | [Rube](../../src/rube/engine.rs), `Drive` / `EvalWarpLanes`, creates `&mut TriggerWad` in multiple jobs. Jobs read current flag bits and write future bits in the same flag bytes. Distinct output IDs do not prevent overlapping bank references or flag races. | Immutable current-state snapshot plus exclusive future results and a commit barrier; remove integer-erased mutable bank pointers and global resets. |
| F7 | P1 | Independent-fabric coverage is three fabrics, one write each, 32 ticks, three workers. It checks aggregate metrics and expected words, not traces, responses, skewed work, or all worker budgets. VPU coverage checks that a repeated-byte buffer changed, not exact f32 results. | Add cycle/state/response equivalence and typed numeric VPU oracles before accepting the parallel policy. |
| F8 | P2 | Serial `step_cycle` always records link and queue metrics; construction always allocates and initializes 1,024 trace records even with tracing disabled. Scoped entry points also add substantial setup costs for one worker. | Measure and separate essential model statistics from optional diagnostics; allocate trace storage on demand; retain an inexpensive serial path. |
| F9 | P1 | Buffer locks give exclusive execution, but do not define modeled ordering between host accesses and VPU work. Public channel access permits outside clients; there is no channel batch/commit contract. | Define launch/snapshot/completion visibility at explicit simulation boundaries, with deterministic handling of conflicting channels and failures. |
| F10 | P2 | Symph describes operation identities but lacks a complete shared binding/layout/numeric contract. CPU source-name heuristics can fall back to Double. CameraTransform recomputes a point transform per output scalar. | Centralize operation specifications, reject unknown sources, define byte/float semantics, and make record-oriented kernels compute each record once. |

The F1/F2 examples were reproduced by evaluating the current formulas in a
read-only PowerShell probe. They are arithmetic demonstrations, not new Rust
regression tests. The current Rust tests do not exercise those boundary cases.

Additional ownership detail: CPU buffers allocate `Buff<u8>` but expose typed
views. Alignment is asserted during casting, not guaranteed by the buffer's
declared allocation layout. Double checks buffer size before acquiring its
operation-wide mutable guard, while `Write` can grow the buffer. Move validation
under the same guard and specify aligned storage or explicit byte decoding.

## Enhancement sequence and acceptance gates

Scheduling direction selected after this assessment: see the
[precedence and chore-affinity implementation plan](karst-affinity-precedence-plan.md).
It refines E3/E5/E6 around persistent workers, explicit required/preferred
placement, and predecessor completion. State-access correctness belongs to the
system designer; production scheduling does not perform per-access validation.
Its nested-execution rules supersede the unconditional inline fallback proposed
below, because required affinity can make that fallback invalid.

### E1 — Correct and consolidate partition contracts

Owners: Heist, Flock, Silo. Dependencies: none. Address F1, F2, and the worker
view surface in F5 before adding further parallel consumers.

- Normalize workers to `min(max(requested,1), total)` for nonempty work.
  Compute `q = total / workers`, `r = total % workers`,
  `length(i) = q + (i < r)`, `start(i) = i*q + min(i,r)`.
  This avoids ceiling-chunk overshoot and keeps sizes within one element.
- Use one partition policy for ranges and moved mutable spans. Keep Flock's
  global index metadata while removing duplicate tail-split algorithms.
- Make empty work a no-op; specify worker ID semantics independently of which
  task it receives. Preserve project Arr/MutArr/USeg APIs.
- Audit safe view constructors, casts, shared mutation, and Send/Sync. Use
  compile-fail ownership cases and memory-safety tooling where supported.

Gate: totals 0,1,2,3,7,8,9,63,64,65,96,192 crossed with workers
0,1,2,3,4,8 and workers greater than total; exact coverage, no overlaps,
valid bounds, balanced nonempty chunks, correct global indices. Exercise
u32-limit arithmetic without allocating u32-sized buffers. Prove all borrowed
work completes before scope return, including unwind paths.

### E2 — Establish parity and serial-performance baselines

Owners: Karst and benchmark/test harness. Dependencies: E1 for new partition
coverage; existing serial measurements can begin immediately.

- Add a non-counting memory snapshot for verification. `read_word` increments
  simulated read statistics, so using it only on one side corrupts comparisons.
- Compare every captured cycle, response/fault identity and order where
  guaranteed, final channel bytes, queue/link counters, and modeled cycles.
- Include both dies, all channels, saturated queues, delayed response draining,
  explicit wrong-die forwarding, injected faults, and repeatable mixed traffic.
- Separate Off/Counters/Trace diagnostics, preserving documented model
  statistics. Make trace allocation lazy and test enable/clear/overflow behavior.
- Record release baseline costs for fabric construction, ordinary serial
  stepping/dispatch, scheduler construction, task submission, and join.

Gate: repeatable serial reference; worker matrix 1/2/3/4/8 agrees cycle by cycle
for independent batches of sizes 0,1,2,3,7,8,9 and larger batches with skewed
traffic. State the captured trace limit and validate longer runs through
bounded windows or response/state checks. No unexplained serial regression.

### E3 — Reusable caller-owned Heist execution

Owners: Heist; Flock/Swarm/Karst as consumers. Dependencies: E1.

- Add explicit executor-taking entry points, conceptually `DispatchWithExecutor`
  and `AdvanceIndependentWithExecutor`; retain existing convenience entry
  points with a documented setup cost and direct serial fallback.
- First reuse scheduler state; then implement persistent workers with a reviewed
  lifetime/completion design for borrowed tasks. Never merely erase borrowing
  lifetimes into the existing WorkPtr queue.
- Use balanced static chunks for uniform memory work. For variable fabric or
  Collatz work, permit bounded smaller tasks and actual work stealing.
- Define queue-full behavior (checked error or synchronous fallback), panic
  propagation, join/drain, shutdown, concurrent callers, and nested execution.
  Initially run nested tasks inline to respect the shared worker budget.
- Record productive tasks, elements processed, steals, queue depth, and wait
  time for the actual scoped execution path. Budget thread count across clients.

Gate: no scheduler reconstruction/thread creation per warm call; no lost work
at capacity; panic and nested-call tests finish under a timeout; repeated and
concurrent clients remain isolated. Retain `Atelier::Reset(3)` as a legacy
stealing regression and add a separate three-worker scoped-queue test with
deterministic ready-work coordination and non-main steal evidence.

### E4 — Complete CPU and Rube ownership integration

Owners: Swarm, Flock, Symph, Rube. Dependencies: E1/E3; Rube is a parallel
workstream rather than a prerequisite for independent Karst simulation.

- Make Symph specify bindings, types, valid dimensions, zero/partial work,
  aliases, numeric overflow/float behavior, and CPU/GPU byte layout.
- Retain operation-wide canonical buffer locking. Validate sizes and types
  under those guards; reject unsupported kernel/backend combinations early.
- Add scoped PointCloud and CameraTransform using complete record partitions;
  compute a camera transform once per point and reuse camera constants.
- Repair Rube with immutable current values/flags, exclusive next-state output,
  validated output ownership, and deterministic commit. Include coroutine
  state ownership and remove per-Drive global Reset/whole-warp clones.

Gate: all five operations agree with serial for partial groups, short buffers,
zero work, malformed dimensions, aliases, and edge numeric inputs. Rube serial
and parallel fast/coroutine/four-state circuits agree; overlapping mutable bank
references and shared current/future flag writes are absent from worker code.

### E5 — Karst independent and exclusive-channel batches

Owner: Karst. Dependencies: E2/E3 and applicable E4 contracts.

- Schedule independent fabrics through the reusable executor, preserving
  sequential cycles within each instance; expose batch-level worker policy.
- Add explicit VPU batch descriptors and per-dispatch results. Validate every
  channel/operation/dimension before launch; reject or deterministically order
  duplicate target channels. Account for successful dispatches only.
- Start with VPU batches between Advance calls, under exclusive fabric access.
  Define host-versus-VPU memory visibility before allowing same-cycle overlap;
  a buffer mutex alone does not establish deterministic simulation order.
- Prefer parallelism across channels for the current small channel buffers.
  Avoid multiplying outer fabric workers by inner Swarm workers.

Gate: exact typed outputs, deterministic host/VPU interactions, error-path
accounting, both-die/all-channel coverage, and worker matrix parity. Benchmark
channel batches against serial channels and within-kernel partitioning.

### E6 — Optional two-die evaluation

Owner: Karst. Dependencies: E2/E3/E5. Treat as an experiment, not a default.

- Preserve `prepare_cycle_inputs` as a sampled immutable boundary. Give each
  die exclusive state for evaluation, join both, then publish/record the cycle.
- Test inter-die forwarding, stalls, faults, and host/VPU visibility explicitly.
- Compare persistent-worker synchronization cost with sequential die work.
  There are only two dies: benefits are intrinsically limited at this level.

Gate: cycle-by-cycle equivalence and repeatable end-to-end release improvement.
If barriers dominate, retain serial die stepping and use outer batches instead.

### E7 — Drove-backed VPU execution

Owners: Symph, Drove, Swarm, Karst. Dependencies: E4/E5 and the existing GPU
migration plan's adapter/ownership gates.

- Implement the Swarm GPU backend using associated resource types and explicit
  upload/dispatch/completion/readback boundaries. Start with Drove Double.
- Keep channels resident where useful; batch GPU submissions; define visibility
  before host/model reads. Report unsupported operations rather than silently
  substituting a CPU/source-name fallback.
- Add hardware-tagged tests that distinguish unavailable/skipped hardware from
  actual readback validation. CPU parity does not validate GPU synchronization.

Gate: adapter-backed numeric parity and complete transfer/synchronization
timings. Enable GPU policy only above a measured end-to-end crossover.

## Performance policy and non-parallel workloads

Default serial dispatch currently avoids thread launch, but that alone does
not establish unchanged performance. Always-on diagnostics, eager trace memory,
buffer locks, and per-scalar CameraTransform work need measurement. A one-worker
call to the current batch/scoped entry point also constructs a full Atelier.

Proposed release matrix:

| Dimension | Cases |
| --- | --- |
| Execution | Direct serial, current scoped baseline, reusable static scope, scoped stealing; GPU and two-die only when available |
| Workers | 1,2,3,4,8; clamp to useful work and record available CPU capacity |
| Fabric traffic | Idle, uniform writes/reads, saturated responses, skewed channels, fault traffic, mixed VPU/host |
| Granularity | One fabric and small/large independent batches; one/eight VPU channels; tiny through large kernel buffers |
| Lifecycle | Cold construction and warm repeated execution measured separately |
| Diagnostics | Off, counters, bounded trace |

Report cycles/second, transactions/second, p50/p95 wall time, allocation count,
peak memory, useful work per worker, scheduling/join time, and steals. Keep
modeled cycles/transaction and modeled latency separate from wall-clock speed.
GPU results must include transfers and synchronization. Record revision,
toolchain/profile, machine, warmup, repetitions, and workload seed.

Proposed acceptance policy: investigate repeatable serial regressions above 3%
once measurement noise is below that threshold; use additional repetitions if
it is not. Require at least 10% repeatable end-to-end improvement before an
automatic parallel policy chooses a path. These are proposed gates, not current
results. Keep explicit Serial and diagnostic policy controls. Measure thresholds
per workload class; do not infer them solely from worker count.

## Verification performed for this assessment

At `50d3465`, ran `cargo test -p trellis --lib <module>:: --offline --
--test-threads=1` for Karst (27), Heist (16), Flock (2), Swarm (21), Rube (16),
and Symph (7): **89 tests passed**. Harness serialization does not disable worker
threads created inside tests. These passing tests do not prove view soundness,
partition balance, scoped stealing, hardware GPU execution, or speedup.

Also inspected ownership, call paths, trace/metric allocation, Rube flags, and
Drove artifact coverage, and reproduced F1/F2 arithmetic without source edits.
No release benchmark, GPU readback, sanitizer, Miri, or new regression test was
run. Next implementation checkpoint is E1 plus E2's baseline/parity harness;
subsequent phases should close their gates before claiming completion.
