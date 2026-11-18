# Fresco Least-Squares Minimization and Automatic Differentiation Plan

Date: 2026-11-17  
Status: Draft for review; implementation has not started  
Scope: Minimize the magnitude of an array of `n` real-valued Fresco equations in `m` active variables  
Recommended first release: Phases 1-3, using verified symbolic derivatives  
Recommended second release: Phases 4-5, adding compiled evaluation and automatic differentiation

## 1. Objective and proposed approach

Provide a reusable Rust numerical solver that accepts Fresco expressions, an initial parameter vector, and explicit variable bindings, then minimizes the combined equation residuals. Support square, overdetermined, and underdetermined systems without requiring an exact root to exist.

Separate three responsibilities:

1. **Problem representation:** equations, active variables, fixed parameters, scales, and diagnostics.
2. **Evaluation and differentiation:** residual values, Jacobians, and eventually Jacobian products.
3. **Optimization:** step computation, acceptance, damping, and termination.

Fresco already supplies symbolic differentiation. Use it to deliver a working least-squares solver and establish an independent reference for later automatic differentiation (AD). Back-propagation is the reverse-mode derivative calculation; it does not choose optimization steps.

The initial optimizer will be Levenberg-Marquardt (LM) with a dense QR linear solver. The compiled execution plan and AD will subsequently replace repeated expression-tree traversal without changing the mathematical problem or result semantics.

This is a local minimization capability. Results depend on the initial guess; a small gradient is a first-order stationarity observation, not proof of a local or global minimum.

## 2. Mathematical contract

### 2.1 Residuals and magnitude

For equations `a_i(x, p) = b_i(x, p)`, construct residual expressions:

\[
r_i(x,p)=a_i(x,p)-b_i(x,p),\quad i=1,\ldots,n.
\]

Here `x` contains the `m` active optimization variables and `p` contains fixed parameters. The solver accepts residual expression IDs; existing Fresco subtraction constructs left-minus-right expressions. Do not interpret a logical equality expression as a numerical residual. High-level vector/matrix equations and aggregate vector variables $\vec{x}$ are represented via `TensorExpr` and automatically unfurl into constituent scalar sub-variables ($x[0], \dots, x[m-1]$) and residual roots per the [Fresco Tensor & Vector Variable Architecture Plan](fresco-tensor-and-vector-variable-plan.md).

Assume that magnitude means the Euclidean norm. Define positive residual scales `s_i` and positive weights `w_i`:

\[
q_i(x)=\frac{\sqrt{w_i}}{s_i}r_i(x,p),\qquad
\Phi(x)=\frac12\|q(x)\|_2^2.
\]

Defaults are `s_i = 1` and `w_i = 1`. Minimizing this objective is equivalent to minimizing the corresponding weighted Euclidean norm. Scaling changes the relative importance of equations; it must be explicit and retained in diagnostics. Reject zero or negative weights in the initial release; callers can omit equations they wish to disable.

Other norms, robust losses, and exact equality/inequality constraints require additional algorithms and are deferred. All residuals are soft least-squares terms in the initial release.

### 2.2 Variable scaling and derivatives

Use fixed positive parameter scales `t_j` and dimensionless coordinates:

\[
x=x_{\mathrm{ref}}+Tz,\quad T=\operatorname{diag}(t_j),\quad
S=\operatorname{diag}(\sqrt{w_i}/s_i).
\]

`x_ref` is the initial guess, `z` initially equals zero, and the default `t_j` is one. For the raw Jacobian `J`, the solver works with:

\[
J_{ij}=\frac{\partial r_i}{\partial x_j},\quad
A=SJT,\quad q=Sr,\quad
\nabla_z\Phi=A^\top q.
\]

The raw gradient is `J^T S^T S r`. Scales and weights are fixed during each solve. They must be applied identically by every derivative backend.

### 2.3 Shape and solution semantics

| System | Expected behavior |
| --- | --- |
| `n = m` | Attempt residual minimization; an exact root may or may not exist. |
| `n > m` | Find a local least-squares fit, potentially with nonzero residual. |
| `n < m` | Allow a solution dependent on initialization and scaling; do not promise uniqueness or a minimum-norm solution. |
| Redundant equations or unused active variables | Allow rank deficiency, retain diagnostics, and use damping for numerical steps. |
| No active variables | Evaluate once and return either residual satisfaction or a `NoActiveVariables` result. |
| No residuals | Reject as an invalid problem in the initial API. |

