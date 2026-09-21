# Karst review and parallelization implementation plan

Reviewed 2026-09-21 against working-tree revision `1b99d1f`. “Symp” is
interpreted as `symph`. The first transport checkpoint is implemented; the
remaining parallel-execution work stays planned. Performance conclusions below
are hypotheses until measured.

## Assessment

Karst currently models concurrent hardware through serial software steps.
`with_workers(4)` does not parallelize fabric advancement. Its VPU path can
use Heist through Swarm, but that dispatch follows the global Atelier rather
than the device's requested worker count. Rube is not part of Karst's current
execution path. Drove provides a Double SPIR-V artifact, but there is no
working Swarm GPU compute adapter connected to Karst.

### Implementation status

Completed on 2026-09-21:

- The Karst pipe now accepts ingress only when its ready signal was asserted
  for that sampled cycle. `KarstFabricNode` supplies the NoC with that same
  pre-step ready value, preventing either duplicated or dropped requests when
  a full pipe begins draining.
- MC responses retire only after their recorded NoC acceptance. The controller
  also continues to accept writes while its read-response staging FIFO is full.
- `NocPipeBackpressureTransfersEachReadOnce` stalls the response link for 300
  cycles, sends 60 reads, then verifies each response address and memory service
  occurs exactly once after draining.
- Heist work-stealing coverage now uses `Atelier::Reset( 3)` and requires a
  non-main maestro to process stolen work.
- Global addresses now decode without modulo aliasing: die/MC selector bits are
  removed before channel access. Checked host submissions reject unaligned,
  out-of-range, and over-width addresses; internally injected invalid requests
  receive an explicit fault response rather than a sentinel value or discarded
  write error.

Still pending: observability counters, Flock access contracts, reusable scoped
Heist execution, Rube ownership repair, Karst
parallel scheduling, and GPU VPU dispatch.

The first objective is deterministic, lossless execution with explicit memory
ownership. Then measure three distinct opportunities: independent simulation
runs, concurrent work on independent memory channels, and parallel evaluation
of the two dies within a cycle. Keep the serial implementation as the reference
and as the automatic choice below measured crossover points.

### Findings and evidence

| Priority | Finding | Evidence and consequence |
| --- | --- | --- |
| P1 | Karst's advertised worker count does not schedule fabric work. | `src/karst/fabric.rs:439` calls both die steps sequentially. The corrected `ConfiguredWorkersPreserveTransport` test now identifies the count as a VPU worker budget rather than fabric-cycle parallelism. |
| P1 | The NoC-to-pipe handshake duplicates requests under congestion. | `src/karst/fabric_node.rs:135` steps the pipe before returning its updated ready to NoC. The pipe accepts the item that fills its FIFO, but NoC sees ready=false and retains that item. On drain/refill, the FIFO remains full and the retained item repeats. |
| P1 | MC read service and pipe retirement can disagree. | `src/karst/fabric_node.rs:115` admits a read using available response capacity; line 125 can consume the last slot; line 132 then recomputes ready=false, so the serviced read remains in the pipe. Response producer retirement also uses NoC readiness from a different step than capture. |
| P1 | Rube's parallel path has unsafe shared state access. | `src/rube/engine.rs:291` reconstructs a mutable reference to the entire trigger bank in each job. Input flag reads at lines 228–229 can race with future-flag writes to the same byte at lines 233/248, even with distinct output trigger owners. |
| P1 | Swarm does not enforce disjoint output access for arbitrary CPU kernels. | `src/swarm/cpu.rs:227` passes the whole output buffer to every chunk through an unsafely Send/Sync pointer wrapper. Partitioning invocation IDs does not constrain a closure's writes. |
| P1 | Worker counts and scheduler lifetime are inconsistent. | Swarm uses `Atelier::Instance()` at `src/swarm/cpu.rs:193`, ignoring `_WorkerCount`. Rube resets the global pool on every Drive at `src/rube/engine.rs:280`. This makes execution depend on other clients and test order. |
| P1 | Large dispatches can silently omit work; nested launches can deadlock. | Swarm posts one job per X workgroup before launch through infallible `PostJob`. Heist has 65,536 slots including sentinel/reserved use; failed allocation returns zero and zero jobs are ignored. `DoLaunch` holds a global non-reentrant lifecycle lock, so a worker calling Dispatch and then DoLaunch cannot complete. |
| P1 | Global addresses alias and memory faults are hidden. | `src/karst/fabric_node.rs:117` uses address modulo channel capacity. For example, 0x0000 and 0x2000 select the same channel and overwrite the same local word. Write errors are discarded and read errors become 0xDEADBEEF. |
| P2 | Scheduler and copying overhead are substantial structural costs. | `src/heist/atelier.rs:276` spawns/joins workers for each launch. Rube clones the entire warp per 64-lane job. Swarm reads every buffer, clones inputs, and copies output back for every dispatch. These costs need separate measurement. |
| P2 | Routing tests and observability overstate coverage. | `DualHindInterDieLink` posts a remote host address, but `src/karst/host_node.rs:208` sends that traffic directly over Link1 to the target die. It does not exercise inter-die KL8/KL9. All forwarded requests currently select KL8. There are no per-link stall/accept counters proving traversal. |

