# Rube Architecture

## Purpose

`rube` is a structural digital-circuit builder and simulator. It separates circuit construction from execution: `Layout` creates and freezes a module/port/netlist description, then `SimEngine` compiles that description into trigger storage and executable warps.

## Structural model

- `Module` identifies a named circuit unit, its parent, input/output port ranges, child ranges, and optional `KernelKind`.
- `PortId` and `PortDesc` identify ports and their types. Port ranges are stored as `silo::USeg` values into layout-owned collections.
- `Layout` owns modules, ports, hierarchy, net connections, and the final port-to-trigger map.
- `Netlist` records drivers and uses `silo::DisjointSet` to merge electrically equivalent ports. It assigns one trigger to each connected root and preserves trigger type metadata.
- Gate and latch headers provide reusable modules, including boolean gates, an RS latch, and larger compositions such as adders.

## Build pipeline

1. A caller constructs modules and their ports in a mutable `Layout`.
2. `Connect` validates hierarchy-aware connection direction: sibling-to-sibling, pass-down, or pass-up connections.
3. The netlist records the driver and unions the source/sink port sets. Conflicting drivers are rejected.
4. `Freeze` seals the layout and derives hierarchy ranges, descendants, trigger assignments, and executable kernel/warp information.
5. `SimEngine::Create` copies the compiled port-to-trigger map, builds trigger storage, and compiles fast warps.

A layout is intended to be frozen before simulation. The engine then treats its topology as immutable while mutable signal values live in trigger storage.

## Signal model

`Reg` represents four-state digital values: true, false, unknown (`X`), and high impedance (`Z`). Kernel operations evaluate bitwise logic, arithmetic, shifts, and inversion over packed raw values. `TriggerWad<uint64_t>` maintains current and future signal state for the compiled trigger set.

Port access is translated through the layout's port-to-trigger table. Setting a port writes an immediate or future trigger value; reading a port resolves its trigger's current value and can also coerce it to boolean.

## Simulation modes

- **Serial mode** repeatedly evaluates pending changes through compiled warps until the circuit settles or the cycle limit is reached.
- **Parallel mode** partitions eligible work and uses `heist::Atelier` to evaluate it concurrently. The mode is selected on `SimEngine` after creation.

The public settle operation returns the number of cycles used, allowing callers and tests to detect whether propagation occurred.

## Dependencies and consumers

`rube` depends heavily on `silo` for storage, ranges, and union-find, and on `stalks`/`heist` for parallel execution support. It is independent of `swarm` at runtime, although both use common data and scheduling layers. `cove` tests logic truth tables, gates, latches, and serial/parallel adder behavior.

## Invariants

- Port IDs index layout-owned port tables and preserve input/output direction.
- A sink has at most one distinct driver.
- All ports in one electrical equivalence class share a trigger.
- Topology must be frozen before the simulation engine is created.
- Unknown and high-impedance values are preserved by the four-state logic rules rather than collapsed into ordinary booleans.