LM damping controls a numerical step. It does not add a persistent preference for one final solution. Explicit regularization, when required, is a separate problem-modeling feature.

## 3. Verified repository baseline

The following observations come from source inspection. Existing documentation sometimes describes broader integration than the code currently provides.

| Component | Current capability | Planning consequence |
| --- | --- | --- |
| [Expression repository](../../src/fresco/exprrepos.rs) | Arena-style `u32` expression IDs, separate variable indices, checked constructors, checkpoints, and rollback. Entries currently contain boxed expression objects. | Preserve existing representation; compiled evaluation can use compact operation arrays without rewriting the AST. |
| [Numerical evaluator](../../src/fresco/eval.rs) | Recursive, depth-limited evaluation, indexed/named bindings, lazy conditionals, short-circuit logic, domain checks, and `NonZero` checks. | Use as the reference evaluator. Named evaluation allocates a binding buffer; prepare indexed bindings for solver calls. |
| [Symbolic differentiation](../../src/fresco/diff.rs) | Arithmetic, powers, and `sin`, `cos`, `exp`, `ln`, `sqrt`; derivative nodes are appended to the repository. | Build Jacobians once, then reuse them. Audit correctness and preparation growth before using as a solver backend. |
| Function coverage | Evaluation also accepts `log`, `log10`, `tan`, hyperbolic functions, `abs`, and rounding functions. Differentiation does not currently cover all of these. | Publish an explicit solver capability table and reject unsupported expressions during preparation. |
| [Polynomial normalization](../../src/fresco/polyexpr.rs) | Canonical polynomial construction and algebra. | Useful for selected checks; do not require polynomial expansion for general minimization. |
| [Sketch constraint solver](../../src/fenst/sketch_constraint.rs) | Specialized geometric relaxation with handwritten corrections; no Fresco import in the implementation. | Treat later integration as a separate adapter and migration task. |
| [Python module](../../src/python/mod.rs) | Existing application bindings; no Fresco minimization binding registered. | Python support is an optional consumer after the Rust contract is stable. |
| Linear algebra | No reusable dense QR, Cholesky, SVD, or least-squares solver found by source search. | Include a bounded dense QR implementation in the MVP scope; confirm module ownership during implementation. |

Existing Fresco tests cover representative evaluation, assumptions, rollback, symbolic derivatives, and polynomial round trips. They are a starting point, not evidence that numerical minimization already works.

## 4. Architecture and public surface

### 4.1 Proposed components

Names and signatures below are design proposals, not existing APIs. Keep the initial implementation concrete; extract small purpose-specific interfaces only when a second backend requires them.

| Component | Responsibility |
| --- | --- |
| `LeastSquaresProblem` | Validated residual roots, active-variable mapping, fixed bindings, weights, and scales. |
| `PreparedLeastSquares` | Prepared symbolic Jacobian or compiled program plus the immutable problem configuration. |
| `LeastSquaresWorkspace` | Reusable bindings, residuals, Jacobian, QR storage, trial vectors, and counters. |
| `LeastSquaresOptions` | Tolerances, initial damping, iteration/evaluation budgets, and diagnostics settings. |
| `LeastSquaresResult` | Best accepted parameters, residuals, cost, termination reason, counts, and optional rank/gradient diagnostics. |
| `ExprProgram` | Owned immutable compiled operations, operand lists, root mappings, and copied variable metadata. |
| `DerivativeBackend` | Backend selection once symbolic and AD implementations both exist. |

The core operations will cover preparation, residual evaluation, Jacobian evaluation into caller-owned storage, solving from an initial guess, and later `Jv` / `J^T u` evaluation. Row-major Jacobian storage is indexed as `i * m + j` with checked size arithmetic.

Use Trellis `Arr`, `MutArr`, `Buff`, `Stash`, and `USeg` at core boundaries. Use private fields on operational structures, passive configuration/result records where appropriate, `u32` indices, and `u32::MAX` sentinels. Follow [engineering directives](../../agents/AGENTS.md) and [formatting rules](../../agents/FORMATTING.md).

### 4.2 Ownership and lifecycle

For the symbolic backend, preparation needs exclusive access while adding derivatives. On success, return a prepared handle holding an immutable borrow of the repository; this prevents mutation or rollback while cached expression IDs are in use. On preparation failure, restore the checkpoint before returning an error.

