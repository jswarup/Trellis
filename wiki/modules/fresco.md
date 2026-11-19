# Fresco: expressions, tensors, and symbolic mathematics

**Path:** `src/fresco/`

**Rust module:** `trellis::fresco`

**Reviewed:** 2026-11-18, commit `9027fba`

Fresco stores scalar expression graphs, parses scalar formulas, evaluates scalar
and tensor expressions, constructs symbolic derivatives, and expands a supported
polynomial subset. Tensor operations lower to scalar expression IDs; there is no
separate tensor execution engine or numerical optimizer.

See the [implementation review and improvement plan](../plans/fresco-review-and-next-steps.md)
for confirmed limitations, evidence, priorities, and acceptance gates. The
[least-squares proposal](../plans/fresco-least-squares-implementation-plan.md) and
[tensor specification](../plans/fresco-tensor-and-vector-variable-plan.md) distinguish
future work from the current supported surface.

The [expression-viewer implementation plan](../plans/fresco-expression-viewer-implementation-plan.md)
proposes a native Fascia math tab, component rendering, and retained tensor notation.

## Components

| File | Types / entry points | Responsibility |
|---|---|---|
| [exprrepos.rs](../../src/fresco/exprrepos.rs) | ExprRepos, ExprEntry, BaseExpr | Boxed scalar nodes, expression/variable IDs, name index, factories, tensor-variable hierarchy, Flux import/export |
| [exprshard.rs](../../src/fresco/exprshard.rs) | ExprGrammar, ExprParseCtx, ExprShard alias | Scalar grammar with temporary semantic staging and publication |
| [eval.rs](../../src/fresco/eval.rs) | Evaluate, EvaluateNamed, EvaluateTensor, EvaluateTensorNamed, EvalError | Recursive numerical interpretation and lazy branches |
| [diff.rs](../../src/fresco/diff.rs) | Differentiate, DifferentiateVector, Jacobian | Symbolic derivative expression construction |
| [polyexpr.rs](../../src/fresco/polyexpr.rs) | PolyExpr, Monomial, CanonicalPolynomial | Partitioned scalar operands and supported polynomial expansion/collection |
| [shape.rs](../../src/fresco/shape.rs) | TensorShape, ShapeError | Rank/dimensions, row-major indexing, slicing, broadcasting |
| [tensor.rs](../../src/fresco/tensor.rs) | TensorExpr, TensorError | Shape plus flat expression IDs, structural and algebraic tensor operations |
| [varexpr.rs](../../src/fresco/varexpr.rs) | VarAttrib, VarExpr, VarKind | Variable identity, shape, aggregate offsets, flags |
| realexpr.rs, sumexpr.rs, prodexpr.rs, powexpr.rs | RealExpr, SumExpr, ProdExpr, PowExpr | Scalar constants and arithmetic |
| logicexpr.rs, condexpr.rs, funcexpr.rs | LogicExpr, CondExpr, FuncExpr | Comparisons/logic, conditional branches, unary functions |
| [termtree.rs](../../src/fresco/termtree.rs) | Term, ITermNode, TermBinNode, TermTree macro | Compile-time tree construction and repository lowering |

## Storage and identity

ExprRepos owns separate Stash collections for expression entries and variable
attributes, plus a sorted name-to-variable index. Expression entries hold
Box<dyn BaseExpr>; scalar composite operands are u32 IDs in project buffers.
This is not a flat allocation-free arena: nodes and child buffers allocate.

Expression IDs and variable IDs are different namespaces:

| Operation | Returned ID / value |
|---|---|
| VarCreate(name, reuse) | Expression ID; VarExpr::VarIndex gives its variable ID |
| ScalarVarCreate(name) | Result containing variable ID |
| VectorVarCreate / MatrixVarCreate / TensorVarCreate | Result containing aggregate variable ID |
| FindVar / FindVarExact | Variable ID, or u32::MAX |
| SubVar / SubVarLinear | Result containing a lower-rank or leaf variable ID |
| SubVarExpr / SubVarLinearExpr | Result containing scalar expression ID |
| Differentiate(repos, expressionId, variableId) | Result containing derivative expression ID |

Name search uses O(log V) comparisons. Insertion into the sorted index shifts O(V)
entries. Explicit raw scalar creation can preserve distinct same-name variables;
aggregate creation checks parent/generated-name collisions. Exact lookup precedes
bracket-alias normalization. IDs are meaningful only in their owning repository
and valid lifetime.

Repository Checkpoint/Rollback exists, but currently truncates a sorted index
without undoing shifted entries. It is not a general safe transaction contract.
The review documents a reproduced lookup failure after rollback.

## Parsing and construction

ExprGrammar implements Shard's grammar interface with a mutable ExprParseCtx.
ExprShard is an alias for ExprGrammar. The grammar stages nodes; the convenience
ExprRepos::ParseExpr checks full input before publication.

Current syntax includes arithmetic, parentheses/braces, comparisons (including
single =), and/or/not/!, if/else with optional then, and unary function calls.
Identifiers support ASCII letters/underscore followed by letters, digits, and
selected punctuation such as dot, @, #, and $. Tensor indexing is available through
repository APIs; bracketed tensor names in FindVar are not proof of bracket syntax
support in the scalar parser.

Compatibility details:

- Powers associate left-to-right in the text parser.
- Unary minus binds before power.
- Incomplete exponent suffixes such as 1e are accepted by legacy numeric recovery.
- NodeTree/TermTree macro grouping is not the same as text-parser precedence;
  explicitly group mixed operations.

