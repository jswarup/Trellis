# Trellis Architecture

## Getting started

Build Trellis from the repository root with CMake and Ninja:

```powershell
cmake -S tools/build -B out/win32-Debug -G Ninja -DCMAKE_BUILD_TYPE=Debug
cmake --build out/win32-Debug
```

Run the complete test suite with `./out/trellis_console.exe --test`, or select matching tests with `--test <filter>`. See the repository [README](../README.md) for prerequisites, test-runner options, and the source layout.

## System synopsis

Trellis is organized as a set of small C++ libraries with explicit ownership and execution boundaries. `silo` provides data representation, `stalks` provides atomic and job primitives, `heist` turns jobs into dependency-aware execution, `rube` models digital circuits, `symph` supplies reusable numerical and shading kernels, `swarm` presents backend-neutral compute dispatch, `crew` coordinates virtual-machine co-simulation, and `cove` verifies the system through self-registering tests.

The core design favors contiguous storage, non-owning views, fixed-width indexes, type erasure at selected boundaries, and explicit scheduler modes. Most domain layers build a description first, then execute it through a separate engine.

## Layered view

```text
                  +-------------------+
                  |   Application      |
                  +---------+---------+
                            |
        +-------------------+-------------------+
        |                                       |
     rube                                  swarm
  circuit model                         compute API
        |                 +-------------------+ |
        |                 |                   | |
        +------------> heist <------------ symph
                     scheduler            kernels
                         |
                      stalks
              jobs, workers, atomics, nodes
                         |
                       silo
         storage, views, traits, union-find

                     cove
          test registration and assertions

                     crew
         VM co-simulation and MMIO message routing
```

      `cove` is a verification layer rather than a runtime dependency. `crew` is an integration layer that connects external virtual machines to deterministic in-memory routing. `symph` is mostly a leaf computation layer; `swarm` adapts it to device-style dispatch. `rube` and `swarm` may both use `heist` for parallel work, while `heist` itself relies on `stalks` and `silo`.

## Cross-cutting data model

`Buff<T>` and `Stash<T>` own contiguous storage. `Arr<T>`, `IAccess<T>`, `IArr<T>`, `Stk<T>`, and `USeg` expose views or algorithm surfaces over that storage. This gives subsystems a common way to pass ranges without copying and makes capacity/size distinctions explicit.

Where a concrete C++ type should not cross a boundary, `silo` trait references and vtables provide compact type erasure. `stalks::WorkPtr` applies the same principle to executable work: a data pointer and function pointer carry a callable into any `IWorker` implementation.

## Execution architecture

### Job execution

`stalks::IWorker` is the smallest execution contract. `Worker` invokes jobs immediately. `heist::Maestro` implements the contract with queue-backed jobs, while `Atelier` owns the pool and chooses immediate, single-threaded, or work-stealing execution.

### Circuit execution

`rube::Layout` is mutable during construction. It registers modules and ports, validates hierarchy-aware connections, and uses a union-find netlist to assign trigger IDs. After freeze, `SimEngine` builds trigger storage and compiled warps. Serial settling evaluates directly; parallel settling delegates eligible work to Heist.

### Compute execution

`symph` functions use explicit indexes, arrays, and sizes so they can run in a direct CPU loop or inside another backend. `swarm` wraps those operations in device, kernel, buffer, and dispatch interfaces. The CPU device is the reference implementation; GPU backends currently preserve the contract while returning typed unsupported errors.

## Typical end-to-end flows

### Testing a subsystem

1. A test translation unit declares `JEEVES_TEST`.
2. Static registration adds it to Cove's intrusive test list.
3. The console runner invokes the selected tests.
4. Assertions record results without aborting the test body.

### Simulating a circuit

1. Build modules and ports in a `rube::Layout`.
2. Connect ports and freeze the layout.
3. Create a `rube::SimEngine` from the compiled layout.
4. Set input port values.
5. Settle in serial or parallel mode.
6. Read output values through port-to-trigger mappings.

### Dispatching a compute operation

1. Create a `swarm::SwarmEngine` and device buffers.
2. Select a `symph` standard operation.
3. Compile the operation into a backend kernel.
4. Dispatch buffers with a workgroup dimension.
5. Synchronize when required by the backend.
6. Read owned bytes and interpret the result.

## Architectural constraints

- Ownership is explicit: owning buffers are distinct from views and type-erased references.
- Hot-path structures prefer contiguous storage and preallocation.
- Scheduler and backend policies are behind small interfaces.
- Topology/description phases are separated from execution phases.
- Parallel paths must have a deterministic immediate or serial mode for testing.
- Unsupported external backends report typed errors instead of pretending to execute.

## Subsystem documents

- [Silo Architecture](architecture-silo.md)
- [Stalks Architecture](architecture-stalks.md)
- [Heist Architecture](architecture-heist.md)
- [Rube Architecture](architecture-rube.md)
- [Symph Architecture](architecture-symph.md)
- [Swarm Architecture](architecture-swarm.md)
- [Crew Architecture](architecture-crew.md)
- [Karst Architecture](architecture-karst.md)
- [Cove Architecture](architecture-cove.md)
