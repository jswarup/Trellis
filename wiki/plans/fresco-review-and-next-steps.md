# Fresco review and next implementation plan

Reviewed on 2026-10-22 and re-reviewed on 2026-10-23 against commit `fb49a03`.
The current working tree also includes the local `VarKind::NonZero = 4` addition.
Other Fresco source matches that commit. This is the current implementation plan
after the correctness and interning changes; it supersedes the outstanding-work order
in [the historical blueprint](fresco-improvement-plan.md). Completed milestones
there are historical claims, not evidence that every boundary or numerical case
is covered.

Scope: every Fresco implementation file and its ten registered tests, the shared
`NodeTree!` macro, Shard parser depth handling, Flux field macros, Silo resizing,
and module documentation. Findings below come from code inspection unless a test
result is explicitly reported. Apart from the already-added enum flag, proposed
fixes and capabilities below remain outstanding.

## Current assessment

The implementation has improved: `!=` no longer collides with identifier scanning,
negative literals allocate one expression, unary NOT exists, failed full-string
parses roll back, nested powers remain binary when lowered from term trees, and
variable lookup uses a sorted index. Existing regression tests cover these paths.

The recommended next milestone is a defined expression-language and numerical
contract, safe repository boundaries, and complete transaction coverage. Evaluation
and persistence should build on that foundation. Performance changes should use
measured construction and evaluation workloads.

### Implementation status at the latest review

| Area | Present in the code | Remaining improvement |
|---|---|---|
| Parsing | Arithmetic, comparisons, NOT, conditionals, unary function calls, and legacy number recovery | Explicit precedence/compatibility contract, detailed errors, and finite budgets |
| Transactions | Repository count checkpoints and local parse rollback | Parser-owned savepoints over temporary semantic state; publication only after outer acceptance |
| Variable assumptions | `VarKind::NonZero = 4`, existing `_Flags` export, and `HasBits` query | Controlled configuration, identity consistency, rewrite premises, and binding validation |
| Repository validation | Private storage, partition constructor assertion, and asserting typed access | Checked IDs/arity, shared validation, and defined custom-node behavior |
| Numerical execution | Expression representation only | Reference evaluator, documented domain policy, lazy branches, and independent numerical tests |
| Persistence | Typed node/repository export and concrete field import hooks | Heterogeneous repository import, roots/schema, validation, and atomic publication |
| Performance | Binary search for existing names and single-node negative literals | Construction baseline; insertion still shifts O(V) IDs; optimize demonstrated costs |

Recommended next implementation changes, in reviewable scopes:

1. Move parse checkpoint ownership into Parser (F2-F3), using staged semantic
   results and outer-parse publication. Add direct/composed grammar regressions
   and symbol-index/root checks. Keep full-string parse behavior compatible.
2. Add checked construction and finite budgets (F4-F5), reusing the transaction
   path for atomic error handling. Preserve convenience wrappers for callers.
3. Complete variable identity and NonZero configuration, then make normalization
   premises explicit (F1, F6). Test same-name distinct variables and unknown facts
   before enabling any new assumption-dependent rewrite.

Begin with focused characterization cases and a construction baseline as described
in change 0 below. Evaluation, repository import, and storage optimization follow
their own acceptance gates; none is required to finish the first two changes.

## Plan review decisions (2026-10-23)

The findings remain relevant, but the earlier delivery sequence was too broad
and left several implementation choices unresolved. Apply these corrections:

- Establish focused reproductions before changing APIs. The ten passing tests
  do not reproduce the checkpoint, grammar-composition, or depth-limit findings.
- Transaction safety and input limits can proceed independently of numerical
  semantics. Do not delay them for a new evaluator or a language redesign.
- Parser owns parse checkpoint boundaries. Stage semantic results until outer
  acceptance, then publish them to ExprRepos. Remove repository checkpoints rather
  than adding repository identity tokens or generation tracking. No checkpoint
  callers outside Fresco were found in `src` or `examples`; document removal for
  possible external callers.
