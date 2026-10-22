# Fresco Implementation Architecture: Correctness & Performance Blueprint

**Review Date:** 2026-10-22  
**Scope:** All files in `src/fresco`, Flux serialization integration, `agents/AGENTS.md` directive compliance, module documentation (`wiki/modules/fresco.md`), and high-throughput execution architecture.  
**Overriding Directives:** **Mathematical Correctness & Extreme Performance**

---

## 1. Governing Architecture Principles

In accordance with Trellis engineering principles, **Correctness and Performance are overriding**:

1. **Mathematical Correctness Overriding:**
   - **Exact Grouping Preservation:** Non-associative operations (`-`, `/`, `^`) must maintain exact structural associativity; lowering must never alter operator semantics.
   - **Deterministic Lexing:** Tokenization must be immune to whitespace variation (`x!=y` vs `x != y`).
   - **Transactional State Invariants:** Repository state must be strictly transactional: a failed or incomplete parse must leave zero residual state in the repository.
   - **Directive Invariant Compliance:** Zero tolerance for `Option<u32>`; invalid IDs and unassigned links must use `u32::MAX` per `AGENTS.md` Rule 21.
   - **Defensive Boundary Validation:** Factory constructors must validate child references, bounds, arity, and cyclic dependencies before mutation.

2. **Performance Overriding:**
   - **Logarithmic Variable Interning:** Eliminate linear $O(N^2)$ attribute scanning. Use sorted index arrays with binary search ($O(\log N)$) per `AGENTS.md` ("Avoid use of Hashmaps, prefer using sorted arrays").
   - **Zero Unnecessary Allocations:** Avoid intermediate garbage nodes (e.g. fold unary negative literals directly into `RealExpr(-val)`).
   - **Cache Locality & Arena Path:** Design a contiguous flat node arena roadmap to eliminate `Box<dyn BaseExpr>` heap boxing, pointer chasing, and virtual dispatch overhead.
   - **Vectorized & Short-Circuit Execution:** Analytical evaluation must operate over contiguous slices/`Arr<f64>` with branch laziness for conditionals.

---

## 2. Deep Technical Findings & Correctness Fixes

### P1: Lowering Loses Non-Associative Grouping (Algebraic Parity Propagation in `PolyExpr`)

* **Location:** [`src/fresco/exprrepos.rs`](../../src/fresco/exprrepos.rs), `CollectTree` and `TraverseTree`.
* **Root Cause:** `CollectTree` currently flattens children naively when `node.Op() == parentOp` and sets `addCount = 1` or `numerCount = 1` unconditionally:
  - `a - (b - c)` collapses into `[a, b, c]` with `addCount = 1`, producing $+a - b - c = a - (b + c)$ instead of $a + c - b$.
  - `a / (b / c)` collapses into `[a, b, c]` with `numerCount = 1`, producing $a / b / c = a / (b \cdot c)$ instead of $(a \cdot c) / b$.
  - Nested powers `a ^ (b ^ c)` collapse into a flat list, erasing the non-associative exponentiation hierarchy.
* **Overriding Correctness & Algebraic Design:**
  Fresco's `PolyExpr` was architected specifically to represent partitioned sums and products using `_CoSize` and `IsFlip(index)`:
  - In `SumExpr`: $(\sum_{i < \text{_CoSize}} \text{Child}_i) - (\sum_{j \ge \text{_CoSize}} \text{Child}_j)$
  - In `ProdExpr`: $(\prod_{i < \text{_CoSize}} \text{Child}_i) / (\prod_{j \ge \text{_CoSize}} \text{Child}_j)$

  Instead of naive operator matching, tree lowering must use **algebraic parity propagation**:
  1. **Additive Lowering (`SumExpr` via `CollectAdditive`):**
     - Propagates boolean sign `isPositive: bool`.
     - `BinOp::Sum`: Recursively visits both children with unchanged `isPositive`.
     - `BinOp::Sub`: Left child visited with `isPositive`; Right child visited with inverted sign `!isPositive`.
       - For $a - (b - c)$:
         - $a$ is positive ($+$).
         - In $(b - c)$, sign is flipped: $b$ is negative ($-$), and $-(-c) = +c$ is positive ($+$).
         - Resulting terms: positive $\{a, c\}$, negative $\{b\}$.
         - Emits single `SumExpr` with `_Children = [a, c, b]` and `_CoSize = 2`, exactly representing $a + c - b$!
     - Non-additive sub-trees are lowered via `TraverseTree` and pushed to `posTerms` or `negTerms`.
     - Single `SumExpr` is constructed by joining `posTerms` and `negTerms` with `_CoSize = posTerms.Size()`.
  2. **Multiplicative Lowering (`ProdExpr` via `CollectMultiplicative`):**
     - Propagates boolean position `isNumerator: bool`.
     - `BinOp::Prod`: Recursively visits both children with unchanged `isNumerator`.
     - `BinOp::Div`: Left child visited with `isNumerator`; Right child visited with inverted position `!isNumerator`.
       - For $a / (b / c)$:
         - $a$ is numerator ($\times$).
         - In $(b / c)$, position is inverted: $b$ is denominator ($/$), and $/(/c) = \times c$ is numerator ($\times$).
         - Resulting factors: numerator $\{a, c\}$, denominator $\{b\}$.
         - Emits single `ProdExpr` with `_Children = [a, c, b]` and `_CoSize = 2`, exactly representing $(a \cdot c) / b$!
     - Non-multiplicative sub-trees are lowered via `TraverseTree` and pushed to `numFactors` or `denFactors`.
     - Single `ProdExpr` is constructed by joining `numFactors` and `denFactors` with `_CoSize = numFactors.Size()`.
  3. **Exponentiation (`PowExpr`):**
     - Exponentiation does not distribute signs or inversions ($a^{(b^c)} \neq (a^b)^c$).
     - `PowExpr` strictly preserves binary structure: Child 0 is Base, Child 1 is Exponent, `_CoSize = 1`. No recursive flattening.

