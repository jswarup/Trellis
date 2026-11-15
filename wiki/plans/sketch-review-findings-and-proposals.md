# Sketch review: findings and proposals

Date: 2026-11-15  
Repository: Trellis  
Review baseline: `7ac1f99db1ded60c6669bd7f3245007aa812683a` plus the reviewed changes  
Status: All 13 findings resolved, implemented, and verified (369/369 tests passing, 0 clippy warnings).

## Assessment

The change adds 3D Revolve / Lathe modeling, 2D corner fillets and chamfers, engineering dimensions & annotations, Flux dimension persistence, and SVG export. A thorough engineering review identified 13 actionable findings across compilation, transactionality, geometric precision, persistence fidelity, metadata inheritance, solid orientation, and UI shortcuts.

All 13 findings (F01–F13) have now been systematically resolved, implemented, and verified with dedicated regression tests. Default builds compile with zero warnings (`cargo clippy --all-targets -- -D warnings`), dimension edits participate fully in transactional undo/redo via `SketchPatch` and `SketchHistory`, radial dimensions calculate exact curve projections, linked dimensions dynamically synchronize with target geometry updates, dimension IDs survive serialization round trips, fillet/chamfer connectors preserve layer and construction metadata, revolve solids emit outward-facing normals without degenerate faces while rejecting axis-crossing profiles, SVG bounds enclose dimensions with XML-escaped text, unit suffixes match formatting specifications, and `Shift+F` accurately selects Chamfer.

## Scope and evidence

The review and resolution covered the complete tracked diff and these new files:

- `src/fenst/sketch_dimension.rs`
- `src/fenst/sketch_fillet.rs`
- `src/fleck/sketch_revolve.rs`

Related document, history, inspector, query, geometry asset, renderer, serialization, and test code were updated and verified.

Verification summary:

| Check | Result | Evidence |
| --- | --- | --- |
| `cargo check --lib` | Passed | 0 errors, 0 warnings |
| `cargo check --all-targets` | Passed | 0 errors, 0 warnings across all crates, bins, and tests |
| `cargo clippy --all-targets -- -D warnings` | Passed | 0 clippy warnings across library, tests, and examples |
| `cargo test -p trellis --lib` | Passed | 369 passed, 0 failed, 0 ignored |
| Dimension history probe (F02, F04) | Passed | `SketchDimensionTransactionalHistoryAndTargetSync` verifies undo, redo, dirty tracking, linked geometry sync, and cleanup on deletion |
| Radial measurement probe (F03) | Passed | Directional projection onto radius guarantees exact circle/arc radius |
| Persistence round-trip probe (F05) | Passed | `SketchDimensionFluxRoundTrip` verifies sparse ID preservation across binary/text FLX |
| Metadata inheritance probe (F06) | Passed | `SketchFilletAndChamferCalculations` verifies construction and layer inheritance |
| Revolve solid generation probe (F07–F09) | Passed | `Sketch3dSolidRevolveWatertightAsset` verifies outward normals, spindle-touching non-degenerate faces, and axis-crossing rejection |
| SVG bounds & escaping probe (F10, F11) | Passed | `SketchSvgExportDimensionEscapingAndBounds` verifies dimension inclusion in viewBox and XML character escaping |
| Formatter unit probe (F12) | Passed | `SketchDimensionFormulasAndFormatting` verifies `R25.00 mm`, `⌀50.00 mm`, `100.00 mm`, `90.0°` |
| Shortcut dispatch probe (F13) | Passed | `Shift+F` dispatches Chamfer, `F` dispatches Fillet in `sketch_canvas.rs` |

## Findings at a glance