- Treat algebraic versus source-ordered evaluation as a contract decision, not
  a requirement to build two execution engines. Keep current parity lowering as
  documented symbolic normalization; implement an ordered path only if preserving
  source computation is a requirement. Do not promise IEEE equivalence for parity
  lowering or ship an evaluator claiming it without preserving source grouping.
- Retain current parsing behavior by default during hardening. New strict syntax
  or changed precedence must be opt-in or explicitly versioned. Avoid silently
  changing the legacy corpus to satisfy conventional mathematical expectations.
- Take construction benchmarks before optimizing. Evaluation benchmarks follow
  the reference evaluator; a benchmark harness does not depend on completing
  persistence.
- Add a per-variable `NonZero` assumption to justify domain-sensitive symbolic
  rewrites. It supplies a mathematical premise, not permission to reorder arbitrary
  floating-point computations. The proposed contract is detailed below.

P1 denotes work to resolve before exposing parsing to unbounded inputs or
releasing a numerical evaluator. F1 is a confirmed representation distinction
and a future evaluation risk; no production evaluator failure is claimed.

### Architecture decision: Parser owns checkpoints

Checkpointing belongs to parsing, not the persistent expression repository. Move
savepoint ownership into `shard::Parser` and keep `ExprRepos` free of public
`Checkpoint`, `Rollback`, and `ExprCheckpoint` APIs.

Moving stream markers alone is insufficient: the current grammar appends nodes
and variables while matching. Replace those side effects with temporary semantic
state supplied to Parser through a grammar-independent mutable context. Fresco
provides the expression staging implementation; Shard must not depend on Fresco
types or gain access to ExprRepos's private containers.

Each savepoint records the stream position and a mark for staged semantic state,
including pending roots. Failure restores both. A successful child match leaves
its results staged under its parent's transaction; it does not publish them.
Only explicit acceptance of the outer parse publishes results. Full-string
parsing checks end of input before acceptance; prefix parsing accepts its parsed
prefix explicitly. Do not publish automatically merely because `ParseGrammar`
returns successfully at marker depth zero.

Keep staged nodes in contiguous Trellis containers with local IDs. At publication,
resolve existing/new variable identities and remap local child/root IDs to final
repository IDs. Prepare and validate the batch before modifying the repository,
so normal validation errors cannot leave a partial append. Publish only live
accepted results, preserving existing reuse and distinct-variable semantics.
Avoid a boxed callback allocation per node solely to support undo.

Internal grammar backtracking must restore semantic marks as well as input marks;
audit direct `SetCurrMark` calls rather than assuming `ParseGrammar` sees every
alternative. Persistent variable flags are not changed speculatively. Any proposed
attribute updates stay pending until acceptance and follow the assumption rules.

Put mutable staging/result state behind the parser's genuine mutable access;
grammars can describe matching without casting `&self` to `&mut Self`. This design
must remove the current cast in `ExprShard::Match`, not relocate it to a helper.
Standalone import and tree lowering still need atomic publication of their own
prepared results; parser checkpoints do not cover non-parser entry points.

## Findings and recommended actions

### F1 — P1: Algebraic lowering does not preserve numerical behavior or domains

Evidence: [exprrepos.rs](../../src/fresco/exprrepos.rs), `CollectAdditive` at
line 315 and `CollectMultiplicative` at line 354.

Parity propagation fixes signs and reciprocal positions algebraically, but
reorders and regroups operations. For example, `a/(b/c)` becomes `(a*c)/b`.
With `a=1`, `b=1`, `c=0`, the original expression contains an inner division by
zero, while the rewrite does not. A checked evaluator would distinguish them;
raw IEEE arithmetic can yield zero in both cases through an intermediate infinity.
Over real arithmetic their domains differ. For finite nonzero values, the rewrite
can change floating-point overflow and rounding: `a=1e308`, `b=1e308`, `c=2` gives
a finite original result
of approximately 2, while multiplying `a*c` first overflows. Additive regrouping
likewise changes rounding and cancellation.

