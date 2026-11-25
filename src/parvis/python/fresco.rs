// fresco.rs ---------------------------------------------------------------------------------------
//! Python bindings for Fresco symbolic expressions, tensors, NumPy array exchange,
//! and weighted nonlinear least-squares solving.

use crate::fresco::{
    DerivativeBackend, Differentiate, EvalLimits, ExprRepos, LeastSquaresLimits,
    LeastSquaresOptions, LeastSquaresProblem, LeastSquaresResult, LeastSquaresSpec,
    LeastSquaresStatus, TensorExpr, TensorShape,
};
use crate::silo::{Arr, Stash};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

//-------------------------------------------------------------------------------------------------

static NEXT_CONTEXT_ID: AtomicU64 = AtomicU64::new(1);

fn AllocateContextId() -> u64
{
    return NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed);
}

fn ShapeFromSeq(shape: Vec<u32>) -> PyResult<TensorShape>
{
    if shape.is_empty()
    {
        return Ok(TensorShape::Scalar());
    }
    return TensorShape::FromArr(Arr::from(&shape[..]))
        .map_err(|e| PyValueError::new_err(e.to_string()));
}

fn ShapeToTuple<'py>(py: Python<'py>, shape: &TensorShape) -> Bound<'py, PyTuple>
{
    let rank = shape.Rank() as usize;
    let mut dims = Vec::with_capacity(rank);
    let mut i = 0u32;
    while i < shape.Rank()
    {
        dims.push(shape.Dim(i));
        i += 1;
    }
    return PyTuple::new(py, dims).unwrap();
}

