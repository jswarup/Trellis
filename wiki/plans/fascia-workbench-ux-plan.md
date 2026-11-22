# Fascia Workbench UX and Web Migration Plan

Give Files, Examples, and Settings the same interaction model. Selecting an item shows its summary in Properties. Opening it brings the appropriate viewer or editor into the center. Running an explicit action reports progress and results in Output. Python can use the same operations through a session API. The active document stays visible throughout.

The first release connects one complete Fresco workflow (symbolic expression inspection) and one complete Rube workflow (digital circuit simulation and waveform publication). Those two integrations exercise different requirements, including mathematical occurrences, circuit signals, simulation jobs, and generated documents.

## Workspace layout

```text
 File   Edit   View   Run   Help       Workspace name       Quick Open
+------------------+---------------------------------+------------------+
| Explorer         | Document tabs                   | Properties       |
| Files            | Document actions                | Target breadcrumb|
| Examples         |                                 | Overview         |
| Settings         | Active content                  | Selection        |
|                  |                                 | Type-specific    |
|                  +---------------------------------+                  |
|                  | Python Console | Output         |                  |
+------------------+---------------------------------+------------------+
 Active document         Current job          Errors         Python state
```

Use a compact top bar, a roomy content area, quiet pane dividers, consistent line icons, and visible focus and selection in both themes. Explorer and Properties are resizable and collapsible. The bottom tray retains the Python session and recent diagnostic history when hidden. Restore pane sizes and visibility on restart. At narrow widths, open the side panes on demand and give the center pane priority. Use the same tab order, focus restoration, Escape behavior, and accessible labels for desktop and browser controls.

### Interaction details

- **Explorer.** Keep one search field for the selected root. Preserve expansion and scroll position when switching roots. Arrow keys navigate the tree, Enter opens the focused item, and an explicit **Run Example** action starts a job. Keep folders, static documents, runnable examples, and settings distinguishable by name and icon.
- **Content.** Use tabs for documents and one reusable Settings tab. Display the dirty state on the tab and keep each tab’s selection and viewport while it is inactive. Quick Open (`Ctrl+P`) searches resources and existing tabs. Command search (`Ctrl+Shift+P`) shows available actions and their shortcuts. Keep both in the same launcher with explicit modes.
- **Properties.** Show the target breadcrumb first, then the few fields users most often need. Put IDs and detailed diagnostics in a collapsible section. Show field-level errors alongside editable properties. Leave the previous good value visible until a change succeeds.
- **Output and Console.** Show a new-error count without automatically opening the tray. Keep Output scroll position when the user is reading earlier entries. Follow new entries when they are already at the end. Keep Python’s input, history, runtime state, and interrupt control in its own tab.

## Data and Event Flow

```mermaid
flowchart TD
    subgraph Explorer ["Explorer (Fenst)"]
        Files["Files (file://)"]
        Examples["Examples (example://)"]
        Settings["Settings (settings://)"]
    end

    subgraph Dispatcher ["Command Dispatcher (Fascia)"]
        CmdRegistry["Command Registry"]
        TargetResolver["Target Resolver"]
    end

    subgraph Center ["Center Area"]
        DocTabs["Document Tabs (DocumentStore)"]
        ActiveView["Active Content Viewer"]
        ActionToolbar["Document Action Toolbar"]
    end

    subgraph Right ["Right Pane"]
        Properties["Properties Inspector"]
    end

    subgraph Bottom ["Bottom Tray"]
        Output["Output (Job Log & Diagnostics)"]
        PyConsole["Python Console (Embedded Runtime)"]
    end

    Operations["Fenst workspace operations"]
    PySession["trellis.Session"]
    Browser["Browser API adapter"]

    Explorer -- "Select, open, or run" --> Dispatcher
    ActiveView -- "Selection change" --> Dispatcher
    Output -- "Reveal linked target" --> Dispatcher
    Dispatcher -- "Focus and layout" --> ActiveView
    Dispatcher -- "Resource or domain request" --> Operations
    PyConsole --> PySession
    PySession --> Operations
    Browser --> Operations
    Operations -- "Document result" --> DocTabs
    Operations -- "Inspection data" --> Properties
    Operations -- "Job events" --> Output
```