Action: document current lowering as algebraic normalization with domain
assumptions. Define the numerical contract before implementing evaluation. If
source-ordered floating-point behavior is required, preserve nested operations
at ingestion and make normalization an explicit transformation. Do not introduce
dual runtime modes without a consumer requirement. Do not call normalization
exact grouping preservation. Keep existing parity tests as normalization tests.

Acceptance: zero, signed-zero, non-finite, cancellation, overflow, and nonzero
finite examples distinguish symbolic equivalence from numerical behavior. Test
each implemented contract without requiring a second mode just for this gate.

#### Variable assumptions: `NonZero`

Use `VarKind::NonZero = 4` in the existing private `_Flags: u32` on `VarAttrib`.
Query it with `HasBits(VarKind::NonZero)`; no separate assumption mask is needed.
Existing role flags retain their bit positions and combine with NonZero.
Configure flags through a repository method using the variable ID, not an
expression ID or name alone. An unset NonZero bit means unknown, not known zero.
The enum flag is now present, but `_Flags` defaults to zero, `VarAttrAt` exposes
only a shared reference, and there is no ordinary flag setter. Configuration,
validation, and assumption-aware rewriting therefore remain planned work.

Add a repository-mediated flag setter accepting a variable ID and `VarKind`, with
controlled enable/disable behavior and an invalid-ID error. It must preserve
unrelated role bits and obey the assumption-freeze rule below. Use `HasBits` for
queries. Avoid exposing mutable variable attributes merely to configure NonZero.

The initial contract is deliberately small:

- Assumptions belong to variable identity. All `VarExpr` nodes sharing a variable
  ID see the same assumption; distinct same-name variables remain independent.
  Name reuse must not silently replace or strengthen existing assumptions.
- In real symbolic arithmetic, `x/x -> 1` requires `NonZero(x)`.
  `a/(b/c) -> (a*c)/b` is justified on the common domain with `NonZero(b)` and
  `NonZero(c)`. If a required fact is unknown, preserve the nested expression or
  retain its explicit domain condition rather than silently broadening its domain.
- A nonzero variable does not establish that an arbitrary expression containing
  it is nonzero: `x-y` may vanish even when both variables are nonzero. Start with
  direct variable facts and exact nonzero constants; add conservative expression
  inference later. Do not infer floating-point nonzero results from real algebra
  when underflow can produce zero.
- Validate assumptions at the numerical binding boundary. Reject both `+0.0` and
  `-0.0` for `NonZero`; reject NaN as a real binding. Define infinity under the
  evaluator's separate finiteness policy. `value != 0.0` alone is insufficient
  because NaN passes that comparison. Validate dependent variable values too if
  they carry assumptions. Removing a variable through rewriting must not remove
  the obligation to validate its assumed binding.
- Freeze assumptions before publishing transformations that depend on them.
  Initially disallow weakening them while those transformed expressions remain
  usable. Mutable assumptions would require premise tracking and rebuilding or
  invalidating dependent expressions; a cache flush alone cannot undo a rewrite.
- Serialize NonZero in the existing `_Flags` attribute, interpret older data with
  the bit unset as unknown, and reject unsupported flag bits. A loaded normalized
  expression must retain the assumptions on which its validity depends.

NonZero addresses division/cancellation domain restrictions. Even finite nonzero
inputs can overflow, underflow, or round differently after regrouping, so F1's
numerical contract is still required. The existing unconditional parity lowering
does not yet enforce these premises; adding metadata alone does not complete F1.

Acceptance: unknown versus NonZero, shared versus distinct variable identity,
valid negative/positive bindings, both zero signs, NaN, violated assumptions after
cancellation, frozen-assumption behavior, and old/new Flux data are covered.
Verify that unknown facts block an unconditional domain-sensitive rewrite.
Setting NonZero must preserve Scalar/Prime/Control/Bridge flags, leave distinct
variables unaffected, and fail atomically for an invalid variable ID.

### F2 — P1: Rollback accepts checkpoints from invalid repository states

Evidence: [exprrepos.rs](../../src/fresco/exprrepos.rs), `ExprCheckpoint`,
`Checkpoint` at line 97, and `Rollback` at line 105;
[stash.rs](../../src/silo/stash.rs), `Resize`.

