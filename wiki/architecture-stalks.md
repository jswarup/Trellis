# Stalks Architecture

## Purpose

`stalks` contains the small concurrency, job, and composition primitives shared by Trellis execution systems. It is intentionally below `heist`: it defines what a job and worker are, but not how a pool schedules them.

## Main building blocks

- `Atm<T>` wraps `std::atomic<T>` with the operations used by Trellis containers and schedulers: load/store, exchange, arithmetic fetches, and compare-exchange.
- `Spinlock` and `SpinLockGuard` provide a low-latency lock with RAII release. The implementation pauses on x86 and yields on other platforms while waiting.
- `WorkPtr` is a fixed-size type-erased job handle. It stores a data pointer and a function pointer. It can wrap a function pointer or allocate a callable that accepts `IWorker*`.
- `IWorker` is the minimal scheduling boundary. It accepts a `WorkPtr` and provides a convenience `Post` template for callables.
- `Worker` is the immediate executor: posting a job invokes it synchronously on the current worker.
- `BinNode` and `UniNode` are generic expression/dependency nodes. `BinOp::Less` and `BinOp::Bor` are used by `heist` to represent sequencing and parallel composition.

## Execution model

A caller creates a `WorkPtr` from a callable or function and posts it to an `IWorker`. The concrete worker decides when and where `DoWork` runs. `Worker` runs inline; `heist::Maestro` implements the same interface using job IDs and queues.

The type-erased job contract keeps the scheduler independent of callable types. Lambda captures are owned by the generated `WorkPtr` trampoline and released after invocation. Function pointers use the pointer as their data payload and do not allocate.

## Composition model

`BinNode` is a structural representation, not an executor. It stores left and right children plus an operation tag. `heist` interprets the `Less` and `Bor` tags when posting a chore tree. Other `BinOp` values leave room for general expression-tree use without coupling `stalks` to a particular domain.

## Dependencies and consumers

`stalks` is a foundational layer. `silo::Stk` uses `Atm`; `heist` uses all three areas: atomics and spinlocks for coordination, `WorkPtr`/`IWorker` for execution, and nodes for DAG composition. `rube` and `swarm` consume the higher-level scheduler rather than the node primitives directly.

## Invariants

- `WorkPtr` is a 16-byte pair of data and function pointers.
- A non-null `WorkPtr` must have a callable function before execution.
- `IWorker` owns the scheduling policy; jobs only receive a worker context.
- Node construction is value-based and does not execute child work.