The original standalone probe compiled the actual `fifo.rs`, `config.rs`,
`link.rs`, `noc.rs`, and `pipe.rs`, coupled them in the same order as
`fabric_node.rs`, and exposed duplicated requests under congestion. Its
tracked successor, `NocPipeBackpressureTransfersEachReadOnce`, now verifies
the corrected exact-once behavior under a 300-cycle response stall.

Existing safeguards should be retained: fixed-capacity FIFOs, sampled external
die inputs, two-slot reservation for simultaneous host responses, little-endian
memory words, and Swarm's associated-type backend boundary. A FIFO's capacity
of four does not itself establish four-cycle retiming latency; specify and test
latency separately before describing it as a physical pipeline model.

## Subsystem responsibilities

| Subsystem | Role in the implementation |
| --- | --- |
| Karst | Own topology, queues, cycle boundaries, address decoding, arbitration, channel access ordering, and simulation metrics. |
| Rube | Supply the reference sample/evaluate/commit pattern and optional signal/VCD integration. Repair its parallel trigger evaluation as a separate consumer of the shared executor. Karst need not become a gate netlist. |
| Heist | Own a reusable executor, bounded tasks, dependency joins, task scopes, worker budgets, and completion/error reporting. |
| Flock | Own CPU buffer storage, safe CPU kernel execution, and Heist scheduling, completing the extraction already proposed in the GPU migration plan. |
| Swarm | Own backend selection, GPU resources, dispatch, synchronization, and compatibility entry points. Preserve `IComputeBackend`; do not encode GPU resources in the CPU buffer type. |
| Symph | Own portable operation contracts: bindings, element types, dimensions, workgroup shape, bounds, arithmetic, and byte/layout conventions. |
| Drove | Implement GPU kernels against those contracts. Start Karst integration with the existing Double operation and hardware readback validation. |

Reuse [the GPU migration plan](gpu-migration-plan.md) for Flock extraction,
shared shader contracts, and the GPU adapter, and [the hardening plan](framework-hardening-plan.md)
for transport invariants. This plan adds the Karst-specific sequencing and gates;
it does not create a second GPU migration or expand into rendering/CUDA work.

### Drove directory distinction

The root-level `C:/Work/Oogway/Trellis/drove` is an unused leftover: its `src`
directory is empty, it has no `Cargo.toml` or tracked files, and its ignored
`target` directory contains 284 build-artifact files totaling about 92 MiB.
Current manifest/build-script/editor references do not use this directory.
It can be removed without affecting the current build; this review leaves it
in place.

Keep `src/drove` in the current architecture. It contains two compilation units:

- `src/drove/mod.rs` and its sibling files belong to the host's `crate::drove`
  module. They expose operation/entry-point mappings and embedded SPIR-V.
- `src/drove/Cargo.toml` and `src/drove/src/*.rs` define the separate `drove`
  workspace member, compiled with the Rust-GPU backend for the shader target.