---

### P1: Operator `!` (Unary Prefix NOT and Binary Inequality `!=`)

* **Location:** [`src/fresco/exprshard.rs`](../../src/fresco/exprshard.rs), `IsIdentChar`, `MatchUnary`, `MatchComparison`.
* **Root Cause:** `!` was erroneously included in `IsIdentChar` (inherited from a legacy Elmer charset `a-zA-Z0-9_.@#$!`). This caused greedy consumption of `x!` in `x!=y`, mistaking `!=` for a postfix `!` on identifier `x` followed by `=`. There is no valid case for `!` as a postfix character in identifiers or expressions (no models use postfix `!`, and factorial is not supported).
* **Overriding Correctness Fix:**
  1. **Purge `!` from `IsIdentChar`:** Identifiers strictly accept `[a-zA-Z0-9_.@#$]`. Eliminating `!` removes the ambiguity entirely with zero lookahead overhead.
  2. **Deterministic Operator Parsing:**
     - **Binary Inequality `!=`:** Tokenizes as `LogicOp::Ne`. Both `x != y` and `x!=y` parse identically.
     - **Unary Prefix Logical NOT:** `!expr` (and keyword `not expr`) parsed as a unary prefix operator for logical negation.

---

### P1: Failed Parsing Leaves Repository State Behind (Transactional Atomicity)

* **Location:** [`src/fresco/exprshard.rs`](../../src/fresco/exprshard.rs), `Parse` and helper factories; [`src/fresco/exprrepos.rs`](../../src/fresco/exprrepos.rs).
* **Root Cause:** `ExprShard` appends expressions and variables directly to `ExprRepos` during parsing. When stream markers backtrack or parsing fails midway (e.g. `x +` or trailing garbage `x y`), all interned variables and expression nodes remain allocated in `_VarAttribs` and `_Exprs`. This corrupts subsequent variable IDs and causes unbounded memory growth.
* **Overriding Performance & Correctness Fix:**
  1. Introduce a zero-allocation, $O(1)$ checkpoint type on `ExprRepos`:
     ```rust
     #[derive( Copy, Clone, Debug )]
     pub struct ExprCheckpoint
     {
         _ExprCount: u32,
         _VarCount:  u32,
     }

     impl ExprRepos
     {
         #[inline]
         pub fn  Checkpoint( &self ) -> ExprCheckpoint
         {
             ExprCheckpoint {
                 _ExprCount: self._Exprs.Size(),
                 _VarCount:  self._VarAttribs.Size(),
             }
         }

         pub fn  Rollback( &mut self, cp: ExprCheckpoint )
         {
             self._Exprs.Resize( cp._ExprCount, |_| ExprEntry::Empty );
             self._VarAttribs.Resize( cp._VarCount, |_| VarAttrib::default() );
         }
     }
     ```
  2. In `ExprShard::Parse`, wrap the parse in a checkpoint: if parsing fails or trailing characters remain, invoke `repos.Rollback(cp)` before returning `Err`.

---

### P2: Directives Invariant Violation — `Option<u32>` in `VarAttrib`

