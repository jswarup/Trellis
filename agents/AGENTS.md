# Segue Project Guidelines & Engineering Standards (Derived from Trellis & Kosh)

## 1. Architecture & Performance
- **Zero-Unnecessary Allocations**:
  - Eliminate redundant heap allocation overhead in hot paths.
  - Prefer stack allocations, non-owning borrowed views (e.g. slices, `Seg`), and move semantics.
  - Keep data representations compact and memory contiguous.
- **Two-Tier Framework Structure**:
  - Core components are modular subsystems (e.g., `cove`, `silo`, `stalks`, etc.).
  - Every component folder has its own `_test` subfolder containing unit tests, console tests, and example tests.
- **Self-Registering Test Harness (`cove`)**:
  - Tests register statically via `cove` macros.
  - Running `-test` executes all tests and enforces assertions.
  - Running `-c` runs console tests with console outputs enabled; if `-test` is omitted, assertions are bypassed.
  - Running `-e` runs example tests with console outputs enabled; if `-test` is omitted, assertions are bypassed.
  - Supplying a filter argument executes matching tests (`suite::name`).
- **Standard Indexing & Numerics**:
  - Use fixed-width integer types (`u32`, `u64`, `usize`).
  - Standardize on unsigned 32-bit (`u32`) for container indexing, counts, and segment bounds where sizes and bit-widths matter, matching Trellis.
- **No External Serialization / Async Dependencies**:
  - Avoid `serde` and `tokio`. Segue features its own dedicated serialization and work-stealing execution framework.

## 2. Strict Formatting & Syntax Standards
Adhere strictly to [`FORMATTING.md`](file:///c:/Work/Oogway/Segue/agents/FORMATTING.md):
- **Indentation & Line Endings**: 4 spaces, Unix (LF) line endings.
- **Separator Lines**:
  `//-------------------------------------------------------------------------------------------------`
  must have an empty line preceding and succeeding it.
- **Naming Conventions**:
  - Types / Structs / Enums: `PascalCase` (e.g., `Buff`, `Seg`, `TestContext`).
  - Traits: `PascalCase` or `IPascalCase` for pure abstract interfaces.
  - Functions / Methods: `PascalCase` (or idiomatic snake_case with PascalCase aliases matching Trellis API where appropriate).
  - Struct Fields: `_PascalCase` or `_camelCase` with leading underscore for internal fields.

## 3. Windows MSVC Debugging
- All data structures must provide `.natvis` definitions in `segue.natvis` for pristine MSVC debugging inspection in VS Code and Visual Studio.

## 4. Verification Principles
- Ensure code compiles cleanly with zero warnings (`cargo check`, `cargo clippy`).
- Verify tests with both `cargo test` and `cargo run -- -test`.
