# Segue Project Guidelines & Engineering Standards (Derived from Trellis & Kosh)

## 1. Architecture & Performance

- **Zero-Unnecessary Allocations**:
  - Eliminate redundant heap allocation overhead in hot paths.
  - Prefer stack allocations, non-owning borrowed views (`Arr`, `MutArr`, `Seg`/`USeg`), and move semantics.
  - Keep data representations compact, cache-friendly, and memory contiguous.
  - Avoid storing `Stash` or dynamic buffers on the heap when stack allocation or fixed-capacity `Buff` suffices.

- **Traits Pattern (`IFoo`)**:
  - Keep interfaces minimal, concise, and sufficient.
  - All pure abstract traits strictly begin with the `I` prefix (e.g., `IArr`, `IArrMut`, `IStream`, `IFluxExportSource`, `IFluxImportSink`, `IGrammar`, `INotify`, `IWorker`, `IZephyrRuntime`).

- **Two-Tier Framework Structure**:
  - Core components are modular subsystems (`cove`, `silo`, `stalks`, `heist`, `flux`, `shard`, `swarm`, `karst`, `zephyr`, `crew`, `fascia`).
  - Every component folder has its own unit, console, and example tests in `_test.rs`.

- **Self-Registering Test Harness (`cove`)**:
  - Tests register statically via `cove` / `segue_test!` macros.
  - Running `-t` (or `-test`) executes all tests and enforces assertions.
  - Running `-c` runs console tests with console outputs enabled; if `-t` is omitted, assertions are bypassed.
  - Running `-e` runs example tests with console outputs enabled; if `-t` is omitted, assertions are bypassed.
  - Supplying a filter argument executes matching tests (`suite::name`).

- **Standard Indexing & Numerics**:
  - Use fixed-width integer types (`u32`, `u64`, `usize`).
  - Standardize on unsigned 32-bit (`u32`) for container indexing, counts, and segment bounds where sizes and bit-widths matter, matching Trellis.

- **No External Serialization / Async Dependencies**:
  - Avoid `serde` and `tokio`. Segue features its own dedicated serialization (`flux`) and work-stealing execution framework (`heist`).

---

## 2. Data Structures, Encapsulation & Construction

- **Data Encapsulation**:
  - Avoid public data members on structs (no `pub _Field`).
  - Struct fields must be internal and prefixed with an underscore (`_PascalCase` or `_camelCase`), e.g., `_First`, `_Last`, `_Ptr`, `_Size`, `_Cap`, `_Inner`.
  - Expose state and operations strictly via methods and accessors (`First()`, `Last()`, `Size()`, `Cap()`, `IsEmpty()`).

- **First-Class Segue Containers**:
  - NEVER leak or use native Rust slices (`[T]`, `&[T]`, `&mut [T]`) or `Vec<T>` in public interfaces or core components.
  - Use `Arr<'a, T>` for immutable borrowed contiguous views.
  - Use `MutArr<'a, T>` for mutable borrowed contiguous views.
  - Use `Buff<T>` for fixed-capacity, heap-allocated dynamic contiguous storage.
  - Use `Stash<T>` for growable LIFO/vector-like storage with dynamic resizing and fast lock-free `Stk` atomic stack extraction.
  - Use `Seg` / `USeg` for closed unsigned segment bounds `[_First, _Last]`.

- **Constructor Macros**:
  - Primary data structures should provide companion macros carrying out construction matching their type:
    - `Buff![ ... ]` for `Buff<T>` inline construction.
    - `Stash![ ... ]` for `Stash<T>` inline construction.
    - `USeg::New(first, last)` and `USeg::FromLen(len)` for segment ranges.

---

## 3. Strict Iteration & Segment Algorithms (Zero-Range, `u32`-Preserving)

- **NEVER use native Rust `for ... in ...` loops or integer range conversions (`0..count`, `0..=255`)**:
  - Native Rust iterators introduce bounds and range overhead inconsistent with Segue's zero-cost abstraction model.