The root manifest lists that member, `tools/build.rs` builds it, and the host
embeds the generated artifact through `DROVE_SPV_PATH`. Removing the folder
breaks these dependencies even when the runtime selects CPU compute. The
separate shader crate is useful; its exact filesystem location and inclusion
in `default-members` are organizational choices, not reasons to remove it.
An optional CPU-only build could be designed later by feature-gating shader
generation and artifact consumers together, but it is not required for the
Karst parallelization work.

## Implementation sequence

### 0. Establish an honest baseline

Files: `src/karst/_tests.rs`, `src/karst/fabric.rs`, `src/karst/noc.rs`,
`src/karst/host_node.rs`, and the component test/benchmark harness.

- Correct the ParallelDrive and remote-routing diagnostics to describe what
  they actually execute. Preserve the existing functional tests.
- Add deterministic traffic generation and a serial reference trace containing
  accepted transfers, responses, channel bytes, and cycle counts. Inspect
  memory without incrementing simulated service counters.
- Add accepted/stalled counts per link, queue occupancy/high-water marks,
  per-channel service counts, and latency distributions. Keep detailed tracing
  optional and compare instrumentation-on/off costs.
- Record simulation throughput separately from modeled throughput:
  cycles/second and transactions/second versus transactions/cycle and latency
  in simulated cycles. Record jobs, participating workers, launch/barrier time,
  bytes copied, allocations, and GPU transfer time when applicable.

Gate: a reproducible workload produces identical traces across repeated serial
runs; baseline results identify CPU, build profile, workload, and instrumentation.
Existing tests passing does not establish congestion safety or parallel speedup.

### 1. Repair transfer accounting and memory semantics

Files: `src/karst/{fabric,fabric_node,host_node,noc,pipe,memchan,link,config}.rs`
and `src/karst/_tests.rs`.

- Define a cycle contract for every boundary. Sample valid/data/ready once,
  derive one transfer decision, and use it for both source retirement and
  destination acceptance. Publish next-cycle outputs after state transitions.
  If same-cycle capacity reuse is supported, compute it consistently at both
  endpoints; never substitute post-enqueue fullness for acceptance.
- Use that same decision for MC memory service and pipe removal. Capture the
  MC-response-to-NoC transfer explicitly instead of inferring previous
  acceptance from newly updated readiness. Check every internal enqueue.
- Add regressions for a pipe filling on the last free slot, full-to-draining
  transitions, response staging at 15/16 entries, NoC response saturation,
  simultaneous host responses, and long randomized consumer stalls. Verify
  exact-once acceptance and eventual drain after producers stop.
- Define request ordering precisely. Preserve FIFO order where guaranteed;
  do not assert total response order across independent channels without a
  reorder mechanism. Use a scoreboard for multiple outstanding destinations.
- Centralize global-to-local address decoding. Recommended mapping removes
  die bit 12 and MC bits 11:10:
  `local = ((addr >> 13) << 10) | (addr & 0x3ff)`.
  With eight 4-KiB channels this exposes 32 KiB of non-aliasing modeled memory.
  This changes the current modulo behavior and associated expectations;
  document it as a memory-layout correction.
- Validate alignment, flit-width limits, and modeled range before accepting a
  host request. Return explicit errors from checked submission APIs; report
  faults for invalid requests injected internally. Eliminate magic read data
  and discarded write errors from the normal service path.
- Keep direct remote Link1 routing as the default topology behavior. Test
  inter-die forwarding by deliberately injecting a request into the wrong
  die's NoC. Assert link counters and response destination, not printed paths.
  Document whether KL9 is reserved or available for a later routing policy.
- Characterize fixed-index NoC arbitration under contention. If bounded
  fairness is part of the model, add persistent round-robin grants per output
  and define output admission bandwidth. Treat this as a modeled-behavior
  change, distinct from speeding up the simulator.

Gate: no loss, duplicates, silent memory errors, or unintended aliasing in
sustained traffic; all accepted work drains under fair downstream readiness.
All existing transport tests pass with explicitly updated address expectations.

### 2. Establish safe compute and trigger ownership

Files: `src/swarm/{cpu,traits,ops,backend}.rs`, `src/flock`,
`src/symph`, `src/rube/{engine,trigger,layout,module}.rs`, and their tests.