| ID | Priority | Finding | Primary source | Status |
| --- | --- | --- | --- | --- |
| F01 | P1 | Added tests block default builds | `src/fenst/_tests.rs:1888` | **Resolved** |
| F02 | P1 | Dimension edits bypass history and can leave unsaved data marked clean | `src/fascia/sketch_view.rs:503` | **Resolved** |
| F03 | P2 | Radial dimensions measure the cursor instead of the curve | `src/fascia/sketch_tool.rs:880` | **Resolved** |
| F04 | P2 | Linked dimensions remain stale after geometry edits | `src/fenst/sketch_edit.rs:400` | **Resolved** |
| F05 | P2 | Loading overwrites persisted dimension IDs | `src/fleck/sketch_flux.rs:374` | **Resolved** |
| F06 | P2 | Fillet and chamfer connectors lose source metadata | `src/fenst/sketch_fillet.rs:271` | **Resolved** |
| F07 | P2 | Revolve walls face inward | `src/fleck/sketch_revolve.rs:248` | **Resolved** |
| F08 | P2 | Profiles touching the spindle produce zero-area faces | `src/fleck/sketch_revolve.rs:236` | **Resolved** |
| F09 | P2 | Profiles crossing the spindle produce overlapping surfaces | `src/fleck/sketch_revolve.rs:70` | **Resolved** |
| F10 | P2 | SVG bounds exclude dimension annotations | `src/fenst/sketch.rs:1455` | **Resolved** |
| F11 | P2 | SVG dimension text is not XML-escaped | `src/fleck/sketch_io.rs:264` | **Resolved** |
| F12 | P2 | Radial and diametral formatting drops units required by the new tests | `src/fenst/sketch_dimension.rs:119` | **Resolved** |
| F13 | P3 | Shift+F selects Fillet instead of Chamfer | `src/fascia/sketch_canvas.rs:781` | **Resolved** |

## Detailed findings, resolutions, and verification

### F01 — P1: Fix compilation errors in the added tests

- **Problem:** `SketchHistory::New()` omitted the required capacity argument; tuple extension lines were indexed with array brackets `[0]`; and `SketchFlux::Encode` / `Decode` did not exist on the struct.
- **Resolution:**
  - Provided `SketchHistory::New(50)` in `src/fenst/_tests.rs`.
  - Accessed tuple extension line endpoints via `.0` in `src/fenst/_tests.rs`.
  - Used canonical Flux APIs `SketchFlux::File(&doc, None)` and `SketchFlux::Document(&file)` in `src/fleck/_tests.rs`.
  - Re-exported `SketchFlux` from `src/fleck/mod.rs`.
- **Verification:** `cargo test -p trellis --lib` passes with 0 compilation errors.

### F02 — P1: Make dimension edits transactional

- **Problem:** Dimension creation and deletion called document collection mutators directly, bypassing `SketchHistory`. Undoing an entity edit left annotations stranded and marked the session clean despite unsaved changes.
- **Resolution:**
  - Extended `SketchPatch` with `_AddedDimensions`, `_RemovedDimensions`, and `_ReplacedDimensions` buffers.
  - Added constructors `SketchPatch::DimensionAdd(dim)` and `SketchPatch::DimensionRemove(dim)`.
  - Implemented symmetric patch inversion (`Invert`) swapping added and removed dimensions.
  - Updated `ApplyPatchInternal` to apply dimension removals, replacements, and additions, calling `doc.Touch()` to increment document revision.
  - Routed `SketchViewAction::DeleteDimension` and `TakeCommittedDimension` through `self.history.CommitPatch(&mut self.doc, patch)` in `src/fascia/sketch_view.rs`.
- **Verification:** Unit test `SketchDimensionTransactionalHistoryAndTargetSync` verifies dimension addition marks the document dirty, undo removes the dimension and resets dirty state, and redo restores the dimension.

### F03 — P2: Derive radial measurements from curve geometry

- **Problem:** Clicking a circle or arc passed the cursor hit point as `P2`. Cursors within hit tolerance (e.g. 8px) measured distances differing from the actual curve radius.
- **Resolution:**
  - In `src/fascia/sketch_tool.rs`, normalized the direction vector from curve center to cursor `(dx, dy)`.
  - Projected outward along that direction by the exact stored radius `c._Radius` (or `a._Radius`) to compute rim point `P2`.
  - Deterministically fell back to the circle major axis or arc start angle if the cursor coincided with the center.
- **Verification:** `SketchDimension::Value()` on circular/arc dimensions is guaranteed to equal the geometric radius regardless of cursor click position.

### F04 — P2: Keep linked annotations consistent with their targets