A checkpoint contains counts only, is public and copyable, and is not tied to a
repository or transaction. The current guard rejects growing counts, but a foreign
checkpoint with smaller counts can still truncate unrelated state. It also cannot
restore changes to existing variable flags. Numeric IDs can be reused after
truncation. Moving the counts into Parser while continuing direct repository
mutation would leave these ownership and restoration problems unresolved.

Action: replace repository checkpoints with Parser-owned savepoints over staged
semantic state, as specified in the architecture decision above. ExprRepos receives
prepared accepted batches only. Keep temporary IDs within their parse context;
return persistent IDs only after publication. Remove `ExprCheckpoint` exports and
checkpoint call sites once the staged path is in use.

Acceptance: failed parses cannot alter repository counts, existing attributes,
name bindings, or roots. Inner success followed by outer failure discards staged
results. Checkpoints cannot be applied to a repository or reused outside their
parse transaction. Existing persistent IDs remain valid; staged IDs never escape.

### F3 — P1: Local rollback does not cover outer grammar failure

Evidence: [exprshard.rs](../../src/fresco/exprshard.rs), `Parse` at line 42,
`IGrammar::Match` at line 735, and `RootToken`.

`Parse` checkpoints and restores on error. The working-tree implementation also
restores repository counts and resets its root on a failed `IGrammar::Match`.
However, successful prefix matching followed by failure of a containing grammar
still has no parser-owned semantic rollback. A child result is persistent too
early, and stream backtracking alone cannot discard it or invalidate that root.

Action: Parser owns nested savepoints, pending roots, and explicit outer acceptance.
Restore stream and staged semantic state together on failure. A successful prefix
is valid for `IGrammar`, but publication waits for its caller's acceptance policy.
Remove persistent side effects and independently cached roots from speculative
grammar matching. Reuse staging capacity without retaining rejected logical results.

Acceptance: run failure/retry tests through both entry points, including a
containing grammar that accepts a Fresco prefix then fails. Assert counts, name
bindings, root-token validity, and subsequent reuse, not just counts.

### F4 — P1: Recursion limits do not cover Fresco's recursive descent

Evidence: [exprshard.rs](../../src/fresco/exprshard.rs), `MatchUnary`,
`MatchPrimary`, and `MatchIf`; [exprrepos.rs](../../src/fresco/exprrepos.rs),
tree collectors; [parser.rs](../../src/shard/parser.rs), `K_MAX_PARSE_DEPTH`.

Shard caps nested `ParseGrammar` markers, but Fresco's recursive calls invoke
its private match methods directly. Deep parentheses, unary chains, nested
conditionals, and term trees therefore bypass that limit. Stack exhaustion can
abort the process instead of returning the parser's advertised `Result`.

Action: add a Fresco depth budget and a node/input budget. Check at recursive
boundaries, unwind without mutation residue, and return a structured limit error.
Use an explicit work stack for tree lowering if supported depths require it.
Count unary recursion, parentheses, conditionals, functions, and tree edges,
not just Shard markers. Reject excessive input before allocating a full copy or
creating nodes. Document what each budget measures and make defaults finite;
exact values need a stack/allocation baseline rather than an arbitrary guarantee.

Acceptance: cases at and beyond configured limits fail predictably, including
term-tree input. Set conservative limits without using stack overflow as a test.

### F5 — P2: Construction permits invalid references and contradictory arity

Evidence: [exprrepos.rs](../../src/fresco/exprrepos.rs), creation methods and
`At` at line 278; [powexpr.rs](../../src/fresco/powexpr.rs), `New`;
[logicexpr.rs](../../src/fresco/logicexpr.rs), `New`, `Unary`, `IsUnary`;
[polyexpr.rs](../../src/fresco/polyexpr.rs), generated import.

