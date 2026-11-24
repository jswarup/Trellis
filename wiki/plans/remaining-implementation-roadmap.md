# Trellis Remaining Implementation Roadmap

Status reviewed against the source on 2026-11-24.

This roadmap contains work that remains unimplemented or lacks its required end-to-end acceptance evidence. Completed implementation history belongs in Git history and the module documentation. Each work package below has a concrete exit gate so it can be removed when delivered.

## Priority and dependencies

| Order | Work package | Depends on |
| --- | --- | --- |
| 1 | Explicit memory and compute contracts | Current Silo and Swarm APIs |
| 2 | Complete Flux file boundaries | Current bounded owned-value codec |
| 3 | Bounded Fresco preparation and execution | Current validated expression repository |
| 4 | Fresco least-squares MVP | Bounded preparation plus the current symbolic derivative baseline |
| 5 | Headless build profiles and WGPU backend integration | Explicit compute contracts |
| 6 | Workbench persistence and remote host boundary | Current desktop and Python golden workflows |
| 7 | Compiled evaluation, automatic differentiation, and Python tensor APIs | Measured Fresco and solver workloads |
| 8 | PDE and neural-network reference workloads | Headless profiles and reusable compute contracts |

## Explicit memory and compute contracts

### Remove Trellis direct use of bytemuck

Trellis still declares and directly uses `bytemuck` for Silo casts, GPU records, tests, and examples. The package will remain transitively present through WGPU, Iced, Rust-GPU, and text dependencies.

Implement an explicit representation vocabulary:

- Replace generic `CastArrFrom<U>` and `CastMutArr<U>` with the supported `u8`, `u32`, and `f32` conversions.
- Keep alignment, complete-element, length-overflow, ownership, and mutability checks at the conversion boundary.
- Replace `Pod` and `Zeroable` derives on `GeometryVertex`, `LabelVertex`, and `ViewUniforms` with `#[repr(C)]` plus compile-time size, alignment, and field-offset assertions.
- Replace the generic viewport upload function with named upload functions for geometry vertices, label vertices, index buffers, uniforms, and regions.
- Migrate tests and performance examples, then remove the direct dependency from `Cargo.toml`.

Exit gate:

- `rg -n "bytemuck" Cargo.toml src examples` returns no Trellis-owned use.
- GPU record layout assertions cover every raw upload record.
- Silo tests cover empty, misaligned, partial, immutable, and mutable conversions.
- CPU tests, all-target checks, strict Clippy, and available GPU parity tests pass.
- `cargo tree -i bytemuck` shows only transitive dependency paths.

### Finish the Arr raw-construction migration

`Arr::FromRaw` is the public unsafe boundary, while the crate-private `Arr::New` preserves existing internal callers. Move ordinary callers to borrow-based construction and use `FromRaw` only where an owning structure proves the pointer lifetime and initialized extent.

Exit gate:

- Internal raw construction remains confined to audited storage, FFI, and hardware boundaries.
- New safe code constructs views through `From<&[T]>`, `From<&[T; N]>`, or owner methods.
- A source check and review list identify every remaining `FromRaw` call and its owner.

### Complete the backend-neutral Swarm contract

`StandardOpDescriptor` now records binding names, element representations, access, item widths, fixed extents, alias rules, workgroup width, active range, and synchronization. CPU dispatch consumes part of this contract. `WgpuComputeEngine` remains a separate API and does not implement `IComputeBackend`.

Implement:

- Move count, dimension, element-layout, alias, and active-range validation into backend-neutral logic usable by every backend resource type.
- Implement the WGPU compute path through `IComputeBackend` with explicit upload, readback, capability, and synchronization behavior.
- Keep renderer resources and render passes outside the compute backend contract.
- Generate a review record from the same operation contract showing inputs, outputs, touched ranges, and completion boundary.

Exit gate:

