# Segue Code Formatting & Style Guide (Adapted from Trellis)

All Rust source files in Segue must adhere strictly to the following formatting and syntactic directives:

---

## 1. File Preambles & Banners

Every source file begins with a top-of-file filename banner followed by a blank line and the first separator:
```rust
//-- <filename>.rs -------------------------------------------------------------------------------------------------

//------------------------------------------------------------------------------------------------------------------
```

---

## 2. Indentation, Line Endings & Encoding

- **Indentation**: 4 spaces (`indent_size = 4`). No hard tab characters (`\t`) in source files.
- **Line Endings**: Unix style (LF, `\n`).
- **Encoding**: UTF-8.

---

## 3. Separator Lines

- **Syntax**:
  ```rust
  //------------------------------------------------------------------------------------------------------------------
  ```
- **Blank Line Rule**: Exactly **one empty line** must precede and succeed every separator line.
- **Placement**: Place separators between major top-level blocks (e.g. between `struct`, `impl`, `trait`, macro definitions, and test suites).

---

## 4. Opening & Closing Braces

- **Newline `{` (Allman / Trellis Style)**:
  Exclusively for top-level definitions:
  - `struct` definitions
  - `enum` definitions
  - `trait` definitions
  - `impl` blocks
  - Function declarations (`fn`)
  - Macro rule bodies (`macro_rules!`)

  ```rust
  pub struct JsonOutStream<W: fmt::Write>
  {
      _OStr:         W,
      _Depth:        u32,
      _EntryFlg:     bool,
      _MultiLineFlg: bool,
  }

  impl<W: fmt::Write> JsonOutStream<W>
  {
      pub fn New(ostr: W, multiLineFlg: bool) -> Self
      {
          Self {
              _OStr:         ostr,
              _Depth:        0,
              _EntryFlg:     false,
              _MultiLineFlg: multiLineFlg,
          }
      }
  }
  ```

- **Same-Line `{` (K&R Style)**:
  For all control flow and closures:
  - `if condition { ... } else { ... }`
  - `while condition { ... }`
  - `loop { ... }`
  - `match value { ... }`
  - Closures: `|item| { ... }`

---

## 5. Keyword & Variable Spacing

- **Keyword Tab-Stop Spacing**:
  In top declarations and statement lines, the keywords `fn`, `let`, and `use` are followed by spaces padding up to the next 4-column tab stop:
  ```rust
  use     crate::silo::{Arr, Buff, USeg};

  fn      ComputeDelta(step: u32) -> u32
  {
      let     mut total = 0u32;
      let     baseVal = step * 2;
      total + baseVal
  }
  ```
- **Use Statement Braces**:
  The outer-most opening brace `{` following `use` remains on the same line as the `use` path:
  ```rust
  use     crate::flux::{
      FieldExp, FieldImp, IFluxExportSource, IFluxImportSource,
  };
  ```

---

## 6. Return Statements

- `return` MUST always be placed on its own line, never inline with an `if` expression or closure branch:
  ```rust
  // CORRECT:
  if !matched {
      return false;
  }

  // PROHIBITED:
  if !matched { return false; }
  ```

---

## 7. Vertical Column Alignment

- **Struct Data Member Declarations**:
  Align member names, colons, and types vertically:
  ```rust
  pub struct Point3D
  {
      _X: f64,
      _Y: f64,
      _Z: f64,
  }
  ```
- **Struct Initializations (RHS Alignment)**:
  Align the colons and right-hand side (RHS) values vertically in constructor bodies:
  ```rust
  let pt = Point3D {
      _X: 10.0,
      _Y: 20.0,
      _Z: 30.0,
  };
  ```
- **Line Continuations**:
  Continuation lines must be indented 2 tab stops (8 spaces) from the enclosing block's indentation level:
  ```rust
  let result = someComplexComputation()
          .TransformStepA()
          .TransformStepB();
  ```

---

## 8. Naming Conventions

| Entity | Convention | Example |
| :--- | :--- | :--- |
| **Types / Structs / Enums** | `PascalCase` | `Buff`, `Seg`, `JsonOutStream`, `FieldExp` |
| **Traits** | `IPascalCase` (Always `I` prefix) | `IArr`, `IArrMut`, `IStream`, `IGrammar`, `IZephyrRuntime` |
| **Functions / Methods** | `PascalCase` | `New`, `Size`, `IsEmpty`, `Traverse`, `DispatchFieldExp` |
| **Std Trait Impls** | `snake_case` (Rust std only) | `fmt`, `from`, `try_from`, `drop`, `default`, `index` |
| **Local Variables** | `camelCase` | `let itemCount = 0u32;`, `let mut isFirst = true;` |
| **Function Parameters** | `camelCase` | `fn Process(itemCount: u32, ostr: W)` |
| **Struct Data Members** | `_PascalCase` / `_camelCase` | `_First`, `_Last`, `_Ptr`, `_Size`, `_Cap` (Always private) |
| **Constants** | `UPPER_SNAKE` or `PascalCase` | `SZ_BITS`, `Str` |

---

## 9. Strict Zero-Range Iteration Patterns

Native Rust `for ... in ...` loops and integer range conversions (`0..n`, `start..=stop`) are **strictly prohibited**. Always use Segue's zero-cost segment traversal:

| Purpose | Prohibited Native Syntax | Required Segue Idiom |
| :--- | :--- | :--- |
| **Forward Range Loop** | `for i in 0..count { ... }` | `USeg::FromLen(count).Traverse(\|i\| { ... });` |
| **Inclusive Range Loop** | `for i in start..=stop { ... }` | `USeg::New(start, stop).Traverse(\|i\| { ... });` |
| **Reverse Range Loop** | `for i in (0..count).rev() { ... }` | `USeg::FromLen(count).TraverseRev(\|i\| { ... });` |
| **Array Iteration** | `for item in arr.iter() { ... }` | `arr.Traverse(\|item\| { ... });` |
| **Reverse Array Iteration**| `for item in arr.iter().rev() { ... }` | `arr.TraverseRev(\|item\| { ... });` |
| **Mutable Array Iteration**| `for item in arr.iter_mut() { ... }` | `arr.TraverseMut(\|item\| { ... });` |
| **Early-Exit Search/Test** | `for ... { if !ok { break; } }` | `arr.Span(\|item\| -> bool { ... });` |

---

## 10. Zero-Cast Generic Signatures

Avoid casting at invocation sites (`val as u32`). Expose generic `Into<u32>` or `Into<u8>` signatures on methods and functions:
```rust
// In method definition:
pub fn GetAt(&mut self, marker: impl Into<u32>) -> u8
{
    self._InStream.At(marker.into())
}

// At caller site (zero-cast):
let b = parser.GetAt(0); // No "0 as u32" needed!
```

---

## 11. Testing Conventions & Layout

- **Test Placement**:
  Every component module has its test suite in `_test.rs` or `_test/mod.rs` (e.g. `src/flux/_test.rs`, `src/shard/_tests.rs`).
- **Registration**:
  Tests register using the self-registering harness (`segue_test!`, `segue_console_test!`, `segue_example_test!`, or `#[test]`).
- **Assertions**:
  Use `assert!`, `assert_eq!`, `segue_assert!`, or `segue_assert_eq!`.