## How Explorer connects to the panes

| Explorer root | Properties on selection | Content on activation | Output | Python Console |
| --- | --- | --- | --- | --- |
| **Files** | Name, type, location, size, and known open state | Open or focus the matching document. Use a type-aware viewer and show a readable fallback for unsupported formats. | Report import progress, parse errors, and save results. Where possible, link diagnostics to the document and source location. | **Use in Python** inserts a snippet for opening the selected resource and displays its target before execution. |
| **Examples** | Description, module, inputs, and available actions | **Open Example** shows a description or opens a static sample. **Run Example** publishes successful results as a typed document, such as a waveform, schematic, or expression. | One run entry tracks progress, errors, cancellation where supported, and links to results. | Insert the equivalent example call with explicit arguments so users can inspect and rerun it. |
| **Settings** | Current value, default, effective scope, and validation details | Open one Settings tab and reveal the selected group or field. Edit and reset through this form. | Attach validation to the field. Report persistence failures and operations that require follow-up in Output. | Read scoped values and apply supported changes through the same settings service and validation rules. |

### Selection, actions, and results

- **Select** changes the inspection target without replacing content or executing a job. **Open** focuses or creates a tab. **Run** starts a job. Keep those actions distinct for both pointer and keyboard users.
- **Decouple Tree Enumeration from Execution**: In `fenst/example.rs`, expanding an `XplrExample` node must **never** call `Run()`. Browsing and expanding nodes returns static metadata and structural hierarchy without simulating circuits or computing expressions. Execution is triggered strictly by an explicit `Run Example` command.
- Maintain Explorer selection and each document’s content selection independently. Properties follows the last explicit selection action. A breadcrumb identifies its target. Navigating within a document or activating a tab restores that tab’s selection. Closing or replacing the target falls back to the active document or an empty state. Moving focus to Output or Python preserves the inspected target.
- Normalize resources through the existing provider boundary (`XplrProvider`). Give a file or static example a canonical resource identity and reuse its open tab. Give every generated result a document identity and preserve its originating run. Retain document revision and a semantic selection key in Properties and Output links. A canvas hit ID may be translated by the viewer before it leaves that viewer.
- Treat loading, running, completion, cancellation, and failure as explicit states. User-initiated commands may open their requested result when ready. Background completion publishes an Output link without stealing focus. Accept results only when the operation and target generation are still valid. A failed import, command, or save leaves the previous document or saved settings usable.
- Retain Python prompts, return values, and normal stdout and stderr in the Console. Route structured workbench diagnostics and job status to Output, with a run link between the two where applicable. Use bounded histories. Console snippets capture the intended target when inserted. Switching tabs before execution keeps that target explicit.

### First workflows to deliver

1. **Fresco inspection.** Start with a typed Math View open operation for the existing built-in document and its `Matrix_A` root. The registered `ExpressionExplorerExample` currently exposes a separate tree with text leaves, including `Matrix_M`. Keep its resource links valid if both fixtures move under one example entry. Properties shows the formula and tensor coordinates via `FrescoProperties`. **Use in Python** inserts an inspection call for the same resource and selection.
2. **Rube simulation.** Extract circuit construction from `DaddaMultiplier4x4` so Open can build a schematic without running the existing verification cases. Add a separate simulation operation with a defined stimulus and waveform result. Inspect a net or port in Properties, then open the waveform from its run link in Output. Expose the same simulation operation from Python.
3. **File and settings integration.** Open a supported VCD file from Files, inspect a signal, then reveal its source from the document tab. Open the layout group from Settings, change a pane preference, and verify the shell reflects the saved value after restart. A failed save preserves the previous effective value and gives the user a diagnostic in Output.

## Menubar, toolbars, and popups

Make every surface invoke the same typed command. A command has a stable ID, label, arguments, availability rules, optional shortcut, and handler. The dispatcher checks the resolved target and availability again when invoked. Start by adapting the existing desktop actions to this dispatcher, then remove forwarding paths as they are migrated.

