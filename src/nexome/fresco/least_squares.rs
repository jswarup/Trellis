// least_squares.rs --------------------------------------------------------------------------------
//! Bounded weighted nonlinear least-squares preparation, symbolic Jacobian, and dense
//! Levenberg-Marquardt solver over Fresco scalar expressions.

use crate::fresco::{
    DiffLimits, DifferentiateManyWithLimits, EvalCounters, EvalError, EvalLimits, ExprReadError,
    ExprRepos, ExprView,
};
use crate::silo::{Arr, Buff, Stash};
use core::fmt;

//-------------------------------------------------------------------------------------------------

/// Resource limits governing least-squares preparation and evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeastSquaresLimits
{
    pub _MaxExprDepth:    u32,
    pub _MaxEvalNodes:    u32,
    pub _MaxEvalEdges:    u32,
    pub _MaxSharedVisits: u32,
    pub _MaxDiffDepth:    u32,
    pub _MaxDiffNodes:    u32,
}

impl Default for LeastSquaresLimits
{
    fn default() -> Self
    {
        Self {
            _MaxExprDepth:    ExprRepos::K_MAX_TREE_DEPTH,
            _MaxEvalNodes:    100_000,
            _MaxEvalEdges:    200_000,
            _MaxSharedVisits: 50_000,
            _MaxDiffDepth:    ExprRepos::K_MAX_TREE_DEPTH,
            _MaxDiffNodes:    100_000,
        }
    }
}

impl LeastSquaresLimits
{
    fn ToEvalLimits(&self) -> EvalLimits<'static>
    {
        EvalLimits {
            _MaxDepth:        self._MaxExprDepth,
            _MaxNodes:        self._MaxEvalNodes,
            _MaxEdges:        self._MaxEvalEdges,
            _MaxResults:      10_000,
            _MaxSharedVisits: self._MaxSharedVisits,
            _Deadline:        None,
            _CancelFlag:      None,
        }
    }

