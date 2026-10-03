// session.rs -----------------------------------------------------------------------------------

//! # Python Session Management
//!
//! Provides the top-level `Session` object serving as the primary entry point
//! for Python callers interacting with Trellis systems.

use pyo3::prelude::*;
use crate::python::geometry::{PyGeometryAsset, PyGeometryService};

//-------------------------------------------------------------------------------------------------

/// Primary stateful session for Python callers interacting with Trellis.
#[pyclass(name = "Session")]
pub struct PySession
{
    geometry: PyGeometryService,
}

impl Default for PySession
{
    fn default() -> Self
    {
        return Self::new();
    }
}

#[pymethods]
impl PySession
{
    #[new]
    pub fn new() -> Self
    {
        return Self {
            geometry: PyGeometryService::new(),
        };
    }

    /// Access geometry loading and parsing services.
    #[getter]
    pub fn geometry(&self) -> PyGeometryService
    {
        return self.geometry;
    }

    /// Convenience shortcut: load a 3D geometry file (.obj, .pts).
    pub fn load_geometry(&self, py: Python<'_>, path: &str) -> PyResult<PyGeometryAsset>
    {
        return self.geometry.load( py, path);
    }

    /// Convenience shortcut: parse OBJ text into a `GeometryAsset`.
    pub fn parse_obj(&self, py: Python<'_>, data: &str) -> PyResult<PyGeometryAsset>
    {
        return self.geometry.parse_obj( py, data);
    }

    /// Convenience shortcut: parse PTS text into a `GeometryAsset`.
    pub fn parse_pts(&self, py: Python<'_>, data: &str) -> PyResult<PyGeometryAsset>
    {
        return self.geometry.parse_pts( py, data);
    }

    /// Returns the Trellis framework version.
    pub fn version(&self) -> &'static str
    {
        return env!( "CARGO_PKG_VERSION");
    }

    fn __repr__(&self) -> String
    {
        return format!( "<Trellis Session v{}>", env!( "CARGO_PKG_VERSION"));
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}
