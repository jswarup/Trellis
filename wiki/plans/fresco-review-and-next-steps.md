# Fresco implementation review and next steps

Reviewed on 2026-10-23 against commit `c34815b` plus the current working-tree
changes in Fresco and its module documentation. The working tree adds semantic
staging and `eval.rs`; uncommitted and untracked implementation files are included
in this review. This document supersedes the outstanding-work order and completion
claims in [the historical blueprint](fresco-improvement-plan.md).

Scope: Fresco implementation and its 14 registered tests, Shard Parser and binary
grammar composition, the shared NodeTree macro, Flux field macros, and module
documentation. This update changes review documentation only. It does not
implement the fixes below. Executed checks and source-only findings are
distinguished in the verification record.

## Assessment and status

Fresco now has a staged text parser, variable-flag configuration, selected checked
factories, and a reference evaluator. Full-string parse failure can discard staged
nodes without changing ExprRepos. Repository checkpoint APIs have been removed.
However, semantic savepoints still belong to ExprShard, not Parser, and publication
does not yet obey the selected outer-acceptance contract.

| Area | Implemented | Remaining |
|---|---|---|
| Repository checkpoints | ExprCheckpoint, Checkpoint, and Rollback removed, including exports | Complete the replacement at Parser's transaction boundary |
| Parsing | StagedNode arena in ExprShard; local stream/staging restoration; full-string check before explicit Publish | Sound mutable state, nested Parser semantic savepoints, root invalidation |
| Publication | Reachable subtree traversal, local-to-persistent ID mapping, variable reuse policy | Pure root queries, explicit outer acceptance, bounded preparation, lifecycle |
| Variables | VarKind::NonZero = 4, HasBits, SetVarKind, FindVar | Consistent identity across construction paths; assumption lifecycle |
| Validation | TryAt, ID checks, checked power/logical/conditional/function factories | Sum/product/general power validation, imported partitions, malformed-node rejection |
| Limits | Text nesting limit 256; tree/evaluator depth limit 128 | Publication depth, input/node/work budgets, checked atomic tree ingestion |
| Evaluation | Indexed/named bindings, arithmetic, powers, lazy conditionals/AND/OR, NOT, unary math functions, EvalError | Numeric/domain consistency, malformed-node safety, assumption coverage, broader tests |
| Persistence | Repository and tagged node export; concrete field import hooks | Validated heterogeneous repository load, roots/schema, atomic publication |
| Performance | Binary-search symbol lookup; one-node negative literals | Measurements; insertion still shifts O(V) IDs; repeated DAG evaluation |

The evaluator should now be hardened, not scheduled as an entirely absent
capability. The next milestone is sound parsing and explicit publication.

## Findings in priority order

### F1 - P1: Shared-reference mutation remains in matching

Evidence: [exprshard.rs](../../src/fresco/exprshard.rs), IGrammar::Match;
[cast.rs](../../src/silo/cast.rs),
IConstPtrMutRefExt::MutRef.

RootToken now takes &mut self and calls Publish directly, removing its pointer
cast. IGrammar::Match still obtains mutable Self from a shared reference through
the raw-pointer helper. ExprShard does not put its mutated state in UnsafeCell. The helper
suppresses invalid_reference_casting; suppression does not make the access sound.
This violates Rust's shared-reference rules even when tests pass. See the
[Rust interior-mutability contract](https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html).

Required change: provide genuinely mutable parse-context access through Parser,
or retain safe interior mutability while that interface is developed. Eliminating
Cell/RefCell is not itself a performance requirement. Moving the cast into another
helper or adding UnsafeCell without an aliasing argument is not a fix.

Acceptance: matching and result access do not manufacture exclusive references
from shared references; grammar composition preserves the mutable-state ownership
contract. Targeted aliasing checks may supplement review, but tests and Clippy alone
cannot prove soundness.

### F2 - P1: Root lookup publishes; Parser does not own semantic acceptance

Evidence: [exprshard.rs](../../src/fresco/exprshard.rs), RootToken, Publish, Match;
[parser.rs](../../src/shard/parser.rs), ParseGrammar;
[binshard.rs](../../src/shard/binshard.rs), sequence matching.

Parser still stores only stream markers. It cannot restore ExprShard's staged
nodes or pending root when a containing grammar rejects a successful child.
RootToken(&mut self) calls Publish as a side effect and ignores its Result. After inner
success followed by parent failure, reading the child's root can publish rejected
work. The existing composition test explicitly expects publication on root lookup;
it does not establish the selected outer-acceptance guarantee.