```text
Command metadata     stable ID, label, group, shortcut
Invocation           command ID, resolved target, typed arguments
Availability         enabled, disabled with reason, checked when applicable
Handler result       UI change, shared operation request, or error
```

Resolve shortcuts against the focused widget and application command scope. Resolve a popup’s target when it opens and revalidate that target at invocation. Labels and shortcuts can change independently of the underlying operation.

| Surface | Purpose and placement |
| --- | --- |
| **File** | New by document type, Open File, Open Folder or Workspace, Recent, Save, Save As, Close Tab, and Exit. |
| **Edit** | Undo, Redo, Cut, Copy, Paste, and Select All. The focused editor owns the operation. |
| **View** | Explorer, Properties, Python Console, Output, Focus Pane, and Reset Layout. Theme and other preferences live in Settings. |
| **Run** | Run the selected example or supported document operation, stop the selected cancellable job, and reveal its Output entry. |
| **Help** | Documentation, shortcuts, and About. |
| **Top bar** | Workspace identity, Quick Open, command search, and pane toggles. Replaces the existing full-width duplicate toolbar with this compact bar. |
| **Document toolbar** | Show operations needed repeatedly in the active view. Fresco gets root navigation and Fit. Rube gets schematic or waveform controls and simulation actions where supported. Move lower-frequency items into overflow using the same commands. |
| **Context menus** | Explorer operations target the clicked row. Document menus target the clicked term or signal. Tab menus offer close and reveal actions. Output menus offer copy and jump to source. Resolve the target from the menu invocation, including when it differs from the active selection. |
| **Status bar** | Show active document state, current job, error count, and Python state. Interactive entries reveal the corresponding pane or run. Avoid duplicate status strips inside viewers when the same information fits here. |

Use popovers for short choices such as Open With, Output filters, or toolbar overflow. Use dialogs for file picking, save or discard decisions, and operations that need multiple inputs before execution. Keep command names, enablement, checked state, and shortcuts consistent across menus, toolbars, search, and keyboard. Place project and application settings in the single Settings editor.

## Shared framework and ownership

| Layer | Responsibility |
| --- | --- |
| **Fenst** | Extend the existing provider registry with backend-neutral resource descriptors and routing. Own shared operations used by Fascia, Python, and the browser adapter. Start with open and inspect, then add run and settings operations as the workflows exercise them. |
| **Fascia** | Own document tabs and lifecycle (`DocumentStore`, `IWorkbenchDocument`), selection and focus, the command dispatcher, pane layout, and the Iced views. Each document adapter supplies its content view and advertises the capabilities it supports. Reuse the existing tab store and lifecycle hooks during migration. |
| **Domain modules** | Rube and Fresco retain their algorithms and domain types. Register built-in workbench contributions at the application composition boundary. Adapters translate a domain model into shared resource, property, and result records. Keep specialized rendering in the appropriate viewer. |
| **Python and browser hosts** | Python calls shared application operations through Parvis bindings. A browser client consumes versioned, owned records and events through transport adapters. Desktop paths, Iced widgets, Python objects, and borrowed domain state stay inside their respective hosts. |

### State and record boundaries

| State or record | Owner and behavior |
| --- | --- |
| Resource reference | Fenst provider. Carries provider identity and a provider-normalized key. Identifies content source and supports locating the corresponding Explorer item. |
| Document and revision | The workspace document owner holds the domain model. Initially adapt the existing Fascia document store for desktop and use session-owned documents for standalone Python. The revision changes when the underlying model changes. Pane resizing, zooming, and focusing leave it stable. |
| Tab and selection state | Fascia shell and viewer adapter. A tab owns view state. A selection identifies a resource or a document revision and a semantic key, such as a Fresco root and tensor coordinate or a Rube net. Remap a selection only when the adapter can establish a valid match after an edit. |
| Command invocation | Fascia dispatcher for UI requests. Carries a command ID, arguments, and an explicit target when applicable. Python invokes the corresponding shared operation through its session. |
| Job and diagnostic events | The operation runner owns job state and terminal results. Events identify the job, stage, severity where applicable, and optional document, selection, and result references. Output keeps a bounded presentation history. Closing or clearing it leaves the job and its result available to API callers. |
| Layout and settings | Host persistence stores versioned layout preferences separately from document content. The settings service validates scope and value, then commits and publishes the effective change. |

