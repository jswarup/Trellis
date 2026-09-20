# Segue

Segue is an algorithms and systems playground in Rust, designed with a 2-tier modular architecture inspired directly by [Trellis](../Trellis) and Kosh.

It emphasizes zero-unnecessary allocations, contiguous memory representations, explicit ownership, and a dedicated self-registering test and assertion framework (`cove`).

## Architecture & Components

Segue adopts a 2-tier component layout where each subsystem resides in its own module and contains a dedicated `_test` subfolder:

| Component | Responsibility |
| --- | --- |
| `cove` | Self-registering test harness, assertions, console runner, and flag dispatcher (`-test`, `-c`, `-e`). |
| `silo` | Contiguous storage: `Buff<T>` (16-byte fixed-capacity heap buffer), `Arr<T>` / `MutArr<T>` (non-owning views into `Buff[0, _Cap]`), `Stash<T>` (24-byte dynamic builder extracting into `Buff`), `Stk<T>` (atomic stack view), `Seg` / `USeg` (closed ranges), `DisjointSet` (Union-Find), `Fifo` (circular queue), and `IArr` / `IArrMut` traits. |
| `stalks` | Synchronization and scheduling primitives (scaffolding for Segue's custom work-stealing engine). |

Every component folder maintains an `_test/` subfolder with test definitions:
```text
src/
├── cove/           Self-registering test and assertion framework
│   └── _test/      Cove verification tests
├── silo/           Contiguous memory buffers, views, and algorithms
│   ├── arr.rs      Non-owning borrowed views (Arr<T>, MutArr<T>)
│   ├── buff.rs     Fixed-capacity 16-byte owning heap buffer (Buff<T>)
│   ├── stash.rs    Dynamic builder container (Stash<T>) extracting into Buff
│   ├── stk.rs      Non-owning lock-free atomic stack view (Stk<T>)
│   ├── seg.rs      Closed integer segment range (Seg, USeg)
│   ├── dset.rs     Union-Find with path compression (DisjointSet)
│   ├── fifo.rs     Static zero-heap circular buffer (Fifo<T, N>)
│   ├── traits.rs   Indexed contiguous traits (IArr, IArrMut)
│   └── _test/      Silo unit, console, and example tests
├── stalks/         Scheduling and work-stealing primitives
│   └── _test/      Stalks tests
├── lib.rs          Library root & cargo test integration
└── main.rs         CLI console runner and flag processor
```

## Running Tests & Examples

The console runner supports fine-grained test execution and assertion masking:

### 1. Run all tests with assertions enabled (`-test`)
```powershell
cargo run -- -test
```
Use `-v 1` or `-v 2` for detailed per-test output and assertion traces:
```powershell
cargo run -- -test -v 2
```

### 2. Filter tests by name
Pass an argument to execute all tests matching the given pattern:
```powershell
# Run with assertions enabled
cargo run -- -test Silo

# Run matching tests without -test (assertions bypassed)
cargo run -- Silo
```

### 3. Run console tests (`-c`)
Runs tests marked as `console` or `example` with console output enabled. If `-test` is omitted, assertions are bypassed:
```powershell
cargo run -- -c
```

### 4. Run examples (`-e`)
Runs tests marked as `example` with console output enabled. If `-test` is omitted, assertions are bypassed:
```powershell
cargo run -- -e
```

### 5. Standard Cargo Test
All Cove tests are also integrated into the standard cargo test harness:
```powershell
cargo test
```

## Geometry Viewer

The OBJ/PTS GPU viewer, controls, verification commands, and remaining milestones
are documented in [docs/geometry-viewer.md](docs/geometry-viewer.md).

## Windows MSVC Debugging & Natvis

Segue provides full MSVC debugger integration:
- Visual Studio Code launch profiles configured with `cppvsdbg` targeting `target/debug/segue.exe`.
- Natvis visualizers defined in [`segue.natvis`](file:///c:/Work/Oogway/Segue/segue.natvis) for instant visual inspection of `Buff<T>`, `Seg`, and `TestContext` structures in debug watches.
- Pre-launch build automation via [`.vscode/tasks.json`](file:///c:/Work/Oogway/Segue/.vscode/tasks.json).

## Engineering Standards

Coding conventions, formatting directives, and architectural principles are detailed in:
- [`agents/AGENTS.md`](file:///c:/Work/Oogway/Segue/agents/AGENTS.md)
- [`agents/FORMATTING.md`](file:///c:/Work/Oogway/Segue/agents/FORMATTING.md)

The Zephyr execution-flavor architecture and configuration model are described in:
- [`docs/zephyr-vm-design.md`](docs/zephyr-vm-design.md)

The local Andes AE350/N25 Renode platform and firmware layout are described in:
- [`tools/zephyr-firmware/README.md`](tools/zephyr-firmware/README.md)
