# Shard: Composable Parser Combinator Framework

**Path:** `src/shard/`  
**Crate Member:** `trellis::shard`  
**Status:** Parsing & Grammar Framework

---

## 1. Module Overview & Mission

`shard` is Trellis's composable, PEG-style (Parsing Expression Grammar) parser combinator framework. It allows complex formal syntax—including JSON, Wavefront OBJ, PTS point clouds, and VCD digital waveforms—to be specified directly as executable grammar trees in Rust without requiring external parser generator tools (such as Lex/Yacc or Pest).

### Design Principles
- **Stream-Driven (`flux::IStream`)**: Operates directly on abstract streams with zero-copy backtracking cursors (`Marker`).
- **Composable Operator Grammar**: Grammars compose naturally using operator overloads (`>>` for sequence, `|` for choice, `*` for repetition).
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

### 4.2 Semantic Actions (`ActionShard`)
Attaches closures directly to the grammar tree:
```rust
let parse_coord = Real().Action(|val: f64| {
    coords.push(val as f32);
});
```

### 4.3 High-Performance Numeric Parsing (`numbers.rs`)
Rather than accumulating ASCII strings and calling `f64::from_str`, `RealShard` and `UIntShard` accumulate mantissas and exponents incrementally using integer multiplication, yielding significantly faster parsing times for large numerical datasets (such as 10M-point PTS point clouds).

---

## 5. Integration Boundaries

- **Upstream Dependencies**: `silo` (`Buff`, `Stash`), `flux` (`IStream`, `FixedStream`).
- **Downstream Consumers**:
  - `fleck`: Parses Wavefront OBJ (`waveobjio.rs`) and PTS point clouds (`ptio.rs`).
  - `rube`: Parses VCD waveforms (`vcdio.rs`).
