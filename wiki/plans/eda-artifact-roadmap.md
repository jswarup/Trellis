# EDA Artifact Compliance Roadmap

## Purpose

This document defines a phased approach to EDA artifact interoperability in Trellis. The aim is useful compatibility with existing simulation, synthesis, and formal tools while preserving Trellis's subsystem boundaries and its Flux and Shard frameworks.

The first deliverable is standards-conformant Value Change Dump (VCD) support. Subsequent formats are ordered by value to the present `rube` simulator and by the abstractions they require.

## Architectural Rules

- Use Flux as the structured model interchange layer. Artifact models expose `IFluxExportSource` and use `IFluxImportSource` / `IFluxImportSink` where structured import is required.
- Use Shard as the text parsing layer. Textual format readers implement `IGrammar` and build validated artifact models.
- Do not add external serialization frameworks, including `serde`.
- Keep format-specific emission in `IFluxExportSink` implementations. A model must not contain a direct `Serialize` method for a particular file format.
- Keep artifact models in their owning subsystem. Waveform and netlist models belong in `rube`; generic parsing primitives remain in `shard`.
- Use `Stash` for incremental construction and finalize owned collections as `Buff`. Use `USeg::Traverse` and related traversal APIs in project algorithms.
- Distinguish strict compliance mode from any future recovery mode. Strict mode must reject unsupported or malformed constructs with a source location and actionable reason.

## Kosh Review

Kosh currently contains one EDA interchange implementation: VCD.

| Kosh module | Capability | Reuse assessment |
| --- | --- | --- |
| `src/rube/vcd.rs` | Writes simulator values as VCD | Useful writer outline: base-94 identifiers and four-state formatting. Needs true hierarchy, Flux output, and stronger declaration rules. |
| `src/rube/vcdio.rs` | Parses a VCD subset with `ShardTree!` | Useful grammar proof-of-concept. Not suitable as a direct port because it silently accepts invalid widths, unmatched scopes, unknown identifiers, and unsupported constructs. |
| `src/rube/vcd_model.rs` | Flattens parsed VCD into signal timelines | Useful display-model shape and binary-search query. It uses a standard `HashMap`, which should be reconsidered against Trellis's allocation and container conventions. |

Kosh does not currently implement Verilog, EDIF, AIGER, SDC, Liberty, SDF, SPEF, LEF/DEF, SPICE, or FST interchange.

## Phased Plan

### Phase 0: Artifact Foundation

Create a small shared artifact support layer before adding formats with complex diagnostics.

- Define `ArtifactDiagnostic` with severity, byte offset, format name, and message.
- Define a parser result convention that returns the partial model only in an explicitly requested recovery mode.
- Establish fixture conventions under `tests/artifacts/<format>/` with valid, invalid, and external-tool-generated samples.
- Add test helpers that verify a writer's output can be parsed back by Trellis and that expected diagnostics are returned for invalid inputs.

**Exit criteria:** VCD work can report an exact input position and diagnostic without inventing an ad hoc error convention.

### Phase 1: VCD Interoperability

Implement VCD as the first standards-facing artifact because `rube` already has simulation time, typed ports, trigger state, and four-state logic.

#### Model and Flux

- Add `VcdVar`, `VcdScope`, `VcdValue`, `VcdTimeStep`, and `VcdModel` in `rube`.
- Keep fields private; expose read-only accessors and validation constructors.
- Implement `IFluxExportSource` for the model types.
- Implement `IFluxImportSource` and `IFluxImportSink` for model construction where general Flux import is useful.
- Preserve `$dumpvars` as initial values, separate from timestamped transitions.

#### VCD Output

- Add `VcdOutStream: IFluxExportSink`, which turns the model's Flux fields into VCD directives and value changes.
- Expose `WriteVcd(model: &VcdModel, out: &mut String)` only as a convenience wrapper that dispatches `FieldExp::FluxSource(model)` to `VcdOutStream`.
- Add `VcdWriter` for `Layout` and `SimEngine`, using printable ASCII base-94 identifier codes and width-aware scalar/vector formatting.
- Preserve hierarchy from `Layout`; do not emit every module as an unrelated root scope.
- Map current trigger flags to VCD `0`, `1`, `x`, and `z` values consistently for scalar and vector signals.

#### VCD Input

- Add `VcdShard: IGrammar` and `ParseVcd(input: &str) -> Result<VcdModel, ArtifactDiagnostic>`.
- Use `ShardTree!` for lexical and repetition structure, while parser actions enforce VCD semantic rules through model validation helpers.
- Support `$date`, `$version`, `$timescale`, `$comment`, `$scope`, `$var`, `$upscope`, `$enddefinitions`, `$dumpvars`, timestamps, scalar changes, and binary vector changes.
- Reject real values, strings, aliases, escaped references, `$dumpall`, `$dumpoff`, `$dumpon`, and declarations after `$enddefinitions` until they are deliberately supported.

