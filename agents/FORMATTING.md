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

- End each file with a separator line
  ```rust
  //-------------------------------------------------------------------------------------------------
  ```


## Indentation & Line Endings:
- 4 spaces, Unix (LF). No literal tabs.  and use UTF-8 encoding.
- Align related struct fields and multi-line initializers .
- Indent chained continuations consistently with their enclosing expression;
- Spaces up to next tab stop (col % 4 == 0) after fn, let, and use. Have atleast 3 spaces
- align all variable and type-declaration and their definition, ( structs, local variables, constructions) to tabstop that follow 3 spaces.
- in function argument definitions need a space following :

## Whitespace and Braces

- Use four spaces tab for indentation, LF line endings, and UTF-8 encoding. Do not use hard tabs.
- uses next-line braces for definitions and control flow.
- For multi-line imports, keep the outer `{` on the `use` line:
  ```rust
  use crate::flux::{
      FieldExp, FieldImp, IFluxExportSource, IFluxImportSource,
  };
  ```

## Alignment
- **In-line Comments**: All trailing/in-line comments (comments sharing a line with code, excluding full-line comments and separator lines) must be formatted to begin at column 72 onwards.
- **Type Alignment**: All type definitions for struct data members must be vertically aligned into consistent columns across field declarations.
- **Initialization Alignment**: The RHS (right-hand side) in struct field initializations must also be vertically column-aligned.
