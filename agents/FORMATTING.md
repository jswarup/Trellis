# Trellis Formatting Rules (Derived from Kosh)

All C++ source and header files in Trellis must adhere to the following formatting directives:

## 1. File & Block Structure
- **Indentation**: 4 spaces (`indent_size = 4`).
- **Line Endings**: Unix line endings (LF).
- **Opening Braces**: Opening braces `{` must be placed on a newline *only* for `struct`, `class`, `enum`, `namespace`, and function definitions. For all control flow and block statements (`if`, `else`, `switch`, `while`, `for`, `do`), the opening brace `{` must remain on the same line.

## 2. Spacing in Parentheses & Brackets
- **Open Parenthesis**: An open parenthesis `(` must always be followed by a space, unless it encloses nothing:
  - Non-empty: `( val)`, `( a, b)`
  - Empty: `()`
- **Open Angular Bracket**: An open angular bracket `<` (used for template parameters) must always be followed by a space, unless it encloses nothing:
  - Non-empty: `Buff< T>`, `std::vector< uint32_t>`, `template< typename T>`
  - Empty: `<>`
  - Comparison operators (`<`) are unaffected: `i < count`.

## 3. Local Variable Declarations
- **Naming Convention**: All local variables and parameters must be named using `camelCase` (e.g., `myLocalVar`, `bufferSize`).
- **Return Statements**: `return` MUST always be on its own line, never inline.

## 4. Separator Lines
- **Blank Lines**: An empty line must always precede and succeed every separator line:
  `//---------------------------------------------------------------------------------------------------------------------------------`
  - *Exception*: When the line above starts with a comment itself, or if the line below is a comment, no empty line is required.

## 5. In-line Comments
- **Alignment**: All trailing/in-line comments (comments sharing a line with code, excluding full-line comments and separator lines) must be formatted to begin at column 72 onwards.

## 6. Struct & Class Data Members
- **Naming Convention**: All struct/class fields (data members) must be named in `PascalCase` preceded by an underscore `_` (e.g., `_Data`, `_Size`, `_Points`).
- **Type Alignment**: All type definitions for struct/class data members must be vertically aligned into consistent columns across field declarations.
- **Initialization Alignment**: The RHS (right-hand side) in struct field initializations must also be vertically column-aligned.

## 7. Naming Conventions Summary
- **Types / Structs / Classes / Enums**: `PascalCase` (e.g., `SimEngine`, `FieldDesc`).
- **Interfaces / Abstract Base Classes**: `PascalCase` with `I` prefix (e.g., `IModule`, `IAccess`).
- **Functions / Methods**: `PascalCase` (e.g., `Advance()`, `New()`).
- **Local Variables & Parameters**: `camelCase` (e.g., `initialVal`, `modId`).
- **Data Members (Fields)**: `PascalCase` preceded by an underscore `_` (e.g., `_Data`, `_Size`).

## 8. Template Declarations
- **Indentation**: All `template <...>` keywords must be placed at **one indentation level less** than the function / method / constructor name definition line.
  - For member functions and constructors declared within a `class` or `struct` (where the method definition line is indented 4 spaces), the `template <...>` prefix must be indented at 0 spaces (outdented by 1 level):
    ```cpp
    struct Foo
    {
    template < typename T>
        void Bar( T val);
    };
    ```
  - For file- or namespace-scope functions (indented at 0 spaces), the `template <...>` line remains at 0 spaces:
    ```cpp
    template < typename T>
    void GlobalFunc( T val);
    ```
  - For nested types (e.g. methods at 8 spaces indentation), the `template <...>` prefix must be placed at 4 spaces.
