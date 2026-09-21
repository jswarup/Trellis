# GPU migration: reviewed status and implementation plan

Reviewed 2026-09-20 against the working tree and the engineering directives in
`agents/AGENTS.md` and `agents/FORMATTING.md`.

## Intended result

- `drove`: Rust compute, geometry, and composite entry points compiled to SPIR-V.
- `swarm`: host GPU devices, buffers, pipelines, dispatch, readback, viewport
  resources, and backend selection.
- `flock`: CPU-equivalent compute operations, storage, and Heist execution.
- `symph`: shared operation contracts, portable math, and explicit buffer layouts.

CPU fallback in this migration covers compute. A complete software rasterizer
and compositor are outside this scope. The existing Iced viewport must continue
to work throughout the migration.

## Verified progress

Commit `1eeef80` introduced the first boundaries:

| Area | Current state |
| --- | --- |
| `src/drove` | Rust-GPU shader workspace member and host-side entry-point contracts. |
| `src/flock` | Active standard CPU kernel closures and their function type. |
| `src/symph/compute.rs` | Shared operation identifiers and labels. |
| `src/swarm` | Uses the new contracts and Flock kernels, but still owns CPU scheduling/storage and a duplicate kernel implementation. |
| GPU rendering | Still uses both WGSL files in `src/symph`. |
| GPU compute | Not implemented; automatic selection remains CPU-only. |

Uncommitted work makes Segue a nightly workspace like Kosh. The `src/drove/`
workspace member pins the Rust-GPU `0.10.0-alpha.1` crates to Kosh's revision
`63d5a41422010cd3f732b587b9dcd1d1a2b42de7` and scopes nightly
`2026-05-22` plus its required compiler components to the root workspace.
The generated Double artifact is embedded through `ComputeSpirV`; its precise
entry point is `compute::double_cs`. Other Drove compute operations fail source
selection until implemented. Geometry and composite files are placeholders.

The root build script now mirrors Kosh: `spirv-builder` is a Cargo build
dependency, the backend uses the root nightly and `rust-lld.exe`, and output
starts at Vulkan 1.1. The earlier MSVC LNK1189 and rust-lld export-limit errors
came from treating `spirv-builder` as a normal executable dependency; they are
resolved by the build-dependency profile.

The main application passes `cargo check --all-targets --offline`, rechecked
during this review. Earlier Clippy and Swarm tests passed, but the hardware test
was skipped. wgpu 27 now accepts SPIR-V input, but the viewport still creates
WGSL modules. GPU validation and parity are still outstanding.

The minimal multi-backend boundary is now in `swarm/backend.rs`. Its associated
buffer and kernel types keep CPU, Rust-GPU, and CUDA-Oxide storage separate;
`SwarmEngine::ExecuteWith` can run an operation through an adapter without
changing the existing CPU-compatible API. CUDA-Oxide remains a dependency
boundary because its current toolchain is Linux-only; the Windows host
continues to build without CUDA crates.

CUDA-Oxide is deliberately split from the root Rust-GPU build. The root
nightly compiles Drove and the host application; a Linux `cargo oxide` build
will compile CUDA kernels to PTX and hand them to a future Swarm adapter. This
keeps each custom rustc backend in its own build boundary while `symph`
contracts and `flock` fallbacks remain shared.

## Corrections to earlier conclusions

1. Kosh's existing SPIR-V artifact and Windows build output justify testing its
   configuration. They do not establish that its Git revision alone fixes our
   linker failure, or that its current checkout rebuilds cleanly.
2. Kosh uses optimized build dependencies. The root Drove build now uses the
   same build-script classification, profile, source revision, nightly, linker,
   and Vulkan target.
3. LLVM linking has already been attempted. Installing Clang is not the first
   next step for this Rust compiler-backend export failure.
4. Segue now intentionally accepts the nightly requirement workspace-wide,
   matching Kosh. The root build script owns shader compilation and no separate
   compiler package is needed.
5. `flock` extraction is incomplete. The old public CPU function in
   `swarm/ops.rs` remains a second implementation, including different camera
   denominator behavior. A compatibility re-export can preserve the API without
   retaining that duplicate.
6. Byte access now uses unaligned reads/writes in `silo`; the earlier alignment
   finding is addressed there. Parallel dispatch still shares writable output
   views with arbitrary closures and needs an explicit disjoint-access contract.

## Execution order and acceptance gates

### 1. Make the native Windows shader build reproducible

- Pin both Rust-GPU dependencies to Kosh's locked revision:
  `63d5a41422010cd3f732b587b9dcd1d1a2b42de7`.
- Verify that revision's own toolchain requirements, then use its matching
  `nightly-2026-05-22` and declared compiler components.
- Pin the root workspace to nightly `2026-05-22`, including `rustc-dev`,
  `rust-src`, and `llvm-tools`.
- Configure LLVM linking for the compiler tool only. Cargo configuration lookup
  depends on the invocation directory, so use an explicit tool working directory
  or configuration argument; `--manifest-path` alone is insufficient.
- Match Kosh's optimized backend build through the root build-dependency
  profile. `spirv-builder` must remain a build dependency.
- Match the shader crate output configuration and start with Vulkan 1.1.
- Version the root workspace `Cargo.lock`; nested firmware locks remain ignored
  because they belong to their own application toolchains.
- Correct scaffold formatting and legitimate shader-target configuration
  warnings. Keep compiler-only transitive dependencies outside the application.