Factories accept arbitrary child IDs. Power factories still accept arbitrary
numbers of bases/exponents although the documentation says powers are strictly
binary. `LogicUnaryCreate(Eq, x)` and binary `LogicCreate(Not, x, y)` are accepted;
unary status is inferred from the right-child sentinel independently of the
operator. `At<T>` panics on invalid IDs or wrong types. Field-based import can
bypass the partition assertion in `PolyExpr::FromParts`.

Action: add checked factories and lookup for boundary callers, define power and
logical arity invariants, and validate imported objects before publication. For an
append-only graph, requiring children to precede the parent cheaply excludes
cycles; support forward references only with explicit graph validation. Define
child enumeration for custom `BaseExpr` implementations before relying on generic
validation. Keep trusted asserting accessors where appropriate.

Also define empty sums/products, the meaning (if any) of a bare `PolyExpr` stored
through `Store`, variable attribute references, and unsupported custom node types.
Centralize built-in validation so parse, direct construction, and future import
cannot disagree. During append-only construction, existing valid child IDs are
sufficient for cycle prevention; do not add a full graph walk to every insertion.

Acceptance: invalid IDs, sentinels, partition counts, power arity, and logical
arity return useful errors with no partial insertion.
Add checked methods alongside existing entry points first. Keep current `u32`
IDs and project sentinels; document cross-repository IDs as unsupported unless
an ownership-aware handle design is separately justified. Preserve `ParseExpr`'s
`Result<u32, String>` wrapper while exposing structured errors through a detailed
entry point, avoiding a simultaneous break for all callers.

### F6 — P2: Text and macro construction disagree on precedence and identity

Evidence: [exprshard.rs](../../src/fresco/exprshard.rs), `MatchPow` at line 397,
`MatchUnary` at line 364, and comparison parsing;
[node.rs](../../src/stalks/node.rs), `NodeTree!`;
[exprrepos.rs](../../src/fresco/exprrepos.rs), line 401.

Text powers associate left-to-right: `2^3^2` means `(2^3)^2`. Unary minus binds
before power: `-2^2` means `(-2)^2`. The macro instead builds the remaining tokens
as the right subtree, without arithmetic precedence: `a*b+c` becomes `a*(b+c)`.
Text parsing reuses variable attributes, but term-tree leaves pass `reuse=false`:
two occurrences of `x` become distinct variable IDs. Neither path exposes a clear
common expression-language contract.

Action: specify precedence, power associativity, unary NOT precedence, comparison
chaining, and name-reuse policy. Preserve legacy syntax through an explicit mode
if required. Give Fresco a dedicated precedence-aware macro or document mandatory
grouping; avoid changing shared `NodeTree!` semantics for unrelated users. Expose
the same explicit variable reuse policy for both construction paths.

Acceptance: structural tests for mixed arithmetic, both power nestings, negative
exponents, `!x == y`, `not x == y`, chained comparisons, and repeated names.
Text and explicitly grouped macro input agree over their shared supported subset.

### F7 — P2: Permissive numeric syntax needs an explicit compatibility contract

Evidence: [exprshard.rs](../../src/fresco/exprshard.rs), `MatchRawNumber` at
line 199; [_tests.rs](../../src/fresco/_tests.rs), legacy parsing corpus.

Incomplete exponent suffixes (`1e`, `1e+`, `1e-`) are consumed and removed before
numeric conversion. Trailing `e` is intentionally covered by legacy tests.
Numeric overflow is not checked for finiteness after conversion. Errors otherwise
provide a generic message or trailing-input position, rather than the failed
token expectation.

Action: retain required compatibility behavior explicitly and offer strict
parsing with structured byte-span diagnostics. Define whether non-finite literals
are allowed; report each compatibility recovery rather than silently hiding it.
Strict syntax is a later opt-in API, not a prerequisite for safe rollback. Make
recovery diagnostics available to callers without introducing logging side effects.

Acceptance: valid scientific notation, each incomplete exponent, decimal edges,
overflow, empty input, and invalid Unicode tokens have precise outcomes.

### F8 — P2: Documentation and completion claims exceed the evidence

Evidence: [module documentation](../modules/fresco.md), sections 1, 3, and 5;
[existing blueprint](fresco-improvement-plan.md), completion table.