#### Tests

- Test nested scopes, scalar/vector values, upper/lower-case `x` and `z`, identifier validation, time ordering, and duplicate changes at a timestamp.
- Test malformed directive termination, bad widths, unmatched and unclosed scopes, values before a legal dump section, and unsupported syntax.
- Round-trip a model through Flux-driven VCD output and Shard-driven parsing.
- Generate a trace from a small `DLatch` simulation and validate its parsed hierarchy and transitions.
- Add fixture tests using VCD files produced by an external simulator and verify Trellis output opens in GTKWave or another VCD consumer in CI when that tool is available.

**Exit criteria:** Trellis can write and strictly parse the supported VCD subset, preserve hierarchy and four-state values, and pass external fixture compatibility tests.

### Phase 2: Waveform Scale and Display

Build on the VCD model without changing its on-disk contract.

- Add `VcdDisplayModel` and `VcdSignal` to flatten scope trees into path-addressable timelines.
- Use sorted timestamp changes and `USeg::BinarySearch` for `ValueAt(time)` queries.
- Coalesce repeated changes to the same signal at a single timestamp.
- Add streaming writer support so long-running simulations do not require the complete trace in memory.
- Evaluate FST only after profiling demonstrates VCD size or parsing time is an operational problem. FST is a binary format with a substantially higher implementation and compatibility burden.

**Exit criteria:** Large VCDs can be queried efficiently, and trace generation is bounded-memory apart from the caller-owned output destination.

### Phase 3: Structural Netlist Interchange

Add a narrow Verilog netlist exporter for the structure Trellis already represents.

- Emit modules, input/output ports, wire declarations, primitive gates, and explicit child instances.
- Define a documented supported subset rather than claiming complete Verilog support.
- Preserve `Layout` hierarchy and port directions exactly.
- Validate exported text with a Verilog parser such as Icarus Verilog or Yosys in integration tests where available.

Add Yosys JSON only if Yosys becomes an active workflow dependency. It is a practical tool interchange format, but it is not a replacement for standards-based Verilog exchange.

**Exit criteria:** A generated structural netlist is accepted by the selected external tool and preserves a representative Trellis layout's connectivity.

### Phase 4: Formal Interchange

Add AIGER import/export when formal equivalence checking, SAT-based analysis, or model checking becomes a concrete product workflow.

- Start with combinational AIGER, then add latches and bad-state properties.
- Define the lowering from `rube` gates and latches to AND-inverter graphs.
- Round-trip small circuits and validate them with an external AIGER consumer.

**Exit criteria:** A selected set of `rube` circuits exports to valid AIGER and preserves Boolean behavior in external formal tooling.

### Phase 5: Timing and Constraints

Add an SDC subset only after Trellis has a consumer for constraints.

- Start with `create_clock`, input/output delays, false paths, and multicycle paths.
- Parse command syntax with Shard and represent constraints as Flux-compatible `rube` data.
- Add SDF only after the simulator has a defined timing and delay model. Importing SDF without an execution semantics would produce data Trellis cannot apply.

**Exit criteria:** Constraints influence a documented timing-analysis or simulation behavior, rather than existing solely as parsed text.

### Phase 6: Technology and Physical Design

Defer Liberty, SPEF, LEF, and DEF until Trellis owns the required semantic models.

- Liberty requires cells, pins, timing arcs, and operating conditions.
- SPEF requires parasitic networks and a timing consumer.
- LEF/DEF requires technology, placement, routing, and geometry models.

These formats should not be started merely because parsers are possible. Each requires downstream behavior that makes imported data meaningful.

**Exit criteria:** The owning subsystem can consume the artifact semantically and can validate it against a real technology or physical-design workflow.

## Priority Summary

| Priority | Artifact | Reason |
| --- | --- | --- |
| 1 | VCD | Direct simulator value, existing Kosh reference, straightforward external validation. |
| 2 | VCD display and streaming | Makes VCD useful for long-running simulation and tooling. |
| 3 | Structural Verilog export | Broad synthesis and simulation interoperability for existing layouts. |
| 4 | AIGER | Best fit for formal workflows on a gate-level simulator. |
| 5 | SDC, then SDF | Valuable only with a timing model and constraint consumer. |
| 6 | Liberty, SPEF, LEF/DEF | Requires technology and physical-design semantics not yet present. |

## Verification Standard

For every artifact format:

1. Keep valid and invalid fixtures under `tests/artifacts/<format>/`.
2. Test parser diagnostics, semantic validation, and model-level invariants.
3. Test model export through Flux and textual output through a format-specific Flux sink.
4. Test text import through a Shard grammar where the format is textual.
5. Round-trip supported features without losing declared hierarchy, identifiers, widths, values, or timing data.
6. Validate at least one artifact with an independent external consumer or producer.
7. Run `cargo check --all-targets`, targeted Cove tests, `cargo test`, and `cargo clippy -- -D warnings` before merging.