Parser accepts mutable context, but its generic transaction restores input marks,
not arbitrary semantic state. An outer grammar failure can leave a staged Fresco
root. RootToken takes mutable context and may publish; it is not a pure query.
Publication is recursive and lacks a complete preparation/budget boundary.
These behaviors are outstanding work, not atomic-acceptance guarantees.

## Tensor shapes and expressions

TensorShape supports rank zero through six. Scalar() returns a scalar shape.
Vector, Matrix, and FromArr return Result and check zero dimensions/count overflow.
LinearIndex/MultiIndex use row-major layout; broadcasting aligns trailing axes and
accepts matching dimensions or singleton dimensions.

Tensor variables materialize all leading-prefix levels and scalar leaves.
For a tensor with shape (2,3,4), T[1] is a (3,4) slice, T[1,2] is a (4,) row,
and T[1,2,3] is scalar. Lower-rank attributes record a root aggregate ID and flat
offset. Recent tests cover multilevel indexing, collisions, shape rejection,
Jacobian prevalidation, and numerical broadcasting.

TensorExpr owns a shape and Buff<u32> of scalar expression IDs. Operations include
SubTensor/Row/Col, Transpose, Reshape, Flatten, elementwise broadcasting, scaling,
VectorDot, MatVecMul, MatMul, and OuterProduct. Construction/operations allocate
buffers and append scalar nodes; indexed tensor evaluation evaluates each root.

Illustrative vector evaluation:

```rust
let mut repos = ExprRepos::New();
let vId = repos.VectorVarCreate("v", 2).unwrap();
let v = TensorExpr::FromVar(&mut repos, vId).unwrap();
let bindings = [("v[0]", 3.0), ("v[1]", 4.0)];
let values = repos.EvaluateTensorNamed(&v, Arr::from(&bindings)).unwrap();
// values[0] == 3.0; values[1] == 4.0
```

Public shape mutation and raw field import can bypass checked constructors.
Hierarchy/output counts and element references need validation at every boundary;
constructor success alone does not establish validity after mutation or loading.

## Numerical evaluation and assumptions

Evaluate binds scalar values by variable ID through Arr<f64>. Named variants
allocate a repository-sized buffer, resolve names, and initialize unspecified slots
with NaN. Tensor variants return a flat Buff<f64> in shape order.

Conditionals select one branch; AND/OR short-circuit. Results of logical operations
are numeric 0 or 1. Evaluation is recursive with a depth limit; it has no general
shared-DAG work budget or memoization.

| Function group | Evaluator | Symbolic differentiation |
|---|---|---|
| sin, cos, exp, ln, sqrt | Supported | Supported, with domain/edge-case limitations |
| log (natural log alias), log10, tan, sinh, cosh, tanh | Supported | Not implemented |
| abs, floor, ceil, round | Supported | Not implemented |
| Logic / conditional nodes | Supported with lazy branches | Not implemented |

Selected errors cover invalid IDs, missing variable slots, division by zero,
negative-base fractional powers, invalid sqrt/log inputs, unsupported nodes/functions,
and depth limits. This is not universal finite-value validation: constants and
arithmetic can yield NaN/infinity, while NaN variable bindings are rejected.

VarKind::NonZero is bit index 4 in existing variable flags. HasBits queries it and
SetVarKind updates it by variable ID. Evaluation rejects zero on a visited NonZero
variable with AssumptionViolated; a NaN binding currently produces DomainError.
Unset means unknown. Flags are not currently used to guard normalization, frozen
after transformations, or inherited automatically across aggregate components.

Term-tree parity lowering changes grouping/domains in some cases. Nonzero
assumptions do not justify arbitrary IEEE floating-point regrouping.

## Symbolic calculus and polynomial transformations

Differentiate handles scalar arithmetic and the listed unary functions, appending
new expression nodes. DifferentiateVector computes one partial derivative per
scalar component; Jacobian returns residual-row / variable-column expression IDs.
Vector/Jacobian helpers include prevalidation/rollback paths, but the underlying
rollback and scalar derivative domain issues remain important.

Current edge cases include an avoidable singularity for the derivative of x^0
at zero and evaluation of singular factors on inactive derivative paths.
Symbolic differentiation here is not forward/reverse automatic differentiation.

Monomial stores a floating coefficient and Buff<(u32, i32)> powers. The fields are
private. CanonicalPolynomial owns terms and supports expansion/collection and
conversion back to expressions for constants, variables, sums, pure products,
and nonnegative integral constant powers. General functions and divisions are
rejected; it does not convert arbitrary mathematical expressions.

Current normalization prunes coefficients below 1e-12 and sorts by total degree
without a complete tie-break. Structural equality is not an order-independent
canonical-equivalence test, nor a proof of exact identity with f64 coefficients.
The review records reproduced value loss and ordering differences.

## Persistence and integrations

Flux hooks support tagged heterogeneous expression/repository import and export.
Repository load currently rebuilds the name index without full graph/shape
validation or atomic destination replacement. Schema versions and external roots
need an explicit format. Successful decoding does not establish a valid model.

No Fresco least-squares solver, QR implementation, compiled expression program,
or AD backend is present in this module. Source inspection found no Fresco-based
differentiation/minimization integration in fenst::sketch_constraint or the inspected
Python/Kaa bindings. Those are future adapters, not current API guarantees.

## Verification

On 2026-11-18, `cargo test -p trellis --lib fresco --offline` passed 27 tests,
with zero failures and 397 filtered out. This focused result does not certify all
boundary, numerical, import, or lifecycle behavior. See the
[review verification matrix](../plans/fresco-review-and-next-steps.md#verification-and-limits)
for additional reproduced failures and the ordered improvement plan.