### Module contributions and property projections

Use `XplrProvider` for discovery, `IWorkbenchDocument` for native tab lifecycle, and domain projections for inspection. Register built-in contributions at application composition. A contribution describes its resource and the operations it supports. Each operation has typed inputs and results. Fresco Open returns an owned expression document and Inspect takes a root or component key. Rube Open returns a circuit, Inspect takes a net or port key, and Run takes a stimulus and produces a waveform reference.

Share property fields such as names, types, and read-only values. Keep math previews, waveform plots, and specialized editors in their viewers. A desktop projection can borrow the owned document for rendering. Python and browser clients receive bounded owned records. Extract a capability trait or shared document store when a second implementation needs the same behavior. Defer dynamic plugins, a generic property editor, and declarative layout until a concrete integration needs them.

### Command and job lifecycle

1. **Resolve at invocation.** An Explorer context menu captures the row it opened on. A toolbar uses the current document and selection. A shortcut routes through the focused widget first when it is an editing action. For a destructive or asynchronous command, keep the resolved target through confirmation and scheduling.
2. **Validate before dispatch.** Resolve the target again, check capabilities and revision, and return a typed failure if the command can no longer apply. Menus and command search use the same availability checks to show disabled actions or their reason.
3. **Publish through one job path.** Return a job ID and report progress and one terminal outcome. Distinguish a cancellation request from completed cancellation. A document-bound job can update its target only while the revision and operation still match. A job that produces a standalone result can finish after its source tab closes according to its declared lifetime. Apply each terminal result once even if the event is repeated.
4. **Reveal linked targets explicitly.** When a user opens a diagnostic, resolve its document and semantic selection. Focus an existing tab or reopen the source when supported. If the position or revision can no longer be resolved, show the available document and explain the stale location.

### Unified Python interface

Use the existing `trellis.Session` entry point for embedded Python, the pip package, and notebooks. Align module registration, package exports, and type stubs before shipping new operations. Preserve existing Python methods during migration. Add a small set of common operations for opening resources, listing and running examples, inspecting handles, and reading or changing settings. Keep Fresco and Rube manipulation on typed handles or domain services.

The following API and Fresco resource key are proposed.

```python
import trellis

session = trellis.Session()

# 1. Resource and document opening
doc = session.open("example://Fresco/MathView")

# 2. Inspecting properties and selection
info = session.inspect(doc)
print(info.formula, info.shape)

# 3. Running explicit operations and jobs
job = session.run("example://Rube/DaddaMultiplier4x4")
print(job.wait())

# 4. Settings read/write
session.settings.set("layout/sidebar", True)
```

An embedded session can also publish results into the workbench. A notebook session returns the same typed handle or job result to the caller. Both use the same operation validation and error types. Long operations expose a job handle and structured progress, with cancellation where the underlying job supports it. Resolve handles against their session and document lifetime so a closed or replaced document produces a clear error.

Resolve an embedded session’s requests through the owning workspace. Standalone sessions own their resources and jobs. Route updates through the owner’s queue and return the committed outcome. A blocking Python wait must release the GIL where appropriate and must never block the UI thread that processes the request. Test a console-driven request while the desktop continues processing input, and use another session to verify that a foreign handle returns a clear error.

### Browser migration boundary

Prove a server-backed slice first. The browser owns panes, navigation, and focus. A headless host exposes the selected Fenst operations and owns the permitted resource set, published documents, and jobs. An isolated Python service owns each interpreter session. Check which desktop and GPU dependencies must be separated to build that host. The first proof requires the Fresco inspection and Rube run paths.

