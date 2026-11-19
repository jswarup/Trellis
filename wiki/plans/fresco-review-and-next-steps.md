# Fresco implementation review and improvement plan

Date: 2026-11-18. Baseline: commit `9027fba`, initially clean working tree.
Scope: scalar/tensor storage, parsing and publication, evaluation, symbolic
differentiation, polynomial normalization, Flux integration, tests, and module docs.
This review updates documentation and plans; implementation fixes remain outstanding.

## Assessment

Fresco provides useful scalar and tensor construction, evaluation, symbolic
derivatives, and polynomial transformations. Recent tensor fixes have regression
coverage. Its current numerical and lifecycle contracts are not sufficient to
treat all generated derivatives or normalized expressions as interchangeable
with the originals, or to build an unrestricted numerical solver on top.

The next milestone should harden transaction ownership, derivative domains,
normalization, and validation. Keep the reference implementation and use measured
workloads before introducing compiled evaluation, AD, or a storage rewrite.

## Current capabilities and evidence

| Area | Implemented | Important boundary |
|---|---|---|
| Scalars | Boxed BaseExpr entries referenced by u32 IDs; constants, variables, sum/product partitions, powers, functions, logic, conditionals | Raw factories accept arbitrary IDs/arity; IDs are not ownership-aware handles |
| Parsing | ExprGrammar with a genuinely mutable ExprParseCtx; staged nodes and full-string publication; Parser accepts generic mutable context | Parser transactions restore stream marks only; semantic rollback remains local to grammars |
| Evaluation | Indexed/named scalar and tensor evaluation, depth bound, lazy conditional/AND/OR behavior, selected domain checks | Recursive; named bindings allocate; non-finite and malformed-node behavior is inconsistent |
| Symbolic calculus | Scalar Differentiate, vector gradients, residual Jacobians; rollback in vector/Jacobian helpers | Supported unary derivatives are sin, cos, exp, ln, sqrt; singular/inactive cases need fixes |
| Tensor shapes | Scalar through rank six, checked dimension constructors, strides, broadcasting | Public mutation and field import can bypass constructor validation |
| Tensor variables | All leading-prefix levels, scalar leaf IDs, shape/offset metadata, duplicate-name preflight, exact-first lookup | Aggregate arithmetic, assumptions, imported metadata, total metadata capacity need explicit validation |
| Tensor expressions | Shape plus flat scalar-expression IDs; structural operations, broadcasting, dot/matrix/outer products | Scalar DAG lowering rather than a tensor execution backend; element IDs are not universally prevalidated |
| Polynomials | Coefficient/power terms, expansion, collection, Add/Subtract/Multiply, conversion back to expressions | Floating coefficients and implicit pruning; ordering is not a complete canonical order |
| Persistence | Tagged heterogeneous node/repository export and import hooks; name-index rebuild | A successful import is not graph/shape validation or atomic load |
| Optimization/AD | Least-squares and tensor architecture plans exist | LM, QR, compiled programs, forward AD, reverse AD, and Fresco Python adapters are not implemented here |

### Earlier tensor findings now addressed

The current source/tests cover multilevel partial indexing, zero/overflow rejection
in checked shape constructors, aggregate/generated-symbol collision checks,
exact-name lookup before bracket aliases, Jacobian residual prevalidation and
error rollback, and numerical broadcasting. Matrix multiplication now indexes
scalar elements directly instead of allocating a row and column per output.
These improvements do not imply that mutable/imported shapes or every failure path
are validated.

## Findings and required changes

### R1 - P1: Repository rollback can corrupt name lookup

Evidence: [exprrepos.rs](../../src/fresco/exprrepos.rs), Checkpoint, Rollback,
VarCreate, InsertSymbolIndex. Confirmed by the review probe.

Rollback truncates the sorted symbol index to its old length, but inserting a name
can shift existing entries. Register z, checkpoint, register a, and roll back:
the retained index can contain the removed variable ID. FindVar(z) then panics.
The public count-only checkpoint can also be applied to another repository or an
invalid lifetime, can grow placeholder storage, and does not restore changed
attributes. The existing rollback test does not cover lexically earlier insertion.

Fix: keep parsing savepoint ownership in Parser; use scoped internal transactions
for derivative/batch preparation. Until replacement, restore the actual index
contents or rebuild a deterministic canonical index from retained variables,
reject growing/foreign/stale checkpoints, and prohibit or journal updates to
existing attributes. RebuildSymbolIndex must preserve the documented canonical
lookup choice for intentional same-name variables.