- **Standard Traversal Idioms**:
  - **Forward iteration**:
    - `arr.Traverse(|item| { ... })`
    - `arr.AsArr().Traverse(|item| { ... })`
    - `USeg::FromLen(count).Traverse(|i| { ... })`
    - `USeg::New(first, last).Traverse(|i| { ... })`
  - **Reverse iteration**:
    - `arr.TraverseRev(|item| { ... })`
    - `useg.TraverseRev(|i| { ... })`
  - **Conditional / Early-Exit Traversal**:
    - `arr.Span(|item| -> bool { ... })`
    - `useg.Span(|i| -> bool { ... })`
  - **Mutable Traversal**:
    - `arr.TraverseMut(|item| { ... })`
    - `arr.TraverseRevMut(|item| { ... })`
- **Segment Algorithms**:
  - Always use `USeg` and container search/sort algorithms (`BinarySearch`, `LowerBound`, `UpperBound`, `LocateBound`, `QSort`, `Partition`, etc.) instead of native `slice::binary_search`, `slice::sort`, or `partition_point`.

---

## 4. Type Conversions & Zero-Cast Generic Signatures

- **Use `From` and `.into()`**:
  - Prefer implementing `From<T>` and leveraging `.into()` over manual re-interpretation or raw transmutes.
- **Zero-Cast Generic Arguments (`Into<u32>`, `Into<u8>`)**:
  - Instead of forcing callers to cast variables at call sites (e.g. `(0 as u32)` or `x as u32`), declare function/method parameters with `impl Into<u32>` or `C: Into<u8>`.

---

## 5. Naming Conventions

- **Types / Structs / Enums**: `PascalCase` (e.g., `Buff`, `Seg`, `Json`, `JsonOutStream`, `TestContext`). Avoid non-standard acronym casing like `JSon`.
- **Traits**: Strictly prefixed with `I` followed by `PascalCase` (`IArr`, `IArrMut`, `IStream`, `IFluxExportSource`, `IFluxImportSink`, `IGrammar`, `INotify`, `IWorker`, `IZephyrRuntime`).
- **Functions & Methods**: `PascalCase` (e.g., `New`, `Size`, `IsEmpty`, `Traverse`, `DispatchFieldExp`, `ParseGrammar`). Only standard library trait implementations (`fmt`, `from`, `try_from`, `drop`, `default`, `index`, `index_mut`) follow Rust standard `snake_case`.
- **Local Variables & Parameters**: Strictly `camelCase` (e.g., `let myLocalVar = ...`, `fn Parse(itemCount: u32)`). Never `snake_case`.
- **Struct Fields**: Internal fields must have a leading underscore with `PascalCase` or `camelCase` (`_First`, `_Last`, `_Ptr`, `_Size`, `_Cap`, `_Inner`). Never public.

---

## 6. Strict Formatting & Syntax Standards

Adhere strictly to [`FORMATTING.md`](FORMATTING.md):
- **Indentation & Line Endings**: 4 spaces (`indent_size = 4`), Unix line endings (LF).
- **Separator Lines**: `//-------------------------------------------------------------------------------------------------` must have an empty line preceding and succeeding it.
- **Opening Braces**:
  - On a **newline** exclusively for top blocks: `struct`, `trait`, `enum`, `impl`, and `fn` definitions.
  - On the **same line** for control flow (`if`, `while`, `match`, `loop`) and closures.
- **Keyword & Variable Spacing**:
  - `fn`, `let`, and `use` are followed by tab-stop equivalent spaces (aligned to next 4-column tab stop).
  - The outer-most opening brace `{` following `use` remains on the same line as the `use` path (e.g. `use     crate::{`).
- **Return Statements**:
  - `return` MUST always be on its own line, never inline.
- **Vertical Column Alignment**:
  - Struct field types must be vertically column-aligned across declarations.
  - Right-hand side expressions in struct initializations must be vertically column-aligned.
  - Line continuations must follow 2 tab stops (8 spaces) from the enclosing block.

---

## 7. Windows MSVC Debugging

- All core data structures must provide `.natvis` visualizers in `segue.natvis` for pristine MSVC debugging inspection in Visual Studio and VS Code.

---

## 8. Verification Principles

- Ensure all code compiles cleanly with zero compiler warnings (`cargo check --all-targets`, `cargo clippy`).
- Verify tests pass with both `cargo test` and `cargo test --lib`.
