# Shard: Composable Parser Combinator Framework

**Path:** `src/shard/`  
**Crate Member:** `trellis::shard`  
**Status:** Parsing & Grammar Framework

---

## 1. Module Overview & Mission

`shard` is Trellis's composable, PEG-style (Parsing Expression Grammar) parser combinator framework. It allows complex formal syntax—including JSON, IEEE-1364 Verilog, Wavefront OBJ, PTS point clouds, and VCD digital waveforms—to be specified directly as executable grammar trees in Rust without requiring external parser generator tools (such as Lex/Yacc or Pest).

### Design Principles
- **Stream-Driven (`flux::IStream`)**: Operates directly on abstract streams with zero-copy backtracking cursors (`Marker`).
- **Composable Operator Grammar**: Grammars compose naturally using operator overloads (`>>` for sequence, `|` for choice, `*` for repetition) and declarative macros (`ShardTree!`).
- **Embedded Semantic Actions**: `ActionShard` allows inline semantic callbacks to populate domain objects during successful branch matches.
- **Bounded Recursion**: Built-in recursion depth protection preventing stack overflows on malicious or heavily nested inputs.

---

## 2. Component Hierarchy & File Map

```mermaid
flowchart TD
    subgraph ParserCore
        Parser[parser.rs: Parser]
        Marker[parser.rs: Marker / Rewind]
        IGrammar[parser.rs: IGrammar Trait]
        TreeMacro[shardtree.rs: ShardTree! Macro DSL]
    end
    subgraph Combinators
        Leaves[leaves.rs: Str, Char, Keyword]
        BinShard[binshard.rs: Seq >>, Choice |]
        RepeatShard[repeatshard.rs: Optional ?, Repetition *]
        ActionShard[actionshard.rs: Semantic Action Callbacks]
    end
    subgraph PrebuiltGrammars
        Charset[charset.rs: Character Sets, Ranges]
        Numbers[numbers.rs: Int, UInt, Real, Hex]
        JsonShard[jsonshard.rs: Complete JSON Parser]
        PrimeShard[primeshard.rs: WSpc Whitespace, Comments]
    end

    Parser --> Marker
    IGrammar --> Leaves
    IGrammar --> BinShard
    IGrammar --> RepeatShard
    IGrammar --> ActionShard
    BinShard --> PrebuiltGrammars
    TreeMacro --> BinShard
```

| File | Primary Types & Functions | Description |
|---|---|---|
| `parser.rs` | `Parser`, `Marker`, `IGrammar` | Core parsing engine managing stream positions, backtrack checkpoints, and recursion limits. |
| `binshard.rs` | `BinShard` | Binary combinators: Sequential composition (`And` / `>>`) and Alternative choice (`Or` / `\|`). |
| `repeatshard.rs` | `RepeatShard` | Unbounded repetition (`*`), positive repetition (`+`), and optional match (`?`). |
| `actionshard.rs` | `ActionShard` | Attaches transformation functions or closure actions to grammar nodes upon match. |
| `leaves.rs` | `Str`, `Ch`, `Eof` | Terminal grammar leaves matching exact byte sequences, strings, or end-of-file. |
| `charset.rs` | `Charset` | Bitset-accelerated character class matching (alphanumeric, whitespace, custom ranges). |
| `numbers.rs` | `Int`, `UInt`, `Real`, `Hex` | High-speed numeric parsers directly extracting integers and floats without intermediate allocations. |
| `primeshard.rs` | `WSpc`, `PrimeShard` | Whitespace handling, line skipping, and lexical token framing. |
| `jsonshard.rs` | `Json`, `JSon` | Full standard RFC 8259 JSON parser grammar returning structured values. |
| `shardtree.rs` | `ShardTree!` | Declarative macro for constructing optimized combinator trees with inline action blocks. |

---

## 3. Core Data Structures & Types

### 3.1 `IGrammar` Trait
Every grammar shard implements `IGrammar`:
```rust
pub trait IGrammar {
    fn Match(&self, parser: &mut Parser) -> bool;
}
```

### 3.2 `Parser` & `Marker`
- **`Parser`**: Wraps a `&mut dyn IStream` from `flux`. Tracks the current character position and parse recursion depth (`_Depth`).
- **`Marker`**: Represents a save-point. If a speculative branch in a choice combinator (`A | B`) fails, the parser rewinds to the `Marker`'s byte offset with zero cost:
  ```rust
  let mut m = parser.Mark();
  if branch.Match(parser) {
      m.Commit();
      true
  } else {
      m.Rewind(parser);
      false
  }
  ```

---

## 4. Feature-by-Feature Deep Dive

### 4.1 Algebraic Grammar Composition
Shards compose seamlessly:
```rust
// Matches: "point" <whitespace> <x: Real> <whitespace> <y: Real>
let grammar = Str("point") >> WSpc() >> Real() >> WSpc() >> Real();
```

### 4.2 Declarative ShardTree Macro (`shardtree.rs`)
Allows embedding complex lexing grammars and semantic actions cleanly:
```rust
let tree = ShardTree!(
    [ "module" ] >> WSpc() >> Identifier >> WSpc() >> [ "(" ]
);
```

### 4.3 High-Performance Numeric Parsing (`numbers.rs`)
Directly consumes ASCII characters from the input stream and computes floating point values and integers using Horner's method, avoiding allocating intermediate string slices or invoking `str::parse::<f64>()`.

---

## 5. Integration Boundaries

- **Upstream Dependencies**: `flux` (`IStream`, `FixedStream`), `silo` (`Arr`).
- **Downstream Consumers**:
  - `rube::verilog_shard`: Parses IEEE 1364 Verilog netlists.
  - `rube::vcdio`: Parses multi-gigabyte VCD digital waveform files.
  - `fleck::waveobjio`: Ingests 3D Wavefront OBJ models.
  - `fresco::exprshard`: Parses infix symbolic algebraic expressions.