    fn ToDiffLimits(&self) -> DiffLimits
    {
        DiffLimits {
            _MaxDepth: self._MaxDiffDepth,
            _MaxNodes: self._MaxDiffNodes,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Configuration options for the Levenberg-Marquardt solver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeastSquaresOptions
{
    pub _MaxIterations:        u32,
    pub _MaxTrials:            u32,
    pub _MaxResidualEvals:     u32,
    pub _MaxJacobianEvals:     u32,
    pub _MaxDampingRetries:    u32,
    pub _ResidualTolerance:    f64,
    pub _GradientTolerance:    f64,
    pub _StepTolerance:        f64,
    pub _InitialLambda:        f64,
    pub _MinLambda:            f64,
    pub _MaxLambda:            f64,
    pub _LambdaUpFactor:       f64,
    pub _LambdaDownFactor:     f64,
    pub _AcceptRatioThreshold: f64,
}

impl Default for LeastSquaresOptions
{
    fn default() -> Self
    {
        Self {
            _MaxIterations:        100,
            _MaxTrials:            500,
            _MaxResidualEvals:     1_000,
            _MaxJacobianEvals:     200,
            _MaxDampingRetries:    20,
            _ResidualTolerance:    1e-10,
            _GradientTolerance:    1e-10,
            _StepTolerance:        1e-12,
            _InitialLambda:        1e-3,
            _MinLambda:            1e-12,
            _MaxLambda:            1e12,
            _LambdaUpFactor:       10.0,
            _LambdaDownFactor:     0.3,
            _AcceptRatioThreshold: 1e-4,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Errors returned while validating or preparing a least-squares problem.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeastSquaresPrepError
{
    EmptyResiduals,
    EmptyActiveVariables,
    DuplicateActiveVariable(String),
    UnknownVariable(String),
    NonScalarVariable(String),
    OverlappingActiveAndFixed(String),
    MissingBinding(String),
    NonFiniteConfiguration(String),
    InvalidScale(String),
    InvalidOptions(String),
    InvalidResidualExpr(u32),
    UnsupportedOperator(u32),
    DerivativePreparationFailed(String),
}

impl fmt::Display for LeastSquaresPrepError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::EmptyResiduals => write!(f, "Residual set cannot be empty"),
            Self::EmptyActiveVariables => write!(f, "Active variable set cannot be empty"),
            Self::DuplicateActiveVariable(name) => {
                write!(f, "Duplicate active variable '{}'", name)
            }
            Self::UnknownVariable(name) => write!(f, "Unknown variable '{}'", name),
            Self::NonScalarVariable(name) => {
                write!(f, "Least-squares variable '{}' must be scalar", name)
            }
            Self::OverlappingActiveAndFixed(name) => {
                write!(f, "Variable '{}' cannot be both active and fixed", name)
            }
            Self::MissingBinding(name) => {
                write!(f, "Missing fixed binding for variable '{}'", name)
            }
            Self::NonFiniteConfiguration(msg) => write!(f, "Non-finite configuration: {}", msg),
            Self::InvalidScale(msg) => write!(f, "Invalid scale: {}", msg),
            Self::InvalidOptions(msg) => write!(f, "Invalid solver options: {}", msg),
            Self::InvalidResidualExpr(id) => write!(f, "Invalid residual expression ID {}", id),
            Self::UnsupportedOperator(id) => {
                write!(f, "Unsupported operator in expression ID {}", id)
            }
            Self::DerivativePreparationFailed(msg) => {
                write!(f, "Derivative preparation failed: {}", msg)
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Distinct termination outcomes reported by the least-squares solver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeastSquaresStatus
{
    ResidualSatisfied,
    Stationary,
    StepToleranceReached,
    AcceptedProgressBudgetExhausted,
    WorkExhausted,
    DampingExhausted,
    DerivativeFailure(String),
    NumericalFailure(String),
}

//-------------------------------------------------------------------------------------------------

/// Diagnostic counters recorded across a least-squares solve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LeastSquaresCounters
{
    pub _AcceptedIterations: u32,
    pub _TrialAttempts:      u32,
    pub _RejectedTrials:     u32,
    pub _ResidualEvals:      u32,
    pub _JacobianEvals:      u32,
    pub _DampingRetries:     u32,
    pub _EvalWork:           EvalCounters,
}

//-------------------------------------------------------------------------------------------------

/// Final outcome and best finite accepted point of a least-squares solve.
#[derive(Clone, Debug)]
pub struct LeastSquaresResult
{
    pub _Status:          LeastSquaresStatus,
    pub _BestX:           Buff<f64>,
    pub _RawResiduals:    Buff<f64>,
    pub _ScaledResiduals: Buff<f64>,
    pub _Objective:       f64,
    pub _GradientNormInf: f64,
    pub _FinalLambda:     f64,
    pub _Counters:        LeastSquaresCounters,
}

//-------------------------------------------------------------------------------------------------

/// Input specification for preparing a weighted nonlinear least-squares problem.
pub struct LeastSquaresSpec
{
    pub _ResidualRoots:  Buff<u32>,
    pub _ActiveVars:     Buff<String>,
    pub _FixedBindings:  Buff<(String, f64)>,
    pub _VarScales:      Option<Buff<f64>>,
    pub _ResidualScales: Option<Buff<f64>>,
    pub _Options:        LeastSquaresOptions,
    pub _Limits:         LeastSquaresLimits,
}

//-------------------------------------------------------------------------------------------------

/// Prepared least-squares problem owning ordered residual/derivative roots, indexed bindings,
/// and reusable solver workspace.
pub struct LeastSquaresProblem
{
    _ResidualRoots:  Buff<u32>,
    _DerivRoots:     Buff<u32>,
    _ActiveVarIds:   Buff<u32>,
    _VarScales:      Buff<f64>,
    _ResidualScales: Buff<f64>,
    _Options:        LeastSquaresOptions,
    _Limits:         LeastSquaresLimits,
    _BindingsBuf:    Buff<f64>,
    _RawResiduals:   Buff<f64>,
    _ScaledRes:      Buff<f64>,
    _Jacobian:       Buff<f64>,
    _TrialX:         Buff<f64>,
    _BestX:          Buff<f64>,
    _Gradient:       Buff<f64>,
    _StepU:          Buff<f64>,
    _JStep:          Buff<f64>,
    _AugMat:         Buff<f64>,
    _AugRhs:         Buff<f64>,
}

//-------------------------------------------------------------------------------------------------

impl LeastSquaresProblem
{
    /// Validates configuration, resolves variable indices once, builds memoized symbolic
    /// derivatives transactionally, and allocates reusable workspace buffers.
    pub fn Prepare(
        repos: &mut ExprRepos,
        spec: LeastSquaresSpec,
    ) -> Result<Self, LeastSquaresPrepError>
    {
        let m = spec._ResidualRoots.Size();
        if m == 0
        {
            return Err(LeastSquaresPrepError::EmptyResiduals);
        }
        let n = spec._ActiveVars.Size();
        if n == 0
        {
            return Err(LeastSquaresPrepError::EmptyActiveVariables);
        }

        Self::ValidateOptions(&spec._Options)?;

        // Resolve active variable IDs and reject duplicates or non-scalars.
        let mut activeIds = Stash::<u32>::WithCapacity(n);
        let mut j = 0u32;
        while j < n
        {
            let name = &spec._ActiveVars[j];
            let varId = repos.FindVar(name);
            if varId == u32::MAX
            {
                return Err(LeastSquaresPrepError::UnknownVariable(name.clone()));
            }
            if !repos.VarAttrAt(varId).Shape().IsScalar()
            {
                return Err(LeastSquaresPrepError::NonScalarVariable(name.clone()));
            }
            let mut prev = 0u32;
            while prev < j
            {
                if activeIds[prev] == varId
                {
                    return Err(LeastSquaresPrepError::DuplicateActiveVariable(name.clone()));
                }
                prev += 1;
            }
            activeIds.Push(varId);
            j += 1;
        }
        let activeIdBuff = activeIds.IntoBuff();

        // Validate fixed bindings and build indexed binding buffer.
        let varCount = repos.SzVar();
        let mut bindingsBuf = Buff::<f64>::FromDispenser(varCount, |_| f64::NAN);
        let fixedCount = spec._FixedBindings.Size();
        let mut f = 0u32;
        while f < fixedCount
        {
            let (ref name, val) = spec._FixedBindings[f];
            if !val.is_finite()
            {
                return Err(LeastSquaresPrepError::NonFiniteConfiguration(format!(
                    "Fixed binding for '{}' is non-finite",
                    name
                )));
            }
            let varId = repos.FindVar(name);
            if varId == u32::MAX
            {
                return Err(LeastSquaresPrepError::UnknownVariable(name.clone()));
            }
            let mut k = 0u32;
            while k < n
            {
                if activeIdBuff[k] == varId
                {
                    return Err(LeastSquaresPrepError::OverlappingActiveAndFixed(
                        name.clone(),
                    ));
                }
                k += 1;
            }
            bindingsBuf[varId] = val;
            f += 1;
        }

        // Validate variable scales T and residual scales S.
        let varScales = match spec._VarScales
        {
            Some(vs) =>
            {
                if vs.Size() != n
                {
                    return Err(LeastSquaresPrepError::InvalidScale(format!(
                        "Expected {} variable scales, got {}",
                        n,
                        vs.Size()
                    )));
                }
                let mut k = 0u32;
                while k < n
                {
                    if !vs[k].is_finite() || vs[k] <= 0.0
                    {
                        return Err(LeastSquaresPrepError::InvalidScale(format!(
                            "Variable scale at index {} must be positive and finite",
                            k
                        )));
                    }
                    k += 1;
                }
                vs
            }
            None => Buff::FromDispenser(n, |_| 1.0),
        };

        let resScales = match spec._ResidualScales
        {
            Some(rs) =>
            {
                if rs.Size() != m
                {
                    return Err(LeastSquaresPrepError::InvalidScale(format!(
                        "Expected {} residual scales, got {}",
                        m,
                        rs.Size()
                    )));
                }
                let mut i = 0u32;
                while i < m
                {
                    if !rs[i].is_finite() || rs[i] <= 0.0
                    {
                        return Err(LeastSquaresPrepError::InvalidScale(format!(
                            "Residual scale at index {} must be positive and finite",
                            i
                        )));
                    }
                    i += 1;
                }
                rs
            }
            None => Buff::FromDispenser(m, |_| 1.0),
        };

        // Prevalidate all residual expression DAGs (IDs, supported differentiable operators,
        // and presence of fixed bindings for non-active referenced variables).
        let mut i = 0u32;
        while i < m
        {
            Self::ValidateResidualDag(
                repos,
                spec._ResidualRoots[i],
                &activeIdBuff,
                &bindingsBuf,
                &spec._Limits,
            )?;
            i += 1;
        }

        // LS2: Build memoized symbolic Jacobian roots inside an exact repository transaction.
        let diffLimits = spec._Limits.ToDiffLimits();
        let derivRoots = DifferentiateManyWithLimits(
            repos,
            spec._ResidualRoots.Arr(),
            activeIdBuff.Arr(),
            &diffLimits,
        )
        .map_err(LeastSquaresPrepError::DerivativePreparationFailed)?;

        return Ok(Self {
            _ResidualRoots:  spec._ResidualRoots,
            _DerivRoots:     derivRoots,
            _ActiveVarIds:   activeIdBuff,
            _VarScales:      varScales,
            _ResidualScales: resScales,
            _Options:        spec._Options,
            _Limits:         spec._Limits,
            _BindingsBuf:    bindingsBuf,
            _RawResiduals:   Buff::FromDispenser(m, |_| 0.0),
            _ScaledRes:      Buff::FromDispenser(m, |_| 0.0),
            _Jacobian:       Buff::FromDispenser(m * n, |_| 0.0),
            _TrialX:         Buff::FromDispenser(n, |_| 0.0),
            _BestX:          Buff::FromDispenser(n, |_| 0.0),
            _Gradient:       Buff::FromDispenser(n, |_| 0.0),
            _StepU:          Buff::FromDispenser(n, |_| 0.0),
            _JStep:          Buff::FromDispenser(m, |_| 0.0),
            _AugMat:         Buff::FromDispenser((m + n) * n, |_| 0.0),
            _AugRhs:         Buff::FromDispenser(m + n, |_| 0.0),
        });
    }

    #[inline]
    pub fn ResidualCount(&self) -> u32
    {
        return self._ResidualRoots.Size();
    }

    #[inline]
    pub fn VariableCount(&self) -> u32
    {
        return self._ActiveVarIds.Size();
    }

    #[inline]
    pub fn RawResiduals(&self) -> Arr<'_, f64>
    {
        return self._RawResiduals.Arr();
    }

    #[inline]
    pub fn ScaledResiduals(&self) -> Arr<'_, f64>
    {
        return self._ScaledRes.Arr();
    }

    #[inline]
    pub fn ScaledJacobian(&self) -> Arr<'_, f64>
    {
        return self._Jacobian.Arr();
    }

    /// LS1: Evaluates raw and scaled residuals and objective `Phi(x) = 0.5 * ||S r(x)||^2`
    /// into reusable workspace buffers without name lookup or buffer growth.
    pub fn EvaluateResiduals(
        &mut self,
        repos: &ExprRepos,
        x: Arr<'_, f64>,
    ) -> Result<(f64, EvalCounters), EvalError>
    {
        let n = self._ActiveVarIds.Size();
        if x.Size() != n
        {
            return Err(EvalError::DomainError(format!(
                "Expected {} active variables, got {}",
                n,
                x.Size()
            )));
        }
        let mut j = 0u32;
        while j < n
        {
            let val = x[j];
            if !val.is_finite()
            {
                return Err(EvalError::DomainError(format!(
                    "Active variable at index {} is non-finite",
                    j
                )));
            }
            self._TrialX[j] = val;
            j += 1;
        }
        return self.EvaluateTrialResiduals(repos);
    }

    fn EvaluateTrialResiduals(
        &mut self,
        repos: &ExprRepos,
    ) -> Result<(f64, EvalCounters), EvalError>
    {
        let n = self._ActiveVarIds.Size();
        let mut j = 0u32;
        while j < n
        {
            let val = self._TrialX[j];
            if !val.is_finite()
            {
                return Err(EvalError::DomainError(format!(
                    "Active variable at index {} is non-finite",
                    j
                )));
            }
            self._BindingsBuf[self._ActiveVarIds[j]] = val;
            j += 1;
        }

        let evalLimits = self._Limits.ToEvalLimits();
        let m = self._ResidualRoots.Size();
        let mut phi = 0.0f64;
        let mut totalCounters = EvalCounters::default();
        let mut i = 0u32;
        while i < m
        {
            let outcome = repos.EvaluateOutcomeWithLimits(
                self._ResidualRoots[i],
                self._BindingsBuf.Arr(),
                &evalLimits,
            );
            totalCounters._NodesVisited = totalCounters
                ._NodesVisited
                .saturating_add(outcome._Counters._NodesVisited);
            totalCounters._EdgesTraversed = totalCounters
                ._EdgesTraversed
                .saturating_add(outcome._Counters._EdgesTraversed);
            totalCounters._SharedVisits = totalCounters
                ._SharedVisits
                .saturating_add(outcome._Counters._SharedVisits);
            totalCounters._ResultsComputed = totalCounters
                ._ResultsComputed
                .saturating_add(outcome._Counters._ResultsComputed);
            totalCounters._PeakDepth = totalCounters._PeakDepth.max(outcome._Counters._PeakDepth);

            let raw = outcome._Value?;
            if !raw.is_finite()
            {
                return Err(EvalError::DomainError(format!(
                    "Residual {} evaluated to non-finite value {}",
                    i, raw
                )));
            }
            let scaled = self._ResidualScales[i] * raw;
            if !scaled.is_finite()
            {
                return Err(EvalError::DomainError(format!(
                    "Scaled residual {} is non-finite",
                    i
                )));
            }
            self._RawResiduals[i] = raw;
            self._ScaledRes[i] = scaled;
            phi += 0.5 * scaled * scaled;
            i += 1;
        }

        if !phi.is_finite()
        {
            return Err(EvalError::DomainError(
                "Least-squares objective overflowed to non-finite".to_string(),
            ));
        }
        return Ok((phi, totalCounters));
    }

    /// LS2: Evaluates the prepared symbolic Jacobian `J_s = S * J * diag(T)` into the
    /// reusable dense row-major workspace buffer without creating any expression nodes.
    pub fn EvaluateJacobian(
        &mut self,
        repos: &ExprRepos,
        x: Arr<'_, f64>,
    ) -> Result<EvalCounters, EvalError>
    {
        let n = self._ActiveVarIds.Size();
        if x.Size() != n
        {
            return Err(EvalError::DomainError(format!(
                "Expected {} active variables, got {}",
                n,
                x.Size()
            )));
        }
        let mut j = 0u32;
        while j < n
        {
            let val = x[j];
            if !val.is_finite()
            {
                return Err(EvalError::DomainError(format!(
                    "Active variable at index {} is non-finite",
                    j
                )));
            }
            self._BindingsBuf[self._ActiveVarIds[j]] = val;
            j += 1;
        }

        let evalLimits = self._Limits.ToEvalLimits();
        let m = self._ResidualRoots.Size();
        let mut totalCounters = EvalCounters::default();
        let mut i = 0u32;
        while i < m
        {
            let resScale = self._ResidualScales[i];
            let mut col = 0u32;
            while col < n
            {
                let derivId = self._DerivRoots[i * n + col];
                let outcome = repos.EvaluateOutcomeWithLimits(
                    derivId,
                    self._BindingsBuf.Arr(),
                    &evalLimits,
                );
                totalCounters._NodesVisited = totalCounters
                    ._NodesVisited
                    .saturating_add(outcome._Counters._NodesVisited);
                totalCounters._EdgesTraversed = totalCounters
                    ._EdgesTraversed
                    .saturating_add(outcome._Counters._EdgesTraversed);
                totalCounters._SharedVisits = totalCounters
                    ._SharedVisits
                    .saturating_add(outcome._Counters._SharedVisits);
                totalCounters._ResultsComputed = totalCounters
                    ._ResultsComputed
                    .saturating_add(outcome._Counters._ResultsComputed);
                totalCounters._PeakDepth =
                    totalCounters._PeakDepth.max(outcome._Counters._PeakDepth);

                let rawDeriv = outcome._Value?;
                if !rawDeriv.is_finite()
                {
                    return Err(EvalError::DomainError(format!(
                        "Jacobian ({}, {}) evaluated to non-finite value {}",
                        i, col, rawDeriv
                    )));
                }
                let scaled = resScale * rawDeriv * self._VarScales[col];
                if !scaled.is_finite()
                {
                    return Err(EvalError::DomainError(format!(
                        "Scaled Jacobian ({}, {}) is non-finite",
                        i, col
                    )));
                }
                self._Jacobian[i * n + col] = scaled;
                col += 1;
            }
            i += 1;
        }
        return Ok(totalCounters);
    }

    /// LS3: Solves the weighted nonlinear least-squares problem from `initialX` using
    /// dense Householder QR on the damped augmented Levenberg-Marquardt system.
    pub fn Solve(
        &mut self,
        repos: &ExprRepos,
        initialX: Arr<'_, f64>,
    ) -> LeastSquaresResult
    {
        let m = self._ResidualRoots.Size();
        let n = self._ActiveVarIds.Size();
        let mut counters = LeastSquaresCounters::default();
        let mut lambda = self._Options._InitialLambda;

        if initialX.Size() != n
        {
            return self.FinishResult(
                LeastSquaresStatus::NumericalFailure(format!(
                    "Initial guess has {} elements, expected {}",
                    initialX.Size(),
                    n
                )),
                f64::NAN,
                f64::NAN,
                lambda,
                counters,
            );
        }

        let mut curX = Buff::<f64>::FromDispenser(n, |j| initialX[j]);
        self._BestX.USeg().Traverse(|j| self._BestX[j] = curX[j]);

        // Initial residual evaluation
        let (mut curPhi, evalCnt) = match self.EvaluateResiduals(repos, curX.Arr())
        {
            Ok(res) => res,
            Err(err) =>
            {
                return self.FinishResult(
                    LeastSquaresStatus::NumericalFailure(err.to_string()),
                    f64::NAN,
                    f64::NAN,
                    lambda,
                    counters,
                );
            }
        };
        counters._ResidualEvals += 1;
        Self::AccumulateEvalWork(&mut counters._EvalWork, evalCnt);

        let mut bestRaw = self._RawResiduals.clone();
        let mut bestScaled = self._ScaledRes.clone();

        if self.MaxAbsScaledResidual() <= self._Options._ResidualTolerance
        {
            return LeastSquaresResult {
                _Status:          LeastSquaresStatus::ResidualSatisfied,
                _BestX:           self._BestX.clone(),
                _RawResiduals:    bestRaw,
                _ScaledResiduals: bestScaled,
                _Objective:       curPhi,
                _GradientNormInf: 0.0,
                _FinalLambda:     lambda,
                _Counters:        counters,
            };
        }

        // Initial Jacobian evaluation
        if counters._JacobianEvals >= self._Options._MaxJacobianEvals
        {
            return self.BuildAcceptedResult(
                LeastSquaresStatus::WorkExhausted,
                bestRaw,
                bestScaled,
                curPhi,
                f64::NAN,
                lambda,
                counters,
            );
        }
        let jacCnt = match self.EvaluateJacobian(repos, curX.Arr())
        {
            Ok(c) => c,
            Err(err) =>
            {
                return self.BuildAcceptedResult(
                    LeastSquaresStatus::DerivativeFailure(err.to_string()),
                    bestRaw,
                    bestScaled,
                    curPhi,
                    f64::NAN,
                    lambda,
                    counters,
                );
            }
        };
        counters._JacobianEvals += 1;
        Self::AccumulateEvalWork(&mut counters._EvalWork, jacCnt);

        let mut gradNormInf = self.ComputeGradientAndNorm();
        if gradNormInf <= self._Options._GradientTolerance
        {
            return self.BuildAcceptedResult(
                LeastSquaresStatus::Stationary,
                bestRaw,
                bestScaled,
                curPhi,
                gradNormInf,
                lambda,
                counters,
            );
        }

        loop
        {
            if counters._AcceptedIterations >= self._Options._MaxIterations
                || counters._TrialAttempts >= self._Options._MaxTrials
                || counters._ResidualEvals >= self._Options._MaxResidualEvals
            {
                let status = if counters._AcceptedIterations > 0
                {
                    LeastSquaresStatus::AcceptedProgressBudgetExhausted
                }
                else
                {
                    LeastSquaresStatus::WorkExhausted
                };
                return self.BuildAcceptedResult(
                    status,
                    bestRaw,
                    bestScaled,
                    curPhi,
                    gradNormInf,
                    lambda,
                    counters,
                );
            }

            // Solve damped augmented system via Householder QR: [J_s; sqrt(lambda) I] du = [-q; 0]
            if let Err(msg) = self.SolveDampedQr(lambda)
            {
                return self.BuildAcceptedResult(
                    LeastSquaresStatus::NumericalFailure(msg),
                    bestRaw,
                    bestScaled,
                    curPhi,
                    gradNormInf,
                    lambda,
                    counters,
                );
            }

            // Predicted linear-model reduction: 0.5 * ||J_s du||^2 + lambda * ||du||^2
            let predReduction = self.ComputePredictedReduction(lambda);
            let mut maxDxInf = 0.0f64;
            let mut j = 0u32;
            while j < n
            {
                let dx = self._VarScales[j] * self._StepU[j];
                self._TrialX[j] = curX[j] + dx;
                maxDxInf = maxDxInf.max(dx.abs());
                j += 1;
            }

            if maxDxInf <= self._Options._StepTolerance
            {
                let status = if gradNormInf <= self._Options._GradientTolerance
                {
                    LeastSquaresStatus::Stationary
                }
                else
                {
                    LeastSquaresStatus::StepToleranceReached
                };
                return self.BuildAcceptedResult(
                    status,
                    bestRaw,
                    bestScaled,
                    curPhi,
                    gradNormInf,
                    lambda,
                    counters,
                );
            }

            counters._TrialAttempts += 1;
            counters._ResidualEvals += 1;
            let trialEval = self.EvaluateTrialResiduals(repos);

            let accepted = match trialEval
            {
                Ok((trialPhi, trialCnt)) =>
                {
                    Self::AccumulateEvalWork(&mut counters._EvalWork, trialCnt);
                    let actualReduction = curPhi - trialPhi;
                    let ratio = if predReduction > 0.0
                    {
                        actualReduction / predReduction
                    }
                    else
                    {
                        -1.0
                    };
                    if trialPhi.is_finite()
                        && actualReduction > 0.0
                        && ratio > self._Options._AcceptRatioThreshold
                    {
                        curPhi = trialPhi;
                        Some(trialPhi)
                    }
                    else
                    {
                        None
                    }
                }
                Err(_) => None,
            };

            if accepted.is_some()
            {
                curX.USeg().Traverse(|idx| {
                    curX[idx] = self._TrialX[idx];
                    self._BestX[idx] = curX[idx];
                });
                bestRaw = self._RawResiduals.clone();
                bestScaled = self._ScaledRes.clone();
                counters._AcceptedIterations += 1;
                counters._DampingRetries = 0;
                lambda = (lambda * self._Options._LambdaDownFactor).max(self._Options._MinLambda);

                if self.MaxAbsScaledResidual() <= self._Options._ResidualTolerance
                {
                    return self.BuildAcceptedResult(
                        LeastSquaresStatus::ResidualSatisfied,
                        bestRaw,
                        bestScaled,
                        curPhi,
                        gradNormInf,
                        lambda,
                        counters,
                    );
                }

                if counters._JacobianEvals >= self._Options._MaxJacobianEvals
                {
                    return self.BuildAcceptedResult(
                        LeastSquaresStatus::AcceptedProgressBudgetExhausted,
                        bestRaw,
                        bestScaled,
                        curPhi,
                        gradNormInf,
                        lambda,
                        counters,
                    );
                }

                let jacCnt = match self.EvaluateJacobian(repos, curX.Arr())
                {
                    Ok(c) => c,
                    Err(err) =>
                    {
                        return self.BuildAcceptedResult(
                            LeastSquaresStatus::DerivativeFailure(err.to_string()),
                            bestRaw,
                            bestScaled,
                            curPhi,
                            gradNormInf,
                            lambda,
                            counters,
                        );
                    }
                };
                counters._JacobianEvals += 1;
                Self::AccumulateEvalWork(&mut counters._EvalWork, jacCnt);

                gradNormInf = self.ComputeGradientAndNorm();
                if gradNormInf <= self._Options._GradientTolerance
                {
                    return self.BuildAcceptedResult(
                        LeastSquaresStatus::Stationary,
                        bestRaw,
                        bestScaled,
                        curPhi,
                        gradNormInf,
                        lambda,
                        counters,
                    );
                }
            }
            else
            {
                counters._RejectedTrials += 1;
                counters._DampingRetries += 1;
                let nextLambda =
                    (lambda * self._Options._LambdaUpFactor).min(self._Options._MaxLambda);
                if counters._DampingRetries >= self._Options._MaxDampingRetries
                    || (nextLambda == lambda && lambda >= self._Options._MaxLambda)
                {
                    return self.BuildAcceptedResult(
                        LeastSquaresStatus::DampingExhausted,
                        bestRaw,
                        bestScaled,
                        curPhi,
                        gradNormInf,
                        lambda,
                        counters,
                    );
                }
                lambda = nextLambda;
                // Restore _RawResiduals and _ScaledRes to best accepted point before next step.
                let mut r = 0u32;
                while r < m
                {
                    self._RawResiduals[r] = bestRaw[r];
                    self._ScaledRes[r] = bestScaled[r];
                    r += 1;
                }
            }
        }
    }

    //---------------------------------------------------------------------------------------------

    fn MaxAbsScaledResidual(&self) -> f64
    {
        let mut maxAbs = 0.0f64;
        let m = self._ScaledRes.Size();
        let mut i = 0u32;
        while i < m
        {
            maxAbs = maxAbs.max(self._ScaledRes[i].abs());
            i += 1;
        }
        return maxAbs;
    }

    fn ComputeGradientAndNorm(&mut self) -> f64
    {
        let m = self._ResidualRoots.Size();
        let n = self._ActiveVarIds.Size();
        let mut maxNorm = 0.0f64;
        let mut j = 0u32;
        while j < n
        {
            let mut sum = 0.0f64;
            let mut i = 0u32;
            while i < m
            {
                sum += self._Jacobian[i * n + j] * self._ScaledRes[i];
                i += 1;
            }
            self._Gradient[j] = sum;
            maxNorm = maxNorm.max(sum.abs());
            j += 1;
        }
        return maxNorm;
    }

    fn ComputePredictedReduction(&mut self, lambda: f64) -> f64
    {
        let m = self._ResidualRoots.Size();
        let n = self._ActiveVarIds.Size();
        let mut normJduSq = 0.0f64;
        let mut i = 0u32;
        while i < m
        {
            let mut sum = 0.0f64;
            let mut j = 0u32;
            while j < n
            {
                sum += self._Jacobian[i * n + j] * self._StepU[j];
                j += 1;
            }
            self._JStep[i] = sum;
            normJduSq += sum * sum;
            i += 1;
        }
        let mut normDuSq = 0.0f64;
        let mut j = 0u32;
        while j < n
        {
            normDuSq += self._StepU[j] * self._StepU[j];
            j += 1;
        }
        return 0.5 * normJduSq + lambda * normDuSq;
    }

    /// Solves `min || [J_s; sqrt(lambda) I] du - [-q; 0] ||_2` in-place using Householder QR
    /// over column-major `_AugMat` (`(m + n) x n`) and `_AugRhs` (`m + n`).
    fn SolveDampedQr(&mut self, lambda: f64) -> Result<(), String>
    {
        let m = self._ResidualRoots.Size();
        let n = self._ActiveVarIds.Size();
        let rows = m + n;
        let sqrtLambda = lambda.sqrt();
        if !sqrtLambda.is_finite()
        {
            return Err("Damping factor sqrt(lambda) is non-finite".to_string());
        }

        // Populate column-major augmented matrix [J_s; sqrt(lambda)*I] and RHS [-q; 0].
        let mut c = 0u32;
        while c < n
        {
            let colBase = c * rows;
            let mut r = 0u32;
            while r < m
            {
                self._AugMat[colBase + r] = self._Jacobian[r * n + c];
                r += 1;
            }
            while r < rows
            {
                self._AugMat[colBase + r] = if r - m == c { sqrtLambda } else { 0.0 };
                r += 1;
            }
            c += 1;
        }

        let mut r = 0u32;
        while r < m
        {
            self._AugRhs[r] = -self._ScaledRes[r];
            r += 1;
        }
        while r < rows
        {
            self._AugRhs[r] = 0.0;
            r += 1;
        }

        // In-place Householder QR factorization.
        let mut k = 0u32;
        while k < n
        {
            let colK = k * rows;
            let mut normSq = 0.0f64;
            let mut i = k;
            while i < rows
            {
                let v = self._AugMat[colK + i];
                normSq += v * v;
                i += 1;
            }
            let norm = normSq.sqrt();
            if !norm.is_finite() || norm == 0.0
            {
                return Err(format!("Singular augmented column {} in Householder QR", k));
            }

            let diag = self._AugMat[colK + k];
            let alpha = if diag >= 0.0 { -norm } else { norm };
            let v0 = diag - alpha;
            self._AugMat[colK + k] = v0;

            // Compute v^T v = v0^2 + (normSq - diag^2) = 2 * norm * (norm + |diag|)
            let mut vNormSq = v0 * v0;
            i = k + 1;
            while i < rows
            {
                let v = self._AugMat[colK + i];
                vNormSq += v * v;
                i += 1;
            }
            if vNormSq > 0.0
            {
                let invVNormSq = 2.0 / vNormSq;
                // Apply reflection to remaining columns col = k+1 .. n
                let mut col = k + 1;
                while col < n
                {
                    let colOff = col * rows;
                    let mut dot = 0.0f64;
                    let mut row = k;
                    while row < rows
                    {
                        dot += self._AugMat[colK + row] * self._AugMat[colOff + row];
                        row += 1;
                    }
                    let scale = dot * invVNormSq;
                    row = k;
                    while row < rows
                    {
                        self._AugMat[colOff + row] -= scale * self._AugMat[colK + row];
                        row += 1;
                    }
                    col += 1;
                }

                // Apply reflection to RHS
                let mut dotRhs = 0.0f64;
                let mut row = k;
                while row < rows
                {
                    dotRhs += self._AugMat[colK + row] * self._AugRhs[row];
                    row += 1;
                }
                let scaleRhs = dotRhs * invVNormSq;
                row = k;
                while row < rows
                {
                    self._AugRhs[row] -= scaleRhs * self._AugMat[colK + row];
                    row += 1;
                }
            }

            // Store R[k, k] = alpha
            self._AugMat[colK + k] = alpha;
            k += 1;
        }

        // Back-substitution for upper triangular R * du = rhs[0..n]
        let mut row = n;
        while row > 0
        {
            row -= 1;
            let mut sum = self._AugRhs[row];
            let mut col = row + 1;
            while col < n
            {
                sum -= self._AugMat[col * rows + row] * self._StepU[col];
                col += 1;
            }
            let rDiag = self._AugMat[row * rows + row];
            if !rDiag.is_finite() || rDiag == 0.0
            {
                return Err(format!("Zero diagonal in QR back-substitution at {}", row));
            }
            let val = sum / rDiag;
            if !val.is_finite()
            {
                return Err("Non-finite step in QR back-substitution".to_string());
            }
            self._StepU[row] = val;
        }

        return Ok(());
    }

    fn AccumulateEvalWork(total: &mut EvalCounters, delta: EvalCounters)
    {
        total._NodesVisited = total._NodesVisited.saturating_add(delta._NodesVisited);
        total._EdgesTraversed = total._EdgesTraversed.saturating_add(delta._EdgesTraversed);
        total._SharedVisits = total._SharedVisits.saturating_add(delta._SharedVisits);
        total._ResultsComputed = total._ResultsComputed.saturating_add(delta._ResultsComputed);
        total._PeakDepth = total._PeakDepth.max(delta._PeakDepth);
    }

    #[allow(clippy::too_many_arguments)]
    fn BuildAcceptedResult(
        &self,
        status: LeastSquaresStatus,
        rawResiduals: Buff<f64>,
        scaledResiduals: Buff<f64>,
        objective: f64,
        gradNormInf: f64,
        finalLambda: f64,
        counters: LeastSquaresCounters,
    ) -> LeastSquaresResult
    {
        return LeastSquaresResult {
            _Status:          status,
            _BestX:           self._BestX.clone(),
            _RawResiduals:    rawResiduals,
            _ScaledResiduals: scaledResiduals,
            _Objective:       objective,
            _GradientNormInf: gradNormInf,
            _FinalLambda:     finalLambda,
            _Counters:        counters,
        };
    }

    fn FinishResult(
        &self,
        status: LeastSquaresStatus,
        objective: f64,
        gradNormInf: f64,
        finalLambda: f64,
        counters: LeastSquaresCounters,
    ) -> LeastSquaresResult
    {
        return LeastSquaresResult {
            _Status:          status,
            _BestX:           self._BestX.clone(),
            _RawResiduals:    self._RawResiduals.clone(),
            _ScaledResiduals: self._ScaledRes.clone(),
            _Objective:       objective,
            _GradientNormInf: gradNormInf,
            _FinalLambda:     finalLambda,
            _Counters:        counters,
        };
    }

    fn ValidateOptions(opts: &LeastSquaresOptions) -> Result<(), LeastSquaresPrepError>
    {
        if !opts._ResidualTolerance.is_finite() || opts._ResidualTolerance < 0.0
            || !opts._GradientTolerance.is_finite() || opts._GradientTolerance < 0.0
            || !opts._StepTolerance.is_finite() || opts._StepTolerance < 0.0
        {
            return Err(LeastSquaresPrepError::InvalidOptions(
                "Tolerances must be finite and non-negative".to_string(),
            ));
        }
        if !opts._InitialLambda.is_finite() || opts._InitialLambda <= 0.0
            || !opts._MinLambda.is_finite() || opts._MinLambda <= 0.0
            || !opts._MaxLambda.is_finite() || opts._MaxLambda < opts._MinLambda
        {
            return Err(LeastSquaresPrepError::InvalidOptions(
                "Lambda bounds must be positive and finite with MinLambda <= MaxLambda".to_string(),
            ));
        }
        if !opts._LambdaUpFactor.is_finite() || opts._LambdaUpFactor <= 1.0
            || !opts._LambdaDownFactor.is_finite() || opts._LambdaDownFactor <= 0.0
            || opts._LambdaDownFactor >= 1.0
        {
            return Err(LeastSquaresPrepError::InvalidOptions(
                "Lambda factors must satisfy Up > 1 and 0 < Down < 1".to_string(),
            ));
        }
        if !opts._AcceptRatioThreshold.is_finite() || opts._AcceptRatioThreshold < 0.0
        {
            return Err(LeastSquaresPrepError::InvalidOptions(
                "AcceptRatioThreshold must be finite and non-negative".to_string(),
            ));
        }
        return Ok(());
    }

    fn ValidateResidualDag(
        repos: &ExprRepos,
        rootId: u32,
        activeIds: &Buff<u32>,
        bindingsBuf: &Buff<f64>,
        limits: &LeastSquaresLimits,
    ) -> Result<(), LeastSquaresPrepError>
    {
        let mut stack = Stash::<(u32, u32)>::WithCapacity(32);
        stack.Push((rootId, 0));
        let mut visited = 0u32;

        while let Some((id, depth)) = stack.Pop()
        {
            if depth > limits._MaxExprDepth
            {
                return Err(LeastSquaresPrepError::InvalidOptions(
                    "Residual expression depth exceeds limit".to_string(),
                ));
            }
            visited = visited.saturating_add(1);
            if visited > limits._MaxEvalNodes
            {
                return Err(LeastSquaresPrepError::InvalidOptions(
                    "Residual expression node count exceeds limit".to_string(),
                ));
            }
            if !repos.IsValidExprId(id)
            {
                return Err(LeastSquaresPrepError::InvalidResidualExpr(id));
            }
            let view = match repos.Inspect(id)
            {
                Ok(v) => v,
                Err(ExprReadError::InvalidId(_) | ExprReadError::EmptyNode(_)) => {
                    return Err(LeastSquaresPrepError::InvalidResidualExpr(id));
                }
                Err(ExprReadError::UnsupportedNode(_)) => {
                    return Err(LeastSquaresPrepError::UnsupportedOperator(id));
                }
            };
            match view
            {
                ExprView::Real(_) => {}
                ExprView::Var(varId) =>
                {
                    if !repos.IsValidVarId(varId)
                    {
                        return Err(LeastSquaresPrepError::InvalidResidualExpr(id));
                    }
                    let mut isActive = false;
                    let mut k = 0u32;
                    while k < activeIds.Size()
                    {
                        if activeIds[k] == varId
                        {
                            isActive = true;
                            break;
                        }
                        k += 1;
                    }
                    if !isActive && bindingsBuf[varId].is_nan()
                    {
                        return Err(LeastSquaresPrepError::MissingBinding(
                            repos.VarNameAt(varId).to_string(),
                        ));
                    }
                }
                ExprView::Sum(poly) | ExprView::Prod(poly) | ExprView::Pow(poly) =>
                {
                    let count = poly.SizeChild();
                    let mut c = 0u32;
                    while c < count
                    {
                        stack.Push((poly.Child(c), depth + 1));
                        c += 1;
                    }
                }
                ExprView::Func { name, arg } =>
                {
                    match name
                    {
                        "sin" | "cos" | "exp" | "ln" | "sqrt" => {}
                        _ => return Err(LeastSquaresPrepError::UnsupportedOperator(id)),
                    }
                    stack.Push((arg, depth + 1));
                }
                ExprView::Cond { .. } | ExprView::Logic { .. } =>
                {
                    return Err(LeastSquaresPrepError::UnsupportedOperator(id));
                }
            }
        }
        return Ok(());
    }
}

//-------------------------------------------------------------------------------------------------
