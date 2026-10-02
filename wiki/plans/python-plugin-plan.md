# Python Integration and In-Process Console Plan

## Scope

Add a supported Python interface to Trellis that can be used from:

1. A regular CPython REPL (`import trellis`).
2. A persistent, in-process Python console hosted by Fascia.
3. A Jupyter notebook (`import trellis` in a kernel using the installed package).

The same public Python API and Rust implementation must serve all three entry
points. This plan does not add a Jupyter server or kernel, nor does it replace
Trellis's Rust subsystem APIs.

## Current Baseline

- Trellis is currently one Rust library/application package, with subsystem
  modules exported from `src/lib.rs`. Fascia is the Iced desktop host, and its
  application uses a message/update/view loop.
- The CLI in `src/main.rs` launches Fascia or runs Cove tests/examples; it is
  not a general-purpose interactive language runtime.
- The only existing Python is the Renode-specific peripheral bridge at
  `tools/renode/scripts/crew_pydev.py`. It is neither a public package nor a
  reusable Trellis Python API.
- No Python binding, Python packaging manifest, or CPython dependency is
  declared today.

## Recommended Shape

Treat Python support initially as a first-class binding/API, not as an
open-ended dynamic plugin framework. Keep the existing Rust modules as the
implementation owners. Add a small, curated Python facade over selected
operations and make each host an adapter to that facade:

```text
Rust subsystems (source of truth)
          |
   Python binding/API
     /      |       \
CPython   Fascia    Jupyter
 REPL     console   notebook import
```

The simplest useful arrangement is one PyO3 binding surface built with the
Trellis library and enabled for both extension-module use and embedding.
Fascia initializes CPython in its own process and executes requests against
that same binding/API. Standard Python and Jupyter load the built extension
package in their own process. Jupyter support means normal Python package
import from an existing kernel; it does not require Trellis to implement or
ship a Jupyter kernel.

Keep the first public API deliberately small. Start with a `Session` object and
one or two high-value vertical slices, such as geometry loading/metadata and
one simulation workflow. Add further modules only when a real use case is
defined. Python-facing objects should own or safely share Rust state through
typed wrappers; do not expose Rust internals, raw pointers, or UI objects.

## Reorganization Guidelines

1. **Preserve the Rust framework boundary.** `silo`, `flux`, `shard`, `heist`,
   `fleck`, `rube`, `karst`, and other modules remain the owners of their
   models and behavior. Python bindings call public, task-level APIs rather
   than reimplementing parsing or simulation in Python.
2. **Add a Python boundary, not a second framework.** Initially keep binding
   code in a focused `python` module in the root library (behind a Cargo
   feature) and add Python packaging metadata in a dedicated `python/`
   directory. Do not move every Rust module into a new workspace crate just to
   support Python.
3. **Split the Rust core only if needed.** If an early PyO3 build/embedding
   spike shows the root library cannot be built and loaded reliably in both
   modes, extract only the non-UI framework into a `trellis-core` crate. Keep
   Fascia and the executable as hosts depending on that core. This is a
   fallback, not an upfront rewrite.
4. **Keep host adapters thin.** Standard REPL and Jupyter use the installable
   Python module directly. Fascia adds console presentation, request routing,
   and lifecycle management; it must not maintain separate implementations
   of Python commands.
5. **Do not use the UI thread for Python evaluation.** Console submissions
   must run outside Iced's update/view path, then return output/errors as
   application messages. Preserve one persistent interpreter namespace per
   console session.
6. **Keep policy-compliant execution explicit.** Embedded Python is in-process,
   not a security sandbox. Document that executed Python has the permissions
   of the Trellis process. Do not advertise arbitrary code execution as
   isolated or safe.

## Phase 0: Prove the Build and Runtime Boundary

### Objective

Verify that a single PyO3 binding surface can be used by a normal Python
process and embedded by the Fascia executable on the supported development
platform before committing to broader restructuring.

### Tasks

1. Choose and document the initial supported CPython versions and platform
   scope. Prefer system CPython initially; defer bundling a Python runtime.
2. Add a minimal PyO3 module exposing only a version function and build it as
   an importable extension.
3. Build a small Fascia-side probe that initializes CPython in-process and
   imports/calls that same binding.
4. Verify the build/link configuration for both extension loading and
   embedding. Keep any Cargo features needed for the two modes explicit.
5. Record the actual packaging/build commands and Python discovery requirements.

### Acceptance Criteria

- `import trellis` works in the selected CPython version and returns a known
  version value.
- A Fascia process can initialize Python and call the same native function.
- The existing default Rust build remains usable when Python support is
  disabled, if the binding is feature-gated.
- If one binding artifact cannot support both cases cleanly, stop and use the
  narrow `trellis-core` extraction fallback before proceeding.

## Phase 1: Define the Stable Python Facade

### Objective

Expose a small, typed and Pythonic API without making the Rust implementation
or internal data structures part of the Python contract.

### Tasks

1. Define the initial module surface, proposed as:

   ```python
   import trellis

   session = trellis.Session()
   asset = session.geometry.load("model.obj")
   print(asset.vertex_count)
   ```

   Finalize the first vertical slice based on the current stable Rust APIs;
   do not promise all Rust subsystems at once.
2. Define ownership/lifetime semantics for `Session` and returned objects.
   Prefer safe owned wrappers and shared references whose lifetime is anchored
   by Python-visible objects.
3. Map Rust errors to meaningful Python exceptions, preserving the operation
   and error detail. Do not return empty success-shaped values on failure.
4. Specify synchronous behavior first. For operations that can block
   substantially, release the GIL while Rust work runs and document that
   behavior.
5. Keep UI-only behavior out of the facade. A notebook or REPL must not need
   Iced or a display server to import and use non-UI APIs.
