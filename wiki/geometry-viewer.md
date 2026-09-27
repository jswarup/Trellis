# GPU geometry viewer

## Shared 3D viewer

Selecting an `.obj` or `.pts` file, or choosing **Cask (3D)** from any Explorer
node's context menu (including the root), opens the shared GPU viewport. This replaces the separate CPU canvas implementations in
`pts_view.rs` and `obj_view.rs`.

Run `cargo run -- --ui`, then select a geometry file in Explorer. Controls use
the application's existing theme, font, and native window shell.

| Input | Action |
| --- | --- |
| Left drag | Orbit |
| Shift-left drag or middle drag | Pan |
| Wheel / trackpad scroll | Zoom |
| Right drag | Dolly |
| F / Fit | Frame the complete object |
| Home / Reset | Reset camera and frame the object |
| 1 / 3 / 7 | Front / side / top view |
| 5 / projection button | Perspective / orthographic |
| Escape | End an active drag |

Keyboard shortcuts apply while the pointer is over the viewport. Each tab keeps
its own camera and display state. OBJ supports solid, solid-with-edges, wireframe,
and point modes. PTS uses adjustable circular point sprites, with RGB, intensity,
or Y-height coloring. Missing color/intensity values receive defaults.

## Implementation boundaries

- `fleck::geometry`: validates parsed data and creates immutable, contiguous
  `Buff`-owned geometry. Positions are centered and normalized to a unit bounding
  sphere; original bounds are retained. Normalization arithmetic uses `f64`, but
  the existing parsers still read source coordinates as `f32`.
- `fascia::geometry_load`: a bounded eight-job queue bridges Iced tasks to Heist
  execution on a background thread. Loading never waits on the UI thread. Cancel
  and close are checked at import-stage boundaries; closed-tab results are
  discarded. Imports are currently serial, and an individual read or parse is
  not preemptible.
- `fascia::camera` and `geometry_view`: document state, navigation, themed controls,
  and the Iced shader adapter. Rendering uses physical pixels for DPI correctness.
- `swarm::viewport`: GPU buffers and offscreen color/depth targets, using Iced's
  existing wgpu device and queue. No separate graphics device or window is created.
- `symph/viewport.wgsl` and `composite.wgsl`: mesh lighting, instanced circular
  points, and clipped composition into the application content area.

Geometry and label atlases upload once per resident document. Camera, transparency,
and depth changes update uniforms and draw ranges without rebuilding geometry;
refresh replaces geometry while retaining the camera, display settings and render targets;
resizing rebuilds only render targets. Closed documents are removed from the
renderer cache when no document/primitive retains their asset. Both meshes and
points are depth-tested. Rendering is event-driven rather than a permanent
animation loop. GPU buffer/texture limits are checked before uploads.

## Cask layout and controls

The Explorer hierarchy is the actual filesystem tree, not fabricated AST/module
boxes. Files and symlinks are leaves; symlinks are not followed. Discovery is
breadth-first and sorted, with no depth or sibling truncation. Unreadable entries
and resource-budget failures are reported instead of displaying partial trees.

Root depth is 1. The right-side depth slider defaults to min(4, tree depth), and
its maximum includes every leaf. A single-node tree displays depth 1 without an
inoperative slider. Filtering does not repack boxes, change colors, reset the
camera, resample surfaces, or upload buffers.

Text is shaped and measured using cosmic-text (also used by Iced), then rasterized
into a cached atlas. Labels lie in world XY planes with baselines along positive X.
Leaf Y/Z extents equal measured text height plus padding. Parents are sized
bottom-up from children packed across Y and Z, reserving a title band and padding;
siblings are disjoint and parents contain their entire descendant layout.
The sampled surface interval is at most the smallest computed dimension / 3.
Each box shares its shell vertices across faces; every dimension has at least
three segments. No minimum dimension is inflated to make sampling cheaper.
Wire mode shows subdivided box outlines, not every surface triangulation edge.
A fixed shuffled pastel cycle stays stable during interaction and refresh.

