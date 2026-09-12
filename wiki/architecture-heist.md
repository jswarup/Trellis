# Heist Architecture

## Purpose

`heist` is Trellis's job orchestration layer. It turns type-erased `stalks::WorkPtr` jobs and declarative chore trees into executable work, supporting inline execution, single-threaded execution, and multi-threaded work stealing through one API.

## Main building blocks

- `Atelier` is the singleton scheduler and job-pool owner. It stores maestros, job records, predecessor counts, successor IDs, free job IDs, and the terminal job.
- `Maestro` is both a worker context and a local scheduler. It owns temporary and runnable queues, constructs jobs, tracks the current successor, and can post child jobs.
- `Chore` is a named executable leaf containing a closure over `stalks::IWorker*`.
- `ChoreTree` composition uses `operator<` for sequential dependency and `operator|` for parallel branching. The result is a `stalks::BinNode` interpreted when posted.
- `DoQSort` integration lets `silo::USeg` submit partition work to a worker and use the same execution substrate for parallel sorting.

## Scheduler modes

- **Immediate mode (`szThreads == 0`)**: one pass-through maestro exists, `PostJob` executes inline, and `DoLaunch` does nothing.
- **Single-threaded mode (`szThreads == 1`)**: the main maestro runs the queue without worker contention.
- **Multi-threaded mode (`szThreads >= 2`)**: one maestro is created per worker thread. Maestros process local work and steal work from peers when their own queues are empty.

## Job lifecycle

1. `Atelier` reserves a job ID and stores its `WorkPtr`.
2. A predecessor count and successor ID encode dependency completion.
3. The owning maestro enqueues the job or places it in a temporary queue while a graph is being built.
4. `DoLaunch` starts execution for the selected scheduler mode.
5. A maestro pops local work, executes the job with itself as `IWorker`, and decrements or releases successor dependencies.
6. Jobs whose dependencies reach zero become runnable; the terminal job marks completion.

Job IDs are compact (`uint16_t`) and backed by preallocated storage with a fixed scheduler capacity. This keeps queues cheap and avoids per-job heap allocation in the scheduler itself.

## Chore DAG semantics

`Chore` leaves are posted recursively. A sequential node (`a < b`) posts `b` only after `a` completes. A parallel node (`a | b`) allows both branches to run independently and joins them before dependent work proceeds. This provides a compact, declarative DAG syntax while retaining explicit job records underneath.

## Synchronization and ownership

Atomic counters track scheduler state and predecessor counts. Spinlocks protect queue operations where a maestro can be accessed by another worker. `Buff`, `Stash`, and `Stk` from `silo` provide the backing storage; `WorkPtr` and `IWorker` from `stalks` define the execution boundary.

## Consumers

`rube::SimEngine` uses Heist's parallel mode for circuit evaluation. `swarm::SwarmEngine` uses it for parallel compute dispatch. `silo::USeg::DoQSort` uses a worker to parallelize sorting. Immediate mode is important for deterministic tests and environments without a worker pool.
