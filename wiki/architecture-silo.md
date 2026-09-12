# Silo Architecture

## Purpose

`silo` is Trellis's low-level data and type-erasure layer. It provides compact, mostly value-semantic containers and non-owning views that are used by the scheduler, circuit simulator, and compute systems. The design favors contiguous storage, explicit sizes, fixed-width indexing, and minimal allocation in hot paths.

## Main building blocks

- `USeg` represents a half-open unsigned range and supplies traversal, slicing, sorting, bound lookup, and binary-search helpers.
- `Arr<T>` is a non-owning mutable contiguous view. It can expose subranges, swap elements, initialize indices, and convert to access facades.
- `IAccess<T>` is the read-only access abstraction. Its static form is optimized for concrete types; its dynamic form is a fat pointer backed by an access trait vtable.
- `IArr<T>` extends the dynamic facade with mutation operations over an externally owned sequence.
- `Buff<T>` owns a resizable contiguous allocation and can be generated, copied, moved, resized, extended, concatenated, or viewed as `Arr`, `IAccess`, and `IArr`.
- `Stk<T>` is a bounded stack view over an array with an externally supplied atomic size. CAS operations support concurrent push/pop and bulk import/export.
- `Stash<T>` is the growable owning collection. It combines `Buff` storage with an atomic size and exposes stack compatibility when needed.
- `DisjointSet` implements union-find with path compression and union-by-rank. `rube::Netlist` uses it to maintain electrically equivalent port groups.
- `traits.h` supplies compact type-erased references, owning type-erased pointers, vtables, and per-trait type metadata.

## Ownership and access model

The owning types are `Buff` and `Stash`. `Arr`, `USeg`, `IAccess`, `IArr`, and `Stk` are views or facades and do not generally own the element storage. This makes ownership visible at the call site and lets higher layers pass slices without copying.

Trait facades store an object pointer plus an operation table. `TraitMeta` assigns type identifiers and stores construction, destruction, move, size, alignment, and operation-table metadata. This gives selected polymorphic behavior without requiring every data structure to use a C++ virtual base.

## Concurrency model

`Stk` and `Stash` use `stalks::Atm<uint32_t>` for size coordination. `Stk::Push` and `Pop` reserve positions with compare-exchange; `Import` and `Export` transfer ranges between stacks. The collection layout remains contiguous, while synchronization is kept in the size/control word.

## Typical flow

1. A subsystem owns data in `Buff` or `Stash`.
2. It exposes a bounded `Arr` or `IAccess` view to algorithms and callers.
3. Algorithms operate on `USeg` ranges and avoid container-specific assumptions.
4. Concurrent queues use `Stk` views over preallocated storage.
5. Type-erased boundaries use `TRef`, `MTRef`, or `TPtr` where concrete template types cannot be propagated.

## Dependencies and consumers

`silo` depends on the C++ standard library and `stalks::Atm` for atomic stack support. `heist` uses `Buff`, `Stash`, and `Stk` for jobs and queues. `rube` uses `Stash`, `Buff`, `USeg`, and `DisjointSet` for layouts and netlists. `swarm` uses `Arr` and `Buff` for byte-oriented device buffers. `cove` tests the public behavior of the layer.

## Invariants

- Indexes and counts use `uint32_t` where the API requires stable width.
- Views must not outlive their backing storage.
- A `Stk` view's capacity is the size of its backing `Arr`; its logical size is the shared atomic size.
- Dynamic trait references are non-owning unless explicitly represented by `TPtr`.