- The same operation descriptor validates CPU and WGPU dispatch.
- CPU and WGPU results match for supported operations at declared precision.
- Unsupported operations and capabilities return structured errors.
- GPU compute builds without requiring the desktop UI profile.

## Complete Flux file boundaries

The version 1 `.flx` and `.tflx` codecs, owned `FluxValue` staging, golden vectors,
bounded parsing, CRC validation, safe sibling-temporary replacement, and Fleck sketch
integration are implemented. The general descriptor and operational facades remain
incomplete.

Implement:

- Replace hard-coded codec bounds with an explicit `FluxReadLimits` contract and
  return structured `FluxFileError` values with offsets and underlying I/O causes.
- Add fallible streaming export and import adapters over `FieldExp` and `FieldImp`,
  including exact container completion and required-field validation hooks.
- Add inspect, verify, binary/text conversion, and format-detection entry points
  without coupling them to the sketch schema.
- Complete short-write, flush, seek, sync, replacement, unsupported-version,
  unknown-feature, malformed-varint, and cardinality failure coverage.
- Reconcile documented limits and diagnostics with the executable defaults.

Exit gate:

- Existing descriptors can round-trip supported values without first constructing a
  complete owned value tree when streaming is possible.
- Every limit and I/O failure is fallible and leaves the destination model and prior
  file unchanged.
- Inspect, verify, and conversion operate on the paired golden files and report
  metadata without application-specific code.
- Flux, Fleck persistence, Fascia open/save, and Shard regression suites pass.

## Bounded Fresco preparation and execution

### Add work budgets and cancellation

The reference evaluator has recursion-depth protection and validated expression structures. It still evaluates recursively without a total node or edge budget, deadline, or cancellation boundary.

Implement:

- Add explicit node, edge, result, and repeated-shared-node work limits.
- Preserve lazy conditional and logical evaluation.
- Add cooperative cancellation and deadline checks at stable traversal checkpoints.
- Return structured budget and cancellation outcomes.
- Keep failed preparation and evaluation atomic with respect to the repository and active document.

Exit gate:

- Deep, wide, and highly shared fixtures exercise independent depth and work limits.
- Inactive invalid branches remain unevaluated.
- Cancellation leaves the last accepted document and numerical state usable.
- Repeated evaluation records work counters without growing the expression repository.

### Measure before changing expression storage

Measure parsing, publication, import, differentiation, polynomial expansion, indexed evaluation, named evaluation, tensor evaluation, and complete solver workloads separately. Record nodes, edges, sharing, live variables, allocations, peak memory, preparation time, and execution time.

Use the measurements to decide whether to introduce reusable binding buffers, derivative memoization, shared-subgraph caching, batched symbol indexing, a shared child arena, or compiled evaluation. Preserve stable expression IDs, child order, polynomial partitions, lazy semantics, Flux round trips, and rollback behavior.

Exit gate:

- Representative small, shared, deep, tensor, and solver workloads have reproducible baselines.
- Each storage or caching change states the measured problem it addresses.
- Performance changes retain the current correctness and lifecycle suites.

### Complete optional Math View capabilities

The native read-only Math View, tensor component selection, layout, zoom, text metrics, properties projection, and tab lifecycle are implemented. The following capabilities remain:

- Retain high-level tensor operation intent for compact displays of matrix multiplication, dot products, Hadamard operations, and broadcasting.
- Add isolated scalar text replacement with full-input acceptance and bounded candidate publication.
- Add numeric binding and evaluation controls after evaluator budgets and cancellation are available.
- Add optional display-text and SVG export from the validated layout command stream.

Exit gate:

- Candidate parsing or compilation failure leaves the current document, revision, roots, and symbol lookup unchanged.
- Numeric results are tagged with document, binding, and request generations; stale results are rejected.
- Visible tensor work scales with the displayed window.
- Export escapes names, observes output budgets, and uses the same validated layout geometry.

## Fresco least-squares and automatic differentiation

### MVP mathematical contract

Implement weighted nonlinear least squares over residuals `r(x)` with active-variable scaling `T` and residual scaling `S`:

