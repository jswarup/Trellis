# Segue Formatting Rules (Adapted from Trellis)

All Rust source files in Segue must adhere to the following formatting directives:

## 1. File Preambles
- Every source file begins with a filename banner:
  ```rust
  // <filename>.rs ---------------------------------------------------------------------------------------------------
  ```

## 2. File & Block Structure
- **Indentation**: 4 spaces (`indent_size = 4`).
- **Line Endings**: Unix line endings (LF).

## 3. Separator Lines
- **Blank Lines**: An empty line must always precede and succeed every separator line:
  ```rust
  //-------------------------------------------------------------------------------------------------
  ```

## 4. Struct & Data Members
- Struct fields use leading underscore (e.g. `_First`, `_Last`, `_Ptr`, `_Size`, `_Capacity`) when encapsulating raw storage or matching Trellis representations.
- Visual alignment: Where applicable, align struct fields and initializations cleanly.

## 5. Testing Layout
- Every subsystem/component module folder contains an `_test/` subfolder (e.g., `src/silo/_test/mod.rs`).
- Tests register using `segue_test!`, `segue_console_test!`, or `segue_example_test!`.
- Assertions use `segue_assert!` and `segue_assert_eq!`.