Successful preparation retains derivative nodes in the repository. Reuse the prepared handle across solves with the same residual structure and active-variable mapping. For temporary preparation, the caller may take a checkpoint before preparation and roll back only after the handle is dropped. Do not promise automatic cleanup through a shared borrow. Record derivative-node growth and impose preparation limits.

For the compiled backend, use an owned snapshot of operations and relevant variable metadata. Existing compiled programs remain valid after repository changes or rollback, and retain their original semantics. Recompile explicitly to observe expression or assumption changes. Compilation validates every reachable reference; it must not infer validity from increasing expression IDs.

A workspace is exclusive to one evaluation/solve at a time. An immutable program may be shared through ordinary borrowing, with separate workspaces for concurrent execution where existing type constraints permit it. Do not add reference-counted expression graphs.

### 4.3 Error and termination model

Preparation errors include invalid/duplicate active variable IDs, invalid roots, incomplete bindings, invalid scales/options, unsupported nodes/functions, graph cycles, size overflow, and configured resource limits.

Evaluation errors identify the residual index and originating expression/operation where possible. Separate undefined inputs, invalid arithmetic domains, nonfinite values, and derivative singularities. Finite inputs and residual values alone do not establish that a derivative exists.

Proposed termination reasons:

| Reason | Meaning |
| --- | --- |
| `ResidualTolerance` | Scaled maximum residual meets the configured target. |
| `GradientTolerance` | Scaled gradient is small; residuals can remain nonzero and minimum status is unproven. |
| `StepTolerance` | An accepted step is too small for further useful progress. |
| `CostTolerance` | An accepted step gives negligible cost reduction. |
| `IterationLimit` / `EvaluationLimit` | Budget exhausted; retain the best accepted finite iterate. |
| `Stalled` | Repeated rejected steps or excessive damping without useful progress. |
| `NoActiveVariables` | Nothing can be adjusted and the residual target is not met. |
| `NumericalFailure` / `EvaluationFailure` | The current accepted point cannot support further valid computation. |

Expose `ResidualSatisfied` separately from termination. Report an unavailable gradient or rank estimate explicitly, for example through a diagnostic enum; do not substitute zero. Never label a small damped step after repeated rejection as successful convergence.

## 5. Delivery phases and dependencies

| Phase | Deliverable | Depends on | Release gate |
| --- | --- | --- | --- |
| 1 | Problem contract and batch residual evaluation | Existing Fresco | Correct bindings, scaling, and errors |
| 2 | Verified reusable symbolic Jacobian | Phase 1 | Analytical and numerical derivative agreement |
| 3 | Dense LM least-squares solver | Phases 1-2 and QR | First usable minimizer |
| 4 | Compiled multi-root evaluation | Phase 1; benchmark against Phase 3 | Reference parity and allocation control |
| 5 | Reverse/forward AD and solver integration | Phases 2-4 | Derivative parity and end-to-end parity |
| 6 | Selected scale and consumer extensions | Phase 5 plus measured workload needs | Per-extension acceptance criteria |

Phase 3 is independently releasable. Phase 4 can be developed once the Phase 1 contract is stable, but it must not delay or alter the Phase 3 correctness baseline. Phase 6 is a set of separately scoped follow-ups, not a requirement to complete every possible optimization feature.

## 6. Phase 1: Problem contract and residual evaluation

### Work packages

1. Define the problem, options, result, and error records with documented units and dimensions.
2. Accept residual expression IDs and an ordered list of active variable indices. Resolve names once at the boundary and preserve caller order.
3. Build one full indexed binding buffer. Map active-vector positions into repository variable slots; all other reachable variables require fixed finite values. Reject overlapping fixed and active specifications rather than silently overriding them.
4. Validate finite constants/inputs, positive finite scales/weights, distinct active variables, root validity, size bounds, and option ranges. For initially unsupported node kinds, fail preparation with a useful diagnostic.
5. Evaluate raw and scaled residuals and objective into reusable buffers. Use overflow-aware norm accumulation and reject an unrepresentable objective; do not convert arithmetic overflow into an apparent optimum.
6. Freeze the supported operator policy for the first release and document unsupported functions at preparation time.

### Deliverables and acceptance

