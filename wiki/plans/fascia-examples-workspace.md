# Fascia Examples Workspace Implementation Proposal

**Draft for design and engineering review · 27 November 2026**

Give Fascia a dedicated **Examples** workspace where a visitor can discover what Trellis does, run an example, and inspect the result. Use a Gallery for discovery and a detail screen for execution. Give a circuit its schematic, a math document its plots and expressions, and a structured result its tree. Keep navigation and run controls in the same place across all three.

The catalog covers every `XplrExample` registered in the current build. The source inventory contains 18 across five suites. The implementation should derive membership and counts from the registry. Curated titles and previews improve discovery, but registration alone must be sufficient to appear, run, and inspect a result.

## Experience and layout

### Gallery

The Gallery opens in the workbench content area. The visitor first sees the range of capabilities, then chooses an example. The desktop layout below shows the opening portion of the catalog.

```text
Examples                                         [ Search examples… ]

All 18      Fresco 9      Fenst 4      Rube 3      Crew 1      Karst 1
━━━━━━

┌────────────────────────────────┐  ┌────────────────────────────────┐
│  [Dadda schematic capture]     │  │  [Fresco plot capture]         │
│  4×4 Dadda multiplier          │  │  Function plotting in 2D and 3D│
│  Inspect gates and signal flow │  │  Explore curves and surfaces   │
│  Rube                          │  │  Fresco                        │
└────────────────────────────────┘  └────────────────────────────────┘

Mechanical sketch                   Hub topology
Inspect layers and entities         Follow a multicast and its replies
Fenst                               Crew

System circuit                      …
Explore the module hierarchy
Karst
```

In **All**, lead with the multiplier, plotting, sketch, hub, and system examples when available. Show each once, followed by the remaining entries in registry order. Source the two illustrations from real example output during development. If a capture or featured entry is absent, render available entries using the standard row. Filtering by suite or typing a query switches to compact results ordered by display title. Match case-insensitively across title, registered name, suite, and description. An empty search offers **Clear filters**. An empty registry explains that this build contains no registered examples.

Use the existing Fascia theme, spacing in multiples of four, a common left edge for titles and summaries, and one accent for focus, selection, and the Run action. Let the schematic or plot carry the visual detail. Use monospace for values, names of signals, and paths. Allow titles to wrap and keep status readable as text in both light and dark themes.

### Detail screen

```text
‹ Examples      Rube / 4×4 Dadda multiplier               [ Run again ]

Schematic     Result tree                              [ Record waveform ]
━━━━━━━━━
┌──────────────────────────────────────────────┬──────────────────────┐
│                                              │ Selected module      │
│                  Schematic                   │ Ports                │
│                                              │ Connections          │
└──────────────────────────────────────────────┴──────────────────────┘
Run 1 · Passed                                       View run details
```

Before execution, show the example title, one sentence describing what to inspect, and **Run example**. After success, use the same space for the returned output. For example, opening Crew's **Hub topology** and selecting **Routing → Multicast_0_to_1_2** should reveal a source of node 0, destinations 1 and 2, and the atomic multicast mode. The interaction also works for a first-time visitor who knows nothing about Crew.

Selecting a tree row changes the inspector. Its disclosure control expands or collapses the branch independently. In a schematic, selecting a module shows its ports and connections. Open the inspector on selection and provide a close action that restores focus to the triggering control. Keep **View run details** next to the run status. A recorded trace adds a **Waveform** tab to the run that requested it.

When the workbench is narrow, stack Gallery entries in one column, allow the suite strip to scroll, and open the inspector as a drawer. Size against the available content area, accounting for open workbench sidebars. Tab reaches search, filters, entries, and Run in reading order. Enter activates the focused entry or button. Back restores the query, filter, scroll position, and focused entry. A completed background run updates its status without moving keyboard focus.

## Catalog coverage

The names below identify registrations exactly. The first view is the proposed presentation of a successful result. The runtime result determines which views can open.

