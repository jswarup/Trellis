# Karst implementation plan: precedence and chore affinity

Design basis: discussion following the assessment of commit `50d3465`.
This is the selected scheduling direction and refines E3/E5/E6 in the
[enhancement plan](karst-parallelization-enhancements.md). Other correctness
findings in that assessment remain applicable. This document proposes the
implementation; no scheduler changes have been made as part of this plan.

## Contract and responsibilities

The system designer encodes all required predecessor relationships, valid
state access, and task lifetimes. Heist runs a chore only after all its
predecessors have completed, publishes their results to the successor, and
honors the chore's placement policy. The runtime does not inspect pointers,
track memory accesses, or perform per-access ownership checks.

Use graph-construction validation for structural errors and debug/test tooling
for designer contracts. Rust access and lifetime invariants still apply:
low-level shared-state access needs a documented unsafe boundary where types
cannot express the graph's guarantees. Affinity does not authorize overlapping
mutable Rust references. Input/output metadata is optional documentation or
debug tooling, not mandatory production scheduling metadata.

Precedence defines a partial order. Independent ready chores may execute in
either order; any state-dependent ordering must be expressed by edges. Chores
on the same worker are serialized but have no implied semantic order beyond
their edges. Completion means the chore's promised work has finished, including
any explicitly joined children or asynchronous device operation.

## Placement policy

| Policy | Execution rule | Stealing |
| --- | --- | --- |
| `Any` | Any executor worker may run the ready chore. | Allowed. |
| `Prefer(worker)` | Enqueue on the designated worker for locality. | Allowed when another worker is idle. |
| `Require(worker)` | Run only on the designated persistent worker thread. | Never allowed. |

Use `Require` for the requested same-thread guarantee. `Prefer` is useful when
locality is desirable but migration is permitted. A busy required worker makes
the chore wait; an unavailable required worker fails the affected run rather
than silently migrating it. Worker identity is stable for the executor lifetime.
Resizing or changing required assignments requires draining runs and explicit
reconfiguration. Optional OS CPU pinning is a separate later optimization:
logical thread identity does not imply execution on the same processor core.

## Existing implementation and necessary changes

| Location | Existing mechanism | Change |
| --- | --- | --- |
| `heist/atelier.rs` | `_SzPreds` and `SetSucc` encode dependencies. `ExecuteLoop` directly continues to a successor after the last predecessor. | Retain dependency accounting; route every newly ready successor through affinity-aware placement. |
| `heist/maestro.rs` | Per-maestro run/temp queues; thieves use `PopJob`. | Separate required work from stealable work and add owner-directed submission/wakeup. |
| `heist/atelier.rs` | `DoLaunch` spawns workers; maestro 0 runs on the calling thread. | Give the affinity executor persistent worker threads, including worker 0. Arbitrary callers submit and join. |
| `heist/choretree.rs` | `Then`/`Par` lower to jobs and fan-out helper chores; a job stores one successor. | Add placement to leaf chores and compile reusable graph metadata. Preserve composition semantics and explicit fan-out lowering. |
| `stalks/work.rs` | `WorkPtr` consumes a boxed `FnOnce`. | Keep one-shot jobs; use a separate reusable chore-body contract for compiled graphs. Do not replay consumed closures. |
| Karst / Swarm | New scoped paths create Atelier instances and bypass maestro queues. | Add executor-taking graph execution paths and reuse initialized state. |

Paths in this table are relative to `src/`. One-successor storage is sufficient
for the existing structured lowering when fan-out is represented by helper
chores. General adjacency storage should be introduced only if required by
concrete graphs; do not implement fan-out by repeatedly overwriting `SetSucc`.

## Phase A — Metadata and graph compilation

Files: `src/heist/choretree.rs`, `src/heist/atelier.rs`, component tests.

- Add private placement metadata with `Any` as the compatibility default.
  Provide explicit preferred/required-worker constructors or builders.
- Compile `Then`/`Par` into an immutable `ChoreGraph`: callable descriptors,
  initial predecessor counts, successor/fan-out encoding, roots, terminals,
  and placement. Use project Buff/Arr/USeg representations.