```math
q(x) = S r(x)
```

```math
Phi(x) = 1/2 ||q(x)||^2
```

The problem owns ordered residual roots, ordered distinct active scalar variables, fixed finite bindings, positive finite scales and weights, options, resource limits, and a prepared workspace. Report residual satisfaction, stationarity, accepted progress, work exhaustion, derivative failure, and numerical failure as distinct outcomes.

### LS1 problem preparation and residual evaluation

- Validate roots, variables, bindings, scales, options, supported operators, and resource limits before iteration.
- Resolve names once and use indexed binding storage during evaluation.
- Evaluate raw and scaled residuals and objective into reusable buffers.
- Reject empty residual sets, overlapping active and fixed variables, missing bindings, and non-finite configuration.

Acceptance:

- Batch residuals match individual reference evaluation.
- Hand-computed scaling and weighting fixtures match.
- Preparation failure leaves the repository unchanged.
- Routine evaluation performs no name lookup or buffer growth.

### LS2 reusable symbolic Jacobian

- Build derivative roots once for each residual and active variable.
- Memoize shared derivative requests and bound derivative preparation.
- Preserve residual domains and specialize zero, one, and integer powers so inactive derivatives do not introduce artificial singularities.
- Apply residual and variable scaling consistently to the dense Jacobian.

Acceptance:

- Symbolic derivatives agree with hand-derived and domain-safe finite-difference fixtures.
- Failed preparation rolls back all appended derivative state.
- Repeated Jacobian evaluation adds no expression nodes.

### LS3 dense Levenberg-Marquardt solver

- Implement a focused dense Householder QR solve for the damped augmented system.
- Reuse contiguous workspace and avoid matrix inversion.
- Accept finite improving trials through a documented ratio and damping policy.
- Bound accepted iterations, trial attempts, residual evaluations, Jacobian evaluations, and damping retries.
- Retain the best finite accepted point and complete diagnostic counters.

Acceptance fixtures:

- Unique linear root `[x+y-3, x-y-1]` reaches `(2, 1)`.
- Inconsistent residuals `[x-1, x-3]` reach the stationary solution `x=2`.
- Weighted inconsistent residuals reach the independently calculated weighted solution.
- Rosenbrock residuals reach the expected root from a documented start.
- Underdetermined, redundant, stationary, invalid-domain, rejected-trial, and exhausted-budget cases return accurate reasons without false uniqueness or convergence claims.

The first solver release is complete when LS1 through LS3 pass and their preparation, memory, and solve profiles are recorded.

### LS4 compiled multi-root evaluation

Compile supported residual and derivative roots into validated topological operation arrays with owned metadata and preallocated value storage. Preserve root order, repeated roots, diagnostics, and the supported operator-domain policy. Retain lazy operations in the reference evaluator until a control-flow-capable compiled representation exists.

Acceptance:

- Compiled and reference residual/Jacobian values agree within justified tolerances.
- Shared nodes execute once per evaluation.
- Source rollback cannot invalidate an owned compiled snapshot.
- Successful repeated evaluation allocates no memory.

### LS5 forward and reverse automatic differentiation

- Implement reverse vector-Jacobian products and forward Jacobian-vector products over the compiled operations.
- Accumulate repeated operands, roots, and shared subexpressions correctly.
- Assemble full dense Jacobians through explicit forward or reverse sweeps.
- Keep symbolic differentiation as the reference backend.

Acceptance:

- Symbolic, forward, reverse, and numerical derivatives agree on supported fixtures.
- `dot(u, Jv) = dot(J^T u, v)` holds for deterministic fixtures.
- Solver outcomes agree across derivative backends.
- Primal, tangent, and adjoint workspaces have explicit reset and input-generation rules.

### Workload-driven solver extensions

Consider sparse Jacobians, matrix-free solves, regularization, bounds, robust loss, additional smooth functions, sketch constraints, parallel evaluation, and accelerator execution only after a measured consumer establishes the requirement. Give each extension an independent contract and acceptance fixture.

