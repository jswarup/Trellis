# Fresco: expressions, tensors, and symbolic mathematics

**Path:** `src/nexome/fresco/` (facade export: `trellis::fresco`)

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
specifies the native Fascia math tab, layout engine, and inspection pipeline delivered
in Phases V0-V2.

## Components

| File | Types / entry points | Responsibility |
|---|---|---|
| [exprrepos.rs](../../src/nexome/fresco/exprrepos.rs) | ExprRepos, ExprEntry, BaseExpr | Boxed scalar nodes, expression/variable IDs, name index, factories, tensor-variable hierarchy, Flux import/export |
| [exprshard.rs](../../src/nexome/fresco/exprshard.rs) | ExprGrammar, ExprParseCtx, ExprShard alias | Scalar grammar with temporary semantic staging and publication |
| [eval.rs](../../src/nexome/fresco/eval.rs) | Evaluate, EvaluateNamed, EvaluateTensor, EvaluateTensorNamed, EvalError | Recursive numerical interpretation and lazy branches |
| [diff.rs](../../src/nexome/fresco/diff.rs) | Differentiate, DifferentiateVector, Jacobian | Symbolic derivative expression construction |
| [polyexpr.rs](../../src/nexome/fresco/polyexpr.rs) | PolyExpr, Monomial, CanonicalPolynomial | Partitioned scalar operands and supported polynomial expansion/collection |
| [shape.rs](../../src/nexome/fresco/shape.rs) | TensorShape, ShapeError | Rank/dimensions, row-major indexing, slicing, broadcasting |
| [tensor.rs](../../src/nexome/fresco/tensor.rs) | TensorExpr, TensorError | Shape plus flat expression IDs, structural and algebraic tensor operations |
| [varexpr.rs](../../src/nexome/fresco/varexpr.rs) | VarAttrib, VarExpr, VarKind | Variable identity, shape, aggregate offsets, flags |
| realexpr.rs, sumexpr.rs, prodexpr.rs, powexpr.rs | RealExpr, SumExpr, ProdExpr, PowExpr | Scalar constants and arithmetic |
| logicexpr.rs, condexpr.rs, funcexpr.rs | LogicExpr, CondExpr, FuncExpr | Comparisons/logic, conditional branches, unary functions |
| [termtree.rs](../../src/nexome/fresco/termtree.rs) | Term, ITermNode, TermBinNode, TermTree macro | Compile-time tree construction and repository lowering |
| [fresco_document.rs](../../src/fenst/fresco_document.rs) | FrescoDocument, FrescoRoot, FrescoRootKind, FrescoDocLimits | Document model, unique monotonic IDs, revisions, cycle/depth/node validation, built-in examples |
| [math_layout.rs](../../src/fenst/math_layout.rs) | MathLayout, MathBox, MathDrawCommand, MathOccurrence, MathFontMetrics | Backend-neutral baseline-aware math box arena, precedence grouping, fractions, scripts, grids, and hit testing |
| [math_text.rs](../../src/fascia/math_text.rs) | CosmicMathFontMetrics | cosmic-text font metrics adapter with accurate paragraph baseline offset measurement |
| [math_view.rs](../../src/fascia/math_view.rs) | FrescoViewerState, FrescoAction, FrescoCanvasProgram, view_math_tab | Native Canvas viewer, 4px threshold drag pan, cursor-anchored zoom, fit/reset, root switching |
| [math_inspector.rs](../../src/fascia/math_inspector.rs) | view_math_inspector | Inspector sidebar for active root metrics and selected term properties/assumptions |

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

## Native Math View and Layout Engine

The initial read-only [expression viewer](../plans/fresco-expression-viewer-implementation-plan.md)
opens in Fascia through **File > New Math View**. Scalar input, evaluation controls,
retained compact tensor operations, editing, export and keyboard term navigation
remain later work.

### Document and ownership

[FrescoDocument](../../src/fenst/fresco_document.rs) owns one ExprRepos and named
scalar or tensor roots. New documents receive a process-unique ID and revision 1;
roots are indexed within that immutable document. Explicit WithIdentity callers
supply their own identity/revision. There is no separate generated RootId.

