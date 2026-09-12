# Trellis Project Guidelines & Engineering Standards (Derived from Kosh)

## 1. Architecture & Performance
- **Zero-Unnecessary Allocations**:
  - Eliminate redundant heap allocation overhead in hot paths.
  - Prefer stack allocations, non-owning borrowed views, and move semantics.
  - Keep data representations compact and memory contiguous.
- **Interface Pattern (`IFoo`)**:
  - Define a strict 1:1 corresponding interface/abstract base (`IFoo`) for public operational methods of concrete classes (`Foo`).
  - Keep internal plumbing, mutable state buffers, and execution loops out of the interface.
  - Keep interfaces minimal, concise, and sufficient.
- **Standard Indexing & Numerics**:
  - Use fixed-width integer types (e.g., `uint32_t`, `uint64_t`) from `<cstdint>` where sizes and bit-widths matter.
  - Standardize on unsigned 32-bit (`uint32_t`) for container indexing and counts.

## 2. Strict Formatting & Syntax Standards
Adhere strictly to [`FORMATTING.md`](file:///c:/Work/Oogway/Trellis/agents/FORMATTING.md):
- **Indentation & Line Endings**: 4 spaces, Unix (LF) line endings.
- **Template Indentation**: `template <...>` keyword lines must be placed at **one indentation level less** than the function / method / constructor name definition line (e.g., member template methods in a struct at 4 spaces have `template <...>` at 0 spaces).
- **Opening Braces**: On a **newline** *only* for `struct`, `class`, `enum`, `namespace`, and function definitions. On the **same line** for control flow (`if`, `else`, `switch`, `while`, `for`, `do`).
- **Spacing in Parentheses & Brackets**:
  - Open parenthesis `(` followed by a space if non-empty: `( val)`. Empty stays `()`.
  - Open angular bracket `<` followed by a space if non-empty: `Buff< T>`. Empty stays `<>`. Less-than `<` unaffected.
- **Return Statements**: `return` MUST always be on its own line, never inline.
- **In-Line Comments**: Must align to column 72 onwards.
- **Separator Lines**: `//---------------------------------------------------------------------------------------------------------------------------------` must have an empty line preceding and succeeding it (unless adjacent to a comment).
- **Naming Conventions**:
  - Types / Classes / Structs: `PascalCase` (e.g., `SimEngine`).
  - Interfaces: `PascalCase` with `I` prefix (e.g., `IModule`).
  - Functions / Methods: `PascalCase` (e.g., `Advance()`, `New()`).
  - Local Variables & Arguments: `camelCase` (e.g., `modId`, `initialVal`).
  - Struct/Class Fields: `PascalCase` preceded by an underscore `_` (e.g., `_Data`, `_Size`).
- **Vertical Alignment**:
  - Type definitions for struct/class data members must be vertically column-aligned.
  - RHS in field initializations must also be vertically column-aligned.

## 3. Code Organization & Includes
- **File Preambles**: All `.h` files begin with a filename banner `// <filename>.h ---...` followed by `#pragma once` (do not use legacy `#ifndef` sentinels). All `.cpp` files begin with `// <filename>.cpp ---...` (no `#pragma once`).
- Group `#include` directives at the file header.
- Avoid inline full-path qualifications; include necessary headers at the top of the file.

## 4. Build & Verification
- Verify all modifications using the project build system (`CMake` in `tools/build`).
- Ensure code compiles cleanly with zero warnings before declaring completion.

## 5. Execution Principles & Agent Workflow
- **Think Before Coding**:
  - State assumptions explicitly.
  - Surface trade-offs before implementing changes.
  - If a simpler approach exists, push back. If unclear, stop and ask.
- **Surgical Precision**:
  - Touch only what you must. Do not "improve" adjacent code, comments, or formatting.
  - Clean up only your own mess (remove unused variables/headers orphaned by your changes).
- **Simplicity First**:
  - Write the minimum code needed to solve the problem. Do not build speculative features, "flexible" abstractions, or unnecessary error handling.
- **Goal-Driven Execution**:
  - Define clear success criteria.
  - Loop and verify independently before declaring completion.
- **Verification**:
  - Always verify modifications with a clean build.
- **Commit Directive**:
  - Never run `git commit` or `git push` without an explicit directive from the user.
- **Always Review**:
  - Review the final diff for minimal footprint and strict compliance with project invariants before declaring completion.