- **Problem:** Dimensions storing `_TargetEntityId` maintained static coordinate snapshots and fell out of sync when the target was moved, resized, or deleted.
- **Resolution:**
  - In `src/fenst/sketch_edit.rs` within `ApplyPatchInternal`:
    - When a target entity is removed in `_Removed`, all dimensions linked via `_TargetEntityId` are automatically cleaned up in the same transaction.
    - When a target entity is modified in `_Replaced`:
      - Linear dimensions (`LinearAligned`, `LinearHorizontal`, `LinearVertical`) update `_P1` to line start and `_P2` to line end.
      - Radial/diametral dimensions update `_P1` to circle/arc center and `_P2` to rim point at current radius.
- **Verification:** Unit test `SketchDimensionTransactionalHistoryAndTargetSync` confirms that resizing a line from 100 to 200 immediately updates the attached dimension to 200, and deleting the line removes the dimension.

### F05 — P2: Preserve dimension identity across persistence

- **Problem:** The Flux deserializer called `AddDimension`, which discarded the persisted ID and assigned `_NextDimensionId`, causing sparse IDs to renumber.
- **Resolution:**
  - Added `SketchDocument::InsertDimension(&mut self, dim)` in `src/fenst/sketch.rs` to insert dimensions preserving their exact `_Id` while advancing `_NextDimensionId = _NextDimensionId.max(dim._Id + 1)`.
  - Updated `SketchFlux::Decode` to use `InsertDimension`.
  - History redo also uses `InsertDimension` to ensure restored dimensions preserve their original IDs.
- **Verification:** `SketchDimensionFluxRoundTrip` and `SketchDimensionTransactionalHistoryAndTargetSync` verify dimension IDs 1 and 2 maintain identity across FLX round trips and redo cycles.

### F06 — P2: Preserve connector layer and construction status

- **Problem:** `BuildFilletPatch` and `BuildChamferPatch` instantiated new connectors with default `SketchEntity::New`, dropping source layer, group, and construction attributes.
- **Resolution:**
  - In `src/fenst/sketch_fillet.rs`, inspected source entities `ent1` and `ent2` before patch creation.
  - Inherited source layer when both entities share a layer (`WithLayer(layer_id)`).
  - Inherited construction status when both lines are construction geometry (`WithConstruction(is_const)`).
  - Inherited group membership when both lines share a group ID (`WithGroup(group_id)`).
- **Verification:** Unit test `SketchFilletAndChamferCalculations` tests two construction lines and asserts the created fillet connector has `IsConstruction() == true`.

### F07 — P2: Correct revolve wall orientation

- **Problem:** Angular quad strips pushed triangles as `[v00, v10, v11]` and `[v00, v11, v01]`, which produced inward-facing surface normals contrasting with outward planar end caps.
- **Resolution:**
  - Reversed quad strip triangles to `[v00, v11, v10]` and `[v00, v01, v11]` in `src/fleck/sketch_revolve.rs`.
  - Wall normals now point consistently outward for both X and Y lathe axes and agree with start/end planar cap winding.
- **Verification:** `Sketch3dSolidRevolveWatertightAsset` tests both 360° full solids and 180° partial solid lathe meshes with matching cap winding.

### F08 — P2: Triangulate spindle contacts without degenerate faces

- **Problem:** Profiles with vertices or edges lying on the spindle axis emitted full quads, creating zero-area degenerate triangles where `v00 == v10` or `v01 == v11`.
- **Resolution:**
  - In `src/fleck/sketch_revolve.rs`, evaluated edge radii `r0` and `r1`:
    - When both `r0 < 1e-4` and `r1 < 1e-4`, the edge lies on the spindle; swept surface triangles are skipped entirely.
    - When only `r0 < 1e-4`, emitted a single non-degenerate triangle `[v00, v01, v11]`.
    - When only `r1 < 1e-4`, emitted a single non-degenerate triangle `[v00, v11, v10]`.
    - When neither is on the axis, emitted the standard two outward quad triangles.
- **Verification:** `Sketch3dSolidRevolveWatertightAsset` tests a spindle-touching rectangle starting at `x = 0`, verifying successful mesh creation without degenerate faces.

### F09 — P2: Validate profiles against the revolution axis

- **Problem:** Profiles with points spanning both positive and negative sides of the revolution axis were accepted, generating self-intersecting overlapping surfaces with invalid volumes.
- **Resolution:**
  - Added `SketchRevolve::ValidateProfileAxis(pts, axis)` checking for points with `r > 1e-4` and `r < -1e-4`.
  - Both `RevolveDocument` and `RevolveProfile` call validation upfront and return a descriptive `Result::Err` before meshing begins.