Gate: earlier/between/later names, multiple and nested rollbacks, duplicate names,
foreign/stale tokens, and existing-root/attribute preservation. Rejection must
not silently corrupt a previously usable repository.

### R2 - P1: Symbolic derivatives introduce avoidable singularities

Evidence: [diff.rs](../../src/fresco/diff.rs), DiffPow and DiffFunc. Both examples
below were reproduced numerically.

For x^0, the evaluator returns 1 at x=0, but differentiation generates a factor
containing x^-1 and evaluation fails with DivisionByZero instead of returning zero.
Differentiating sqrt(x) with respect to an independent y generates a zero factor
multiplied by a singular expression; at x=0 it also fails instead of returning zero.
The general power rule introduces logarithm/division domain requirements, and
Differentiate itself has no depth, cycle, memoization, or transactional boundary.

Fix: specialize constant zero/one/integer exponents and skip provably inactive
derivative paths before constructing domain-sensitive factors. Define derivative
domains independently of primal domains. Validate the graph and active variable,
memoize requests by expression/variable ID, and bound derivative growth. Reject
unsupported functions before appending derivatives. Keep existing variable IDs
and return structured errors without partial preparation.

Gate: x^0/x^1/x^2 at zero, integer powers at negative bases, inactive sqrt/log/power
inputs, zero-valued products, independent variables, unsupported nodes, cycles,
shared graphs, and failed-preparation restoration. Numerical differences must use
domain-safe perturbations and independent analytical expectations.

### R3 - P1: Polynomial normalization is lossy and not fully canonical

Evidence: [polyexpr.rs](../../src/fresco/polyexpr.rs), Constant, Normalize,
FromExpr, derived PartialEq. Confirmed by the probe.

Constant and Normalize drop coefficients with magnitude below 1e-12 without a
caller-selected approximation policy. For 1e-13*x at x=1e13, conversion back to an
expression changes the value from 1 to 0. Normalize sorts by total degree only;
x+y and y+x therefore produce unequal polynomial structures when using identical
variable IDs. Floating coefficient arithmetic is not an exact symbolic equivalence
proof even after deterministic ordering is implemented.

Fix: make coefficient pruning explicit and optional; preserve nonzero coefficients
by default. Establish a total order over variable/exponent signatures, normalize
monomials including zero powers, and collect deterministically. Validate finite,
bounded integral exponents before casting/expansion and cap terms, degree, memory,
and work. Choose whether public negative powers are supported Laurent terms or
rejected by the polynomial contract. Do not promise exact equivalence from f64
structural equality.

Gate: commuted equal-degree terms, repeated powers, tiny/scaled coefficients,
cancellation, zero powers, non-finite coefficients, large exponents, and bounded
expansion. Approximate comparisons state their tolerance and domain assumptions.

### R4 - P1: Parsing context exists, but outer semantic transactions are incomplete

Evidence: [parser.rs](../../src/shard/parser.rs), ParseGrammar and Transaction;
[exprshard.rs](../../src/fresco/exprshard.rs), ExprParseCtx, RootToken, Publish,
PublishSubtree, and ExprGrammar::Match. Outer rejection was reproduced.

The former shared-to-mutable cast is gone: IGrammar receives mutable context.
However, Parser still checkpoints stream marks only. A successful expression
inside a rejecting outer grammar leaves a pending root; RootToken can then publish
it. Reusing a published context does not consistently reset publication state.
PublishSubtree recursively traverses flat-chain DAGs without its own bound and
appends nodes before a complete preparation/validation pass. Public operational
context fields also let callers bypass its invariants.

Fix: give Parser grammar-independent semantic savepoint support, including staged
nodes and pending results. Child success remains provisional until explicit outer
acceptance. Make result queries pure; initially reject reuse after acceptance.
Prepare/remap/validate a bounded live-node batch before publication, using an
explicit stack or ordered pass. Keep ExprParseCtx fields private. Input, node,
string-length, publication-work, and graph-depth limits must cover long flat input
as well as syntactic nesting. Preserve established legacy syntax unless explicitly
versioned or exposed through an opt-in API.