- Batch evaluation has the same raw values as individual reference evaluations at valid inputs.
- Non-contiguous variable IDs, fixed parameters, duplicate residual roots, and unused active variables behave consistently.
- Named lookup and full binding allocation occur during preparation/workspace setup, not on every iteration.
- Empty residual arrays are rejected; no-active-variable problems evaluate once as specified.
- Scaling and weights reproduce hand-computed objectives, and invalid configurations fail before optimization begins.

## 7. Phase 2: Verified symbolic Jacobian

### Work packages

1. Validate the reachable graph and supported operators before derivative construction. Add cycle/depth guards and preparation node/work limits so low-level or imported malformed expressions cannot cause unbounded recursion or allocation.
2. Build `n * m` derivative roots once, using variable indices rather than expression IDs. Cache `(expression ID, variable index)` derivative results in preparation where needed to avoid repeated construction through shared subexpressions.
3. Fix derivative-domain defects needed for supported operators. In particular, specialize literal powers zero and one before constructing `u^(p-1)`, and preserve valid derivatives at zero for positive integer powers.
4. Preserve original residual evaluation before derivative evaluation: algebraic simplification of a derivative must not make an undefined original residual admissible. Avoid broad rewrites that change domains or floating-point behavior.
5. Wrap derivative construction transactionally. Return structured context for unsupported nodes/functions and roll back partial work on failure.
6. Evaluate the dense raw Jacobian, then apply row/column scaling consistently to produce `A`.

### Initial operator policy

| Operator | First-release policy |
| --- | --- |
| Constants, variables, sums, differences | Supported with finite-value checks. |
| Products | Supported, including zero factors; derivatives must not depend on dividing by each factor. |
| Quotients | Supported only where the evaluated denominator is nonzero and arithmetic remains finite. |
| Literal integer powers | Support negative bases and zero where the expression and derivative are defined. Specialize exponents zero/one and test repeated roots. |
| Literal noninteger powers | Initially require positive base for derivative evaluation. |
| Nonliteral exponent | Initially require positive base; supporting a fixed integer-valued exponent at a negative base needs explicit specialization. |
| `sin`, `cos`, `exp` | Supported with finite intermediate/output checks. |
| `ln` | Supported on positive arguments. |
| `sqrt` | Residual valid at zero, but derivative evaluation initially requires positive argument. Check residual satisfaction before requesting derivatives. |
| `log` alias, `log10`, `tan`, hyperbolic functions | Reject initially; add smooth coverage in later reviewed work. `log` may be normalized to `ln` as a small explicit extension. |
| `abs`, `floor`, `ceil`, `round`, logic, conditionals | Reject initially; these need a separate nonsmooth/control-flow policy. |

The literal-zero-power case should preserve the existing evaluator's value semantics, including its treatment of `0^0`, while removing the spurious derivative singularity. Record that convention explicitly in tests; do not silently redefine all power semantics during solver work.

### Deliverables and acceptance

- Compare symbolic derivatives with hand-derived values and finite differences at multiple step sizes away from singularities.
- Include cross-variable derivatives, shared nodes, repeated operands, negative bases, zero factors, division, powers, and singularity rejection.
- Establish a precomputed Jacobian memory/node budget and verify repeated evaluations do not add expression nodes.
- Verify rollback restores repository counts after a failed preparation.
- Do not rely solely on the existing polynomial and sine derivative examples.

## 8. Phase 3: Dense Levenberg-Marquardt solver

### 8.1 Linear algebra work package

Implement a focused dense Householder QR solve for the augmented system:

\[
\underset{\Delta z}{\operatorname{minimize}}\;
\frac12\left\|
\begin{bmatrix}A\\\sqrt{\lambda}I\end{bmatrix}\Delta z-
\begin{bmatrix}-q\\0\end{bmatrix}
\right\|_2^2,\qquad \lambda>0.
\]

Use reusable contiguous storage, checked dimensions, stable reflector norms, and explicit factorization failure handling. Do not form a matrix inverse. Validate linear solves independently before integrating them with the nonlinear solver. QR is the proposed baseline because forming normal equations squares the Jacobian's condition number; see the [Ceres linear solver discussion](https://ceres-solver.readthedocs.io/latest/nnls_solving.html#linear-solver).