No evaluator, simplifier, or substitution engine exists in Fresco. Concrete
nodes have Flux import-source macros, but `ExprEntry` and `ExprRepos` only export;
there is no complete heterogeneous repository import. The docs describe arbitrary
power factories as strictly binary, and describe insertion as logarithmic.
The blueprint quotes different historical suite totals without a reproducible
baseline and still describes negative-literal wrapper allocation that the current
implementation has already removed.

Action: correct capability and complexity statements, distinguish intended uses
from actual repository consumers, and report exact verification commands and
current results. Keep historical milestones separate from outstanding work.

### F9 — P3: Interning and lowering still need performance measurements

Evidence: [exprrepos.rs](../../src/fresco/exprrepos.rs), `VarCreate` at line 135,
`Rollback` at line 105, and recursive collectors.

Lookup is O(log V) name comparisons, but insertion shifts O(V) IDs. Reverse-sorted
unique symbols still cause O(V^2) total shifts. Rollback scans the symbol index,
even if a failed parse added no variables. Collectors allocate temporary `Stash`
instances for individual subtree results; power lowering creates separate result
stacks for its two children. These costs remain despite one-node negative literals.

Action: benchmark ascending, descending, random, and repeated names; repeated
invalid input against a populated repository; and large nested/chained trees.
Return a child ID directly from traversal where practical, sharing scratch storage
instead of allocating one-element stacks. Consider batch index construction or
another project-compatible ordered index only if insertion dominates. Gate a tagged
arena migration on allocation/cache measurements and preserve the extension API.

Acceptance: record input size, profile/toolchain, node counts, allocations, peak
memory, and timing. Include insertion cost in complexity claims. Do not equate
vectorization or a storage rewrite with an established speedup.

## Delivery plan

| Change | Scope and dependency | Acceptance gate | Relative size |
|---|---|---|---|
| 0 | Reproduction cases, documented current semantics, construction baseline; F1-F9 | Structural expectations and failure cases recorded; focused suite and benchmark commands reproducible | Small-medium |
| 1 | Parser-owned savepoints and semantic staging; F2-F3, after change 0 | Failed/direct/composed parses leave ExprRepos unchanged; outer acceptance publishes validated results; repository checkpoint APIs removed | Medium-large |
| 2 | Shared validation, detailed errors, and finite budgets; F4-F5, after change 1 | Invalid arity/references and over-budget input return errors atomically; compatibility wrapper still works | Medium |
| 3 | Language/identity policy, NonZero variable assumptions, and numerical contract; F1, F6-F8, characterized in change 0 | Explicit assumption configuration, unknown-fact handling, and rewrite premises tested; behavior changes use opt-in/versioning | Medium |
| 4 | Reference evaluator, after changes 1-3 | Supported nodes, lazy branches, binding-assumption validation, and domain behavior verified numerically | Medium-large |
| 5 | Complete Flux import, after changes 2-3 | Bounded atomic load; schema/root format, variable identity, and rewrite assumptions preserved | Medium |
| 6 | Optimize measured bottlenecks against change 0 baseline; F9 | Before/after construction results; evaluation results after change 4; arena migration separately justified | Measurement-dependent |

Relative sizes describe scope, not calendar commitments. Changes 1-2 form the
recommended first implementation milestone. Change 3 can progress independently
after characterization; decisions affecting representation must settle before
evaluation or persistence formats freeze. Changes 4-5 add capabilities and are
not needed to finish the parsing-safety milestone. Measurement starts in change 0;
optimization does not have to wait for every capability addition.

Recommended first milestone cases:

1. Parser savepoint scope, complete discard of rejected staging, and preservation
   of pre-existing repository roots and variable attributes.
2. Direct grammar failure/retry, success followed by failure, and outer grammar
   failure after a successful Fresco prefix. Include a prefix-success control.
3. New names inserted before/between/after existing sorted names, rollback, and
   subsequent reuse. Include same-name distinct variables and nested transactions.