- Execute arbitrary legacy CPU closures serially until an enforceable access
  contract exists. Define standard operations using immutable input views and
  exclusive bounded output spans; pass a global index/base separately from
  the local writable span. A caller-supplied “parallel safe” boolean is not an
  ownership proof.
- Deduplicate aliased buffer bindings and acquire any necessary buffer guards
  in a stable order. Hold exclusive output ownership through dispatch
  completion. Concurrent writes or VPU dispatches on the same channel must
  serialize or be rejected rather than lose updates during read/copy/writeback.
- Make in-place operations explicit. Double can operate directly on its owned
  span without cloning an unused input snapshot. Retain snapshots only for
  operations whose declared semantics require them.
- Define dimension semantics and checked arithmetic. Existing standard kernels
  use X only; initially require Y=Z=1 for them, with documented zero-work
  behavior, rather than executing in-place Double repeatedly over Y/Z.
  Keep CPU chunk size separate from the shader workgroup size of 64.
- Finish the Flock extraction with compatibility re-exports and a single CPU
  implementation. Reject unknown kernel sources instead of guessing Double
  from names or shader text. Use Symph contracts for typed VPU data; a byte
  fill is insufficient evidence of numeric Double correctness.
- Split Rube's immutable current values/flags from future outputs/flags.
  Validate output ownership when compiling the layout. For scattered trigger
  outputs, use reusable per-partition result buffers followed by deterministic
  commit unless direct disjoint ownership is proven. Resolve or reject
  multiple drivers explicitly. Remove overlapping whole-bank mutable references.
- Keep coroutine instances on their owning execution context initially and
  specify their evaluation/commit order relative to fast warps. Borrow or share
  immutable compiled warp data rather than deep-cloning each warp per chunk.

Gate: exact serial/partitioned parity for integer operations, documented float
comparisons, X/I-state and chained-gate tests, aliasing tests, short buffers,
partial workgroups, zero work, invalid dimensions, and overflow rejection.
Memory-safety tooling should exercise isolated CPU kernels where supported;
passing numerical tests alone is not a proof of safe aliasing.

### 3. Make Heist reusable and composable

Files: `src/heist/{atelier,maestro,choretree}.rs`, `src/heist/_tests.rs`,
and executor ownership in Rube and Flock.

- Give each execution context an explicit reusable Atelier handle. Multiple
  clients may share a caller-owned context; they must not reset a process-wide
  pool during dispatch. Make configured worker counts effective and report
  actual participation separately from the maximum budget.
- Add a scoped task-group/join boundary so borrowed partitions cannot outlive
  their storage, including submission failure and panic paths. Preserve
  completion accounting and report failures instead of ignoring failed joins.
  In particular, `SpawnQuellNode` currently erases its borrowed data into an
  unscoped integer pointer when converted to `ChoreNode`; carry ownership or
  the scope lifetime through that conversion before using it for channel work.
- Reuse worker threads across launches with a bounded queue and a wake/park
  mechanism. Keep the initiating thread available for work. Limit lifecycle
  synchronization to setup/reset/shutdown, not execution of arbitrary jobs.
- Define nested behavior: the first safe implementation may run inner work
  inline when already on an executor worker. A later cooperative join can
  distribute nested tasks on the same budget. Never recursively block on the
  launch-wide lifecycle lock or create another pool per VPU.
- Bound chunk count by the worker budget and cost estimate, rather than one
  queued job per GPU workgroup. Use checked submission or bounded waves on
  capacity pressure; propagate errors, and never drop a zero job ID silently.
- Test repeated launches, fan-in reuse, slot exhaustion, nested dispatch,
  concurrent clients, panic recovery, and clean shutdown. Add deterministic
  worker-participation checks with enough controlled work to require overlap.

Gate: repeated execution creates no additional workers after initialization,
worker budgets 1/2/4/8 are respected, failures leave no borrowed work running,
and capacity/nested tests complete without missing work or deadlock.

### 4. Integrate coarse parallel work into Karst

Files: `src/karst/{fabric,fabric_node,vpu,memchan}.rs`, Rube/Flock executor
callers, and component tests.

- Add explicit serial and parallel execution policies plus an automatic
  policy, preserving existing constructors as compatibility wrappers. Expose
  which policy actually ran; worker configuration alone is not a mode claim.