### Directional transparency

Casks default to directional cutaway at full strength; OBJ defaults to opaque.
The inspector offers **Directional surfaces** and a smooth 0.5% strength slider.
Let V be the camera-to-scene ray and N the outward surface normal:

`transparency = strength * (1 - clamp(dot(V, N), -1, 1)) / 2`

At full strength: opposite directions give 100% transparency, equal directions
give 0%, and perpendicular directions give 50%. Perspective and orthographic
cameras use their actual rays. Turning directional mode off restores uniform
transparency. Directional strength affects surfaces only; point samples, labels,
and outlines stay visible. In uniform mode, points also obey transparency.

Transparent surfaces use weighted blended order-independent transparency, without
depth writes. Opaque geometry keeps ordinary depth testing. This avoids invisible
surfaces masking descendants and avoids per-frame triangle sorting. Weighted
blending is an approximation, not physically exact refraction or ordered alpha
compositing. Two intermediate targets cost approximately 10 bytes per viewport
pixel, allocated lazily and retained until resize or document release.

Explicit import limits are 100,000 hierarchy entries, 4 million sampled vertices,
8 million triangles, and a label atlas no larger than 4096 by 4096. These are
resource limits, not hidden depth limits; oversized roots produce an actionable
error. OBJ/PTS imports retain their existing format limitations. Legacy generic
2D cask widgets remain available but are not a second document viewer; the old
fabricated filesystem builder and duplicate cask viewer state have been removed.

## Verification

```powershell
cargo check --all-targets --offline
cargo run --offline -- -t Geometry
cargo run --offline -- -t Fascia
cargo run --offline -- -t Fleck

# Optional: requires a working GPU adapter; skipped without the environment flag.
$env:TRELLIS_GPU_TEST = '1'
cargo run --offline -- -t ViewportGpu
Remove-Item Env:TRELLIS_GPU_TEST
```

The hardware tests validate WGSL/pipelines, near-point occlusion, two independent
viewport regions, upload reuse, resizing, and cache cleanup. Transparency readbacks verify both layers contribute, order
independence, zero uniform opacity, directional endpoints and camera reversal.
CPU tests cover deep filesystem discovery, containment/disjointness, sampling,
label atlases, slider bounds and refresh-generation cancellation. It does not replace
manual checks of mouse input, native window composition, or DPI changes.

Manual checks: select one file of each format; exercise every control; switch tabs;
resize the window; change themes; close a loading tab; open an empty or malformed
file. Geometry should remain within the content area and errors should appear in
the document instead of blocking the shell.

## Remaining milestones

This is a GPU rendering foundation, not the complete high-scale viewer roadmap.
Known boundaries and next work:

1. Import fidelity: concave-polygon triangulation, OBJ smoothing/normal seams,
   materials/MTL and textures, diagnostics, and double-precision source coordinates.
   Current OBJ rendering uses fan triangulation and flat shading.
2. Large assets: chunked reads/uploads, progressive previews, cancellation within
   parsing, memory budgets, spatial indexing, frustum culling, and point-cloud LOD.
   Current files and prepared geometry must fit in CPU/GPU memory; first GPU upload
   can still cause a frame hitch.
3. Rendering quality: MSAA, improved lighting, grid/axes/orientation gizmo, and
   configurable appearance. Current mesh rendering is single-sample; point edges
   use shader smoothing.
4. Interaction: cursor-anchored zoom, picking, measurement, selection, and persisted
   per-document preferences. Current zoom is centered on the camera target.
5. Hardening: explicit GPU out-of-memory/device-loss recovery, performance budgets,
   benchmark scenes, and manual Windows/Linux multi-DPI validation. Current GPU
   allocation failures beyond advertised limits use wgpu's normal error handling.

Kosh's format behavior and subsystem separation informed this integration. The
renderer is an Iced/wgpu implementation, not a transplant of Kosh's graphics API.
