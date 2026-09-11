# Trellis Workspace & Formatting Rules (Derived from Kosh)

Welcome to Trellis! You **MUST** strictly adhere to these non-standard formatting and workflow rules.

## 1. Strict Formatting & Syntax
- **Indentation & Braces**: 4 spaces, UNIX (LF) line endings. Opening braces `{` MUST be on a **newline** for `class`, `struct`, `enum`, `namespace`, and functions. For control flow (`if`, `switch`, `while`, `for`), keep `{` on the same line.
- **Template Indentation**: All `template <...>` keywords must be placed at **one indentation level less** than the function / method / constructor definition line (e.g., member template methods in a struct at 4-space indent have `template <...>` at 0 spaces).
- **Spacing in Brackets**: Open parenthesis `(` and angular bracket `<` MUST have a trailing space if not empty (e.g., `( val)`, `Buff< T>`). Empty remains `()` and `<>`.
- **Return Statements**: `return` MUST always be on its own line, not inline.
- **Comments & Separators**: Trailing comments must align to column 72. Separator lines (`//---...`) MUST be padded with one blank line before and after.

## 2. Naming Conventions
- **PascalCase**: Structs, Classes, Enums, Types (e.g., `SimContext`), Methods/Functions (e.g., `Advance()`).
- **Interfaces**: PascalCase with an `I` prefix (e.g., `IAccess`).
- **camelCase**: Local variables, function arguments (e.g., `initialVal`).
- **Struct Fields**: PascalCase preceded by an underscore `_` (e.g., `_Data`, `_Size`).
- **Alignment**: Struct/class field types and right-hand side initializations MUST be vertically aligned into consistent columns.

## 3. Code Organization
- **Includes**: All `#include` statements must be placed strictly at the file header, logically grouped.

## 4. Agent Workflow Rules
- **Verify**: After modifications, always verify with the project CMake build. Ensure it builds with zero warnings.
- **Commit**: Never commit without an explicit directive from the user.
- **Think Before Coding**: State assumptions explicitly. Surface trade-offs. If a simpler approach exists, push back. If unclear, stop and ask.