fn BindingsFromDict(bindings: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<(String, f64)>>
{
    let mut out = Vec::new();
    if let Some(dict) = bindings
    {
        for (k, v) in dict.iter()
        {
            let name: String = k.extract()?;
            let val: f64 = v.extract()?;
            if !val.is_finite()
            {
                return Err(PyValueError::new_err(format!(
                    "Binding for '{}' must be finite",
                    name
                )));
            }
            out.push((name, val));
        }
    }
    return Ok(out);
}

fn StatusToStr(status: &LeastSquaresStatus) -> (&'static str, bool, Option<String>)
{
    return match status
    {
        LeastSquaresStatus::ResidualSatisfied => ("ResidualSatisfied", true, None),
        LeastSquaresStatus::Stationary => ("Stationary", true, None),
        LeastSquaresStatus::StepToleranceReached => ("StepToleranceReached", true, None),
        LeastSquaresStatus::AcceptedProgressBudgetExhausted => {
            ("AcceptedProgressBudgetExhausted", false, None)
        }
        LeastSquaresStatus::WorkExhausted => ("WorkExhausted", false, None),
        LeastSquaresStatus::DampingExhausted => ("DampingExhausted", false, None),
        LeastSquaresStatus::DerivativeFailure(msg) => {
            ("DerivativeFailure", false, Some(msg.clone()))
        }
        LeastSquaresStatus::NumericalFailure(msg) => {
            ("NumericalFailure", false, Some(msg.clone()))
        }
    };
}

//-------------------------------------------------------------------------------------------------

/// Scoped Fresco symbolic repository owning expressions, variables, and compiled/solver state.
#[pyclass(name = "FrescoContext", unsendable)]
pub struct PyFrescoContext
{
    _Id:    u64,
    _Repos: Rc<RefCell<ExprRepos>>,
}

impl Default for PyFrescoContext
{
    fn default() -> Self
    {
        return Self::new();
    }
}

#[pymethods]
impl PyFrescoContext
{
    #[new]
    pub fn new() -> Self
    {
        return Self {
            _Id:    AllocateContextId(),
            _Repos: Rc::new(RefCell::new(ExprRepos::New())),
        };
    }

    #[getter]
    pub fn id(&self) -> u64
    {
        return self._Id;
    }

    #[getter]
    pub fn expr_count(&self) -> u32
    {
        return self._Repos.borrow().Size();
    }

    #[getter]
    pub fn var_count(&self) -> u32
    {
        return self._Repos.borrow().SzVar();
    }

    #[pyo3(signature = (name, reuse=true))]
    pub fn var(&self, name: &str, reuse: bool) -> u32
    {
        return self._Repos.borrow_mut().VarCreate(name.to_string(), reuse);
    }

    pub fn real(&self, value: f64) -> u32
    {
        return self._Repos.borrow_mut().RealCreate(value);
    }

    pub fn parse(&self, text: &str) -> PyResult<u32>
    {
        return self
            ._Repos
            .borrow_mut()
            .ParseExpr(text)
            .map_err(PyValueError::new_err);
    }

    pub fn differentiate(&self, expr_id: u32, var_name: &str) -> PyResult<u32>
    {
        let mut repos = self._Repos.borrow_mut();
        let varId = repos.FindVar(var_name);
        if varId == u32::MAX
        {
            return Err(PyValueError::new_err(format!(
                "Unknown variable '{}'",
                var_name
            )));
        }
        return Differentiate(&mut repos, expr_id, varId).map_err(PyValueError::new_err);
    }

    #[pyo3(signature = (expr_id, bindings=None))]
    pub fn evaluate(&self, expr_id: u32, bindings: Option<&Bound<'_, PyDict>>) -> PyResult<f64>
    {
        let pairs = BindingsFromDict(bindings)?;
        let borrowed: Vec<(&str, f64)> = pairs.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let repos = self._Repos.borrow();
        let outcome = repos
            .EvaluateNamedWithLimits(expr_id, Arr::from(&borrowed[..]), &EvalLimits::default())
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(outcome._Value);
    }

    pub fn vector_var(&self, name: &str, length: u32) -> PyResult<PyTensor>
    {
        let mut repos = self._Repos.borrow_mut();
        let varId = repos
            .VectorVarCreate(name, length)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let tensor = TensorExpr::FromVar(&mut repos, varId)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(PyTensor {
            _ContextId: self._Id,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     tensor,
        });
    }

    pub fn matrix_var(&self, name: &str, rows: u32, cols: u32) -> PyResult<PyTensor>
    {
        let mut repos = self._Repos.borrow_mut();
        let varId = repos
            .MatrixVarCreate(name, rows, cols)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let tensor = TensorExpr::FromVar(&mut repos, varId)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(PyTensor {
            _ContextId: self._Id,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     tensor,
        });
    }

    pub fn tensor_zeros(&self, shape: Vec<u32>) -> PyResult<PyTensor>
    {
        let tShape = ShapeFromSeq(shape)?;
        let mut repos = self._Repos.borrow_mut();
        let tensor = TensorExpr::Zeros(&mut repos, tShape);
        return Ok(PyTensor {
            _ContextId: self._Id,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     tensor,
        });
    }

    pub fn tensor_from_list(&self, shape: Vec<u32>, values: Vec<f64>) -> PyResult<PyTensor>
    {
        let tShape = ShapeFromSeq(shape)?;
        let mut repos = self._Repos.borrow_mut();
        let tensor = TensorExpr::FromRealsSlice(&mut repos, tShape, &values[..])
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(PyTensor {
            _ContextId: self._Id,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     tensor,
        });
    }

    /// Creates an owned symbolic `Tensor` by copying elements from a NumPy array (`float64`).
    ///
    /// Copy/ownership semantics: values are copied into `FrescoContext` constant nodes, so
    /// subsequent mutations to the source NumPy array do not affect the `Tensor`.
    pub fn tensor_from_numpy(&self, py: Python<'_>, array: &Bound<'_, PyAny>) -> PyResult<PyTensor>
    {
        let np = py
            .import("numpy")
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to import numpy: {}", e)))?;
        let arr = np
            .call_method1("ascontiguousarray", (array, np.getattr("float64")?))
            .map_err(|e| PyValueError::new_err(format!("Invalid array for tensor: {}", e)))?;
        let shapeTuple: Vec<u32> = arr.getattr("shape")?.extract()?;
        let flatList: Vec<f64> = arr.call_method0("ravel")?.call_method0("tolist")?.extract()?;
        let tShape = ShapeFromSeq(shapeTuple)?;
        let mut repos = self._Repos.borrow_mut();
        let tensor = TensorExpr::FromRealsSlice(&mut repos, tShape, &flatList[..])
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(PyTensor {
            _ContextId: self._Id,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     tensor,
        });
    }

    #[pyo3(signature = (
        residuals,
        active_vars,
        *,
        fixed_bindings=None,
        var_scales=None,
        residual_scales=None,
        backend="symbolic",
        max_iterations=100,
        residual_tolerance=1e-10,
        gradient_tolerance=1e-10
    ))]
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_least_squares(
        &self,
        residuals: Vec<u32>,
        active_vars: Vec<String>,
        fixed_bindings: Option<&Bound<'_, PyDict>>,
        var_scales: Option<Vec<f64>>,
        residual_scales: Option<Vec<f64>>,
        backend: &str,
        max_iterations: u32,
        residual_tolerance: f64,
        gradient_tolerance: f64,
    ) -> PyResult<PyLeastSquaresSolver>
    {
        let derivBackend = match backend
        {
            "symbolic" => DerivativeBackend::Symbolic,
            "forward_ad" => DerivativeBackend::ForwardAd,
            "reverse_ad" => DerivativeBackend::ReverseAd,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown derivative backend '{}'; expected 'symbolic', 'forward_ad', or 'reverse_ad'",
                    other
                )));
            }
        };

        let fixedPairs = BindingsFromDict(fixed_bindings)?;
        let mut fixedStash = Stash::<(String, f64)>::WithCapacity(fixedPairs.len() as u32);
        for pair in fixedPairs
        {
            fixedStash.Push(pair);
        }

        let mut resStash = Stash::<u32>::WithCapacity(residuals.len() as u32);
        for r in residuals
        {
            resStash.Push(r);
        }

        let mut varStash = Stash::<String>::WithCapacity(active_vars.len() as u32);
        for v in active_vars
        {
            varStash.Push(v);
        }

        let varScalesBuff = var_scales.map(|vs| {
            let mut st = Stash::<f64>::WithCapacity(vs.len() as u32);
            for s in vs
            {
                st.Push(s);
            }
            st.IntoBuff()
        });
        let resScalesBuff = residual_scales.map(|rs| {
            let mut st = Stash::<f64>::WithCapacity(rs.len() as u32);
            for s in rs
            {
                st.Push(s);
            }
            st.IntoBuff()
        });

        let spec = LeastSquaresSpec {
            _ResidualRoots:  resStash.IntoBuff(),
            _ActiveVars:     varStash.IntoBuff(),
            _FixedBindings:  fixedStash.IntoBuff(),
            _VarScales:      varScalesBuff,
            _ResidualScales: resScalesBuff,
            _Options:        LeastSquaresOptions {
                _MaxIterations:     max_iterations,
                _ResidualTolerance: residual_tolerance,
                _GradientTolerance: gradient_tolerance,
                _DerivativeBackend: derivBackend,
                ..LeastSquaresOptions::default()
            },
            _Limits:         LeastSquaresLimits::default(),
        };

        let mut repos = self._Repos.borrow_mut();
        let problem = LeastSquaresProblem::Prepare(&mut repos, spec)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        return Ok(PyLeastSquaresSolver {
            _ContextId: self._Id,
            _Repos:     Rc::clone(&self._Repos),
            _Problem:   problem,
        });
    }

    fn __repr__(&self) -> String
    {
        let repos = self._Repos.borrow();
        return format!(
            "<FrescoContext id={} exprs={} vars={}>",
            self._Id,
            repos.Size(),
            repos.SzVar()
        );
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}

