# Segue Rust Formatting Guide

`rustfmt.toml` defines the enforced mechanical style. This guide records the source-level conventions that are not fully expressed by `rustfmt`.

## File Layout

- Begin implementation files with a filename banner, for example:
  ```rust
  // arr.rs ---------------------------------------------------------------------------------------------------------
  ```
- Use separator lines to divide substantial top-level sections or major implementation groups:
  ```rust
  //-------------------------------------------------------------------------------------------------
  ```
- Surround a separator with one blank line when it appears within a file. Do not add a separator merely to frame an empty block.
- Keep tests in the component's established `_test.rs` or `_test/mod.rs` layout.

## Whitespace and Braces

- Use four spaces tab for indentation, LF line endings, and UTF-8 encoding. Do not use hard tabs.
- Let `rustfmt` control brace placement. The repository configuration currently uses next-line braces for definitions and control flow.
- For multi-line imports, keep the outer `{` on the `use` line:
  ```rust
  use crate::flux::{
      FieldExp, FieldImp, IFluxExportSource, IFluxImportSource,
  };
  ```
- Align related struct fields and multi-line initializers when doing so improves scanning. Do not manually fight `rustfmt`.
- Indent chained continuations consistently with their enclosing expression; let `rustfmt` make the final decision.

## Naming and Syntax

- Use `PascalCase` for types, functions, and methods.
- Prefix pure interface traits with `I`, such as `IArr` and `IStream`.
- Use `camelCase` for local variables and parameters.
- Keep standard Rust trait implementation methods in `snake_case`.
- Use private, underscore-prefixed struct fields, such as `_First`, `_Size`, and `_Ptr`.
- Place `return` on its own statement line.

## Traversal and Conversions

- Do not use `for` loops or integer ranges in project algorithms. Use `USeg` and the Segue traversal methods (`Traverse`, `TraverseRev`, `TraverseMut`, `TraverseRevMut`, and `Span`).
- Use Segue search and sort operations rather than the corresponding slice methods.
- Prefer `From` and `.into()` to routine call-site casts. Accept `impl Into<u32>` or another appropriate conversion bound when it keeps the call site cast-free.

## Test Registration

Register tests with the existing `cove` patterns: `segue_test!`, `segue_console_test!`, `segue_example_test!`, or `#[test]` where that module already uses it. Use the project's assertion helpers or Rust assertions as appropriate.