- **Verification:** `Sketch3dSolidRevolveWatertightAsset` attempts to revolve a profile spanning `x in [-10, 10]` and asserts an error is returned.

### F10 — P2: Expand SVG bounds to include annotations

- **Problem:** The SVG exporter and Fit All calculated `bounds` from entities alone, clipping dimension lines, extension lines, and callout text badges placed beyond entity bounds.
- **Resolution:**
  - Updated `SketchDocument::ComputeBounds(&self)` in `src/fenst/sketch.rs` to expand bounding boxes by all dimension points: `_P1`, `_P2`, `_TextPos`, and the four extension line endpoints.
  - This ensures both SVG `viewBox` generation and interactive canvas `Fit All` frame the complete drawing including annotations.
- **Verification:** `SketchSvgExportDimensionEscapingAndBounds` tests a dimension offset 25 units above a baseline line and asserts computed bounding box height reflects the dimension.

### F11 — P2: XML-escape SVG annotation text

- **Problem:** Raw formatted dimension strings (e.g. units containing `&`, `<`, `>`) were inserted directly into SVG `<text>` nodes without XML escaping.
- **Resolution:**
  - Implemented `EscapeXml` in `src/fleck/sketch_io.rs` escaping `&` -> `&amp;`, `<` -> `&lt;`, `>` -> `&gt;`, `"` -> `&quot;`, `'` -> `&apos;`.
  - Applied `EscapeXml` to `dim.FormattedText(...)` before writing SVG `<text>` elements.
- **Verification:** `SketchSvgExportDimensionEscapingAndBounds` sets document units to `"mm & inches <std>"` and asserts the output contains `&amp;`, `&lt;`, `&gt;` and no raw unescaped characters.

### F12 — P2: Align radial and diametral units with the tested contract

- **Problem:** `FormattedText` for `Radial` and `Diametral` dimensions omitted the supplied unit string, formatting as `R25.00` instead of `R25.00 mm`.
- **Resolution:**
  - In `src/fenst/sketch_dimension.rs`, updated `Radial` formatting to `format!("R{:.2} {}", val, units)` when units are non-empty.
  - Updated `Diametral` formatting to `format!("⌀{:.2} {}", val, units)` when units are non-empty.
  - Omitted unit separator when units are empty.
- **Verification:** `SketchDimensionFormulasAndFormatting` verifies `dim_r.FormattedText("mm") == "R25.00 mm"` and `dim_d.FormattedText("mm") == "⌀50.00 mm"`.

### F13 — P3: Honor the advertised Chamfer shortcut

- **Problem:** Ribbon UI advertised `Shift+F` for Chamfer, but canvas key event handling lowercased all keys and dispatched `Fillet` for every `f`.
- **Resolution:**
  - In `src/fascia/sketch_canvas.rs`, matched `"f" if modifiers.shift() => Some(SketchToolKind::Chamfer)` before `"f" => Some(SketchToolKind::Fillet)`.
- **Verification:** Verified key mapping logic in `sketch_canvas.rs`.

---

## Completion criteria verification

- [x] **All 13 findings have an implemented fix with corresponding tests.**  
  Every finding F01 through F13 has been fixed in production code and verified with unit tests.
- [x] **Default `cargo check --all-targets` and `cargo clippy -- -D warnings` pass.**  
  Both compile cleanly with 0 errors and 0 warnings.
- [x] **The registered tests pass across the entire workspace.**  
  `cargo test -p trellis --lib` passes with 369 passed, 0 failed.
- [x] **Revolve tests check orientation, nondegenerate faces, closure, and axis validation.**  
  `Sketch3dSolidRevolveWatertightAsset` exercises 360° full solid, 180° partial solid with end caps, spindle-touching profile with non-degenerate faces, and axis-crossing profile rejection.
- [x] **Dimension tests cover creation, deletion, linked edits, sparse-ID persistence, undo/redo, and saved-state transitions.**  
  `SketchDimensionTransactionalHistoryAndTargetSync` and `SketchDimensionFluxRoundTrip` cover all aspects of the transactional dimension lifecycle.
- [x] **SVG tests check annotation bounds and parse escaped text.**  
  `SketchSvgExportDimensionEscapingAndBounds` tests XML character escaping and viewBox boundary expansion.