Reuse also has no complete lifecycle. Successful matching changes _StagedRoot
without resetting _IsPublished. After one publication, Publish after a second
successful match can return the previous _RootToken. Failure invalidates roots
without resetting publication state. These are source traces, not executed
reuse/outer-failure reproductions in this review.

Required change: Parser owns nested semantic savepoints and explicit outer
acceptance; root getters are pure. Initially make accepted contexts single-use,
rejecting further matching, unless a completely resettable lifecycle is needed.
Do not publish merely because a child or top-level ParseGrammar call succeeds.

Acceptance: outer failure after inner success, failed alternation, prefix
acceptance, trailing garbage, query after failure, repeated matching, and repeated
publication verify repository counts, attributes, name bindings, and root validity.
Reading a root never grows the repository.

### F3 - P1: Publication bypasses the syntactic depth limit

Evidence: [exprshard.rs](../../src/fresco/exprshard.rs), PublishSubtree (line 102),
MatchAddSub, and MatchMulDiv.

Long flat arithmetic chains are parsed iteratively but create deeply nested staged
graphs. PublishSubtree recursively walks those graphs without a depth/work bound,
appending persistent nodes during the walk. Long accepted input can exhaust the
native stack during publication. Shorter accepted expressions can exceed the
evaluator's depth bound of 128. The parentheses test does not cover flat chains.

Required change: use an explicit work stack or a bounded prepared traversal.
Staged children precede parents, so an ordered live-node pass is an option after
reachability/validation. Check IDs, arity, limits, and remapping before persistent
mutation. Normal validation errors must not partially append. Define whether
expressions above evaluator limits are rejected at acceptance or evaluated
iteratively.

Acceptance: long flat chains, nested functions/unary operators, large conditionals,
and failed preparation succeed within stated limits or return errors without
repository mutation. Do not use deliberate stack overflow as a test.

### F4 - P1: Tree normalization produces observable domain differences

Evidence: [exprrepos.rs](../../src/fresco/exprrepos.rs), CollectAdditive,
CollectMultiplicative, and PostTermTree; [eval.rs](../../src/fresco/eval.rs),
product evaluation. Confirmed by the standalone probe below.

Term-tree lowering normalizes a/(b/c) into (a*c)/b without consulting assumptions.
Explicit nested construction retains the inner division. For a=1, b=1, c=0,
nested construction returns DivisionByZero while term-tree lowering returns zero.
Finite nonzero values can also differ through overflow, underflow, and rounding.

Required change: preserve nesting when required domain facts are unknown and
make assumption-dependent normalization explicit. Document evaluation order of
stored n-ary sums/products. NonZero supplies real-domain premises, not IEEE
equivalence. Keep parity tests as tests of explicit normalization rather than
source-order preservation.

Acceptance: independent numerical/error expectations across text, nested factories,
and grouped term trees, including zero, both zero signs, cancellation, overflow,
underflow, and safe finite values.

### F5 - P2: Evaluation can panic or ignore malformed operands

Evidence: [eval.rs](../../src/fresco/eval.rs), power branch (line 186) and binary
Not unreachable branch (line 292); raw factories in
[exprrepos.rs](../../src/fresco/exprrepos.rs); PolyExpr field import.

Checked factories coexist with raw public construction. A binary Not built by
LogicCreate(Not, left, right) reaches unreachable during Evaluate. A three-child
power evaluates its first two children and silently ignores the third and its
partition. Both behaviors were reproduced. Invalid sum/product partitions can
reach asserting access, and generated field import bypasses constructor checks.

Required change: centralize built-in validation and defensively reject malformed
nodes in the evaluator. Enforce exactly two power children with the expected
partition and coherent logical arity. Define empty sums/products, bare PolyExpr,
custom nodes, and forward references. An append-only child-before-parent rule
avoids needing a whole-graph cycle scan on every ordinary insertion.

Acceptance: each malformed built-in produces an EvalError or checked-construction
error rather than a panic, ignored operands, or partial insertion. Document trusted
asserting access separately.

### F6 - P2: Tree variable identity conflicts with named evaluation

Evidence: [exprrepos.rs](../../src/fresco/exprrepos.rs), TraverseTree (line 516),
VarCreate, FindVar; [eval.rs](../../src/fresco/eval.rs), EvaluateNamed.