//-------------------------------------------------------------------------------------------------

/// Symbolic tensor expression scoped to a `FrescoContext`.
#[pyclass(name = "Tensor", unsendable)]
#[derive(Clone)]
pub struct PyTensor
{
    _ContextId: u64,
    _Repos:     Rc<RefCell<ExprRepos>>,
    _Inner:     TensorExpr,
}

impl PyTensor
{
    fn CheckSameContext(&self, other: &Self) -> PyResult<()>
    {
        if self._ContextId != other._ContextId
        {
            return Err(PyValueError::new_err(
                "Tensor handles belong to different FrescoContext instances",
            ));
        }
        return Ok(());
    }
}

#[pymethods]
impl PyTensor
{
    #[getter]
    pub fn context_id(&self) -> u64
    {
        return self._ContextId;
    }

    #[getter]
    pub fn shape<'py>(&self, py: Python<'py>) -> Bound<'py, PyTuple>
    {
        return ShapeToTuple(py, self._Inner.Shape());
    }

    #[getter]
    pub fn rank(&self) -> u32
    {
        return self._Inner.Rank();
    }

    #[getter]
    pub fn total_elements(&self) -> u32
    {
        return self._Inner.TotalElements();
    }

    pub fn elem(&self, indices: Vec<u32>) -> PyResult<u32>
    {
        return self
            ._Inner
            .ElemSlice(&indices[..])
            .map_err(|e| PyValueError::new_err(e.to_string()));
    }

    pub fn subtensor(&self, i: u32) -> PyResult<Self>
    {
        let sub = self
            ._Inner
            .SubTensor(i)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     sub,
        });
    }

    pub fn row(&self, r: u32) -> PyResult<Self>
    {
        let res = self
            ._Inner
            .Row(r)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    pub fn col(&self, c: u32) -> PyResult<Self>
    {
        let res = self
            ._Inner
            .Col(c)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    pub fn transpose(&self) -> PyResult<Self>
    {
        let res = self
            ._Inner
            .Transpose()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    pub fn reshape(&self, shape: Vec<u32>) -> PyResult<Self>
    {
        let tShape = ShapeFromSeq(shape)?;
        let res = self
            ._Inner
            .Reshape(tShape)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    pub fn flatten(&self) -> PyResult<Self>
    {
        let res = self
            ._Inner
            .Flatten()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    pub fn dot(&self, other: &Self) -> PyResult<u32>
    {
        self.CheckSameContext(other)?;
        let mut repos = self._Repos.borrow_mut();
        return TensorExpr::VectorDot(&mut repos, &self._Inner, &other._Inner)
            .map_err(|e| PyValueError::new_err(e.to_string()));
    }

    pub fn outer(&self, other: &Self) -> PyResult<Self>
    {
        self.CheckSameContext(other)?;
        let mut repos = self._Repos.borrow_mut();
        let res = TensorExpr::OuterProduct(&mut repos, &self._Inner, &other._Inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    pub fn scale(&self, scalar: f64) -> Self
    {
        let mut repos = self._Repos.borrow_mut();
        let sExpr = repos.RealCreate(scalar);
        let res = TensorExpr::TensorScale(&mut repos, &self._Inner, sExpr);
        return Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        };
    }

    pub fn __add__(&self, other: &Self) -> PyResult<Self>
    {
        self.CheckSameContext(other)?;
        let mut repos = self._Repos.borrow_mut();
        let res = TensorExpr::TensorAdd(&mut repos, &self._Inner, &other._Inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    pub fn __sub__(&self, other: &Self) -> PyResult<Self>
    {
        self.CheckSameContext(other)?;
        let mut repos = self._Repos.borrow_mut();
        let res = TensorExpr::TensorSub(&mut repos, &self._Inner, &other._Inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    pub fn __mul__(&self, other: &Self) -> PyResult<Self>
    {
        self.CheckSameContext(other)?;
        let mut repos = self._Repos.borrow_mut();
        let res = TensorExpr::TensorHadamard(&mut repos, &self._Inner, &other._Inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    pub fn __truediv__(&self, other: &Self) -> PyResult<Self>
    {
        self.CheckSameContext(other)?;
        let mut repos = self._Repos.borrow_mut();
        let res = TensorExpr::TensorDiv(&mut repos, &self._Inner, &other._Inner)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    pub fn __matmul__(&self, other: &Self) -> PyResult<Self>
    {
        self.CheckSameContext(other)?;
        let mut repos = self._Repos.borrow_mut();
        let res = match (self._Inner.Rank(), other._Inner.Rank())
        {
            (2, 2) => TensorExpr::MatMul(&mut repos, &self._Inner, &other._Inner),
            (2, 1) => TensorExpr::MatVecMul(&mut repos, &self._Inner, &other._Inner),
            (1, 1) => TensorExpr::VectorDot(&mut repos, &self._Inner, &other._Inner)
                .map(TensorExpr::FromScalar),
            (rA, _rB) => Err(crate::fresco::TensorError::RankMismatch {
                expected: 2,
                actual:   rA,
            }),
        }
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Self {
            _ContextId: self._ContextId,
            _Repos:     Rc::clone(&self._Repos),
            _Inner:     res,
        });
    }

    #[pyo3(signature = (bindings=None))]
    pub fn evaluate(&self, bindings: Option<&Bound<'_, PyDict>>) -> PyResult<Vec<f64>>
    {
        let pairs = BindingsFromDict(bindings)?;
        let borrowed: Vec<(&str, f64)> = pairs.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let repos = self._Repos.borrow();
        let outcome = repos
            .EvaluateTensorNamedWithLimits(
                &self._Inner,
                Arr::from(&borrowed[..]),
                &EvalLimits::default(),
            )
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let mut out = Vec::with_capacity(outcome._Value.Size() as usize);
        let mut i = 0u32;
        while i < outcome._Value.Size()
        {
            out.push(outcome._Value[i]);
            i += 1;
        }
        return Ok(out);
    }

    /// Evaluates the tensor and returns a newly allocated NumPy `ndarray` (`dtype=float64`).
    ///
    /// Copy/ownership semantics: returns an independent NumPy copy; mutating the returned
    /// array does not affect the `Tensor` or `FrescoContext`.
    #[pyo3(signature = (bindings=None))]
    pub fn to_numpy(
        &self,
        py: Python<'_>,
        bindings: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>>
    {
        let flat = self.evaluate(bindings)?;
        let np = py
            .import("numpy")
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to import numpy: {}", e)))?;
        let pyList = PyList::new(py, flat)?;
        let arr = np.call_method1("array", (pyList, np.getattr("float64")?))?;
        let shapeTuple = ShapeToTuple(py, self._Inner.Shape());
        let shaped = arr.call_method1("reshape", (shapeTuple,))?;
        return Ok(shaped.unbind());
    }

    fn __repr__(&self) -> String
    {
        let shape = self._Inner.Shape();
        let mut dims = Vec::with_capacity(shape.Rank() as usize);
        let mut i = 0u32;
        while i < shape.Rank()
        {
            dims.push(shape.Dim(i).to_string());
            i += 1;
        }
        return format!(
            "<Tensor context={} shape=({})>",
            self._ContextId,
            dims.join(", ")
        );
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}

//-------------------------------------------------------------------------------------------------

/// Structured outcome and diagnostic counters from a `LeastSquaresSolver.solve()` call.
#[pyclass(name = "LeastSquaresDiagnostics")]
#[derive(Clone)]
pub struct PyLeastSquaresDiagnostics
{
    #[pyo3(get)]
    pub status:              String,
    #[pyo3(get)]
    pub converged:           bool,
    #[pyo3(get)]
    pub message:             Option<String>,
    #[pyo3(get)]
    pub best_x:              Vec<f64>,
    #[pyo3(get)]
    pub raw_residuals:       Vec<f64>,
    #[pyo3(get)]
    pub scaled_residuals:    Vec<f64>,
    #[pyo3(get)]
    pub objective:           f64,
    #[pyo3(get)]
    pub gradient_norm_inf:   f64,
    #[pyo3(get)]
    pub final_lambda:        f64,
    #[pyo3(get)]
    pub accepted_iterations: u32,
    #[pyo3(get)]
    pub trial_attempts:      u32,
    #[pyo3(get)]
    pub rejected_trials:     u32,
    #[pyo3(get)]
    pub residual_evals:      u32,
    #[pyo3(get)]
    pub jacobian_evals:      u32,
    #[pyo3(get)]
    pub damping_retries:     u32,
}

impl PyLeastSquaresDiagnostics
{
    fn FromResult(res: LeastSquaresResult) -> Self
    {
        let (statusStr, converged, msg) = StatusToStr(&res._Status);
        let mut bestX = Vec::with_capacity(res._BestX.Size() as usize);
        let mut i = 0u32;
        while i < res._BestX.Size()
        {
            bestX.push(res._BestX[i]);
            i += 1;
        }
        let mut rawRes = Vec::with_capacity(res._RawResiduals.Size() as usize);
        i = 0;
        while i < res._RawResiduals.Size()
        {
            rawRes.push(res._RawResiduals[i]);
            i += 1;
        }
        let mut scaledRes = Vec::with_capacity(res._ScaledResiduals.Size() as usize);
        i = 0;
        while i < res._ScaledResiduals.Size()
        {
            scaledRes.push(res._ScaledResiduals[i]);
            i += 1;
        }
        return Self {
            status:              statusStr.to_string(),
            converged,
            message:             msg,
            best_x:              bestX,
            raw_residuals:       rawRes,
            scaled_residuals:    scaledRes,
            objective:           res._Objective,
            gradient_norm_inf:   res._GradientNormInf,
            final_lambda:        res._FinalLambda,
            accepted_iterations: res._Counters._AcceptedIterations,
            trial_attempts:      res._Counters._TrialAttempts,
            rejected_trials:     res._Counters._RejectedTrials,
            residual_evals:      res._Counters._ResidualEvals,
            jacobian_evals:      res._Counters._JacobianEvals,
            damping_retries:     res._Counters._DampingRetries,
        };
    }
}

#[pymethods]
impl PyLeastSquaresDiagnostics
{
    /// Returns a newly allocated 1D NumPy `ndarray` (`dtype=float64`) copy of `best_x`.
    pub fn best_x_numpy(&self, py: Python<'_>) -> PyResult<Py<PyAny>>
    {
        let np = py
            .import("numpy")
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to import numpy: {}", e)))?;
        let pyList = PyList::new(py, &self.best_x)?;
        let arr = np.call_method1("array", (pyList, np.getattr("float64")?))?;
        return Ok(arr.unbind());
    }

    fn __repr__(&self) -> String
    {
        return format!(
            "<LeastSquaresDiagnostics status='{}' objective={:.6e} iters={}>",
            self.status, self.objective, self.accepted_iterations
        );
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}

//-------------------------------------------------------------------------------------------------

/// Prepared weighted nonlinear least-squares solver scoped to a `FrescoContext`.
#[pyclass(name = "LeastSquaresSolver", unsendable)]
pub struct PyLeastSquaresSolver
{
    _ContextId: u64,
    _Repos:     Rc<RefCell<ExprRepos>>,
    _Problem:   LeastSquaresProblem,
}

#[pymethods]
impl PyLeastSquaresSolver
{
    #[getter]
    pub fn context_id(&self) -> u64
    {
        return self._ContextId;
    }

    #[getter]
    pub fn residual_count(&self) -> u32
    {
        return self._Problem.ResidualCount();
    }

    #[getter]
    pub fn variable_count(&self) -> u32
    {
        return self._Problem.VariableCount();
    }

    pub fn evaluate_residuals(&mut self, x: Vec<f64>) -> PyResult<(f64, Vec<f64>, Vec<f64>)>
    {
        let repos = self._Repos.borrow();
        let (phi, _) = self
            ._Problem
            .EvaluateResiduals(&repos, Arr::from(&x[..]))
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let raw = self._Problem.RawResiduals();
        let scaled = self._Problem.ScaledResiduals();
        let mut rawOut = Vec::with_capacity(raw.Size() as usize);
        let mut scaledOut = Vec::with_capacity(scaled.Size() as usize);
        let mut i = 0u32;
        while i < raw.Size()
        {
            rawOut.push(raw[i]);
            scaledOut.push(scaled[i]);
            i += 1;
        }
        return Ok((phi, rawOut, scaledOut));
    }

    pub fn evaluate_jacobian(&mut self, x: Vec<f64>) -> PyResult<Vec<Vec<f64>>>
    {
        let repos = self._Repos.borrow();
        self._Problem
            .EvaluateJacobian(&repos, Arr::from(&x[..]))
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let m = self._Problem.ResidualCount();
        let n = self._Problem.VariableCount();
        let jac = self._Problem.ScaledJacobian();
        let mut out = Vec::with_capacity(m as usize);
        let mut r = 0u32;
        while r < m
        {
            let mut row = Vec::with_capacity(n as usize);
            let mut c = 0u32;
            while c < n
            {
                row.push(jac[r * n + c]);
                c += 1;
            }
            out.push(row);
            r += 1;
        }
        return Ok(out);
    }

    pub fn solve(&mut self, initial_x: Vec<f64>) -> PyLeastSquaresDiagnostics
    {
        let repos = self._Repos.borrow();
        let res = self._Problem.Solve(&repos, Arr::from(&initial_x[..]));
        return PyLeastSquaresDiagnostics::FromResult(res);
    }

    fn __repr__(&self) -> String
    {
        return format!(
            "<LeastSquaresSolver context={} residuals={} vars={}>",
            self._ContextId,
            self._Problem.ResidualCount(),
            self._Problem.VariableCount()
        );
    }

    fn __str__(&self) -> String
    {
        return self.__repr__();
    }
}

//-------------------------------------------------------------------------------------------------