Start with a request endpoint and a job event stream. Send bounded, owned records with an explicit version and stable identifiers. Reconnect by loading the current document and job snapshot, then resuming events. If retained history has a gap, refresh the snapshot. Replaying an event leaves one job entry and one result. Choose a concrete transport after proving that flow. Add a transport trait, browser-local execution, or worker messaging when a second implementation needs it.

---

## Implementation review

Reviewed the desktop and Python implementation at `db25b93`. The commit adds a command registry, a new shell layout with a bottom tray and Properties, and working native Fresco term selection. Several phase gates remain open. The most consequential gaps are the Console tray action route, Rube waveform provenance, and the absence of shared workspace operations behind the new Python API.

| Priority | Finding and impact | Task and acceptance |
| --- | --- | --- |
| P1 | **The tray Console and the Console tab have different state.** The tray renders `self.console`, sends actions with `TabId(0)`, and the action handler only looks in the tab document store. **Use in Python** fills the tray input through a separate route. Menu commands still open center tabs for Console and Output. | Pick one console owner and route tray input, submit, completion, clear, and interrupt to that session. Make menu and shortcut actions reveal the tray tabs. Test typing and submitting from the tray, then hide and reopen it during execution and verify the same transcript and session survive. [App routing](../../src/parvis/fascia/app.rs) |
| P1 | **The Rube result link opens a fixed waveform.** The Run handler executes the registered verification example and keeps a summary and a URI. The result-link handler constructs a hard-coded VCD on click. Clicking again creates another waveform document for the same link. | Split circuit construction from simulation. Give the run explicit stimulus, capture its actual transitions, and publish a result ID for the recorded waveform. Opening the same result twice should focus one tab. Verify that two stimuli yield different traces and that every trace links to its originating run. [Run and result-link handlers](../../src/parvis/fascia/app.rs) · [Dadda verification example](../../src/nexome/rube/_tests.rs) |
| P1 | **Python parity is currently an API shape.** `open` classifies URI text and returns the same document ID. `inspect` returns preset metadata for Fresco and Rube. `run` executes a registered callback synchronously and reports success for an unknown example. Python settings use an isolated string map, and the package initializer omits the new top-level functions and types listed in its stubs. | Put resource resolution and inspection behind real, session-scoped handles and call shared Fenst operations from the desktop and Python. Reject unknown resources and stale or foreign handles. Use the same settings validation and persistence. Align package exports and stubs. Test observed properties and results against the opened models in both an embedded interpreter and an installed package. [Session implementation](../../src/parvis/python/session.rs) · [Package exports](../../truss/kaa/trellis/__init__.py) · [Type stubs](../../truss/kaa/trellis/__init__.pyi) |
| P1 | **Stop changes the displayed job state while the callback continues.** Run starts a task with no cancellation handshake, the current path leaves jobs pending until completion, and a duplicate or late completion can still append an Output link after the terminal state was already set. Output retention also pops the newly appended entry after the limit is reached, preserving old entries and hiding new diagnostics. | Let the runner own state and publish events when accepted transitions occur. Expose Stop only for cancellable operations and acknowledge actual cancellation. Preserve new entries by evicting the oldest and bound retained job metadata. Test cancellation versus completion, duplicate or unknown terminal events, and overflow with a recent error and result link. [Job dispatch](../../src/parvis/fascia/app.rs) · [Output retention and terminal handling](../../src/parvis/fascia/output.rs) |
| P1 | **Some browse paths still execute examples.** Catalog entries present as leaves, but `ToggleFolder` probes `Circuit()`. Example circuit lookup, explicit branch enumeration, and subpath resolution can call the example callback. The new tree test checks the metadata surface and does not exercise those routes. | Split catalog metadata from materialized results. Route selection and expansion through metadata-only methods, and make opening a circuit use a construction operation separate from Run. Use a counter fixture and exercise actual Explorer messages, circuit lookup, and nested resource resolution. [Explorer update](../../src/parvis/fascia/app.rs) · [Example provider](../../src/parvis/fenst/example.rs) |
| P2 | **Command targeting and enablement are partial.** Common menu, toolbar, and shortcut actions map to a registry, while tab and context actions still have direct routes. The dispatcher accepts only a command ID, Save is enabled whenever a tab exists, and Run can fall back to the first registered example without a valid target. | Add typed invocations with a captured target and resolve availability from the same data on each surface. Disable Save for read-only targets and Run when there is no runnable target. Exercise right-click on an unselected row, a focused editor shortcut, and a stale popup target. [Registry](../../src/parvis/fascia/command.rs) · [Dispatcher](../../src/parvis/fascia/app.rs) |
| P2 | **The native shell and Properties are partial.** The fixed-width Properties pane and fixed-height tray are in place. Quick Open opens the existing search sidebar. There is no integrated pane resize or restart persistence path. The Fresco view still has its own inspector, and the shell uses a formatted URI and display string for the selected term. The generated Python call passes a `term` keyword the binding does not accept. | Make the shell own pane size and persistence. Use an explicit document revision and semantic selection key in Properties. Move the existing formula preview into the shared pane, reconcile selection on tab switch and close, and generate a snippet against the working Python API. Test opening Fresco through Explorer and executing its generated snippet for a specific matrix cell. [Shell](../../src/parvis/fascia/shell.rs) · [Properties](../../src/parvis/fascia/properties.rs) · [Math View](../../src/parvis/fascia/math_view.rs) · [Bottom tray](../../src/parvis/fascia/bottom_tray.rs) |
| P2 | **Settings and file integration remain local.** Settings Explorer activation can toggle live UI flags instead of opening the Settings form. Existing viewers can open files, including VCD, while the shared resource, diagnostic, and preference persistence paths remain incomplete. | Route a Settings resource into one editor and use a validated update with explicit scope. Connect a VCD open and reveal flow to the shared resource identity. Verify path aliases reuse a tab, invalid settings and failed saves preserve the effective value, and a committed preference survives restart. [File and settings dispatch](../../src/parvis/fascia/app.rs) · [Settings provider](../../src/parvis/fenst/settings.rs) |
| P3 | **The browser boundary remains planned.** The repository has display records and native UI state. A headless host, versioned API adapter, isolated Python service, and reconnect proof are still open. | Prove Fresco inspection and a Rube run through one server-backed host after the native and Python callers share working operations. Verify current job status and result deduplication across reconnect before generalizing transport. [Fenst provider registry](../../src/parvis/fenst/provider.rs) |