- First support independent Karst simulations in one Heist task group. Each
  simulation owns its queues/memory and advances serially inside a coarse task.
  This avoids a synchronization barrier per simulated cycle.
- Add a VPU batch API for independent channels. Group work into one task group;
  parallelize channels before subdividing small individual channel buffers.
  Use one worker budget across both levels and exclusive channel access.
  Reuse Heist's parallel/sequence and spawn/join concepts after scope safety
  is established. Its current `GpuAuto` SpawnQuell branch merely avoids CPU
  chunking; actual GPU submission and completion must go through Swarm.
- Define VPU visibility at explicit simulation boundaries. Initially complete
  a batch between cycle advances. Asynchronous GPU/CPU tasks can later use
  completion dependencies, but may not change a channel halfway through a
  sampled cycle. Host wall-clock duration must not determine simulated latency.
- Add opt-in die-parallel evaluation: snapshot both dies' external inputs,
  evaluate two independently owned die states, join, then expose the next
  cycle. Hosts can remain a compact serial phase initially. Preserve identical
  sample/evaluate/commit semantics in both execution policies.
- Reuse descriptors and buffers. Do not add jobs for each link, queue operation,
  or word access. Do not advance one connected die several cycles ahead without
  proving a lookahead bound from modeled link latency.
- Use Rube for optional tracing/co-simulation through a boundary adapter, with
  clear ownership of simulation time. The shared executor and cycle semantics
  are useful integration points; a wholesale Karst-to-Rube rewrite is not a
  prerequisite.

Gate: worker counts 1/2/4/8 produce the same cycle-by-cycle accepted-transfer
trace, responses, memory, and modeled statistics as the corrected serial
reference, including saturation and concurrent VPU/host workloads. Independent
runs remain isolated. Tests prove real worker participation when requested.

### 5. Connect validated Drove kernels through Swarm

Dependencies: shared Symph contracts, safe channel ownership, and the GPU
adapter milestones in `gpu-migration-plan.md`.

- Package the minimal shader-compatible Symph contract/math subset for both
  host and Drove builds. Keep host containers outside the shader compilation
  boundary and document little-endian memory versus typed kernel interpretation.
- Validate the existing Drove Double artifact through a real Swarm compute
  adapter and readback. Specify binding count/layout, element count, workgroup
  dimensions, and behavior for the partial final group.
- Make VPU dispatch accept an operation and declared bindings instead of
  permanently owning a CPU Double kernel. Select kernels through backend
  contracts; retain distinct CPU/GPU resource types.
- Batch independent channel work and retain GPU resources across operations
  when useful. Define upload, ownership transfer, completion, and readback
  explicitly. Because NoC/MC simulation accesses words on the CPU, frequent
  CPU/GPU ownership changes may erase any GPU benefit; measure that workload.
- Use CPU fallback for unavailable hardware. Do not silently replay partially
  submitted work after a GPU execution error. Report hardware skips as
  unverified and leave GPU automatic selection disabled without measured parity
  and crossover evidence.

Gate: actual hardware readback matches the CPU reference for whole/partial
groups and batched channels, with explicit synchronization and no stale host
reads. Shader compilation by itself does not pass the gate.

### 6. Tune policy using end-to-end measurements

| Workload | Sweep | Decision supported |
| --- | --- | --- |
| One fixed Karst fabric | Idle, local, remote Link1, explicit inter-die, all-channel, hotspot, read-backpressure; workers 1/2/4/8 | Whether two-die parallel stepping beats serial advancement. |
| Independent fabrics | 1, 2, 8, 32 instances; same workloads and seeds | Coarse task size and throughput scaling without cycle barriers. |
| CPU VPU work | One/eight channels, current 4 KiB then configurable larger capacities; Double and a heavier validated operation | Channel batching, serial cutoff, chunk size, and memory-copy savings. |
| GPU VPU work | One-shot and repeated resident batches, including host reads/writes between operations | End-to-end offload threshold including transfers and synchronization. |
| Rube | Small/large, dense/sparse activity, homogeneous/mixed warps | Chunk cost, immutable warp reuse, and whether a later active-worklist optimization is justified. |