| Suite | Registered example | First view and inspection target |
| --- | --- | --- |
| Rube | `FullAdderExample` | Schematic of the full adder, with selectable modules and ports. |
| Rube | `Adder8Example` | Eight-bit adder schematic with hierarchy drill-down. |
| Rube | `DaddaMultiplier4x4` | Multiplier schematic. Offer trace recording as a separate action. |
| Fresco | `ExpressionExplorerExample` | Tree of scalar calculus, matrix algebra, and evaluated values. |
| Fresco | `SymbolicParsingAndDag` | Math document for parsed expressions and their shared structure. |
| Fresco | `PolynomialsAndFunctions` | Math document for polynomial and function examples. |
| Fresco | `LogicAndConditionals` | Math document for predicates and conditional expressions. |
| Fresco | `TensorsAndShapes` | Math document for tensors and their dimensions. |
| Fresco | `SymbolicDiffAndJacobian` | Math document for derivatives and the Jacobian. |
| Fresco | `CompiledProgramAndAd` | Math document for compiled evaluation and automatic differentiation. |
| Fresco | `LeastSquaresOptimization` | Math document for the least-squares example and its results. |
| Fresco | `FunctionPlotting2dAnd3d` | Math document with integrated 2D curve and 3D surface plot views. |
| Fenst | `FilesystemExplorer` | Lazy tree rooted at the returned directory, with bounded file previews. |
| Fenst | `SketchExample` | Layers, groups, and entities. Add a canvas once geometry can pass from result to viewer. |
| Fenst | `TraverseDepthFullOrder` | The mock tree used to verify traversal order. |
| Fenst | `XplrExampleProtocolDemo` | Subsystem hierarchy and inspectable leaves. |
| Crew | `HubTopology` | Nodes, routing entries, and the traffic report. |
| Karst | `System` | System schematic with drill-down through the returned circuit hierarchy. |

Registration sources are the [Rube](../../src/nexome/rube/_tests.rs), [Fresco](../../src/nexome/fresco/_tests.rs), [Fenst](../../src/parvis/fenst/_tests.rs), [Crew](../../src/nexome/crew/_tests.rs), and [Karst](../../src/nexome/karst/_tests.rs) suites.

## Integration and ownership

**Entry point.** Add an **Examples** command that opens or focuses one workspace tab. Register it through Fascia's command, tab, document, and content dispatch. Keep the existing Explorer visibility toggle and its meaning. App state already owns an `ExampleBrowser` and routes `ExampleAction`. Extend that owner to hold catalog selection and run results. The workspace tab projects that state. Closing the tab releases its view resources. A running job can finish into app state and be inspected after reopening the workspace. [Commands](../../src/parvis/fascia/command.rs) · [Tab payloads](../../src/parvis/fascia/document.rs) · [Application dispatch](../../src/parvis/fascia/app.rs)

**Catalog identity.** A registry entry supplies a static suite, name, and function. Use the pair or its canonical `example://<suite>/<name>` URI for durable lookup. Inside the app, filtered and featured rows carry the original catalog index. A filtered row's display position must never select a different function. Exclude the legacy synthetic `FRESCO_EXAMPLE_URI` (`example://Fresco/MathView`) link from the Gallery so visible membership matches `XplrExample::Registered()` 1:1 (retaining it only for Explorer backwards compatibility). Keep optional titles, summaries, featured ordering, and capture references in Fascia. A sorted table of passive records is sufficient for this inventory. [Registry and URI resolution](../../src/parvis/fenst/example.rs)

**Run handoff.** Assign a run ID when the user presses Run. Capture the catalog entry and run ID before dispatching to `HeistUiRuntime::Submit` (`JobPriority::Normal`) with cooperative `CancellationToken` support rather than spawning ad-hoc `Task::perform` threads. Reuse `XplrExample::Run()` assertion-enabled execution and a single-consumption completion envelope bridged through `HeistUiEvent`. On success, move the returned `Box<dyn Xplr + Send>` into the UI-owned node store. On failure, attach the diagnostic to the run. Associate the Output job with the same run ID. One active example run is sufficient for the first release. While it runs, browsing and inspecting previously completed results stay available. [Heist UI bridge](../../src/parvis/fascia/heist_bridge.rs) · [Execution and result storage](../../src/parvis/fascia/examples.rs) · [Output jobs](../../src/parvis/fascia/output.rs)