Gate: rejected outer grammar after inner success, alternation, trailing input,
prefix acceptance, repeated publication/reuse, long flat chains, malformed staged
graphs, and result access after failure. No rejected parse publishes a root.

### R5 - P1: Flux load accepts invalid repository graphs

Evidence: [exprrepos.rs](../../src/fresco/exprrepos.rs), ExprEntry import and
ExprRepos import validation; [shape.rs](../../src/fresco/shape.rs) and
[polyexpr.rs](../../src/fresco/polyexpr.rs), field import. Confirmed by loading a
VarExpr referring to variable ID 999 into a repository with no variables.

The repository validator rebuilds the index and returns true without checking
references, cycles, partitions, arity, shape rank/counts, aggregate links, flags,
or required fields. Decoding writes directly into the destination. Constructor
checks therefore cannot establish validity or atomicity for imported data.

Fix: decode into bounded temporary storage, validate supported discriminators and
required/duplicate fields, then validate the whole graph and aggregate metadata.
Define versions, roots, unknown-field behavior, sentinel compatibility, duplicate
symbol policy, and assumption persistence. Publish only after successful validation.
An explicit schema avoids guessing defaults that turn malformed data into a model.

Gate: invalid IDs/cycles/ranks/partitions, missing and duplicate fields, truncated
input, unknown versions/types/flag bits, exact identity preservation, and failure
leaving an existing destination unchanged. Test scalar and hierarchical tensor
round trips, not only successful scalar import.

### R6 - P2: Checked tensor construction can be bypassed

Evidence: [shape.rs](../../src/fresco/shape.rs), SetDim, Strides and field import;
[exprrepos.rs](../../src/fresco/exprrepos.rs), TensorVarCreate/MaterializeTensorLevels;
[tensor.rs](../../src/fresco/tensor.rs), contraction output sizing;
[diff.rs](../../src/fresco/diff.rs), ComputeJacobianElements.

SetDim can turn a valid Vector(2) into a zero-dimensional shape; TensorVarCreate
accepts that mutated shape. This was reproduced. Imported shape state also bypasses
checked constructors. Products may fit u32 while the sum of all hierarchy levels,
plus existing variables, does not. MatMul/OuterProduct/Jacobian calculate output
products for capacity before checking their final shape; those products need
checked arithmetic and a practical allocation budget.

Fix: centralize Validate on shapes and call it at mutable/imported/batch boundaries.
Make invariant-changing shape mutation checked or internal. Precompute checked
element, hierarchy, ID, and output counts before name enumeration or repository
mutation. Apply node/memory budgets and prevalidate tensor element IDs. Raw scalar
evaluation should reject aggregate variable IDs rather than treating a vector
parent as an independent scalar slot.

Gate: mutated/imported zero/overflow shapes, rank bounds, total hierarchy overflow,
output-product overflow without huge allocation, invalid element IDs, aggregate
parents in scalar expressions, and atomic failure of tensor operations.

### R7 - P2: Numerical evaluation still has malformed-node and domain gaps

Evidence: [eval.rs](../../src/fresco/eval.rs), power/logical branches and binding
handling; [exprrepos.rs](../../src/fresco/exprrepos.rs), raw factories/tree lowering.

Binary Not can reach unreachable, extra power children are ignored, invalid
partitions can panic, and constants/functions/arithmetic may produce NaN/infinity
while variable NaN is rejected. Missing named bindings share the NaN error path.
Assumption checks run only when a VarExpr is visited. Tree normalization still
changes division domains and numerical ordering; repeated names in TermTree still
use distinct variable IDs unless an explicit reuse policy is added.

Fix: reject malformed built-ins defensively and define finite/non-finite, power,
truth-conversion, missing/duplicate-binding, and named-ambiguity contracts.
Preserve lazy branches. Retain VarKind::NonZero in the existing mask: unset means
unknown, and nonzero facts do not prove arbitrary expressions such as x-y nonzero.
Before cancellation/reordering, preserve required domain obligations and freeze or
invalidate dependent results on assumption changes. NonZero does not establish
IEEE equivalence. Add checked atomic term-tree ingestion rather than depth asserts
after partial insertion. Bound repeated shared-DAG evaluation work.

Gate: malformed arity/partitions, zero signs, NaN/infinity constants and bindings,
overflow/underflow, all function domains, lazy OR/NOT/conditional cases, ambiguous
names, text/tree domain differences, and assumption-lifetime tests.

