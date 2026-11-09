# Trellis

## Aims and Overview

Trellis is a Rust systems and algorithms framework equipped with a native desktop workbench. The project is designed to support high-performance 3D visualization of diverse dynamic data, originating from domains such as mechanical assemblies, ASIC/SoC simulations, and reaction kinetics. At its core, the software is built around a highly parallel simulation engine.

The framework consolidates several critical capabilities:

* Compact storage, parsing, and serialization
* Task execution and CPU/GPU-oriented compute
* Geometry viewing
* Circuit and memory-fabric simulation
* Guest-machine co-simulation

Architecturally, Trellis favors explicit ownership, contiguous storage where applicable, bounded queues, and deterministic, testable boundaries. As an active research and engineering workspace, its documentation deliberately separates currently implemented behavior from planned future work.

## Start here

From the repository root:

```powershell
# Build and validate the workspace.
cargo build --offline
cargo check --all-targets --offline
cargo test --offline

# Run the self-registering test runner with assertions enabled.
cargo run --offline -- -t Karst

# Start the desktop workbench.
cargo run --offline -- --ui
```

Offline commands require dependencies and the pinned Rust-GPU toolchain
components to be installed already. Root builds compile the shader workspace
member even when the intended runtime path is CPU-only.

The CLI also supports:

```text
trellis -t [filter]       Run all or matching tests with assertions enabled
trellis -c [filter]       Run console tests
trellis -e [filter]       Run example tests
trellis -v [0|1|2]        Set test-runner verbosity
trellis --help            Show all options
```

Most component tests use `jeeves_test!` and live beside their implementation
in `_tests.rs`; they also run through Rust's standard test harness.

## Architecture

The main library modules are:

| Area | Purpose |
| --- | --- |
| `silo` | Compact containers, borrowed views, fixed queues, and traversal primitives. |
| `cove` | Self-registering tests, assertions, and the command-line test runner. |
| `heist` and `stalks` | Job execution, work stealing, synchronization, and coroutine primitives. |
| `flux` and `shard` | Stream/field serialization and composable parsing. |
| `fleck`, `fenst`, and `fascia` | Geometry import, explorer providers, and the Iced desktop workbench. |
| `swarm`, `flock`, `symph`, and `drove` | Compute contracts, CPU kernels, shared math/shaders, and Rust-GPU artifacts. |
| `rube` | Digital-circuit simulation, VCD handling, and waveform models. |
| `karst` | Host, link, NoC, memory-channel, and VPU fabric simulation. |
| `crew` and `zephyr` | Guest communication, runtime simulation, and Renode integration. |

For the full dependency map, execution flows, and current implementation
boundaries, see the [architecture guide](wiki/architecture.md).

## Interactive Desktop Viewers & Workbench

Trellis integrates a native desktop application (`cargo run -- --ui`) built on Iced and wgpu that provides three specialized, high-performance visualization modalities:

### 1. Classical Circuit Schematic & Cask Viewer
Built on `fenst::circuit_layout` and `fascia::cask_view`, this viewer renders publication-grade 2D EDA block schematics and 3D Cask component scenes from any entity implementing `CircuitXplr`:
- **Topological Component Staging**: Submodules and gates are staged from left to right according to signal flow.
- **Perimeter Pin Terminals**: Inputs are positioned along the West (left) boundary with horizontal lead stubs, outputs along the East (right) boundary, accompanied by directional port arrowheads.
- **Orthogonal Manhattan Routing**: Interconnect wires route through dedicated collision-free channels with soldered junction dots (`•`) at net fanout split points.
- **Interactive Net & Pin Hit-Testing**: Hovering over wires dynamically highlights connected netpaths; hovering over terminal pins displays port names and driver/sink metadata.
- **Virtual Example Navigation**: Opening `example://` URIs allows interactive browsing and submodule drill-down for hardware models such as the 4x4 Dadda Multiplier (`example://Rube/DaddaMultiplier4x4`) and the Karst multi-die memory fabric (`example://Karst/System`).

### 2. GPU-Backed 3D Geometry Viewport
Embedded `wgpu` shader canvas rendering complex 3D meshes (Wavefront OBJ) and dense point clouds (PTS):
- **Real-Time Orbit Camera**: Interactive spherical orbit controls (`OrbitCamera`) supporting smooth azimuth, elevation, zoom, and pan.
- **Asynchronous Ingestion**: Multi-stage loading pipeline delegating file parsing and normal calculations to background `heist` worker threads, keeping the desktop UI responsive during multi-hundred-megabyte asset loading.

### 3. Digital Waveform Timing Viewer
Timeline diagram visualizer rendering IEEE 1364 four-state logic (`0`, `1`, `X`, `Z`/`I`) signal transitions parsed from standard Value Change Dump (VCD) traces:
- **Hierarchical Scope Browser**: Expandable signal scopes preserving net declaration hierarchy.
- **Interactive Navigation**: Horizontal timeline panning and temporal zooming with transition edge markers.

## Current capabilities

- The desktop workbench includes GPU-backed 3D geometry viewing, digital waveform inspection, and interactive 2D/3D circuit schematic capture.
- General compute executes on the CPU today. Backend contracts and a
  Rust-to-SPIR-V artifact exist, but built-in hardware compute dispatch is not
  yet complete.
- Rube provides circuit and VCD model infrastructure; Karst provides a
  cycle-stepped interconnect and memory-fabric model.
- Karst transport includes bounded backpressure handling, non-aliasing striped
  memory addressing, and explicit invalid-memory fault responses.
- Guest execution has library and Renode runtime paths. The hypervisor flavor
  remains planned.

Hardware and emulator verification are opt-in. GPU tests require a compatible
adapter and `TRELLIS_GPU_TEST=1`; Renode tests require its emulator, guest ELF,
and `TRELLIS_RUN_RENODE_TESTS=1`. Passing ordinary tests does not validate those
external integrations.

## Further reading

- [Architecture and folder guide](wiki/architecture.md)
- [Geometry viewer](wiki/geometry-viewer.md)
- [Karst parallelization plan](wiki/plans/karst-parallelization-plan.md)
- [GPU migration plan](wiki/plans/gpu-migration-plan.md)
- [Framework hardening plan](wiki/plans/framework-hardening-plan.md)
- [Zephyr VM design](wiki/zephyr-vm-design.md)
- [Firmware setup notes](tools/zephyr-firmware/README.md)

## Development

Follow the repository's [engineering directives](agents/AGENTS.md) and
[formatting guide](agents/FORMATTING.md). Debugger visualizers for MSVC are
provided in [`tools/trellis.natvis`](tools/trellis.natvis).
