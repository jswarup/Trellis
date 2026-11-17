# Silo: Storage, Traversal & Memory Primitives

**Path:** `src/silo/`  
**Crate Member:** `trellis::silo`  
**Status:** Core Foundation Module

---

## 1. Module Overview & Mission

`silo` is Trellis's foundational memory, storage, and traversal subsystem. It was engineered from first principles to enforce explicit ownership, contiguous memory layout, index-based referencing with 32-bit offsets (`u32`), zero-copy slicing, and minimal allocator pressure.

### Design Principles
- **Contiguous Heap Storage**: Heavy preference for cache-local arrays over pointer-chasing linked graphs.
- **32-bit Indexing (`u32`)**: Most data sizes in Trellis fit within 4 billion elements. Using `u32` avoids 64-bit reference overhead in dense structures.
- **Zero Prohibited Types**: Strictly zero `std::rc::Rc`, zero `std::cell::Cell` / `std::cell::RefCell`, and zero `Option<u32>` (all optional indices use `u32::MAX` sentinels).
- **Borrowed Views (`Arr` / `MutArr`)**: Direct equivalents of slices (`&[T]` and `&mut [T]`) but designed to interoperate smoothly with C-style raw pointers, memory reinterpretation, and aliasing contracts.
- **Explicit Growth vs. Fixed Capacity**: Clear separation between dynamically growable accumulators (`Stash<T>`) and frozen, immutable heap buffers (`Buff<T>`).
- **Structured Traversal (`USeg`)**: Project algorithms use `Traverse` and `USeg::Traverse` rather than raw integer ranges or native `for` loops.

---

## 2. Component Hierarchy & File Map

```mermaid
flowchart TD
    subgraph Storage
        Buff[buff.rs: Buff&lt;T&gt;]
        Stash[stash.rs: Stash&lt;T&gt;]
    end
    subgraph Views
        Arr[arr.rs: Arr&lt;'a, T&gt;]
        MutArr[arr.rs: MutArr&lt;'a, T&gt;]
    end
    subgraph Algorithms
        USeg[useg.rs: USeg Range Indexer & Traverse]
        Fifo[fifo.rs: Fifo&lt;T&gt; Circular Queue]
        Stk[stk.rs: Stk&lt;T&gt; Atomic Stack]
        Dset[dset.rs: DisjointSet Union-Find]
    end
    subgraph TypeCasting
        Cast[cast.rs: Pointer Casting & Reinterpretation]
        Traits[traits.rs: IArr, IArrMut Indexing Traits]
    end
    
    Stash -->|Freeze / ExtractBuff| Buff
    Buff -->|Arr / MutArr| Arr
    Buff -->|MutArr| MutArr
    USeg -->|Index Slicing| Arr
    Cast --> Arr
    Cast --> MutArr
```

| File | Primary Types & Functions | Description |
|---|---|---|
| `buff.rs` | `Buff<T>`, `BuffGuard<T>` | Contiguous owned heap buffer with fixed capacity, panic-safe unwinding, and RAII deallocation. |
| `stash.rs` | `Stash<T>` | Amortized growable buffer; used during construction, can be frozen into a `Buff<T>` via `.ExtractBuff()`. |
| `arr.rs` | `Arr<'a, T>`, `MutArr<'a, T>` | Borrowed immutable and mutable array views with slice conversion and bounds-checked indexing. |
| `useg.rs` | `USeg`, `Traverse`, `Span` | Compact `[start, len)` range segment abstraction supporting binary search, partition, and structured traversal. |
| `fifo.rs` | `Fifo<T>` | Power-of-two circular ring buffer for FIFO queues. |
| `stk.rs` | `Stk<T>` | Lock-free / atomic stack representation using CAS operations. |
| `dset.rs` | `DisjointSet` | Array-backed disjoint-set (union-find) with path compression and rank optimization. |
| `cast.rs` | `ICastExt`, `IAllocRawExt`, etc. | Zero-copy reinterpretation traits for raw pointers and byte arrays. |
| `traits.rs` | `IArr`, `IArrMut` | Common indexing abstraction for containers exposing contiguous memory. |

---

## 3. Core Data Structures & Types

### 3.1 `Buff<T>`: Contiguous Fixed Heap Storage
`Buff<T>` is an owned, heap-allocated buffer of fixed length $N$. Unlike `Vec<T>`, it has no separate capacity field; its allocation size exactly equals its element count.
- **Memory Layout**: A single 64-bit pointer and a 32-bit count:
  ```rust
  pub struct Buff<T> {
      pub(crate) ptr: *mut T,
      pub(crate) len: u32,
  }
  ```
- **Drop Semantics**: In `drop`, it explicitly drops each element from $0 \dots N-1$, then deallocates the backing memory using Rust's global allocator layout.
- **Dispenser Constructor**: `Buff::FromDispenser(count, |idx| expr)` populates elements with RAII safety using an inner unwinding guard (`BuffGuard<T>`).

### 3.2 `Stash<T>`: Growable Builder
`Stash<T>` is a dynamic, growable buffer similar to `Vec<T>`, but with a specialized conversion method `.ExtractBuff()` that transfers heap ownership directly into a `Buff<T>` without reallocating if capacity matches length.
- **Growth Strategy**: $1.5\times$ or $2\times$ geometric growth.
- **Pre-allocation**: `Stash::WithCapacity(n)` ensures zero-allocation collection pipelines.

### 3.3 `Arr<'a, T>` & `MutArr<'a, T>`: Borrowed Array Slices
Explicit fat pointer views into contiguous storage:
- `Arr<'a, T>` holds `ptr: *const T`, `len: u32`, and `PhantomData<&'a T>`.
- `MutArr<'a, T>` holds `ptr: *mut T`, `len: u32`, and `PhantomData<&'a mut T>`.
- **Unsafe Aliasing**: `MutArr::Alias(&self) -> MutArr<'a, T>` allows unsafe explicit duplication when partitioning across non-overlapping worker indices.

### 3.4 `USeg`: Range & Structured Traversal
`USeg` is an unsigned half-open interval $[start, start + len)$:
```rust
pub struct USeg {
    pub _Start: u32,
    pub _Len:   u32,
}
```
- Implements `Traverse(&self, mut f: impl FnMut(u32))` replacing native `for` loops across all core project algorithms.
- Provides closed/open interval arithmetic, intersection testing, binary search, and contiguous span slicing.

---

## 4. Feature-by-Feature Deep Dive

### 4.1 Strict Architectural Directives Adherence
1. **Zero Prohibited Types**: No `Rc`, `Cell`, `RefCell`, or `Arc` wrapped domain models.
2. **Sentinel Invariant**: Optional indices never use `Option<u32>` (which wastes space and complicates FFI); `u32::MAX` represents unassigned or null indices.
3. **Structured Range Traversal**: Slicing and iteration over buffer segments use `USeg::Traverse` to maintain consistent loop bounds and avoid off-by-one indexing errors.

### 4.2 Panic-Safe RAII Construction (`BuffGuard`)
When constructing buffers of non-copy types, if a constructor panics midway through element generation:
- `BuffGuard` catches the unwind.
- Drops only the elements that were successfully initialized up to that index.
- Deallocates the uninitialized memory block, preventing memory leaks during unwinding.

---

## 5. Integration Boundaries

- **Upstream Dependencies**: None (pure core foundation).
- **Downstream Consumers**:
  - Foundational to **every single subsystem** in Trellis (`stalks`, `heist`, `flux`, `shard`, `fresco`, `rube`, `karst`, `swarm`, `fleck`, `fenst`, `fascia`, `zephyr`, `python`).