### Evidence and follow-up

- The targeted offline Rust run passed 79 tests, including the Fascia suite and the Rube Dadda example. The default-feature all-targets check passed. The Python-feature parity test was blocked during the PyO3 build because the environment could not find a Python 3 interpreter. A running desktop and browser were outside the executed checks.
- The current desktop tests exercise real application state updates for Fresco selection and several command and shell actions. The Rube UI test injects a `JobFinished` message, and the Fresco snippet test supplies a prepared string. These checks cover state routing and leave actual execution, result provenance, and snippet compatibility open. [Desktop tests](../../src/parvis/fascia/_tests.rs)
- The Python parity test exercises binding calls against the current placeholder behavior. Add assertions for distinct documents, resource errors, a foreign handle, a selected tensor cell, and a waveform generated by the actual run. Verify installed-package imports alongside the embedded module. [Python tests](../../src/parvis/python/_tests.rs)
- Build the shared operation slice around one typed Fresco resource first. Use Rube to validate that the same boundary can handle owned job inputs and results. Explore reusable canvas or property-editor abstractions after both workflows work through the actual entry points.
- After functional gates close, verify pointer and keyboard focus, disabled and checked states, narrow layouts, light and dark themes, and persistence in a running desktop session.

### Suggested order for the next implementation pass

1. Route the tray to one Console owner and make Run and Stop availability reflect real targets and cancellation support.
2. Establish one Fenst open and inspect path for the typed Fresco document. Use it from Explorer and Python, then execute the generated snippet as an acceptance test.
3. Add the Rube simulation operation and publish actual waveform data through the job runner. Link it from Output and verify cancellation, retention, and repeat delivery.
4. Finish Settings and file integration, pane persistence, keyboard routing, and visual review. Use the confirmed desktop and Python workflows as the browser API fixtures.