Text parsing reuses variable identities. Term-tree leaves pass reuse=false, so
TermTree!("x" + "x") creates two variable IDs. FindVar resolves one canonical ID;
one named binding leaves the other at NaN. The resulting evaluation fails despite
all displayed names being provided. The probe reproduced this. A NonZero flag on
one ID likewise says nothing about the distinct second variable.

Required change: provide consistent explicit reuse policies across construction
paths, preserving intentional same-name distinct symbols. Define named-binding
ambiguity: reject it or require explicit ID bindings rather than silently merging
identities or broadcasting assumptions.

Acceptance: repeated-name text/tree inputs, existing names, distinct symbols,
indexed/named binding, duplicate entries, and unknown names have documented
outcomes. Flags follow variable identity across all VarExpr occurrences.

### F7 - P2: Numeric and assumption enforcement is partial

Evidence: [eval.rs](../../src/fresco/eval.rs), real/variable branches, arithmetic,
math functions, and EvaluateNamed; VarAttrib and SetVarKind.

Implemented: visited VarExpr nodes reject NaN with DomainError and reject either
zero sign for NonZero with AssumptionViolated. Named evaluation fills unspecified
variable slots with NaN. Conditionals and AND/OR are lazy.

Remaining: RealExpr can return NaN; arithmetic/functions can return NaN/infinity;
infinite bindings are accepted. Missing named bindings and explicit NaN share a
DomainError path. Assumptions are checked when variables are visited, not in a
binding-wide preflight. Flags can change without a freeze or premise tracking.
No optimizer currently cancels variables, so validating eliminated premises remains
a requirement for future transformations rather than an existing feature.

Required change: specify a consistent non-finite/error policy and numeric truth
conversion. Retain NonZero in the existing VarKind/_Flags design; unset means
unknown. Configure by variable ID, preserving other bits and distinct identities.
Before rewrites, freeze premises or retain obligations after variable elimination.
Nonzero variables do not prove that arbitrary expressions such as x-y are nonzero.

Acceptance: both zero signs, NaN/infinity constants and bindings, overflow/underflow,
power edge cases, missing/duplicate names, unknown functions, supported unary
functions, lazy OR/NOT, and assumption changes are covered.

### F8 - P2: Atomic tree ingestion and total resource bounds remain incomplete

Evidence: [exprrepos.rs](../../src/fresco/exprrepos.rs), PostTermTree and depth
assertions; parse limits in ExprShard; recursive evaluation.

PostTermTree returns u32 and asserts at its depth limit after potentially storing
earlier children. There is no checked atomic tree-ingestion API. Parsing lacks a
total input/node budget. Evaluation bounds depth but not total work: repeated
AddCreate(previous, previous) creates a small DAG with exponentially repeated
recursive evaluation. The DAG cost is inferred from source, not benchmarked here.

Required change: add a checked tree-ingestion path using preparation/staging.
Specify finite input, node, string-length, and work budgets. Count graph depth
consistently rather than helper transitions. Consider lazy memoization for pure
built-ins after measurement; do not eagerly evaluate unused branches.

Acceptance: each recursive shape at/below/above its boundary, flat chains, malformed
custom ITermNode inputs, and a small shared DAG fail predictably or complete within
the stated budget, without partial repository changes.

### F9 - P2/P3: Compatibility, persistence, documentation, and measurements

The parser retains incomplete-exponent recovery (1e, 1e+, 1e-), left-associative
powers, and unary-minus-before-power behavior. NodeTree consumes remaining tokens
as a right subtree without standard arithmetic precedence: a*b+c becomes a*(b+c).
Characterize these behaviors; change syntax only through explicit opt-in/versioning.
Provide detailed byte-span errors with a compatible Result<u32, String> wrapper.

Complete repository import is absent. Use bounded temporary Flux type dispatch,
validate references/arity/partitions, rebuild the sorted name index, and publish
only on success. Define roots, schema versions, missing/duplicate fields, unknown
types/flag bits, same-name distinct symbols, and nullable-to-sentinel compatibility.
Old unset NonZero bits mean unknown; generated field import alone is not a
validated repository loader.

Symbol lookup is O(log V) name comparisons, but insertion shifts O(V) IDs;
reverse-sorted unique names still cause O(V^2) total shifts. Publication clones
staged nodes/strings and EvaluateNamed allocates a repository-sized binding buffer.
Measure before changing storage, introducing SIMD, or compiling execution plans.