6. Add API examples and tests for Python-visible signatures, errors, object
   lifetime, and repeated use of a session.

### Acceptance Criteria

- The first documented operation works from Python without reaching into
  private Rust implementation details.
- Python exceptions identify failures; representative invalid-input and I/O
  failures are covered.
- Closing/dropping Python wrappers does not leave worker threads, GPU/UI
  resources, or Rust state with unclear ownership.
- Rust APIs remain the source of behavior and are tested independently of the
  Python adapter.

## Phase 2: Package for CPython REPL and Jupyter

### Objective

Make the facade installable and usable as a conventional Python package,
including from an existing Jupyter kernel.

### Tasks

1. Add Python packaging metadata in `python/` (for example, a `pyproject.toml`
   using Maturin) and define how it builds the native module.
2. Support a developer install/build workflow and produce platform wheels for
   the agreed initial Python versions.
3. Add package metadata, version handling, and a concise top-level API export.
   Avoid exposing internal binding module names as the user-facing API.
4. Add smoke tests that launch Python, import the package, create a session,
   execute the first operation, and verify a returned value.
5. Add a notebook example that imports Trellis and exercises that same API.
   Treat notebook cells as an integration example, not a new runtime.
6. Document installation, supported interpreters, known platform limits, and
   how to install the package into the active Jupyter kernel environment.

### Acceptance Criteria

- A clean environment can install the development package using documented
  steps and successfully `import trellis`.
- The same documented API example runs in a plain REPL and in an existing
  Jupyter kernel.
- Wheel/build failures for unsupported interpreter/platform combinations are
  explicit rather than silently falling back to an incomplete package.
- Importing the package does not initialize Fascia, create a window, or
  require Jupyter-specific dependencies.

## Phase 3: Add the Fascia In-Process Python Console

### Objective

Host a persistent Python console inside Fascia, using the same API and
interpreter semantics as the external Python package.

### Tasks

1. Add a console document/tab and a small console state model. Keep it separate
   from the text editor and existing test/example runner.
2. Add a Python runtime owner for the Fascia process. Initialize it once, keep
   a persistent namespace, and expose the same `trellis` package/API available
   to external Python. Do not initialize a fresh interpreter per command.
3. Implement basic console interaction: submit input, preserve a cell/command
   history, display standard output/error, and display structured exceptions.
   Preserve multi-line Python input using Python's completeness semantics
   rather than writing a Trellis-specific parser.
4. Route evaluation away from Iced's UI update/render path. Return completion
   and output through Iced messages; keep submission ordering deterministic.
5. Define shutdown and cancellation behavior. Ensure closing a console or the
   application does not deadlock while Python is executing or leave a worker
   thread running.
6. Keep the first version a basic REPL, not a full terminal emulator, debugger,
   shell, or Jupyter frontend.

### Acceptance Criteria

- In Fascia, multiple consecutive submissions share Python variables and
  imported Trellis objects.
- A slow Rust operation does not block normal window interaction.
- Output, syntax errors, runtime exceptions, and Rust-originated failures are
  visible in the console.
- The console calls the same facade methods and returns the same results as
  plain Python/Jupyter.
- Fascia remains usable without Python support if that build mode is retained;
  otherwise missing Python-runtime configuration fails with a clear startup
  message.

## Phase 4: Hardening, Documentation, and Release Gate

### Objective

Make the three usage modes maintainable and verifiable as one feature.

### Tasks

1. Add focused Rust tests for binding conversions/errors and Python integration
   smoke tests for import, session lifetime, repeated calls, and failure paths.
2. Add a Fascia console integration checklist for persistence, UI
   responsiveness, interpreter initialization failure, and clean shutdown.
3. Exercise the supported OS/Python matrix in CI where the project can
   provision CPython. Keep GPU/GUI requirements out of ordinary package
   import tests.
4. Document installation and examples for plain Python, Fascia, and Jupyter,
   with a single shared API reference.
5. Document that Python code is trusted in-process code with the application's
   permissions and resources.
6. Update the architecture guide and Fascia module guide when the feature is
   implemented, clearly distinguishing implemented support from planned
   support.

### Acceptance Criteria

- CI verifies the supported Python import path and relevant Rust feature
  combinations.
- All three host modes exercise at least one shared end-to-end API workflow.
- Fascia's UI and shutdown tests/checklist show no UI-thread blocking or
  interpreter lifecycle leak.
- Documentation accurately describes the tested platform/interpreter matrix
  and does not imply sandboxing or full Python coverage of all Rust modules.

## Suggested Delivery Order

| Phase | Deliverable | Depends on |
|---|---|---|
| 0 | Import-and-embed feasibility proof | None |
| 1 | Small stable Python facade | Phase 0 |
| 2 | REPL/Jupyter-installable package | Phase 1 |
| 3 | Fascia in-process console | Phase 0 and Phase 1; can follow Phase 2 |
| 4 | CI, docs, and release readiness | Phases 2 and 3 |

The first useful release is the end of Phase 2: it makes Trellis available to
Python and notebooks without requiring UI work. Phase 3 adds the Fascia-hosted
console. Do not delay external Python access on a complete redesign of the
Rust framework.

## Non-Goals for the Initial Release

- A generic third-party plugin discovery/registration system.
- A custom Trellis Python interpreter or Python syntax implementation.
- A Jupyter kernel, server, or notebook editor embedded in Fascia.
- A promise to expose every Rust module or every internal type to Python.
- Sandboxing, permissions isolation, or safe execution of untrusted code.
- Bundling CPython into application installers before system-runtime
  integration is proven.
- Moving the complete Rust framework into new crates without evidence that the
  binding/embedding build boundary requires it.