A positive damping term makes the augmented problem full column rank in exact arithmetic, including when `n < m`. It does not establish the rank of the original Jacobian. Report original rank only if separately estimated from an undamped rank-revealing factorization; otherwise report it as unavailable. Pivoting/rank diagnosis is a separate enhancement if the initial QR implementation does not include it.

### 8.2 Iteration work package

1. Validate the initial point and compute `r`, `q`, and cost. Check residual satisfaction before derivative evaluation.
2. Evaluate `A` at the current accepted point and compute the scaled gradient.
3. Solve the augmented subproblem and form the trial point `x + T * deltaZ`.
4. Evaluate trial residuals and the actual cost decrease.
5. Compare actual reduction with the undamped local model prediction:

\[
\operatorname{pred}=\frac12\bigl(\|q\|^2-\|q+A\Delta z\|^2\bigr),\qquad
\rho=\frac{\Phi(x)-\Phi(x+T\Delta z)}{\operatorname{pred}}.
\]

6. Accept only finite, improving trials with adequate positive prediction and acceptance ratio. Update damping using a documented bounded policy. On rejection, increase damping and retry using the same current Jacobian.
7. Refresh derivatives after acceptance when another iteration needs them. Preserve the best accepted point and diagnostics on every exit.

Use one deterministic damping/acceptance policy with documented initial values and thresholds. Record the selected rule and numerical defaults in the implementation review. The structure follows standard [LM/trust-region least-squares methods](https://ceres-solver.readthedocs.io/latest/nnls_solving.html#levenberg-marquardt).

### 8.3 Termination and failure handling

- Residual target: `max(abs(q_i)) <= residualTolerance`.
- Stationarity target: `max(abs((A^T q)_j)) <= gradientTolerance`; retain nonzero residual information.
- Relative accepted-step target: `norm(deltaZ) <= stepTolerance * (1 + norm(z))`.
- Relative accepted-cost target: actual reduction is at most `costTolerance * (1 + previousCost)`.
- Bound accepted iterations, total trial attempts, residual/Jacobian evaluations, and damping retries. Rejected trials count toward work budgets.
- Invalid initial residuals are errors. A domain-invalid trial is rejected with increased damping; persistent invalid trials become `Stalled`, with diagnostic context.
- A valid accepted point with an undefined required derivative returns a derivative/evaluation failure rather than a fabricated gradient.
- Nonfinite matrix arithmetic, nonpositive prediction, and damping overflow have explicit bounded exits.

### Deliverables and acceptance

- Solve exact linear, inconsistent linear, smooth nonlinear, redundant, and underdetermined fixtures.
- Preserve fixed parameters and variable ordering.
- Demonstrate accepted-step cost reduction and exercise rejected-step recovery.
- Report stationarity separately from equation satisfaction and handle intentionally stationary starts honestly.
- Return the best finite accepted point at budget exhaustion or recoverable failure.
- No expression construction or routine buffer growth occurs in the iteration loop. Remaining reference-evaluator allocations, if any, are measured and addressed by Phase 4.

## 9. Phase 4: Compiled multi-root evaluation

### Work packages

1. Compile all residual roots and, for the symbolic backend, derivative roots into validated topological operation arrays. Preserve root order and repeated roots.
2. Deduplicate evaluation of shared expression IDs. Structural equivalence detection across different IDs is deferred.
3. Store operation kinds, operand indices, constant values, variable slots, and diagnostic source IDs in compact buffers. Lower variable-length sums/products with a documented evaluation order.
4. Copy relevant variable assumptions and binding metadata into an owned program snapshot. Reject invalid references, cycles, unsupported nodes, and capacity overflow during compilation.
5. Evaluate into preallocated value buffers with no recursive tree traversal and no successful-path allocation after workspace setup.
6. Preserve the operator-domain policy from Phase 2 and finite-value diagnostics. Match reference arithmetic order where practical and document tolerance-based differences.
7. Compare tree and compiled backends on the same solver fixtures before selecting compiled evaluation by default.

### Deliverables and acceptance

- Raw residual and symbolic Jacobian values agree with the reference backend within justified tolerances.
- Shared-node fixtures demonstrate one value computation per reachable operation.
- Program snapshots remain usable after source repository rollback; recompilation observes changed source metadata.
- Invalid graphs fail compilation without partial usable programs.
- Capture compile time, steady-state evaluation time, operation/edge counts, memory, and allocations separately.
- Keep conditionals unsupported here; an eager topological plan must not silently replace Fresco's lazy branch semantics.

## 10. Phase 5: Automatic differentiation and back-propagation

### 10.1 Reverse mode

Add local vector-Jacobian-product rules to supported compiled operations. Evaluate primal values, clear adjoints, seed output adjoints, and traverse operations in reverse, accumulating every contribution with addition.

For raw residual outputs seeded by arbitrary `u`, return `J^T u`. For the weighted objective, seed:

\[
u=S^\top q=S^\top Sr,
\]

then multiply the resulting raw gradient by `T` to obtain the scaled gradient. Alternatively, differentiate a program whose outputs are `q` and seed with `q`; document which representation each method uses.

Repeated residual roots and repeated operand edges must accumulate independently. Reset adjoints between calls; never leak a previous seed or linearization into a later evaluation.

Handle products with zero-safe local rules, such as prefix/suffix products or binary lowering. Do not implement multiplication derivatives as `output / input`. Specialize integer powers to avoid artificial logarithms and division by zero. Do not evaluate an invalid derivative expression for an input that is not active in the selected differentiation path.

### 10.2 Forward mode and full Jacobians

Implement tangent propagation to compute `Jv`. Assemble a dense Jacobian through `m` input seeds in forward mode or `n` output seeds in reverse mode, reusing workspace and cached primal values where valid. A scalar objective gradient uses one reverse sweep; it does not provide the full Jacobian needed by dense LM.

For dense Jacobians, forward mode generally favors many residuals and few variables; reverse mode generally favors few residuals and many variables. Treat dimensions as a heuristic and benchmark the actual graph, following the distinction described in the [JAX AD cookbook](https://docs.jax.dev/en/latest/notebooks/autodiff_cookbook.html). Start with explicit backend selection before adding any automatic choice.

Any cached primal/tangent/adjoint state must be tied to the current input values and fixed bindings. After an accepted step, invalidate the previous linearization.

### Deliverables and acceptance

- AD, symbolic Jacobians, and independent numerical checks agree at representative valid points.
- Verify the adjoint identity `dot(u, Jv) = dot(J^T u, v)` for several deterministic inputs/seeds.
- Test shared subexpressions, repeated output roots, repeated operands, zero-valued products, constant outputs, and inactive inputs.
- The LM solver produces equivalent residual/cost outcomes across symbolic, forward-AD, and reverse-AD backends; exact iteration counts need not match under roundoff.
- Confirm no hot-path allocation and report forward/reverse sweep counts separately from primal evaluation counts.
- Retain symbolic differentiation as a supported reference and useful small-problem backend.

## 11. Phase 6: Workload-driven extensions

Scope each extension independently after benchmark results and consumer needs are available.

| Extension | Work | Acceptance criterion |
| --- | --- | --- |
| Sparse Jacobians | Determine structural variable dependencies; store sparse rows; consider coloring. | Match dense derivatives and demonstrate memory/runtime improvement on sparse fixtures. |
| Matrix-free LM | Use `Jv` / `J^T u` with an iterative damped least-squares method and suitable stopping/preconditioning. | Match dense costs on reference cases and solve a case that exceeds the dense memory budget. |
| Explicit regularization | Add prior residuals such as `sqrt(mu) * (x_j - prior_j)`, with documented units. | Known underdetermined fixture selects the intended solution; distinguish prior cost from equation cost. |
| Bound constraints | Use a method with correct bound handling and projected optimality tests. | Bound-active solutions and projected stationarity verified; simple clipping is insufficient. |
| Additional smooth functions | Extend symbolic, compiled, forward, and reverse rules together. | Capability table and domain tests remain consistent across backends. |
| Conditional/nonsmooth expressions | Specify branch-local derivatives, switching behavior, and failure/subgradient policy. | Preserve lazy evaluation of inactive branches and test switching surfaces explicitly. |
| Robust losses | Define loss/derivative semantics and matching step models for outlier handling. | Controlled outlier fixtures match the selected robust objective. |
| Sketch adapter | Translate selected geometric constraints to residuals; map fixed coordinates; preserve document transactions. | Compare against existing geometric fixtures and define behavior for unconstrained motion and units. |
| Python API | Expose residual construction, bindings, solving, and structured diagnostics through the existing facade. | Binding-level tests reproduce Rust results and preserve error context. |
| Parallel evaluation | Use Heist facilities with per-worker workspaces and explicit reduction order. | Demonstrate benefit on measured workloads without shared mutable AD buffers. |

GPU execution, full symbolic Hessians, differentiation through the solver itself, global optimization, and automatic branch smoothing are outside the planned releases. Revisit them only with a concrete consumer requirement.

## 12. Verification fixtures and quantitative checks

Use existing `jeeves_test!` registration in component `_tests.rs` files. Keep expected solutions independent of the implementation.

| Fixture | Independent expected behavior |
| --- | --- |
| `r = [x + y - 3, x - y - 1]` | Unique zero at `(2, 1)`. |
| `r = [x - 1, x - 3]` | Stationary least-squares solution `x = 2`, raw residuals `[1, -1]`, cost `1` with unit scaling. |
| Same residuals with weights `[1, 3]` | Solution `x = 2.5`, cost `1.5`; verifies weighting rather than unweighted averaging. |
| `r = [x + y - 3]` | Residual tends to zero; do not assert a unique parameter vector. |
| `r = [x - 1, 2 * (x - 1)]` with extra active `y` | Redundant rows and unused variable; no spurious rank/uniqueness claim. |
| `r = [10 * (y - x^2), 1 - x]` from a documented start | Standard nonlinear least-squares fixture with zero at `(1, 1)`. |
| `r = [x^2 - 1]` from `x = 0` | Nonzero residual and zero gradient; report stationarity, not equation satisfaction or a certified minimum. |
| Product `x * y` at `(0, 2)` | Raw derivative `[2, 0]`; catches division-based AD rules. |
| Powers `x^0`, `x^1`, `x^2` at zero and integer powers at negative bases | No artificial derivative singularities; preserve documented value domains. |
| `sqrt(x)` at `x = 0` | Zero residual can satisfy the target before differentiation; `sqrt(x) - 1` there requires an explicit derivative-domain failure. |
| Log/quotient trial crossing a domain boundary | Reject invalid trials, keep the previous accepted point, and exit within retry limits. |
| Shared nodes and duplicate roots | Correct derivative accumulation and root multiplicity. |
| Invalid references, cyclic low-level graph, unsupported functions | Preparation fails with context; no uncontrolled traversal or partial derivative growth. |
| Tight budgets and repeated rejected steps | Bounded exit, accurate counters, and best accepted point retained. |

For well-scaled smooth derivative fixtures, begin with a mixed comparison `abs(error) <= 1e-9 + 1e-8 * abs(expected)` between analytical/symbolic/AD values. Central-difference comparisons can start at a looser `1e-6` relative scale with multiple perturbation sizes. These are fixture targets, not universal guarantees; choose domain-safe perturbations and justify any fixture-specific tolerance.

For exact solver fixtures, set explicit scaled residual targets such as `1e-8`; for inconsistent fixtures, check known cost and stationarity rather than requiring a root. Test QR independently on known solutions, rectangular augmented systems, nearly dependent columns, and large/small finite magnitudes using normwise residual checks.

Measure preparation/compilation separately from repeated evaluation and full solve time. Include small dense systems, tall and wide systems, shared graphs, and locally sparse systems. Before scaling work, select representative target sizes and memory budgets with the actual consumer; this draft makes no unmeasured throughput promises.

## 13. Proposed source changes and review units

| Path | Proposed change |
| --- | --- |
| `src/fresco/least_squares.rs` | Problem preparation, workspace, result semantics, LM implementation, and symbolic backend. Split only when responsibilities warrant it. |
| `src/fresco/dense_qr.rs` | Focused reusable internal QR primitive; consider promotion only when another consumer exists. |
| `src/fresco/diff.rs` | Required correctness fixes, preparation safeguards, and memoization support. |
| `src/fresco/program.rs` | Owned compiled expression program and evaluation workspace, Phase 4. |
| `src/fresco/autodiff.rs` | Forward/reverse local rules and derivative products, Phase 5. |
| `src/fresco/mod.rs` | Register modules and expose the intentionally supported public surface. |
| `src/fresco/_tests.rs` | Residual, derivative, QR, solver, lifecycle, and cross-backend cases. |
| `wiki/modules/fresco.md` | Document implemented capabilities and distinguish symbolic differentiation from AD and optimization. |
| `src/fenst/sketch_constraint.rs`, `src/python/`, `truss/kaa/` | Touch only in explicitly selected Phase 6 integration work. |
| `tools/trellis.natvis` | Add/update visualization for new core buffers/program structures when useful for inspection. |

Suggested review units: contracts/evaluation; symbolic correctness and preparation; QR primitive; LM and diagnostics; compiled evaluator; reverse mode; forward mode and backend integration. Each unit includes its relevant tests and documentation. Keep dependency additions out of the baseline unless a concrete linear-algebra tradeoff warrants a separate review decision.

## 14. Implementation validation and release gates

For each implementation unit, run the narrowest meaningful tests and a clean build using the repository's pinned toolchain and cached dependencies:

```powershell
cargo test --lib --offline fresco
cargo build --offline
```

For a release containing new numerical APIs, run broader checks as required by the engineering directives:

```powershell
cargo check --all-targets --offline
cargo clippy --all-targets --offline -- -D warnings
cargo test --lib --offline
cargo run --offline -- -t Fresco
```

If Python integration is selected, include feature-enabled checks and Python binding tests. Root builds also compile the Rust-GPU workspace member; report missing toolchain/cache prerequisites explicitly rather than treating an infrastructure failure as numerical verification.

The MVP release gate requires Phases 1-3 acceptance criteria, passing numerical fixtures, documented operator/termination semantics, and a measured preparation/memory profile. The AD release gate additionally requires Phase 4 parity, Phase 5 derivative/product checks, lifecycle tests, and measured allocation/sweep behavior. No phase is complete solely because its APIs compile.

This planning change itself introduces no runtime code. Numerical/build results above are future implementation requirements, not claimed test results for this draft.

## 15. Risks, decisions, and review focus

| Risk or decision | Proposed treatment | Review point |
| --- | --- | --- |
| Meaning of magnitude | Weighted Euclidean least squares; unit weights/scales by default. | Confirm that maximum absolute error or an exact constraint solver is not the primary objective. |
| First useful release | Ship symbolic derivatives plus dense LM before AD. | Confirm milestone boundary. |
| Symbolic growth | Prepare once, memoize repeated derivative requests, cap resources, and reuse handles. | Agree preparation node/memory limits for target sizes. |
| Linear algebra scope | Focused internal Householder QR; no dependency addition initially. | Review numerical implementation cost and whether an existing approved dependency is preferable. |
| False convergence | Separate residual satisfaction, stationarity, progress limits, and failures. | Confirm downstream callers consume reasons rather than one convergence flag. |
| Rank deficiency | Damped steps supported; no uniqueness or minimum-norm promise. | Decide whether explicit prior residuals or rank diagnostics are needed in the first consumer. |
| Domain differences | Publish restricted derivative domains; reject unsupported cases early. | Confirm initial operator list and power conventions. |
| Repository mutation | Borrowed symbolic preparation; owned compiled snapshots with explicit recompilation. | Confirm intended reuse and editing lifecycle. |
| Performance goals | Benchmark actual `n`, `m`, graph sizes, sharing, and sparsity. | Select representative consumer workloads before Phase 6. |
| Existing documentation claims | Source is the implementation baseline; update affected module docs when implementing. | Avoid assuming sketch/Python minimization integration already exists. |

Approve the mathematical contract, MVP scope, operator policy, and ownership model first. Numerical defaults, resource budgets, and any selected integration can be finalized in their implementation reviews without redesigning the residual/derivative/solver separation.

## 16. References

- [Fresco Tensor & Vector Variable Architecture Plan](fresco-tensor-and-vector-variable-plan.md) — architectural specification for vector/matrix/tensor variables, subvars, shapes, broadcasting, and linear algebra interactions.
- [Ceres: solving nonlinear least squares](https://ceres-solver.readthedocs.io/latest/nnls_solving.html) — reference for least-squares structure, LM/trust-region steps, and linear solver tradeoffs.
- [JAX: the autodiff cookbook](https://docs.jax.dev/en/latest/notebooks/autodiff_cookbook.html) — reference for forward/reverse differentiation, Jacobian products, and Jacobian shape considerations.
- [Fresco module documentation](../modules/fresco.md) — repository overview; consult the source baseline above for current implementation boundaries.
- [Trellis engineering directives](../../agents/AGENTS.md) — project container, ownership, API, testing, and workflow requirements.