The [module documentation](../modules/fresco.md) also needs alignment: it overstates
automatic outer acceptance/atomic publication and complete repository import.
Its NaN-to-AssumptionViolated statement differs from the actual DomainError path.
Normalization does not preserve all source grouping, and sorted insertion is not
logarithmic. Correct those statements when documenting the corresponding behavior.

## Required Parser-owned checkpoint design

This remains the selected architecture. Repository checkpoint removal is complete;
generic Parser integration is not.

1. Parser owns nested stream and semantic savepoints via a grammar-independent
   mutable context. Fresco supplies staging; Shard does not depend on Fresco types.
2. Rejection rewinds staged nodes, pending roots, and input. Audit manual
   SetCurrMark paths as well as ParseGrammar boundaries.
3. Child success remains staged under its parent. Explicit outer acceptance checks
   the caller's full-string or prefix policy before publication. Root getters are pure.
4. Validate/prepare live nodes, variable identities, remapping, and budgets before
   persistent mutation. Attribute changes remain pending until acceptance and
   follow the assumption-lifetime contract.
5. Staged IDs remain local; expose persistent IDs after publication. Initially
   reject context reuse after acceptance unless a resettable lifecycle is required.
   Repeated Publish for the same accepted result can be idempotent.

The mutable context must remove the shared-reference casts, not hide them elsewhere.
Import/tree ingestion require their own prepared publication path; parser
checkpoints do not automatically cover non-parser entry points.

## Delivery sequence and acceptance gates

| Change | Scope | Acceptance gate |
|---|---|---|
| 1 | Sound mutation and Parser-owned acceptance; F1-F2 | No shared-reference casts or publishing getters; outer failure cannot publish or expose stale roots |
| 2 | Bounded prepared publication and checked tree ingestion; F3, F8 | Long flat/nested input succeeds within limits or returns errors; rejected work leaves repository intact |
| 3 | Node validation and variable identity; F5-F6 | Malformed nodes return errors; repeated names have consistent binding rules |
| 4 | Assumption-aware normalization and evaluator policy; F4, F7 | Domain-sensitive rewrites require premises; independent numeric/error cases and laziness tests pass |
| 5 | Compatibility diagnostics and full Flux load; F9 | Legacy behavior retained; syntax changes explicit; schema/roots and bounded atomic round trips tested |
| 6 | Optimize measured costs; F8-F9 | Reproducible construction/evaluation before/after results; semantic tests remain green |

Changes 1-2 are the next milestone. Changes 3-4 harden the existing evaluator.
Begin workload measurement alongside correctness work; benchmarks do not block
soundness fixes. Import follows stable representation/assumption contracts.

Keep changes separately reviewable. Add meaningful jeeves_test regressions in the
owning Fresco/Shard test files. Follow agents/AGENTS.md: private fields, Trellis
containers/traversal, u32 IDs and sentinels, no Serde/Tokio, measured allocations.
Update Natvis if storage representation changes.

## Verification record

On 2026-10-23, `cargo test -p trellis --lib fresco --offline` rebuilt successfully:
14 passed, zero failed, 329 filtered out. This is a focused baseline, not proof of
aliasing soundness, outer semantic rollback, or coverage of every finding.

A temporary standalone probe was run with
`cargo run -p trellis --example fresco_review_probe --offline`. It used direct
factories/tree construction, avoiding ExprShard's shared-reference casts.

| Probe | Observed result |
|---|---|
| Explicit nested 1/(1/0) | Err(DivisionByZero) |
| Term-tree lowering of 1/(1/0) | Ok(0.0) |
| Power children [2, 3, 100], base partition 1 | Ok(8.0), extra operand ignored |
| Binary Not through raw factory | Panic, caught by the probe |
| RealExpr(NaN) | Ok(NaN) |
| TermTree x+x with one named binding x=2 | Two variable IDs; DomainError for the second uninitialized variable |

The probe completed successfully and its source was removed afterwards.
The findings on publication reuse, outer failure, unbounded traversal, and DAG
cost are source inspections; no crashing stack experiment was performed.

Historical Python-feature/full-suite/Clippy totals were not rerun or independently
established for this working tree. Broad implementation changes should run the
repository-required all-target checks, Clippy, and relevant Cove suite. Review
documentation updates do not make earlier totals current verification.