- Validate worker IDs, graph edges, cycles where arbitrary edges are accepted,
  predecessor-count overflow, and job capacity before making roots runnable.
  Repeated edge wiring must be rejected or have explicit semantics.
- Separate compiled metadata from per-run counters/status. Reuse a bounded run
  arena; allow only one active run of a stateful graph initially. Reset runtime
  counters only after all work from the previous run has drained.
- Preserve existing one-shot chore APIs. Reusable bodies must explicitly supply
  replayable behavior and stable state, not depend on FnOnce captures.

Gate: existing composition tests pass; chain, fork, join, diamond, zero-work,
invalid-worker, and capacity cases produce correct graphs or checked errors.
Graph construction performs all structural validation before execution.

## Phase B — Persistent workers and executor lifetime

Files: `src/heist/atelier.rs`, `src/heist/maestro.rs`, `src/heist/_tests.rs`.

- Add a caller-owned executor mode with W persistent workers, numbered 0..W-1,
  and bounded preallocated graph/run storage. Keep legacy launch behavior behind
  its compatibility API until consumers migrate.
- Bind every worker ID to one OS thread. The submitting caller does not execute
  required chores, including worker 0 chores. Explicit serial mode executes on
  the caller and is a separate contract.
- Park idle workers and wake them through a queue predicate checked with the
  same synchronization protocol; prevent lost wakeups. Retain bounded spinning
  only where measurements justify it.
- Separate pending-run completion from temporary ready-queue emptiness. A graph
  with blocked successors is not complete merely because queues are empty.
- Preallocate metadata once, park between runs, and join threads at shutdown.
  Do not use global `Reset` for per-operation execution.

Gate: thread IDs remain identical across repeated runs and different submitting
threads; idle workers wake reliably; shutdown drains/joins; warm runs allocate
no scheduler state and create no threads.

## Phase C — Readiness, publication, and affinity-aware queues

Files: `src/heist/atelier.rs`, `src/heist/maestro.rs`.

- Complete each chore exactly once, then decrement each encoded successor's
  remaining-predecessor count. Only the transition from one to zero makes a
  successor eligible. Construct the entire run before publishing roots.
- Preserve a happens-before chain from every predecessor's output writes to
  the successor's reads. Begin with the existing conservative atomic ordering;
  document and test join-counter and queue publication ordering before reducing
  it. This is completion/queue synchronization, not per-data-access checking.
- Keep required chores in an owner-only ready queue with remote submission;
  thieves cannot remove them. Put `Any` and `Prefer` in stealable queues.
  Start with existing lock-based queue primitives; optimize only after profiling.
- Route newly ready successors according to their own placement. Retain direct
  continuation only when the current worker is eligible; apply the same rule
  to roots, fan-out helpers, spawned children, and coroutine resumptions.
- Interleave owner-required and stealable work with a bounded fairness policy
  so one class cannot starve the other. Steal only ready, migration-permitted
  tasks; queued work is never duplicated by fallback handling.

Gate: required chores never migrate under stealing pressure; preferred/any
chores can steal. A join observes writes from every predecessor and runs once.
Cross-worker successors are routed correctly even when the last predecessor
finishes on a different worker. Required queue occupancy cannot hide stealable
work from idle workers.

## Phase D — Failure, lifetimes, and repeated execution

- Initial compiled runs use executor-owned state or captures with explicit
  stable lifetimes. A later borrowed `RunScoped` must drain on both normal return
  and unwinding; it cannot erase references into an unrestricted static queue.
- Treat a Rust panic as run failure, drain executing tasks, and suppress normal
  dependent execution. Signal the waiting caller; do not leave predecessor
  counters waiting forever. Do not retry chores with partial side effects.
- Reserve admission capacity before launch. If capacity is exhausted, return a
  checked error before publishing the run. Dynamic insertion requires its own
  reservation/completion protocol and is deferred from the first version.
- Reject blocking nested graph launches from worker callbacks initially.
  Later support scheduler-aware joins if required. Inline fallback is invalid
  when a child requires a different worker; blocking its required worker can
  deadlock. Express such work as continuations and predecessor edges.