## Detailed Work-Steps and Implementation Milestones

### Phase 1: Command System & Dispatcher Unification [Partial]
*Unify Menubar, Toolbar, Context Menus, and Shortcuts under a typed command registry.*

* **Architectural Scope**:
  - Replace fragmented UI actions (`MenuAction`, `ToolBarAction`, `TabBarAction`) with a centralized command dispatcher.
  - Commands check contextual enablement (e.g. read-only document disables Save; no selection disables Run).
  - Centralize keyboard shortcut handling in the command registry.
* **Concrete File Targets**:
  - Create `src/parvis/fascia/command.rs`: Define `CommandId`, `CommandDef`, `CommandRegistry`, `CommandContext`, and `CommandDispatcher`.
  - Modify `src/parvis/fascia/menubar.rs`: Adapt menu items to dispatch registered `CommandId`s.
  - Modify `src/parvis/fascia/toolbar.rs`: Adapt quick buttons to dispatch registered `CommandId`s.
  - Modify `src/parvis/fascia/app.rs`: Route shared GUI actions through the dispatcher with a resolved target and typed arguments.
* **Verification Gates**:
  - Unit tests for command lookup, keyboard shortcut matching, and contextual enablement.
  - `cargo test --all-targets` and `cargo clippy --all-targets` pass with zero warnings.

### Phase 2: Example Decoupling & Output Job Pipeline [Partial]
*Separate tree inspection from execution; build the diagnostic and run event log.*

* **Architectural Scope**:
  - `BranchXplr::Children()` for `ExampleItemXplr` must inspect static metadata (inputs, ports, descriptions) **without calling `Run()`**.
  - Track execution and terminal results in the operation runner. Use `OutputStore` for the bounded diagnostic and progress history shown in the pane.
  - Explicit command `workbench.action.runExample` triggers background execution and publishes a typed document link in Output.
* **Concrete File Targets**:
  - Modify `src/parvis/fenst/example.rs`: Decouple `ExampleItemXplr` and `XplrExample` tree traversal from `Run()`.
  - Create `src/parvis/fascia/output.rs`: Implement `OutputStore`, `JobRecord`, `JobStatus`, and `OutputDiagnostic`.
  - Modify `src/parvis/fascia/app.rs`: Integrate `OutputStore` into `AppState`; handle job lifecycle events.
* **Verification Gates**:
  - Listing or expanding either example never calls its executable callback. Use a fixture that exposes unexpected calls to prove this.
  - Explicit execution logs progress in `OutputStore` and publishes a clickable result link.

### Phase 3: Shell Restructuring (Top Bar, Properties & Bottom Tray) [Partial]
*Implement the 3-column + center-bottom tray workbench layout.*

* **Architectural Scope**:
  - Replace duplicate toolbar with a compact top bar (menus, workspace title, Quick Open trigger, pane toggles).
  - Add collapsible `Properties` pane on the right side showing target breadcrumb, overview, and type-specific inspection.
  - Add collapsible bottom tray with `[Python Console | Output]` tabs.
  - Maintain independent selection: selecting in Explorer updates Properties without changing the active document tab.
* **Concrete File Targets**:
  - Modify `src/parvis/fascia/shell.rs`: Redesign shell layout container into top bar, center-bottom tray, and right panel.
  - Create `src/parvis/fascia/properties.rs`: Implement `PropertiesView`, breadcrumb rendering, and field-level error display.
  - Modify `src/parvis/fascia/explorer.rs`: Ensure selection updates inspected target independently of open document tabs.
  - Modify `src/parvis/fascia/app.rs`: Wire layout persistence, pane resize handlers, and toggle commands.
* **Verification Gates**:
  - Keyboard navigation: `Escape` restores focus, `Ctrl+B` toggles sidebar, `Ctrl+J` toggles bottom tray.
  - Restoring saved layout geometry on application restart.