4. Every recursive shape just below/at/above its configured limit; invalid child
   IDs and contradictory logical/power arity. Avoid intentionally crashing a stack.

Add meaningful regressions with each fix; do not leave failing tests in the
delivered baseline. Characterizations of legacy behavior should be named as such,
so they do not accidentally declare that behavior the permanent language contract.

For evaluation, use `Arr<'_, f64>` bindings and explicit result/error types. Decide
numeric truth conversion, unknown functions, division by zero, power domains, and
non-finite propagation. Evaluate only the selected conditional branch and required
logical operands. A topological plan must preserve laziness rather than evaluate
all nodes eagerly. Add simplification after an evaluator provides an oracle, and
attach domain assumptions to rewrites such as `x/x -> 1`.

Start with a bounded interpreter using the existing storage representation and
explicit unsupported-node errors. Define function purity before caching or
deduplicating calls. Do not require a tagged arena, SIMD, or a compiled execution
plan for the reference evaluator. Test against independent numerical expectations,
including absolute/relative tolerances; comparing two Fresco paths alone can
preserve the same bug.

For import, use Flux type dispatch into temporary storage, then validate references,
arity, partitions, supported discriminators, and schema version. Rebuild the derived
symbol index deterministically, preserve distinct same-name variables, and publish
only after success. Define the root format and compatibility for prior nullable
attributes versus the current sentinel representation.

Apply input, node, child-count, and string-length budgets while decoding, before
large allocations. Define duplicate/missing fields, unsupported versions/types,
multiple roots, and failure on truncated input. Treat raw generated field import
as a decoding mechanism, not validated repository publication.

Keep each change reviewable, use Trellis containers/traversal and private fields,
place new `jeeves_test!` cases in Fresco's `_tests.rs`, and update Natvis if storage
changes. Avoid introducing Serde, Tokio, or speculative storage abstractions.

## Verification record

On 2026-10-22, `cargo test -p trellis --lib fresco --offline` built the test target
and passed all 10 Fresco tests (zero failures; 329 other tests filtered out).
Existing tests establish a baseline; the additional cases above have not been
implemented by this review. On 2026-10-23, source and call-site inspection confirmed
the same source revision; changes from this re-review affect only plan documents.
The passing baseline was not rerun for documentation edits. No full-suite or
Python-feature result is claimed for this review.

Follow-up on 2026-10-23: added `VarKind::NonZero = 4` to the existing flag enum.
`cargo test -p trellis --lib fresco --offline` rebuilt successfully and passed
all 10 Fresco tests (zero failures; 329 filtered out). This verifies enum integration;
the assumption validation and rewriting gates above remain planned work.

Latest review on 2026-10-23 reconfirmed that the enum addition is the sole Fresco
source change relative to `fb49a03`. The status table and next implementation scopes
were refreshed against that working tree. No production code was changed during
this review; the successful focused run above remains the verification baseline.

Implementation update on 2026-10-23:

This records the earlier repository-rollback approach. It does not establish
completion of the Parser-owned staging design selected above; that migration and
its acceptance gates remain outstanding. The shared-to-mutable reference cast
used by the RefCell removal must also be replaced with sound mutable access.

- Implemented Changes 1 & 2: transaction sealing, defensive checkpoint rollback guards,
  checked node constructors, VarKind::NonZero configuration, and AST tree recursion limits.
- Eliminated `RefCell` and `Cell` from `ExprShard`: replaced `RefCell<&'a mut ExprRepos>`
  with `&'a mut ExprRepos`, replaced `_Depth: Cell<u32>` with a zero-cost call-stack parameter
  `depth: u32`, and simplified `_RootToken` to a scalar `u32`.
- Verification: 13/13 Fresco unit tests passed (`cargo test --features python -- fresco`),
  351/351 full test suite passed (`cargo run --features python -- -t`), 0 clippy warnings
  (`cargo clippy --features python -- -D warnings`), and verified 100% pure Unix LF line endings.

Broad implementation changes should also run `cargo check --all-targets`,
`cargo clippy -- -D warnings`, and the relevant Cove suite per repository guidance.