Use optimized builds, fixed seeded workloads, warm-up, and repeated trials.
Report median and tail times, variance, speedup `T1/Tp`, and efficiency
`T1/(p*Tp)`. Time complete workloads, including copies, submission, joins, and
readback; also report those costs separately to locate bottlenecks. Compare
against both the original baseline and the corrected optimized serial path.

There are only two independent die tasks per cycle in the current partition;
that portion has at most two-way concurrency, and serial host/snapshot/commit
work reduces total speedup further. Eight host ports are not evidence of
eight-way CPU parallelism. The current total channel storage is only 32 KiB;
GPU profitability for it is an open measurement question.

Select automatic parallel/GPU paths only where repeated measurements show a
stable end-to-end win beyond run-to-run noise. Keep serial execution below
that crossover. Sparse Rube activity scheduling, finer Karst channel tasks,
KL8/KL9 load balancing, and event skipping are follow-ups only when counters
show a benefit and the ordering/timing contract can be preserved.

## Delivery and verification

Deliver phases as reviewable changes, with the ownership/transport changes
separated from throughput policy. Phase 0 precedes phase 1; phases 1 and 2 can
then be developed independently. Phase 3's executor design must incorporate
phase 2's ownership contract. Phase 4 depends on phases 1–3; phase 5 can reuse
parallel progress in the existing GPU plan. Phase 6 decides defaults rather
than assuming that every available parallel path should be enabled.

### Execution-ready change sequence

| Change | Scope | Prerequisite | Completion condition |
| --- | --- | --- | --- |
| 1. Karst transport reference | Add transfer/queue metrics and the tracked congestion regression; correct misleading routing and parallel-test descriptions. | None | Serial trace records the duplicated-transfer case before the repair. |
| 2. Atomic cycle handshakes | Make pipe, NoC, memory-controller, and host transitions consume one sampled transfer decision; make memory faults explicit. | 1 | Backpressure, saturation, remote routing, and randomized-stall tests are exact-once and drain. |
| 3. CPU access contract | Move standard CPU execution behind Flock-compatible owned output regions; reject unsafe parallel dispatches and bad dimensions. | None | Serial and partitioned kernels have parity, including aliases and partial workgroups. |
| 4. Heist task scope | Replace global-reset-per-launch behavior with caller-owned reusable task groups and bounded, failure-aware submission. | 3 | Repeated, nested, capacity, and concurrent-client tests complete with configured worker budgets. |
| 5. Rube parallel repair | Apply immutable-snapshot plus exclusive-output/commit semantics to trigger evaluation. | 4 | Race-detection-oriented and serial/parallel state tests agree for fast and coroutine warps. |
| 6. Karst CPU parallel policy | Add independent-fabric batching, channel-batched VPUs, and optional two-die sample/evaluate/commit execution. | 2, 3, 4 | Full cycle-by-cycle parity holds for workers 1, 2, 4, and 8. |
| 7. Drove-backed VPU | Complete the Symph contract and Swarm GPU adapter, then expose validated Drove operations through VPU batches. | 3, 4, GPU migration gates | Hardware readback parity and ownership/synchronization checks pass. |
| 8. Benchmark-driven defaults | Run the phase-6 matrix in optimized builds and set automatic cutoffs from results. | 6, optionally 7 | Default policy has repeatable end-to-end gains; serial remains the fallback below cutoff. |

Changes 1-2 form the first implementation milestone. Changes 3-5 establish
the shared execution substrate and can proceed in parallel with transport work.
No code from changes 6-8 should be enabled by default before its completion
condition is met.

Use `jeeves_test!` in each owning component's `_tests.rs`. During shared
global-Atelier compatibility, run relevant libtests with `--test-threads=1`:

```powershell
cargo test -p trellis --lib karst:: -- --test-threads=1
cargo test -p trellis --lib heist:: -- --test-threads=1
cargo test -p trellis --lib rube:: -- --test-threads=1
cargo test -p trellis --lib swarm:: -- --test-threads=1
cargo test -p trellis --lib symph:: -- --test-threads=1
```

