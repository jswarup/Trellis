# Segue Engineering Directives

These directives apply to all code in Segue. Follow the configured formatter and the conventions already established in the owning module.

## Architecture and Performance

- Keep hot paths allocation-conscious. Prefer stack storage, borrowing (`Arr`, `MutArr`, `USeg`), moves, and compact contiguous representations.
- Use fixed-capacity `Buff` where practical. Do not add heap-backed storage without a clear ownership or growth requirement.
- Keep traits small and purpose-specific. Pure interface traits use the `I` prefix, such as `IArr`, `IStream`, and `IWorker`.
- Preserve subsystem boundaries. The core subsystems are `cove`, `silo`, `stalks`, `heist`, `flux`, `shard`, `swarm`, `karst`, `zephyr`, `crew`, and `fascia`.
- Do not introduce `serde` or `tokio`; use the project's `flux` serialization and `heist` execution facilities.

## Data and APIs

- Keep struct fields private and name them with a leading underscore, such as `_Ptr`, `_Size`, or `_Inner`. Expose state through methods.
- Do not expose Rust slices or `Vec<T>` in core public APIs. Use `Arr<'a, T>`, `MutArr<'a, T>`, `Buff<T>`, `Stash<T>`, and `USeg` as appropriate.
- Prefer the project construction helpers: `Buff![...]`, `Stash![...]`, `USeg::New`, and `USeg::FromLen`.
- Use `u32` for container indexes, counts, and segment bounds unless another width is required by an external API or address space.
- Prefer `From` and `.into()` for conversion. Where callers would otherwise need routine numeric casts, accept `impl Into<u32>` or an equivalent bounded generic type.

## Traversal

- Do not use native `for` loops or integer ranges for project algorithms.
- Use `Traverse` and `TraverseRev` for forward and reverse traversal, `TraverseMut` and `TraverseRevMut` for mutable traversal, and `Span` for early exit.
- Use `USeg` and Segue search and sort operations instead of standard slice range, search, or sort helpers.

## Naming and Formatting

- Use `PascalCase` for types, functions, and methods; use `camelCase` for locals and parameters.
- Keep Rust standard trait method names in `snake_case`.
- Follow [FORMATTING.md](FORMATTING.md) for source layout, braces, separators, and test conventions. `rustfmt.toml` is the final authority where a formatter setting applies.

## Tests and Verification

- Put component tests in that component's established `_test.rs` or `_test/mod.rs` location, and register them through the `cove` harness when applicable.
- `cargo run -- -test` runs the registered test suite. `-c` runs console tests and `-e` runs examples; without `-test`, their assertions are disabled.
- Before completing a change, run the narrowest relevant check. For broad changes, run `cargo check --all-targets`, `cargo clippy -- -D warnings`, and the relevant test commands.
- Maintain `segue.natvis` visualizers when changing a core data structure that needs MSVC debugger inspection.