| Event | Required state transition |
| --- | --- |
| Run requested while idle | Allocate a run and output job, mark it running, and dispatch once. |
| Run requested while busy | Keep the active job and show which example is running. Guard duplicate activation in the update handler as well as the button. |
| Matching success | Consume the completion once, publish its root, and mark the job passed. Show its default view if the user is still on that run. |
| Matching failure or panic | Mark the job failed and expose the diagnostic and Retry. Keep the previous successful run separately selectable. |
| Duplicate or stale completion | Check run identity and terminal state before publishing. Drop any unclaimed payload. |
| Switch example or close tab | Preserve the run's identity. Completion updates its own record regardless of the current selection. |

**Inspection.** Store each returned node once and address it by a run-local node ID or a checked index into the existing node store, replacing the existing `unsafe` pointer unpacking in `ExampleBrowser::Inspect` with a safe `Buff`/`Stash` drain helper. Cache a branch's child IDs on its first successful expansion. Read a leaf through `ReadChunk(0, 4096)` and indicate truncation from the returned chunk. On a read or expansion error, show the diagnostic at that node and let the user retry. Filesystem-backed branches can change between reads. Their displayed contents reflect the provider's response when inspected.

**The execution boundary matters.** The current example provider's `ExampleItemXplr::Children` calls the registered function, and its circuit accessor can trigger execution. Some application Run paths publish a summary or reopen a URI. The Gallery should enumerate descriptors directly. After Run, every view must consume the completed root or data derived from it. A run-specific Output link should resolve the owned result in app state. Keep canonical catalog links available for navigating to the example's detail screen. This distinction prevents a second execution when opening a tab, returning from Output, or expanding a result. [Example provider](../../src/parvis/fenst/example.rs) · [Existing run dispatch](../../src/parvis/fascia/app.rs)

## Renderer contracts

| Returned capability or artifact | Integration |
| --- | --- |
| Circuit | Borrow the completed node's `CircuitXplr` and use Fascia's `CircuitSchematic` with the current palette, plus an optional `3D Hierarchy` view via `CircuitBuild` → `GeometryAsset`. Cache the resulting layout for the run. Retain a module selection that can be resolved against that run. [Circuit presentation](../../src/parvis/fascia/circuit_scene.rs) |
| Fresco document | Call `BuildFrescoDocument` once when preparing the rich view. Retain the returned document in viewer state and offer `MathView \| 2D Plot \| 3D Surface \| Result tree` switching directly in the Detail screen via `SampleFrescoPlot2d` / `ViewPlot2d` and `SampleFrescoSurface3d`. If `BuildFrescoDocument` yields no document, open the result tree or leaf. The expression explorer exercises this fallback within Fresco itself. [Fresco result adapter](../../src/parvis/fenst/fresco_resource.rs) · [2D Plotter](../../src/parvis/fascia/plot2d.rs) · [3D Surface](../../src/parvis/fascia/plot3d.rs) |
| Branch or leaf | Use lazy child expansion and bounded text preview. Make this view reachable alongside a rich renderer so raw values and structure remain inspectable. [Xplr contracts](../../src/parvis/fenst/xplr.rs) |
| Dadda trace | **Record waveform** starts the existing Dadda stimulus job. Attach the resulting waveform to the requesting run ID. A cancelled or failed recording updates the trace action and leaves the successful schematic available. [Dadda simulation](../../src/parvis/fenst/rube_resource.rs) |
| Sketch geometry | Ship the entity tree first. The sketch viewer accepts an owned `SketchDocument` while the result projects the document into `SketchXplr`. Add a read-only canvas only after a typed handoff preserves primitive geometry, layer membership, and coordinates. Verify against the rectangle, circle, arc, and ellipse in the registered example. [Sketch projection](../../src/parvis/fenst/sketch_xplr.rs) · [Sketch viewer](../../src/parvis/fascia/sketch_view.rs) |