### Phase 4: Two Golden Workflows (Fresco & Rube) [Partial]
*Deliver the two complete end-to-end integration workflows.*

* **Workflow 1: Fresco Symbolic Expression**:
  - Select the typed Math View example in Explorer $\rightarrow$ Properties shows description and metadata without evaluating.
  - Explicitly open $\rightarrow$ activates Math View (`math_view.rs`).
  - Select term or matrix cell $\rightarrow$ `FrescoProperties` populates formula, occurrence ID, and coordinate breadcrumb.
  - Context menu "Use in Python" generates an inspection snippet in Python Console.
* **Workflow 2: Rube Digital Simulation**:
  - Select `example://Rube/DaddaMultiplier4x4` in Explorer $\rightarrow$ Properties displays module ports and instance counts.
  - Explicitly open $\rightarrow$ renders schematic in Cask2D (`cask_view.rs`).
  - Explicitly run $\rightarrow$ simulation job logs progress in Output; on completion, output link opens Waveform viewer (`waveform.rs`).
* **Concrete File Targets**:
  - Modify `src/parvis/fenst/fresco_properties.rs`: Ensure compatibility with Properties pane projection.
  - Modify `src/parvis/fascia/properties.rs`: Add `InspectFrescoProperties` and circuit pin/net/module projections.
  - Modify `src/parvis/fascia/app.rs`: Synchronize Fresco and Cask2D selection directly to `properties`; route output links.
  - Add comprehensive integration tests in `src/parvis/fascia/_tests.rs`.
* **Verification Gates**:
  - Test select, open, and inspect for Fresco. Test explicit simulation, job completion, result reveal, cancellation, and repeat completion delivery for Rube.
  - Exercise actual Explorer opening, command execution, simulation output, and console submission in end-to-end tests.

### Phase 5: Python API Parity & Web Transport Boundary [Python API scaffold, browser proof open]
*Expose uniform session operations to Python and isolate web/remote serialization contracts.*

* **Architectural Scope**:
  - Align in-process Python Console and external `trellis` package around `trellis.Session` methods: `open()`, `inspect()`, `run()`, and `settings`.
  - Expose typed handles: `Document`, `InspectionInfo`, `Job`, and `Settings`.
  - Define owned inspection and job records for boundary operations.
* **Concrete File Targets**:
  - Modify `src/parvis/python/session.rs` and `src/parvis/python/mod.rs`: Add `open`, `inspect`, and `run` methods to `PySession` and module functions.
  - Update Python stubs: `truss/kaa/trellis/__init__.pyi`.
  - Add Python session tests in `src/parvis/python/_tests.rs`.
* **Verification Gates**:
  - PyO3 test suite exercises `trellis.Session` methods with valid and stale document handles.
  - Round-trip tests for the selected boundary records.
  - Run workspace and Python checks with zero warnings.

---

## Source references

- [Fascia document lifecycle](../../src/parvis/fascia/document.rs) · [Current menubar](../../src/parvis/fascia/menubar.rs) · [Current toolbar](../../src/parvis/fascia/toolbar.rs) · [Shell layout](../../src/parvis/fascia/shell.rs)
- [Fenst provider registry](../../src/parvis/fenst/provider.rs) · [Examples](../../src/parvis/fenst/example.rs) · [Settings](../../src/parvis/fenst/settings.rs)
- [Fresco Properties model](../../src/parvis/fenst/fresco_properties.rs) · [Math View](../../src/parvis/fascia/math_view.rs) · [Fresco expression viewer plan](fresco-expression-viewer-implementation-plan.md)
- [Python module registration](../../src/parvis/python/mod.rs) · [Python session](../../src/parvis/python/session.rs) · [Package exports](../../truss/kaa/trellis/__init__.py) · [Type stubs](../../truss/kaa/trellis/__init__.pyi)
- [Rube Dadda Multiplier Example](../../src/nexome/rube/_tests.rs) · [Fresco Explorer Example](../../src/nexome/fresco/_tests.rs)