## Python tensor and solver APIs

The Python session can open and inspect the built-in Fresco resource. Native tensor construction, tensor algebra, NumPy array exchange, matrix multiplication syntax, least-squares preparation, solving, and structured diagnostics remain open.

Implement these APIs through the same validated Fresco and solver boundaries used by Rust. Keep session handles scoped and reject stale or foreign handles.

Exit gate:

- Python tensor shapes, values, and errors match Rust fixtures.
- NumPy exchange documents copy and ownership behavior.
- Installed-package smoke tests and embedded-interpreter tests cover tensor and solver workflows.
- Type stubs and package exports match the runtime module.

## Headless profiles and package boundaries

The source has moved into Pedestel, Salvus, Nexome, and Parvis tiers with root compatibility re-exports. The root package still resolves Iced, WGPU, Cosmic Text, and the Rust-GPU build dependency under `--no-default-features`.

Implement:

- Add explicit CPU-headless, GPU-compute, desktop, SPIR-V-artifact, and Python feature boundaries.
- Gate exports, CLI branches, tests, examples, and shader artifact generation consistently.
- Keep the root crate as the compatibility facade while proving independent consumers.
- Extract packages only when an independent consumer or measured build improvement justifies the boundary.

Exit gates:

- CPU-headless checks and dependency trees exclude Iced, WGPU, Cosmic Text, and `spirv-builder`.
- GPU compute runs a WGSL parity fixture without Iced or the Rust-GPU builder.
- Default desktop preserves the current viewer, test registration, and provider behavior.
- Python builds and smoke tests run from a freshly built package.
- The resulting package graph is acyclic.

## Workbench persistence and remote host boundary

The command registry, document store, Properties pane, bottom tray, persistent console session, bounded output store, cancellable jobs, metadata-only example browsing, Fresco inspection, Rube waveform provenance, and Python golden workflows are implemented.

Remaining desktop work:

- Make Explorer, Properties, and bottom-tray dimensions resizable and persist visibility and sizes across restart.
- Turn Quick Open and command search into one target-aware launcher with explicit modes.
- Route Settings through one validated editor with scope and durable persistence.
- Preserve resource identity across path aliases and reveal operations.
- Complete keyboard focus restoration and a running light/dark/narrow-layout accessibility review.

Remaining remote-host work:

- Define versioned owned request, document, inspection, job, diagnostic, and result records.
- Put Fresco inspection and Rube execution behind one headless host.
- Add bounded event history, snapshot refresh, result deduplication, reconnect, stale-handle rejection, and cancellation acknowledgement.
- Select a concrete transport after the host contract passes in-process tests.

Exit gate:

- Restart restores pane state and validated settings.
- Quick Open, command search, menus, toolbar, context menus, and shortcuts use the same target and enablement rules.
- A remote client can reconnect during a Rube run, recover current state, and open one deduplicated waveform result.
- Desktop, Python, and remote callers observe equivalent Fresco selection and Rube result provenance.

## Reference model workloads

Introduce these only after the headless and compute boundaries pass:

- A CPU heat-diffusion workload with known refinement behavior, explicit field layout, boundary conditions, and accepted-state preservation.
- A small affine neural-network layer with a fixed activation, hand-computed output, declared precision, and CPU/accelerator parity.

Promote numerical storage or execution contracts only after these workloads and an existing consumer share the same layout, lifetime, precision, and error semantics.

## Verification for every completed work package

Run the narrowest owning tests during implementation. Before removing a work package from this roadmap, run:

```powershell
cargo check --all-targets --offline
cargo test --offline
cargo clippy --all-targets --offline -- -D warnings
cargo run --offline -- -t
git diff --check
```

Also run feature-specific Python, GPU, Renode, packaging, and desktop checks when the work package touches those boundaries. Record unavailable hardware or external tools separately from source failures.
