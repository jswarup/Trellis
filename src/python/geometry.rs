// geometry.rs ----------------------------------------------------------------------------------

//! # Python Geometry Binding
//!
//! Provides `GeometryAsset` and `GeometryService` bindings for manipulating
//! 3D meshes and point clouds from Python.

use std::path::Path;
use pyo3::prelude::*;
use pyo3::exceptions::{PyFileNotFoundError, PyValueError};
use crate::fleck::geometry::GeometryAsset;
use crate::fleck::{ParsePts, ParseWaveObj};
use crate::silo::IArr;

//-------------------------------------------------------------------------------------------------

/// Represents a validated, normalized 3D geometry asset (mesh or point cloud).
#[pyclass(name = "GeometryAsset")]
pub struct PyGeometryAsset
{
    pub(crate) inner: GeometryAsset,
}

impl PyGeometryAsset
{
    pub fn new( inner: GeometryAsset) -> Self
    {
        return Self { inner };
    }
}

#[pymethods]
impl PyGeometryAsset
{
    /// Total number of vertices in the asset.
    #[getter]
    pub fn vertex_count(&self) -> u32
    {
        return self.inner.VertexCount();
    }

    /// Total number of polygonal/triangular faces in the asset.
    #[getter]
    pub fn face_count(&self) -> u32
    {
        return self.inner.FaceCount();
    }

    /// Total number of points in the asset.
    #[getter]
    pub fn point_count(&self) -> u32
    {
        return self.inner.PointCount();
    }

    /// True if the asset is a point cloud, false if it is a polygonal mesh.
    #[getter]
    pub fn is_point_cloud(&self) -> bool
    {
        return self.inner.IsPointCloud();
    }

    /// Axis-aligned bounding box of the geometry before normalization: ((min_x, min_y, min_z), (max_x, max_y, max_z)).
    #[getter]
    pub fn bounds(&self) -> ([f32; 3], [f32; 3])
    {
        return self.inner.Bounds();
    }

    /// Returns a list of vertex 3D positions in local normalized coordinates [-1.0, 1.0].
    pub fn vertex_positions(&self) -> Vec<[f32; 3]>
    {
        let vertices = self.inner.Vertices();
        let mut list = Vec::with_capacity( vertices.Size() as usize);
        vertices.Traverse(|v| {
            list.push( v.Position());
        });
        return list;
    }

    /// Returns a list of vertex colors as RGBA floats [0.0, 1.0].
    pub fn vertex_colors(&self) -> Vec<[f32; 4]>
    {
        let vertices = self.inner.Vertices();
        let mut list = Vec::with_capacity( vertices.Size() as usize);
        vertices.Traverse(|v| {
            list.push( v.Color());
        });
        return list;
    }

    /// Returns a list of triangular face vertex indices `[i0, i1, i2]`.
    pub fn faces(&self) -> Vec<[u32; 3]>
    {
        let triangles = self.inner.Triangles();
        let mut list = Vec::with_capacity( triangles.Size() as usize);
        triangles.Traverse(|t| {
            list.push( *t);
        });
        return list;
    }

    fn __repr__(&self) -> String
    {
        if self.inner.IsPointCloud()
        {
            return format!(
                "<GeometryAsset type=point_cloud points={}>",
                self.inner.PointCount()
            );
        }
        else
        {
            return format!(
                "<GeometryAsset type=mesh vertices={} faces={}>",
                self.inner.VertexCount(),
                self.inner.FaceCount()
            );
        }
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}

//-------------------------------------------------------------------------------------------------

/// Service for loading and parsing 3D geometry assets from Python.
#[pyclass(name = "GeometryService")]
#[derive(Clone, Copy, Default)]
pub struct PyGeometryService;

#[pymethods]
impl PyGeometryService
{
    #[new]
    pub fn new() -> Self
    {
        return Self;
    }

    /// Parses Wavefront OBJ content string into a `GeometryAsset`.
    pub fn parse_obj(&self, py: Python<'_>, data: &str) -> PyResult<PyGeometryAsset>
    {
        let data_str = data.to_string();
        let result: Result<GeometryAsset, String> = py.allow_threads(move || {
            let model = ParseWaveObj( &data_str).map_err(|e| e)?;
            let asset = GeometryAsset::FromObj( model).map_err(|e| e)?;
            return Ok( asset);
        });

        match result
        {
            Ok( asset) => return Ok( PyGeometryAsset::new( asset)),
            Err( err) => return Err( PyValueError::new_err( format!( "OBJ parse error: {}", err))),
        }
    }

    /// Parses PTS point cloud content string into a `GeometryAsset`.
    pub fn parse_pts(&self, py: Python<'_>, data: &str) -> PyResult<PyGeometryAsset>
    {
        let data_str = data.to_string();
        let result: Result<GeometryAsset, String> = py.allow_threads(move || {
            let cloud = ParsePts( &data_str).map_err(|e| e)?;
            let asset = GeometryAsset::FromPts( cloud).map_err(|e| e)?;
            return Ok( asset);
        });

        match result
        {
            Ok( asset) => return Ok( PyGeometryAsset::new( asset)),
            Err( err) => return Err( PyValueError::new_err( format!( "PTS parse error: {}", err))),
        }
    }

    /// Loads a 3D geometry file (.obj or .pts) from the given path.
    pub fn load(&self, py: Python<'_>, path: &str) -> PyResult<PyGeometryAsset>
    {
        let path_obj = Path::new( path);
        if !path_obj.exists()
        {
            return Err( PyFileNotFoundError::new_err( format!( "File not found: {}", path)));
        }

        let extension = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let path_buf = path_obj.to_path_buf();
        let content = py.allow_threads(move || {
            return std::fs::read_to_string( &path_buf);
        }).map_err(|e| PyValueError::new_err( format!( "Failed to read file '{}': {}", path, e)))?;

        match extension.as_str()
        {
            "obj" => return self.parse_obj( py, &content),
            "pts" => return self.parse_pts( py, &content),
            other => return Err( PyValueError::new_err( format!(
                "Unsupported geometry format '.{}'. Supported formats: .obj, .pts",
                other
            ))),
        }
    }

    fn __repr__(&self) -> String
    {
        return "<GeometryService>".to_string();
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}
