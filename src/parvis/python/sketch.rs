//-- sketch.rs -------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::fenst::sketch::SketchDocument;
use	crate::fenst::sketch_constraint::ConstraintSolver;
use	crate::fleck::sketch_extrude::SketchExtrude;
use	crate::python::geometry::PyGeometryAsset;
use	pyo3::exceptions::PyValueError;
use	pyo3::prelude::*;

//--------------------------------------------------------------------------------------------------
/// Python wrapper around 2D CAD SketchDocument.
#[pyclass( name = "SketchDocument")]
pub struct PySketchDocument
{
    pub(crate) _Inner: SketchDocument,
}

impl Default for PySketchDocument {
    fn	default() -> Self
    {
        Self::new()
    }
}

#[pymethods]
impl PySketchDocument
{
    #[new]
    pub fn	new() -> Self
    {
        Self {
            _Inner: SketchDocument::New(),
        }
    }

    pub fn	add_line( &mut self, x1: f64, y1: f64, x2: f64, y2: f64) -> PyResult< u32>
    {
        self._Inner.AddLine( [x1, y1].into(), [x2, y2].into())
            .map_err( PyValueError::new_err)
    }

    pub fn	add_circle( &mut self, cx: f64, cy: f64, r: f64) -> PyResult< u32>
    {
        self._Inner.AddCircle( [cx, cy].into(), r)
            .map_err( PyValueError::new_err)
    }

    pub fn	add_arc(
        &mut self, cx: f64, cy: f64, r: f64, start: f64, sweep: f64,
    ) -> PyResult< u32>
    {
        self._Inner.AddArc( [cx, cy].into(), r, start, sweep)
            .map_err( PyValueError::new_err)
    }

    #[getter]
    pub fn	entity_count( &self) -> u32
    {
        self._Inner.Entities().Size()
    }

    pub fn	solve_constraints( &mut self, max_iterations: u32) -> PyResult< u32>
    {
        let  	( mut solver, mapping) = ConstraintSolver::ExtractFromDocument( &self._Inner);
        let  	result = solver.Solve( max_iterations, 1e-6);
        solver.ApplyToDocument( &mut self._Inner, &mapping);
        Ok( result._Iterations)
    }

    pub fn	extrude( &self, depth: f64) -> PyResult< PyGeometryAsset>
    {
        let  	asset = SketchExtrude::ExtrudeDocument( &self._Inner, depth)
            .map_err( PyValueError::new_err)?;
        Ok( PyGeometryAsset::new( asset))
    }
}
