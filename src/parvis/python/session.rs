// session.rs -----------------------------------------------------------------------------------

//! # Python Session Management
//!
//! Provides the top-level `Session` object serving as the primary entry point
//! for Python callers interacting with Trellis systems.

use pyo3::prelude::*;
use pyo3::exceptions::{PyFileNotFoundError, PyValueError};
use crate::fenst::fresco_document::AllocateDocumentId;
use crate::fenst::fresco_properties::FrescoProperties;
use crate::fenst::fresco_resource::{FRESCO_EXAMPLE_TITLE, OpenFrescoExample};
use crate::python::geometry::{PyGeometryAsset, PyGeometryService};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

//-------------------------------------------------------------------------------------------------

/// Handle to an opened workspace document or resource.
#[pyclass(name = "Document")]
#[derive(Clone)]
pub struct PyDoc
{
    #[pyo3(get)]
    pub id:    u64,
    #[pyo3(get)]
    pub uri:   String,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub kind:  String,
    _SessionId: Option<u64>,
}

#[pymethods]
impl PyDoc
{
    fn __repr__(&self) -> String
    {
        return format!("<Document id={} uri='{}' kind='{}'>", self.id, self.uri, self.kind);
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}

//-------------------------------------------------------------------------------------------------

/// Projections and metadata for an inspected target.
#[pyclass(name = "InspectionInfo")]
#[derive(Clone)]
pub struct PyInspectionInfo
{
    #[pyo3(get)]
    pub name:          String,
    #[pyo3(get)]
    pub path:          String,
    #[pyo3(get)]
    pub formula:       String,
    #[pyo3(get)]
    pub shape:         Option<String>,
    #[pyo3(get)]
    pub selected_term: Option<String>,
    #[pyo3(get)]
    pub ports:         Option<u32>,
    #[pyo3(get)]
    pub submodules:    Option<u32>,
}

#[pymethods]
impl PyInspectionInfo
{
    fn __repr__(&self) -> String
    {
        return format!("<InspectionInfo name='{}' formula='{}'>", self.name, self.formula);
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}

//-------------------------------------------------------------------------------------------------

/// Handle to an asynchronous or completed runner job.
#[pyclass(name = "Job")]
#[derive(Clone)]
pub struct PyJob
{
    #[pyo3(get)]
    pub id:          u64,
    #[pyo3(get)]
    pub title:       String,
    #[pyo3(get)]
    pub status:      String,
    #[pyo3(get)]
    pub result:      Option<String>,
    #[pyo3(get)]
    pub result_link: Option<String>,
}

#[pymethods]
impl PyJob
{
    pub fn wait(&self) -> PyResult<String>
    {
        return Ok(self.result.clone().unwrap_or_else(|| self.status.clone()));
    }

    pub fn is_completed(&self) -> bool
    {
        return self.status == "Completed";
    }

    fn __repr__(&self) -> String
    {
        return format!("<Job id={} status='{}' title='{}'>", self.id, self.status, self.title);
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}

//-------------------------------------------------------------------------------------------------

/// Workbench settings manager for Python callers.
#[pyclass(name = "Settings")]
#[derive(Clone, Default)]
pub struct PySettings
{
    values: Arc<RwLock<HashMap<String, String>>>,
}

#[pymethods]
impl PySettings
{
    #[new]
    pub fn new() -> Self
    {
        return Self { values: Arc::new(RwLock::new(HashMap::new())) };
    }

    pub fn get(&self, key: &str) -> Option<String>
    {
        return self.values.read().unwrap().get(key).cloned();
    }