### R8 - P3: Measure preparation and execution separately

The representation allocates a box per scalar node and child buffers for composite
nodes. Differentiation clones entries recursively and repeats work; polynomial
expansion can grow rapidly. Sorted name lookup is O(log V), insertion O(V).
Hierarchical variable creation materializes every prefix level and formats names
in multiple passes. Named evaluation allocates a repository-sized binding buffer;
tensor evaluation traverses each scalar root separately.

Measure construction, derivative preparation, expansion, import, indexed/named
evaluation, and complete solve workloads separately. Record graph sharing, nodes,
edges, live variables, allocations, memory, profile/toolchain, and timing. First
consider reusable scratch/bindings, derivative memoization, shared pure-subgraph
caching, batched symbol indexing, and exponentiation by squaring. Gate compiled
execution or a tagged arena on evidence and preserve lazy semantics/custom-node
boundaries. Do not add Serde, Tokio, or new reference-counted domain graphs.

## Delivery plan

| Unit | Scope | Acceptance gate / dependency |
|---|---|---|
| 1 | Scoped state restoration and Parser semantic ownership; R1, R4 | Rollback/outer-failure probes become permanent regressions; no stale roots or broken lookup |
| 2 | Shared graph/shape validation and bounded publication/load; R5-R7 | Malformed input returns structured errors; destination/repository unchanged on rejection |
| 3 | Domain-safe, bounded symbolic preparation; R2 | Analytical edge cases pass; no inactive derivative singularities or failed-preparation growth |
| 4 | Deterministic, explicit polynomial policy; R3 | No implicit loss of nonzero coefficients; order-independent term signatures; bounded expansion |
| 5 | Numerical and assumption contract; R7 | Consistent domain/binding/error behavior, text/tree policy, lazy branches, and premise lifetime |
| 6 | Least-squares MVP from the existing solver plan | Units 1-3 and 5 pass for its supported subset; unit 4 is required if normalization is used |
| 7 | Measured optimization, compiled evaluation, AD, and integrations | Workloads justify each extension; independent numerical parity and lifecycle gates pass |

Start benchmark capture alongside correctness fixes; measurements do not block
urgent lifecycle or numerical fixes. Keep units separately reviewable. Use the
existing jeeves_test registration and component test files, small private types,
Arr/MutArr/Buff/Stash/USeg, u32 IDs/sentinels, and compile-time exclusive mutation.
Make operational context fields private and retire public slice adapters or document
an explicit compatibility migration. Update Natvis when core storage changes.

The [tensor specification](fresco-tensor-and-vector-variable-plan.md) describes
implemented APIs and future interoperability; its phase labels do not prove all
hardening gates. The [least-squares plan](fresco-least-squares-implementation-plan.md)
remains a future implementation proposal. Symbolic differentiation is not forward
or reverse AD; a Jacobian helper is not an optimizer or linear solver.

## Verification and limits

On 2026-11-18, `cargo test -p trellis --lib fresco --offline` rebuilt successfully:
27 passed, zero failed, 397 filtered out. Existing regressions cover the earlier
tensor fixes. No full-suite, Python-feature, all-target, or Clippy result is claimed
for this review.

A temporary direct-API probe ran with
`cargo run -p trellis --example fresco_audit_probe --offline`, then was removed.

| Check | Observed result |
|---|---|
| Evaluate x^0 at x=0, then its symbolic derivative | Primal Ok(1); derivative DivisionByZero |
| Partial derivative of sqrt(x) with respect to independent y at x=0 | DivisionByZero |
| Normalize x+y and y+x with identical variable IDs | Structural equality false |
| Normalize 1e-13*x and evaluate at x=1e13 | Original Ok(1); transformed Ok(0) |
| Register z, checkpoint, register a, rollback, FindVar(z) | Panic, caught by probe |
| Outer grammar rejects a successfully staged expression, then read RootToken | Root 0 published; repository grows to one node |
| Mutate Vector(2) to dimension zero and declare an aggregate | Accepted |
| Import VarExpr(999) with no variable attributes | Accepted |

Unbounded traversal risks, stale-context reuse, non-atomic import paths, and resource
costs are source findings, not crash/performance measurements. No deliberate
stack-exhaustion or huge-allocation experiment was performed. Broad implementation
units should run the required all-target checks, Clippy, and relevant registered
suite in addition to focused regressions.
