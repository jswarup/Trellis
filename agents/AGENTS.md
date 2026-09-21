# Trellis Engineering Directives

These directives apply to all code in Trellis. Follow the configured formatter and the conventions already established in the owning module.

## Architecture and Performance

- Keep hot paths allocation-conscious. Prefer stack storage, borrowing (`Arr`, `MutArr`, `USeg`), moves, and compact contiguous representations.
- Use fixed-capacity `Buff` where practical. Do not add heap-backed storage without a clear ownership or growth requirement.
- Keep traits small and purpose-specific. Pure interface traits use the `I` prefix, such as `IArr`, `IStream`, and `IWorker`.
- Preserve subsystem boundaries. The core subsystems are `cove`, `silo`, `stalks`, `heist`, `flux`, `shard`, `swarm`, `karst`, `zephyr`, `crew`, and `fascia`.
- Do not introduce `serde` or `tokio`; use the project's `flux` serialization and `heist` execution facilities.

## Data and APIs

- Keep struct fields private and name them with a leading underscore, such as `_Ptr`, `_Size`, or `_Inner`. Expose state through methods.
- Do not expose Rust slices or `Vec<T>` in core public APIs. Use `Arr<'a, T>`, `MutArr<'a, T>`, `Buff<T>`, `Stash<T>`, and `USeg` as appropriate.
- Avoid use of Hashmaps, prefer using sorted arrays.
- Prefer the project construction helpers: `Buff![...]`, `Stash![...]`, `USeg::New`, and `USeg::FromLen`.
- Use `u32` for container indexes, counts, and segment bounds unless another width is explicitly required by an external API or address space.
- Prefer `From` and `.into()` for conversion. Where callers would otherwise need routine numeric casts, accept `impl Into<u32>` or an equivalent bounded generic type.

## Traversal

- Do not use native `for` loops or integer ranges for project algorithms.
- Use `USeg` and the Trellis traversal methods.
    - Use `Traverse` and `TraverseRev` for forward and reverse traversal, `TraverseMut` and `TraverseRevMut` for mutable traversal, and `Span` for early exit.
- Use `USeg` search and sort operations rather than the corresponding std methods.
- Prefer `From` and `.into()` to routine call-site casts.
- Accept `impl Into<u32>` or another appropriate conversion bound when it keeps the call site cast-free.


## Naming and Syntax
- Use `PascalCase` for types, functions, and methods.
- Prefix pure interface traits with `I`, such as `IArr` and `IStream`.
- Use `camelCase` for local variables and parameters.
- Keep standard Rust trait implementation methods in `snake_case`.
- Use private, underscore-prefixed struct fields, such as `_First`, `_Size`, and `_Ptr`.
- Place `return` on its own statement line.

## Formatting
- Follow [FORMATTING.md](FORMATTING.md) for source layout, braces, separators conventions.

## Tests and Verification
- Put component tests in that component's established `_tests.rs`, and declare them with `jeeves_test!` so each case is registered with both Cove and Rust's standard test harness.
- `cargo run -- -t` runs the registered test suite. `-c` runs console tests and `-e` runs examples; without `-t`, their assertions are disabled.
- Before completing a change, run the narrowest relevant check. For broad changes, run `cargo check --all-targets`, `cargo clippy -- -D warnings`, and the relevant test commands.
- Maintain `trellis.natvis` visualizers when changing a core data structure that needs MSVC debugger inspection.

##  Execution Principles & Agent Workflow
- **Think Before Coding**:
  - State assumptions explicitly.
  - Surface trade-offs before implementing changes.
  - If a simpler approach exists, push back. If unclear, stop and ask.
- **Surgical Precision**:
  - Touch only what you must. Do not "improve" adjacent code, comments, or formatting.
  - Clean up only your own mess (remove unused variables/headers orphaned by your changes).
- **Simplicity First**:
  - Write the minimum code needed to solve the problem. Do not build speculative features, "flexible" abstractions, or unnecessary error handling.
- **Goal-Driven Execution**:
  - Define clear success criteria.
  - Loop and verify independently before declaring completion.
- **Verification**:
  - Always verify modifications with a clean build.
- **Commit Directive**:
  - Never run `git commit` or `git push` without an explicit directive from the user.
- **Always Review**:
  - Review the final diff for minimal footprint and strict compliance with project invariants before declaring completion.