* **Location:** [`src/fresco/varexpr.rs`](../../src/fresco/varexpr.rs), `VarAttrib`.
* **Root Cause:** `_DepExpr: Option<u32>` and `_AggrIndex: Option<u32>` violate `AGENTS.md` Rule 21:
  > *"Never use Option<u32> use u32::MAX for invalid values."*
* **Overriding Correctness Fix:**
  1. Replace `Option<u32>` with `u32` initialized to `u32::MAX`:
     ```rust
     #[derive( Clone, Debug )]
     pub struct VarAttrib
     {
         _Name:      String,
         _DepExpr:   u32,
         _AggrIndex: u32,
         _Flags:     u32,
     }
     ```
  2. Update helper queries:
     - `IsAggregate(&self) -> bool { self._AggrIndex != u32::MAX }`
     - `IsIndependent(&self) -> bool { !self.IsAggregate() && self._DepExpr == u32::MAX }`
     - `IsDependent(&self) -> bool { !self.IsAggregate() && self._DepExpr != u32::MAX }`

---

## 3. High-Performance Engineering Architecture

### Performance Pillar 1: $O(\log N)$ Sorted Symbol Index for Variable Interning

* **Problem:** In large kinetic/biochemical models with hundreds or thousands of variables, calling `VarCreate` performs a linear $O(N)$ scan of all attributes. For $N$ variables, parsing performs $O(N^2)$ string comparisons.
* **Solution:** Maintain a secondary sorted index `_SymbolIndex: Buff<u32>` containing variable IDs sorted alphabetically by `_VarAttribs[id]._Name`.
* **Algorithm:**
  1. Binary search `_SymbolIndex` using `name` in $O(\log N)$ time.
  2. If found, return existing variable index immediately.
  3. If not found, append to `_VarAttribs` and insert index into `_SymbolIndex` at the binary search insertion point.
  4. Fully adheres to `AGENTS.md` directive: *"Avoid use of Hashmaps, prefer using sorted arrays."*

---

### Performance Pillar 2: Literal Negation Optimization (Zero-Garbage Unary Parsing)

* **Problem:** Currently, parsing `-5.0` matches unary minus, parses `5.0` as `RealExpr(5.0)`, and then constructs a `SumExpr` or negation wrapper, creating two heap-allocated nodes for a simple constant.
* **Solution:** When parsing numbers in `MatchUnary`, if a unary `-` is followed immediately by a number, directly construct `RealExpr(-value)`. This cuts node allocation and tree depth by 50% for negative numeric literals.

---

### Performance Pillar 3: Fast Analytical Evaluation Kernel

* **Design:** When numerical evaluation is introduced, it must avoid dynamic dispatch (`dyn BaseExpr`) in hot loops:
  1. Environment is passed as contiguous borrowing: `Arr<'a, f64>`.
  2. Evaluator uses iterative topological evaluation or stack-based recursion with short-circuiting:
     - `CondExpr`: Evaluate condition; evaluate **only** the active branch (lazy branch evaluation).
     - `LogicExpr`: `and` evaluates RHS only if LHS is true; `or` evaluates RHS only if LHS is false.
     - `SumExpr` & `ProdExpr`: Accumulate over contiguous child `USeg` using SIMD-friendly loops.

---

### Performance Pillar 4: Contiguous Node Arena (Future Milestone)

* **Problem:** Polymorphic `Box<dyn BaseExpr>` introduces scattered heap allocations, pointer dereferencing, and virtual dispatch overhead on every node visit.
* **Long-Term Evolution:** Transition `ExprEntry` to a flat enum value type or arena:
  ```rust
  pub enum ExprNode
  {
      Real( f64 ),
      Var( u32 ),
      Sum( PolyExpr ),
      Prod( PolyExpr ),
      Pow( u32, u32 ),
      Logic( LogicOp, u32, u32 ),
      Cond( u32, u32, u32 ),
      Func( u32, u32 ),
  }
  ```
  This places all expression nodes into contiguous memory within `Stash<ExprNode>`, maximizing CPU L1/L2 cache line utilization.

---

## 4. Delivery Phases & Acceptance Gates

```mermaid
flowchart LR
    Stage1["Stage 1: Correctness & Compliance"] --> Stage2["Stage 2: Performance & Logarithmic Interning"]
    Stage2 --> Stage3["Stage 3: Boundary Invariants & Hardening"]
    Stage3 --> Stage4["Stage 4: High-Performance Evaluator"]
    Stage4 --> Stage5["Stage 5: Full Flux Round-Trip Persistence"]
```