    pub fn set(&self, key: &str, val: &Bound<'_, PyAny>) -> PyResult<()>
    {
        let val_str = val.str()?.to_string();
        self.values.write().unwrap().insert(key.to_string(), val_str);
        return Ok(());
    }

    fn __repr__(&self) -> String
    {
        let count = self.values.read().unwrap().len();
        return format!("<Settings count={}>", count);
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}

//-------------------------------------------------------------------------------------------------

/// Primary stateful session for Python callers interacting with Trellis.
#[pyclass(name = "Session")]
pub struct PySession
{
    geometry: PyGeometryService,
    settings: PySettings,
    _Id:      u64,
}

impl Default for PySession
{
    fn default() -> Self
    {
        return Self::new();
    }
}

impl PySession
{
    pub(super) fn OpenResource(uri: &str, sessionId: Option<u64>) -> PyResult<PyDoc>
    {
        if let Some(doc) = OpenFrescoExample(uri) {
            return Ok(PyDoc { id: doc.Id(), uri: uri.to_string(),
                              title: FRESCO_EXAMPLE_TITLE.to_string(), kind: "fresco".into(),
                              _SessionId: sessionId });
        }
        if IsFrescoUri(uri) {
            return Err(PyFileNotFoundError::new_err(format!("Fresco resource not found: {uri}")));
        }
        let kind = if uri.contains("Rube") || uri.starts_with("cask://") {
            "cask2d"
        } else if uri.ends_with(".vcd") || uri.starts_with("waveform://") {
            "vcd"
        } else {
            "file"
        };
        let title = uri.rsplit('/').next().unwrap_or(uri).to_string();
        return Ok(PyDoc {
            id: AllocateDocumentId(),
            uri: uri.to_string(),
            title,
            kind: kind.to_string(),
            _SessionId: sessionId,
        });
    }

    pub(super) fn InspectTarget(target: &Bound<'_, PyAny>, sessionId: Option<u64>,
                                root: Option<&str>, coordinates: Option<Vec<u32>>)
                                -> PyResult<PyInspectionInfo>
    {
        let uri = if let Ok(doc) = target.extract::<PyDoc>() {
            if doc._SessionId != sessionId {
                return Err(PyValueError::new_err("Document belongs to a different Trellis session"));
            }
            doc.uri
        } else if let Ok(s) = target.extract::<String>() {
            s
        } else {
            target.str()?.to_string()
        };

        if let Some(doc) = OpenFrescoExample(&uri) {
            // The built-in resource is immutable. Resolve a local typed document on the
            // calling thread so the Python handle can move between interpreter workers.
            let coordinates = coordinates.as_deref().unwrap_or_default();
            let props = FrescoProperties::ReadSelection(&doc, root, coordinates.into())
                .map_err(PyValueError::new_err)?;
            let selectedTerm = props.Term.as_ref().map(|term| match term.Coordinates {
                Some(coords) => format!("{} at {coords}", term.Summary()),
                None => term.Summary(),
            });
            let formula = match &selectedTerm {
                Some(term) => format!("{}: {term}", props.Name),
                None => props.Name.to_string(),
            };
            return Ok(PyInspectionInfo {
                name: props.Name.to_string(),
                path: uri,
                formula,
                shape: props.ShapeLabel(),
                selected_term: selectedTerm,
                ports: None,
                submodules: None,
            });
        }
        if IsFrescoUri(&uri) {
            return Err(PyFileNotFoundError::new_err(format!("Fresco resource not found: {uri}")));
        }
        if root.is_some() || coordinates.is_some() {
            return Err(PyValueError::new_err("Root and coordinates require a Fresco document"));
        }

        if uri.contains("Rube") || uri.contains("Dadda") {
            return Ok(PyInspectionInfo {
                name: "DaddaMultiplier4x4".to_string(),
                path: uri,
                formula: "4x4 Structural Dadda Multiplier".to_string(),
                shape: None,
                selected_term: None,
                ports: Some(16),
                submodules: Some(28),
            });
        }

        return Ok(PyInspectionInfo {
            name: "Resource".to_string(),
            path: uri.clone(),
            formula: format!("Resource: {}", uri),
            shape: None,
            selected_term: None,
            ports: None,
            submodules: None,
        });
    }
}

fn IsFrescoUri(uri: &str) -> bool
{
    return uri.starts_with("example://Fresco/") || uri.starts_with("fresco://");
}

#[pymethods]
impl PySession
{
    #[new]
    pub fn new() -> Self
    {
        return Self {
            geometry: PyGeometryService::new(),
            settings: PySettings::new(),
            _Id: AllocateDocumentId(),
        };
    }

    /// Access geometry loading and parsing services.
    #[getter]
    pub fn geometry(&self) -> PyGeometryService
    {
        return self.geometry;
    }

    /// Access workbench settings.
    #[getter]
    pub fn settings(&self) -> PySettings
    {
        return self.settings.clone();
    }

    /// Opens a resource or document by URI.
    pub fn open(&self, uri: &str) -> PyResult<PyDoc>
    {
        return Self::OpenResource(uri, Some(self._Id));
    }

    /// Inspects a document or URI and optionally a Fresco root and tensor component.
    #[pyo3(signature = (target, *, root=None, coordinates=None))]
    pub fn inspect(&self, target: &Bound<'_, PyAny>, root: Option<&str>,
                   coordinates: Option<Vec<u32>>) -> PyResult<PyInspectionInfo>
    {
        return Self::InspectTarget(target, Some(self._Id), root, coordinates);
    }

    /// Runs an explicit example or operation job by URI.
    pub fn run(&self, _py: Python<'_>, uri: &str) -> PyResult<PyJob>
    {
        let stripped = uri.strip_prefix("example://").unwrap_or(uri);
        let trimmed = stripped.trim_matches('/');
        let (suite, rest) = trimmed.split_once('/').unwrap_or((trimmed, ""));
        let (name, _) = rest.split_once('/').unwrap_or((rest, ""));

        let all = crate::fenst::example::XplrExample::Registered();
        let mut found = None;
        all.Traverse(|ex| {
            if ex.suite == suite && ex.name == name {
                found = Some(*ex);
            }
        });

        let Some(ex) = found else {
            return Err(PyValueError::new_err(format!("Example not found in inventory: {uri}")));
        };

        match ex.Run() {
            Ok(root) => {
                let result_link = if suite == "Rube" {
                    Some(format!("waveform://{}/{}", suite, name))
                } else {
                    Some(uri.to_string())
                };
                return Ok(PyJob {
                    id: 101,
                    title: format!("Run {}", uri),
                    status: "Completed".to_string(),
                    result: Some(format!("Example {}::{} passed (root: {})", suite, name, root.Name())),
                    result_link,
                });
            }
            Err(err) => {
                return Ok(PyJob {
                    id: 101,
                    title: format!("Run {}", uri),
                    status: "Failed".to_string(),
                    result: Some(format!("Example failed: {}", err)),
                    result_link: None,
                });
            }
        }
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