- Prevent stale completions from touching a reused run arena: drain before reuse
  and use a run generation identifier where handles can outlive a submission.
- Required thread-local resources and suspended coroutines need owner-thread
  initialization, resume, and destruction. Keep existing Send bounds initially;
  supporting non-Send thread-local payloads requires a separate reviewed API.

Gate: failed, repeated, concurrent-client, and shutdown cases terminate under
timeouts without stale jobs or borrowed-state escape. Panic handling preserves
executor thread identity when recovery is supported; actual worker loss makes
its required placement unavailable and is reported explicitly.

## Phase E — Karst integration

Files: `src/karst/fabric.rs`, `fabric_node.rs`, `vpu.rs`, and `_tests.rs`;
Swarm/Flock dispatch boundaries as needed.

1. Independent fabrics: compile one advance chore per fabric or measured chunk,
   with stable preferred/required placement chosen by the designer. Each chore
   retains serial cycle stepping; a join completes the batch. Reuse the graph
   and executor over successive calls.
2. VPU channels: add an explicitly ordered batch between host-advance phases.
   Bind stateful channel chores to required workers where appropriate. Encode
   same-channel ordering with edges; run independent channels concurrently.
   Keep inner Swarm execution serial when outer channel chores consume the
   worker budget. Validate exact typed numeric output and completion accounting.
3. Optional die graph: `Sample >> (Die0 | Die1) >> Commit`. Sample establishes
   inputs, each die has its placement, and Commit records/publishes the cycle
   only after both complete. Complete a run before reusing next-cycle state;
   keep cycles separate from the acyclic per-cycle graph.

Choose explicit worker mappings at graph construction. Worker budgets 1/2/3/4/8
must all have valid mappings; two chores may share a worker without gaining an
implicit ordering edge. A one-worker persistent executor still honors its
thread guarantee; a direct serial policy is separately selected.

Gate: per-cycle traces, responses, memory, modeled statistics, and faults match
the serial reference. Verify required thread identity, not only logical worker
bitmasks. The existing partition defects must be fixed before using partitioned
scoped helpers in these paths. Keep two-die execution optional until measured.

## Phase F — Other consumers and tuning

- Rube: reuse compiled warp chores with stable placement and explicit evaluate/
  commit dependencies. The designer must fix overlapping trigger-bank mutable
  references and shared flag updates in its kernels. Heist does not dynamically
  detect those accesses. Sample/future storage is one suitable implementation.
- Swarm/Flock: retain explicit serial and executor-driven paths; add balanced
  chunks with `Any` or `Prefer` for portable kernels. Required affinity is a
  caller decision, not an unconditional default for memory kernels.
- Drove: a CPU submission chore is complete for dependency purposes only when
  its promised device work is complete, or when a separate GPU-completion chore
  gates consumers. Submission alone cannot release memory readers.
- Measure required-only, preferred-plus-stealing, and unrestricted policies.
  Capture productive work, owner queue wait, migrations, steals, worker idle
  time, and graph dispatch cost with optional instrumentation.
- Evaluate OS CPU affinity after logical affinity is stable. It requires its
  own platform implementation and hardware measurement; it is not needed for
  the same-thread guarantee.

Gate: release results separate cold setup from warm execution and compare with
ordinary serial operation. Use the enhancement plan's benchmark matrix and
proposed regression/crossover gates. Affinity is expected to help locality;
actual benefit and loss from idle required workers remain measurements.

## First implementation checkpoint

Deliver Phases A-C for fixed, reusable graphs with static/owned payloads and
checked admission, plus the basic failure/shutdown rules from D. Demonstrate
`Sample >> (A.Require(0) | B.Require(1)) >> Commit.Require(2)` on a persistent
three-worker executor. Verify precedence, same OS thread across repeated runs,
cross-worker joins, required-task non-stealing, and successful stealing of
separate `Any` chores. Then integrate independent Karst batches before expanding
to channel and die graphs.

Testing belongs in established `jeeves_test!` suites. This plan was checked
against current scheduler/chore source and the prior assessment. No new runtime
test or benchmark result is claimed by this documentation-only change.