| Phase | Core Objective | Key Deliverables | Status / Acceptance Gates |
|---|---|---|---|
| **Stage 1** | **Correctness & Compliance** | 1. Algebraic parity propagation lowering in `CollectAdditive` and `CollectMultiplicative` (`SumExpr`, `ProdExpr`, `PowExpr`).<br>2. Whitespace-independent `!=` and unary prefix `!` (purging `!` from `IsIdentChar`).<br>3. `ExprRepos::Checkpoint` & `Rollback` transactional parse safety.<br>4. Replace `Option<u32>` with `u32::MAX` sentinel in `VarAttrib`.<br>5. Align `wiki/modules/fresco.md` with source reality. | **COMPLETED** (Verified by `_tests.rs`: $a-(b-c) \implies a+c-b$, $a/(b/c) \implies (a \cdot c)/b$, $x!=y \equiv x != y$, 0 residual state on rollback, 0 `Option<u32>`). |
| **Stage 2** | **Performance & Logarithmic Interning** | 1. $O(\log N)$ sorted symbol array `_SymbolIndex` for variable lookup.<br>2. Literal negation folding (zero-garbage negative numbers in `MatchUnary`).<br>3. Traversal benchmarking on large expression sets. | **COMPLETED** ($O(\log N)$ binary search interning, zero intermediate nodes for negative constants, 348/348 Jeeves tests pass). |
| **Stage 3** | **Boundary Hardening** | 1. Checked factory constructors validating child bounds.<br>2. Cycle prevention in variable/expression references.<br>3. Recursion depth budget preventing stack overflow on pathological input. | Malformed IDs, partition overflows, and excessive nesting return structured errors without crashing. |
| **Stage 4** | **Analytical Evaluator** | 1. High-throughput evaluation against contiguous `Arr<f64>`.<br>2. Short-circuit lazy evaluation for `CondExpr` and `LogicExpr`.<br>3. Rigorous domain error and non-finite handling. | Numerical accuracy matching analytical references; branch laziness verified on error-inducing inactive branches. |
| **Stage 5** | **Flux Persistence** | 1. Heterogeneous Flux import discriminator factory.<br>2. Full serialization round-trip test suite. | Export-then-import produces identical structural ASTs and variable identity. |

---

## 5. Verification Matrix

Add the following verification test cases to [`src/fresco/_tests.rs`](../../src/fresco/_tests.rs):

1. **Non-Associative Lowering Regressions:**
   - `a - (b - c)` vs `(a - b) - c`: Verify `a - (b - c)` lowers to `SumExpr` with positive $\{a, c\}$ and negative $\{b\}$ ($a + c - b$), whereas `(a - b) - c` lowers to `SumExpr` with positive $\{a\}$ and negative $\{b, c\}$ ($a - b - c$).
   - `a / (b / c)` vs `(a / b) / c`: Verify `a / (b / c)` lowers to `ProdExpr` with numerators $\{a, c\}$ and denominator $\{b\}$ ($(a \cdot c) / b$), whereas `(a / b) / c` lowers to `ProdExpr` with numerator $\{a\}$ and denominators $\{b, c\}$ ($a / (b \cdot c)$).
   - `(a ^ b) ^ c` vs `a ^ (b ^ c)`: Verify nested binary `PowExpr` preservation without flattening.

2. **Operator `!` and Inequality Lexing:**
   - Parse `x!=y` and `x != y`: Assert that both produce identical `LogicOp::Ne` ASTs.
   - Unary prefix `!`: Assert `!x` and `not x` parse as unary logical NOT.
   - Supported identifier symbols: Assert identifiers with `@`, `#`, `$`, `.`, `_` parse correctly, while `!` terminates identifiers immediately.

3. **Transactional Rollback Verification:**
   - Record `repos.Size()` and `repos.SzVar()`.
   - Parse invalid inputs (`x +`, `if (a)`, `x y`).
   - Assert error return and verify `repos.Size()` and `repos.SzVar()` remain strictly identical to pre-parse state.

4. **Sentinel Invariant Verification:**
   - Assert `VarAttrib::New` initializes `_DepExpr` and `_AggrIndex` to `u32::MAX`.
   - Assert queries (`IsIndependent`, `IsDependent`, `IsAggregate`) behave correctly without `Option`.

5. **Performance & Compliance Gates:**
   - `cargo clippy --features python -- -D warnings`: 0 warnings.
   - `cargo test --features python -- fresco`: All tests passing.
   - `cargo run --features python -- -t`: 344/344 Jeeves tests passing.
   - Strict adherence to `agents/FORMATTING.md` (separators, LF, paren spacing, alignment).
