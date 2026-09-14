# Trellis

Trellis is a C++20 systems playground built from small, composable libraries. It explores explicit ownership, contiguous storage, deterministic execution, digital-circuit simulation, backend-neutral compute dispatch, and virtual-machine co-simulation.

The project keeps descriptions and execution separate where possible: build a layout, dependency graph, circuit, or kernel description first; execute it through a dedicated engine second.

## Components

| Component | Responsibility |
| --- | --- |
| `silo` | Contiguous storage, views, traits, and union-find primitives. |
| `stalks` | Atomics, work items, workers, and scheduling primitives. |
| `heist` | Dependency-aware jobs and immediate, serial, or work-stealing execution. |
| `rube` | Digital-circuit layouts, netlists, and serial or parallel simulation. |
| `symph` | Reusable numerical and shader kernels. |
| `swarm` | Backend-neutral compute devices, buffers, kernels, and dispatch. |
| `crew` | Renode-based virtual-machine co-simulation and MMIO message routing. |
| `karst` | High-radix memory fabric simulation framework (Karst KarstFore/MFab KarstHind/DChan). |
| `cove` | Self-registering tests and assertion support. |

The complete design overview and component-specific notes are in the [architecture wiki](wiki/architecture.md).

## Build

Requirements:

- CMake 3.20 or newer
- A C++20 compiler
- Ninja

Configure and build from the repository root:

```powershell
cmake -S tools/build -B out/win32-Debug -G Ninja -DCMAKE_BUILD_TYPE=Debug
cmake --build out/win32-Debug
```

The console executable is written to `out/trellis_console.exe` on Windows.

## Tests

Run the complete self-registering test suite:

```powershell
.\out\trellis_console.exe --test
```

Use `--test <filter>` to run matching tests and `-v 1` or `-v 2` for increasing detail:

```powershell
.\out\trellis_console.exe --test VirtualExchangeProtocol -v 2
```

`-c` executes the selected test bodies while suppressing assertion evaluation.

## Repository layout

```text
src/          Trellis libraries, console runner, and subsystem tests
tools/build/  CMake build definition for the console and tests
wiki/         Architecture overview and subsystem design notes
src/zephyr/   Zephyr firmware and Renode co-simulation assets
agents/       Project engineering and formatting guidance
```

## Documentation

- [System architecture](wiki/architecture.md)
- [Silo](wiki/architecture-silo.md), [Stalks](wiki/architecture-stalks.md), and [Heist](wiki/architecture-heist.md)
- [Rube](wiki/architecture-rube.md), [Symph](wiki/architecture-symph.md), and [Swarm](wiki/architecture-swarm.md)
- [Crew](wiki/architecture-crew.md), [Karst](wiki/architecture-karst.md), and [Cove](wiki/architecture-cove.md)