Gate: **passed** through the root `cargo check --all-targets --offline` path.
The build produced the pinned Drove SPIR-V module under the nightly target
directory and exposed its path to the host build.

### 2. Prove SPIR-V through Segue's GPU stack

- SPIR-V input is now enabled in Segue's existing wgpu 27 dependency. Kosh uses wgpu 30;
  its artifacts do not establish compatibility with Segue's version.
- The root Drove build embeds the generated Double artifact and maps it to its
  exact SPIR-V entry point. Other operations are rejected until Drove emits them.
- The root `cargo build` now generates shaders before the host build through
  `tools/build.rs`, with source-change tracking and a `DROVE_SPV_PATH` output.
- Embed the generated artifact and dispatch Double through validated wgpu APIs.
- Check values by GPU readback, including a partial final workgroup, against CPU
  results. Surface shader validation errors directly.

Gate: an actual hardware dispatch passes through Segue's wgpu version. Successful
Rust compilation alone does not pass this gate.

### 3. Finish shared contracts and the Flock extraction

- Package a minimal shader-compatible `symph` subset that both compilation units
  can consume; do not pull the full Segue application into the shader crate.
- Centralize operation IDs, binding contracts, arithmetic, and buffer layouts.
  Keep legacy projected-point output distinct from viewport matrix transforms.
- Move CPU device/storage/scheduling to Flock. Keep only necessary compatibility
  re-exports in Swarm, then update Karst consumers.
- Remove duplicate CPU algorithms and shader-text/name guessing. Unknown shader
  input must not silently execute Double.
- Give parallel jobs disjoint output regions. Keep arbitrary closures serial
  unless their access contract supports safe partitioning.
- Check dispatch/index overflow before multiplication and unify bounds and
  camera-plane behavior. Add focused tests for those behavior changes.
- Use project containers, private fields, naming, and traversal conventions.
  Keep mandatory shader ABI slice parameters confined to the compiler boundary.

Gate: CPU tests pass with one and multiple workers, short/mismatched buffers,
zero-size work, overflow inputs, and camera-plane cases. Both old compatibility
paths and new calls resolve to the same implementation.

### 4. Implement all compute operations and the Swarm GPU driver

- Implement remaining Drove compute entry points against shared contracts.
- Add the Rust-GPU adapter and a Linux-only CUDA-Oxide adapter behind the
  `IComputeBackend` contract. Keep CUDA-Oxide's `cuda-core` buffers and streams
  in the adapter; do not make `ComputeBuffer` a host-memory façade for both.
- Keep CUDA-Oxide kernel sources in a separate `cargo oxide` build target. Do
  not add its Linux-only runtime crates to the default Windows dependency graph.
- Add real GPU buffer/kernel/device implementations and cache pipelines by their
  full program identity and layout.
- Keep buffers resident across operations; make synchronization/readback explicit.
- Define automatic GPU selection, explicit backend requests, and CPU fallback.
  Shader bugs must remain errors. Fallback must account for buffer residency;
  do not replay partially submitted work silently after device failure.
- Replace the placeholder CUDA/PTX path after updating callers and tests.

Gate: integer CPU/GPU parity is exact; floating-point parity uses documented
tolerances. Automatic fallback works when no usable GPU is present.

### 5. Migrate geometry and composite shaders

- Implement Drove mesh, wire, point-sprite, and composite stages with the existing
  visual behavior, uniform/vertex layouts, and explicit entry-point mapping.
- Validate derivatives, point-edge discard/alpha, texture sampling, depth, and
  output color conversion through wgpu 27.
- Retain Iced's device and queue. Switch each pipeline only after validation.
- Preserve all generated modules when the shader compiler uses multiple outputs;
  do not select an arbitrary first module.

Gate: hardware readback tests cover depth occlusion, rendering modes, both output
color-format paths, clipping, independent viewports, resizing, and upload reuse.
Manually check UI interaction and DPI behavior.

### 6. Optimize resource ownership, then remove superseded paths

- Split shared geometry buffers from per-view uniforms and render targets.
- Add explicit view cleanup so closing one view releases its targets even when
  another view still owns the same asset.
- Measure upload counts, resident bytes, and frame behavior for shared assets;
  do not attribute speed improvements to folder moves or Rust source alone.
- Delete both project WGSL files and embedded WGSL only after replacement tests
  pass. Dependencies such as Iced may still use WGSL internally.
- Remove obsolete compatibility paths once all consumers have migrated.
- Update architecture, setup, and geometry-viewer documentation.

Gate: no project-authored WGSL remains, all standard compute operations have a
working Flock fallback, and multiple views share geometry while maintaining
independent render state.

## Verification and delivery

Run the narrow relevant checks at each stage. For the completed migration run a
clean build, `cargo check --all-targets`, `cargo clippy -- -D warnings`, component
Cove tests, shader artifact validation, and the opted-in hardware tests. Add
tests through `jeeves_test!` in component `_tests.rs` files. Report hardware skips
as unverified, not as GPU passes.

Apply the repository formatter/conventions only to touched files and review the
diff for unrelated changes. Preserve existing work. Each stage should be a
reviewable checkpoint; committing still requires an explicit user instruction.

The immediate next checkpoint is SPIR-V validation and a successful Double GPU
readback through wgpu 27, with generated artifact wiring and a tracked build
command.
