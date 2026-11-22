// src/python/mod.rs ----------------------------------------------------------------------------------

//! # Python Integration and In-Process Runtime
//!
//! Provides the PyO3 binding facade for external CPython/Jupyter use
//! and the in-process interpreter probe and host for Fascia.

use pyo3::prelude::*;

#[cfg( feature = "tests")]
pub mod _tests;
pub mod circuit;
pub mod console;
pub mod geometry;
pub mod session;
pub mod sketch;

pub use circuit::{PyLayout, PySimEngine};
pub use console::{ConsoleExecutionResult, PythonConsoleEngine};
pub use geometry::{PyGeometryAsset, PyGeometryService};
pub use session::{PyDoc, PyInspectionInfo, PyJob, PySession, PySettings};
pub use sketch::PySketchDocument;

//-------------------------------------------------------------------------------------------------

fn populate_module( m: &Bound< '_, PyModule>) -> PyResult< ()>
{
    m.add_class::<session::PySession>()?;
    m.add_class::<session::PyDoc>()?;
    m.add_class::<session::PyInspectionInfo>()?;
    m.add_class::<session::PyJob>()?;
    m.add_class::<session::PySettings>()?;
    m.add_class::<geometry::PyGeometryAsset>()?;
    m.add_class::<geometry::PyGeometryService>()?;
    m.add_class::<circuit::PyLayout>()?;
    m.add_class::<circuit::PySimEngine>()?;
    m.add_class::<sketch::PySketchDocument>()?;
    m.add_function( wrap_pyfunction!( version, m)?)?;
    m.add_function( wrap_pyfunction!( load_geometry, m)?)?;
    m.add_function( wrap_pyfunction!( open, m)?)?;
    m.add_function( wrap_pyfunction!( inspect, m)?)?;
    m.add_function( wrap_pyfunction!( run, m)?)?;
    return Ok( ());
}

/// Top-level `trellis` Python module.
#[pymodule]
pub fn trellis( m: &Bound< '_, PyModule>) -> PyResult< ()>
{
    return populate_module( m);
}

/// Native extension module `_trellis` for the `trellis` Python package.
#[pymodule]
pub fn _trellis( m: &Bound< '_, PyModule>) -> PyResult< ()>
{
    return populate_module( m);
}

//-------------------------------------------------------------------------------------------------

/// Returns the Trellis framework version string.
#[pyfunction]
pub fn version() -> &'static str
{
    return env!( "CARGO_PKG_VERSION");
}

/// Convenience function to load 3D geometry from a file path.
#[pyfunction]
pub fn load_geometry( py: Python<'_>, path: &str) -> PyResult<PyGeometryAsset>
{
    return PyGeometryService.load( py, path);
}

/// Convenience function to open a document or resource by URI.
#[pyfunction]
pub fn open(uri: &str) -> PyResult<session::PyDoc>
{
    return session::PySession::OpenResource(uri, None);
}

/// Convenience function to inspect a document or resource.
#[pyfunction(signature = (target, *, root=None, coordinates=None))]
pub fn inspect(target: &Bound<'_, PyAny>, root: Option<&str>, coordinates: Option<Vec<u32>>)
               -> PyResult<session::PyInspectionInfo>
{
    return session::PySession::InspectTarget(target, None, root, coordinates);
}

/// Convenience function to run an example or operation job.
#[pyfunction]
pub fn run(py: Python<'_>, uri: &str) -> PyResult<session::PyJob>
{
    let s = session::PySession::new();
    return s.run(py, uri);
}

//-------------------------------------------------------------------------------------------------

static INIT: std::sync::Once = std::sync::Once::new();

/// Ensures CPython is configured and initialized in-process with the `trellis` module registered.
pub fn ensure_initialized()
{
    INIT.call_once( || {
        ensure_python_home();
        pyo3::append_to_inittab!( trellis);
        pyo3::append_to_inittab!( _trellis);
        pyo3::prepare_freethreaded_python();
    });
}

/// Registers the `trellis` module in CPython's inittab table so that
/// in-process Python can execute `import trellis` without needing
/// a compiled .pyd on disk or PYTHONPATH manipulation.
pub fn register_inittab() -> PyResult< ()>
{
    ensure_initialized();
    return Ok( ());
}

/// Discovers and configures `PYTHONHOME` if not explicitly specified in the environment.
fn ensure_python_home()
{
    if std::env::var( "PYTHONHOME").is_ok() {
        return;
    }
    if let Ok( output) = std::process::Command::new( "python")
        .args( [ "-c", "import sys; print(sys.base_prefix, end='')"])
        .output()
        && output.status.success()
        && let Ok( path) = String::from_utf8( output.stdout)
    {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            unsafe {
                std::env::set_var( "PYTHONHOME", trimmed);
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Phase 0 probe: Initializes Python in-process, imports `trellis`,
/// invokes `trellis.version()`, and returns the result.
pub fn probe_runtime() -> Result< String, String>
{
    ensure_initialized();
    Python::with_gil( |py| {
        let trellis_mod = py.import( "trellis").map_err( |e| {
            e.print_and_set_sys_last_vars( py);
            format!( "Failed to import trellis: {e}")
        })?;
        let ver_obj = trellis_mod.getattr( "version").map_err( |e| {
            format!( "Failed to get version function: {e}")
        })?.call0().map_err( |e| {
            format!( "Failed to call version(): {e}")
        })?;
        let ver_str: String = ver_obj.extract().map_err( |e| {
            format!( "Failed to extract version string: {e}")
        })?;
        return Ok( ver_str);
    })
}