Prepare larger layouts once per result and measure first-open and repaint behavior on the multiplier and Karst system. If preparation blocks input, move that work to the worker while it still owns the root, then transfer the prepared data with the completion. Keep UI-only state on the UI thread. Session history retains owned trees and derived views, so measure repeated runs before choosing a retention limit. Any later eviction must release the tree and derived views together and leave the run's status intelligible.

## Delivery sequence

| Milestone | Implementation work | Review demonstration |
| --- | --- | --- |
| 1. Catalog and navigation | Wire the command and workspace tab. Populate from `Registered()`, apply optional metadata, and implement search, suite filters, selection, and Back. | Open through the normal workbench command. Find all registrations, then find the multiplier by either title or registered name. A counter fixture records zero executions. |
| 2. First complete runs | Connect the browser worker, run IDs, result store, and Output. Implement generic inspection and the circuit view. Use Crew's hub and Rube's full adder as the first pair. | Run the adder, navigate to Crew during execution, then return to the adder's schematic. Run Crew and read a routing leaf. Each activation produces one owned result. |
| 3. Full renderer coverage | Wire Fresco documents, both larger Rube circuits, Karst hierarchy navigation, and Fenst tree and leaf results. Connect trace recording to Dadda. | Exercise every row of the catalog. Switch among available views and follow an Output link without increasing the example execution count. |
| 4. Presentation and accessibility | Add verified captures, tune responsive sizing, scope global `Escape`/`ArrowUp`/`ArrowDown` subscription handling so keyboard navigation works inside the workspace, track widget IDs for `focus()` restoration when closing the inspector drawer, and handle empty, busy, failed, truncated, and retry states. | Review both themes with the sidebar open and closed. Complete discovery, execution, inspection, and Back with the keyboard. Inspect a failed run beside a previous pass. |
| 5. Release verification | Run the registry-driven coverage checks and established Fascia and Fenst tests. Profile repeated opening and rerunning of the largest results. | Supply the acceptance evidence below and record the observed build inventory. Evaluate the optional sketch canvas separately from baseline tree coverage. |

## Acceptance evidence

- **Catalog completeness.** Compare visible entry identities with the live registry in the same build. Assert set equality and uniqueness. Include an uncurated fixture, duplicate display titles from different suites, an empty registry, and a search with no matches.
- **Execution identity.** Instrument a fixture to count calls. Run once, expand children, switch renderer, follow its Output link, and reopen the tab. The count should remain one. Rerun explicitly and verify that results and diagnostics belong to the correct run. Deliver a duplicate completion and a late completion after navigation.
- **Readable failures.** Cover assertion failure, panic, provider read error, and unavailable rich document. Verify the failed attempt has no successful root, the previous run remains identifiable, and an optional view failure still leaves the returned tree accessible.
- **Result fidelity.** Inspect a real Crew route, a Rube port, a Karst child module, a Fresco curve or surface, a Fenst sketch entity, and a truncated filesystem leaf. Record and cancel Dadda traces on separate attempts. Verify traces attach only to the requesting run.
- **Workbench compatibility.** Exercise existing Explorer links, command enablement, Output navigation, and opening other workbench tabs. Start the filesystem smoke check from the repository root because its fixture reads `src` and `Cargo.toml`.
- **Engineering checks.** Extend the owning component's `_tests.rs` with `jeeves_test!` cases. Run focused lifecycle tests first, followed by `cargo check --all-targets`, `cargo clippy -- -D warnings`, and the registered suite with `cargo run -- -t`. Record the manual keyboard, theme, sizing, and rich-view checks alongside the automated results. [Fascia tests](../../src/parvis/fascia/_tests.rs) · [Engineering directives](../../agents/AGENTS.md)

The first review should use the full adder and the hub topology side by side. A reviewer should be able to find both from the Gallery, run each once, identify a signal or route, and return to the same Gallery position. That demonstration validates the shared interaction and ownership model before the team invests in the remaining visual integrations.