New validates reachable expression/variable IDs, cycles, depth/work limits, power
and logical arity, and tensor dimensions/element counts. Defaults are depth 128,
10,000 visited expression occurrences and 256 roots. Names are limited to 1,024
UTF-8 bytes each, with a 65,536-byte budget across root labels and reachable
variable/function names (including repeated occurrences). Validation checks these
limits before comparing or copying labels. It uses an explicit work stack and
checks wide-node work before enqueueing its children.

[FrescoViewerState](../../src/fascia/math_view.rs) uniquely owns its document and
the complete optional layout. AppState keeps viewers in a private sorted project
container keyed by TabId; OpenFrescoDocument moves a prepared document into a tab.
Individual and all-tab closing dispose the corresponding viewers. No evaluation
jobs run in this release.

### Layout and semantic fidelity

[MathLayout](../../src/fenst/math_layout.rs) has flat project arenas for boxes,
child indices, drawing commands and semantic occurrences. Construction, measurement
and placement currently recurse with a bounded expression depth (default 64).
Box and occurrence limits are 4,096 and 2,048. Displayed identifiers are limited to
128 UTF-8 bytes; all generated text, including operators and omission labels, shares
a 65,536-byte layout budget checked before shaping. Failures become visible
diagnostics, clear the previous layout and disable stale hit testing.

Supported constructs include identifiers, constants, sums/products, fractions,
superscripts, functions, conditionals, comparisons, logical operators and matrix
grids. Parentheses preserve nested grouping and negative power bases. Constants
use round-trip formatting, with scientific notation for small/large values.
Identifiers remain literal names; bracketed text in a name is not tensor metadata.
Variable styling uses actual shape metadata.

Vector and matrix grids initially show up to 16 rows and 16 columns, with explicit
omitted row/column counts. Their measured bounds include padding, cells and omission
labels. Higher-rank roots report that slice navigation is required. General
subscript typesetting, radicals and slice navigation are not yet exposed.

Distinct MathOccurrence records identify each displayed occurrence and tensor cell.
Rank-aware MathCoordinates records keep vector coordinates such as [i] distinct
from matrix coordinates [i,j], without allocating per-occurrence vectors. Hit testing
chooses the smallest containing occurrence. No hit uses u32::MAX.

### Native canvas and inspector

[The text adapter](../../src/fascia/math_text.rs) uses Iced's shared paragraph/font
database through its advanced API. Measurement and painting use the same font,
bold/italic style, advanced shaping, alignment and line-height settings. Layout
stores paragraph-baseline offsets and glyph ink rectangles separately from advance
widths. Raster bounds and conservative outline bounds use the same shared font IDs
as the canvas. Placed bounds include negative side bearings, vertical overhangs and
all child extents; fractional exponents contribute both ascent and descent.
Text and geometry scale together under zoom.

The canvas supports click selection, a four-logical-pixel pan threshold, anchored
wheel zoom, root switching, Fit and Reset. A size sensor supplies the actual canvas
dimensions. Fit follows resizing until manual pan/zoom or Reset changes the view.
Manual zoom and Fit share a 0.000001x minimum; manual zoom is capped at 10x, Fit at
2.5x. Zoom Out cannot jump upward from a fitted scale. Wheel events with zero or
non-finite vertical delta are ignored, including horizontal-only scrolling.
Semantic color roles resolve against the current palette during painting.

[The inspector](../../src/fascia/math_inspector.rs) receives the active layout and
selection. It shows root kind/shape/document identity, selected node identity/kind,
literal or variable details, NonZero and tensor cell coordinates where available.
It does not currently report root node/depth statistics.

## Verification

On 2026-11-20, the affected suites passed 49 Fenst, 60 Fascia and 29 Fresco tests
(138 total), together with offline all-target checks and strict all-target Clippy.
Regressions cover script/ink bounds, fitted zoom, rank-aware coordinates, text
budgets before shaping, and horizontal wheel events. Desktop visual smoke testing
remains outstanding.

On 2026-11-18, `cargo test -p trellis --lib fresco --offline` passed 27 tests,
with zero failures and 397 filtered out. This focused result does not certify all
boundary, numerical, import, or lifecycle behavior. See the
[review verification matrix](../plans/fresco-review-and-next-steps.md#verification-and-limits)
for additional reproduced failures and the ordered improvement plan.