Add Flock tests as extraction introduces device/storage code there. Use the
Cove runner with assertions enabled (`cargo run -p trellis -- -t Karst`) when
checking suite diagnostics. For broad implementation changes, run a clean
build, `cargo check --all-targets`, and `cargo clippy -- -D warnings`, plus the
relevant suites and explicitly enabled hardware checks. Follow project
containers, traversal, naming, formatting, and debugger visualization rules
for touched implementation files. Do not widen the patch into unrelated
style cleanup.

The first implementation checkpoint is the phase-1 congestion regression and
consistent transfer decisions, with corrected diagnostics. No parallel mode
should be promoted until the serial transport reference is trustworthy.

### Verification performed during this review

- `cargo test -p trellis --lib karst:: --offline -- --test-threads=1` built the
  current sources and passed all 16 existing Karst cases.
- The resulting test executable passed the Heist (13), Rube (16), Swarm (11),
  and Symph (7) filtered suites. Swarm's opt-in `ViewportGpu` case returned
  without exercising hardware; this is not GPU validation.
- The standalone probe confirmed duplicate transfers and failure to drain at
  the NoC/pipe boundary. Existing suite success therefore does not close the
  congestion finding.
- The new document passed whitespace checks. The only tracked-content change
  is this added plan; application implementation files were not modified.

No release benchmarks or hardware compute measurements were performed. The
plan's automatic execution thresholds remain to be established in phase 6.

## Implementation follow-up review

Reviewed on 2026-09-21 at committed revision `bce185d`, after the transport
repairs, project rename, and Swarm safety gate. The working tree was clean
before this documentation update. The findings below supersede status claims
that conflict with the current sources.

### Current state

| Area | Status | Evidence |
| --- | --- | --- |
| Serial Karst transport | Working for the covered cases | 20 Karst tests passed, including 60 stalled reads, stripe alias prevention, checked host rejection, and injected read/write faults. |
| Karst parallel execution | Not implemented; serial reference tracing available | `KarstFabric::step_cycle` calls die 0 then die 1 serially. An opt-in bounded trace records sampled ingress and resulting egress signals for both dies, but no execution policy, independent-fabric batch API, or channel VPU batch API exists. |
| CPU VPU worker use | One sealed serial operation; parallel work remains disabled | `Double` now holds its buffer lock for the whole operation and validates its binding/dimensions. `ComputeDevice::SupportsParallelDispatch()` still returns false. |
| Heist work stealing | Startup participation stabilized; lifecycle redesign remains | A per-launch worker-start barrier gives worker maestros access to queued work before maestro 0 drains it. The `Reset(3)` work-stealing case passed 10 consecutive isolated runs. |
| Drove GPU VPU | Not connected | Drove supplies an artifact only. Karst has no Swarm GPU adapter, dispatch, readback, or visibility boundary. |

### Findings

1. The Swarm safety gate is containment, not a completed CPU access contract.
   The old parallel branch remains in `src/swarm/cpu.rs`, guarded by a method
   that always returns false. The serial path no longer initializes the global
   Atelier, but the old branch still builds. The four-worker Double test checks
   numerical output but cannot prove that the unsafe branch was not used.

2. A partitioned-output API cannot safely be built on the current Silo views.
   `Arr::GetMut(&self)` and `Arr<u8>::AsMutSlice(&self)` manufacture mutable
   access from a shared view. `MutArr::GetMut` and `MutArr::Slice` return
   views with the storage lifetime rather than the borrow lifetime. These APIs,
   together with safe raw-pointer constructors, cannot express exclusive,
   disjoint output ownership across worker tasks.

   Follow-up implementation: `MutArr::New` and mutable conversion from an
   `Arr` are now explicit `unsafe` operations. Ordinary `MutArr` reads,
   writes, slices, and byte-slice conversion use their current borrow lifetime,
   preventing a short-lived view from manufacturing a storage-lifetime mutable
   reference. `Stash` and `Stk` retain compatibility escape hatches for their
   atomic-storage APIs; those now construct their long-lived raw views in
   explicit unsafe blocks and remain candidates for replacement before worker
   tasks borrow their output spans.

3. Legacy serial Swarm dispatch does not protect against concurrent callers.
   It snapshots buffers through `ComputeBuffer::Read`, executes, then writes
   back through a separate lock acquisition. Concurrent legacy dispatches
   targeting the same buffer can both read an old value and overwrite each
   other. `Double` now bypasses that path: its tagged standard source holds one
   buffer lock for the complete operation, requires one f32-sized
   binding, and accepts linear dimensions only. The other standard operations
   and arbitrary closures still need operation-wide ownership before batching.

4. Heist is not ready to own borrowed Karst tasks. `Atelier::DoLaunch` holds a
   global lifecycle lock while it runs user jobs, creates and joins threads on
   every launch, and ignores join failures. Nested launches can deadlock.
   `SpawnQuellNode` erases borrowed data to an integer pointer without a task
   scope. Rube separately resets the global Atelier and reconstructs a mutable
   whole trigger bank in each job, so it must be repaired as another consumer.

   Follow-up implementation: `DoLaunch` now waits for every spawned worker to
   enter its execution loop before maestro 0 starts draining the run queue.
   This fixes the observed worker-participation race without extending the
   global lifecycle lock or changing task ownership. It is a scheduling
   reliability repair, not the caller-owned scoped-task model required for
   Karst batching.

5. Several transport gates remain open. Fault tests do not cover last-word
   boundaries, unaligned injected accesses, backend failures, response-fault
   saturation, both dies, or valid `0xDEADBEEF` data. The remote-routing test
   prints a KL8 traversal even though host routing uses direct Link1. Per-link
   accepted/stalled counts, queue high-water marks, response latency, and a
   deterministic serial trace are absent. A bounded opt-in cycle trace now
   records ingress/egress signals and aggregate host traffic; per-link and
   queue/latency metrics remain absent.

6. The standard-operation contract is only partially specified. `Double` has
   explicit operation metadata (rather than name inference), rejects X
   overflow, non-linear dimensions, extra bindings, and non-f32-sized buffers.
   Other standard kernels still use X only, can repeat work over Y/Z, and lack
   binding, aliasing, zero-work, and partial-group contracts. Unknown shader
   source strings can still be treated as Double, so source validation remains
   a separate compiler boundary issue.

### Revised implementation order

1. Repair or replace the mutable-view boundaries, then add a sealed Flock
   standard-operation contract with immutable inputs, a single exclusive output
   span, global base index, bounded count, checked dimensions, and operation-
   wide buffer ownership. Keep legacy closures serial.
2. Add deterministic partition parity, aliasing, short-buffer, partial-group,
   zero-work, invalid-dimension, and overflow tests. Remove the guarded
   raw-pointer dispatch path once the replacement is exercised.
3. Give Heist caller-owned scoped task groups, checked submission, panic-aware
   completion, and deterministic three-worker participation. Then make workers
   persistent and define nested execution. Repair Rube against that contract.
4. Complete Karst transport instrumentation and fault/routing coverage. Add
   serial traces before comparing policies.
5. Add independent-fabric batches, then exclusive-channel VPU batches, then
   optional two-die sample/evaluate/commit execution. Require cycle-by-cycle
   equivalence for worker budgets 1, 2, 3, 4, and 8.
6. Connect Drove only after CPU ownership and executor contracts hold. Require
   adapter-backed readback parity and explicit synchronization. Measure release
   workloads before selecting any automatic parallel or GPU policy.

### Verification in this follow-up

- `cargo test -p trellis --lib karst:: --offline -- --test-threads=1`:
  20 passed.
- `cargo test -p trellis --lib swarm:: --offline -- --test-threads=1`:
  14 passed, including overflowing-X rejection, Double's sealed binding
  contract, and explicit standard-operation metadata. The opt-in viewport case
  is not Drove compute validation.
- `cargo test -p trellis --lib heist:: --offline -- --test-threads=1`:
  12 passed and `WorkStealing` failed its non-main-worker participation
  assertion. This run did not reproduce the earlier access violation.
- `cargo test -p trellis --lib silo:: --offline -- --test-threads=1` after the
  mutable-view follow-up: 33 passed.
- `cargo test -p trellis --lib karst:: --offline -- --test-threads=1` after
  cycle-trace instrumentation: 21 passed, including worker-budget trace parity
  and bounded-capture behavior.

No release measurements, hardware compute readback, Rube suite, Symph suite,
or memory-safety tooling were run as part of this documentation review.
