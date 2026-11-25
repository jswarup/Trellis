// compiled.rs -------------------------------------------------------------------------------------
//! LS4 compiled topological multi-root evaluation and LS5 forward/reverse automatic
//! differentiation over owned, snapshot-isolated operation arrays.

use crate::fresco::{EvalError, ExprReadError, ExprRepos, ExprView, VarKind};
use crate::silo::{Arr, Buff, MutArr, Stash};
use core::fmt;

//-------------------------------------------------------------------------------------------------

/// Resource limits for compiling Fresco expression DAGs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompileLimits
{
    pub _MaxDepth: u32,
    pub _MaxOps:   u32,
}

impl Default for CompileLimits
{
    fn default() -> Self
    {
        Self {
            _MaxDepth: ExprRepos::K_MAX_TREE_DEPTH,
            _MaxOps:   200_000,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Errors returned when compiling Fresco roots into topological operation arrays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileError
{
    EmptyRoots,
    InvalidExprId(u32),
    InvalidVarId(u32),
    CycleDetected(u32),
    DepthExceeded(u32),
    OpBudgetExceeded(u32),
    UnsupportedLazyNode(u32),
    UnknownFunction(String),
    InvalidArity(u32),
}

impl fmt::Display for CompileError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::EmptyRoots => write!(f, "Cannot compile empty root list"),
            Self::InvalidExprId(id) => write!(f, "Invalid expression ID {}", id),
            Self::InvalidVarId(id) => write!(f, "Invalid variable ID {}", id),
            Self::CycleDetected(id) => write!(f, "Cycle detected at expression ID {}", id),
            Self::DepthExceeded(max) => write!(f, "Compilation depth exceeds limit of {}", max),
            Self::OpBudgetExceeded(max) => {
                write!(f, "Compiled operation count exceeds limit of {}", max)
            }
            Self::UnsupportedLazyNode(id) => {
                write!(
                    f,
                    "Lazy conditional/logic node {} is retained in the reference evaluator",
                    id
                )
            }
            Self::UnknownFunction(name) => write!(f, "Unknown function '{}'", name),
            Self::InvalidArity(id) => write!(f, "Invalid operator arity at expression ID {}", id),
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Errors returned during forward or reverse automatic differentiation sweeps.
#[derive(Clone, Debug, PartialEq)]
pub enum AdError
{
    Eval(EvalError),
    StalePrimal
    {
        input_gen:  u64,
        primal_gen: u64,
    },
    DimensionMismatch
    {
        expected: u32,
        actual:   u32,
    },
    UnsupportedAdFunction(&'static str),
}

impl From<EvalError> for AdError
{
    fn from(err: EvalError) -> Self
    {
        Self::Eval(err)
    }
}

impl fmt::Display for AdError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::Eval(e) => write!(f, "Evaluation error: {}", e),
            Self::StalePrimal {
                input_gen,
                primal_gen,
            } => write!(
                f,
                "Primal workspace is stale (input generation {}, primal generation {})",
                input_gen, primal_gen
            ),
            Self::DimensionMismatch { expected, actual } => {
                write!(f, "Dimension mismatch: expected {}, got {}", expected, actual)
            }
            Self::UnsupportedAdFunction(name) => {
                write!(f, "Function '{}' is not differentiable in AD sweep", name)
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompiledFuncKind
{
    Abs,
    Sqrt,
    Exp,
    Ln,
    Log10,
    Sin,
    Cos,
    Tan,
    Sinh,
    Cosh,
    Tanh,
    Floor,
    Ceil,
    Round,
}

impl CompiledFuncKind
{
    fn FromName(name: &str) -> Option<Self>
    {
        match name
        {
            "abs" => Some(Self::Abs),
            "sqrt" => Some(Self::Sqrt),
            "exp" => Some(Self::Exp),
            "ln" | "log" => Some(Self::Ln),
            "log10" => Some(Self::Log10),
            "sin" => Some(Self::Sin),
            "cos" => Some(Self::Cos),
            "tan" => Some(Self::Tan),
            "sinh" => Some(Self::Sinh),
            "cosh" => Some(Self::Cosh),
            "tanh" => Some(Self::Tanh),
            "floor" => Some(Self::Floor),
            "ceil" => Some(Self::Ceil),
            "round" => Some(Self::Round),
            _ => None,
        }
    }

    fn Name(&self) -> &'static str
    {
        match self
        {
            Self::Abs => "abs",
            Self::Sqrt => "sqrt",
            Self::Exp => "exp",
            Self::Ln => "ln",
            Self::Log10 => "log10",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Sinh => "sinh",
            Self::Cosh => "cosh",
            Self::Tanh => "tanh",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Round => "round",
        }
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompiledOp
{
    Const(f64),
    Var
    {
        varId:   u32,
        nonZero: bool,
    },
    Sum
    {
        start:    u32,
        addCount: u32,
        subCount: u32,
    },
    Prod
    {
        start:    u32,
        numCount: u32,
        denCount: u32,
    },
    Pow
    {
        base: u32,
        exp:  u32,
    },
    Func
    {
        kind: CompiledFuncKind,
        arg:  u32,
    },
}

//-------------------------------------------------------------------------------------------------

/// Owned, snapshot-isolated compiled topological program for multi-root evaluation and AD.
#[derive(Clone, Debug)]
pub struct CompiledProgram
{
    _Ops:       Buff<CompiledOp>,
    _Operands:  Buff<u32>,
    _RootOps:   Buff<u32>,
    _VarCount:  u32,
    _VarNames:  Buff<String>,
    _ExecCount: u32,
}

impl CompiledProgram
{
    /// Compiles ordered expression `roots` (preserving order and repeated roots) into an
    /// owned topological program independent of future `ExprRepos` mutations or rollbacks.
    pub fn Compile(
        repos: &ExprRepos,
        roots: Arr<'_, u32>,
        limits: &CompileLimits,
    ) -> Result<Self, CompileError>
    {
        let rootCount = roots.Size();
        if rootCount == 0
        {
            return Err(CompileError::EmptyRoots);
        }

        let repoSize = repos.Size();
        let mut exprToOp = Buff::<u32>::FromDispenser(repoSize, |_| u32::MAX);
        let mut state = Buff::<u8>::FromDispenser(repoSize, |_| 0);
        let mut ops = Stash::<CompiledOp>::WithCapacity(32);
        let mut operands = Stash::<u32>::WithCapacity(64);
        let mut rootOps = Stash::<u32>::WithCapacity(rootCount);
        let mut stack = Stash::<(u32, u32, bool)>::WithCapacity(64);

        let mut rIdx = 0u32;
        while rIdx < rootCount
        {
            let rootExpr = roots[rIdx];
            if !repos.IsValidExprId(rootExpr)
            {
                return Err(CompileError::InvalidExprId(rootExpr));
            }

            if exprToOp[rootExpr] == u32::MAX
            {
                stack.Push((rootExpr, 0, false));
                while let Some((exprId, depth, postVisit)) = stack.Pop()
                {
                    if postVisit
                    {
                        if exprToOp[exprId] != u32::MAX
                        {
                            state[exprId] = 2;
                            continue;
                        }
                        if ops.Size() >= limits._MaxOps
                        {
                            return Err(CompileError::OpBudgetExceeded(limits._MaxOps));
                        }
                        let view = repos
                            .Inspect(exprId)
                            .map_err(|_| CompileError::InvalidExprId(exprId))?;
                        let op = match view
                        {
                            ExprView::Real(val) => CompiledOp::Const(val),
                            ExprView::Var(varIdx) =>
                            {
                                let nonZero = repos.VarAttrAt(varIdx).HasBits(VarKind::NonZero);
                                CompiledOp::Var {
                                    varId: varIdx,
                                    nonZero,
                                }
                            }
                            ExprView::Sum(poly) =>
                            {
                                let start = operands.Size();
                                let coSize = poly.CommonSize();
                                let total = poly.SizeChild();
                                let mut c = 0u32;
                                while c < total
                                {
                                    operands.Push(exprToOp[poly.Child(c)]);
                                    c += 1;
                                }
                                CompiledOp::Sum {
                                    start,
                                    addCount: coSize,
                                    subCount: total - coSize,
                                }
                            }
                            ExprView::Prod(poly) =>
                            {
                                let start = operands.Size();
                                let coSize = poly.CommonSize();
                                let total = poly.SizeChild();
                                let mut c = 0u32;
                                while c < total
                                {
                                    operands.Push(exprToOp[poly.Child(c)]);
                                    c += 1;
                                }
                                CompiledOp::Prod {
                                    start,
                                    numCount: coSize,
                                    denCount: total - coSize,
                                }
                            }
                            ExprView::Pow(poly) =>
                            {
                                let base = exprToOp[poly.Child(0)];
                                let exp = exprToOp[poly.Child(1)];
                                CompiledOp::Pow { base, exp }
                            }
                            ExprView::Func { name, arg } =>
                            {
                                let kind = CompiledFuncKind::FromName(name)
                                    .ok_or_else(|| CompileError::UnknownFunction(name.to_string()))?;
                                CompiledOp::Func {
                                    kind,
                                    arg: exprToOp[arg],
                                }
                            }
                            ExprView::Cond { .. } | ExprView::Logic { .. } =>
                            {
                                return Err(CompileError::UnsupportedLazyNode(exprId));
                            }
                        };
                        let opId = ops.Size();
                        ops.Push(op);
                        exprToOp[exprId] = opId;
                        state[exprId] = 2;
                        continue;
                    }

                    if depth > limits._MaxDepth
                    {
                        return Err(CompileError::DepthExceeded(limits._MaxDepth));
                    }
                    if !repos.IsValidExprId(exprId)
                    {
                        return Err(CompileError::InvalidExprId(exprId));
                    }
                    if state[exprId] == 1
                    {
                        return Err(CompileError::CycleDetected(exprId));
                    }
                    if state[exprId] == 2
                    {
                        continue;
                    }

                    let view = match repos.Inspect(exprId)
                    {
                        Ok(v) => v,
                        Err(ExprReadError::InvalidId(_) | ExprReadError::EmptyNode(_)) => {
                            return Err(CompileError::InvalidExprId(exprId));
                        }
                        Err(ExprReadError::UnsupportedNode(_)) => {
                            return Err(CompileError::UnsupportedLazyNode(exprId));
                        }
                    };

                    state[exprId] = 1;
                    stack.Push((exprId, depth, true));

                    match view
                    {
                        ExprView::Real(_) => {}
                        ExprView::Var(varIdx) =>
                        {
                            if !repos.IsValidVarId(varIdx)
                            {
                                return Err(CompileError::InvalidVarId(varIdx));
                            }
                        }
                        ExprView::Sum(poly) | ExprView::Prod(poly) =>
                        {
                            if poly.CommonSize() > poly.SizeChild()
                            {
                                return Err(CompileError::InvalidArity(exprId));
                            }
                            let mut c = poly.SizeChild();
                            while c > 0
                            {
                                c -= 1;
                                let child = poly.Child(c);
                                if !repos.IsValidExprId(child)
                                {
                                    return Err(CompileError::InvalidExprId(child));
                                }
                                if state[child] == 1
                                {
                                    return Err(CompileError::CycleDetected(child));
                                }
                                stack.Push((child, depth + 1, false));
                            }
                        }
                        ExprView::Pow(poly) =>
                        {
                            if poly.SizeChild() < 2
                            {
                                return Err(CompileError::InvalidArity(exprId));
                            }
                            let exp = poly.Child(1);
                            let base = poly.Child(0);
                            if !repos.IsValidExprId(exp)
                            {
                                return Err(CompileError::InvalidExprId(exp));
                            }
                            if !repos.IsValidExprId(base)
                            {
                                return Err(CompileError::InvalidExprId(base));
                            }
                            if state[exp] == 1
                            {
                                return Err(CompileError::CycleDetected(exp));
                            }
                            if state[base] == 1
                            {
                                return Err(CompileError::CycleDetected(base));
                            }
                            stack.Push((exp, depth + 1, false));
                            stack.Push((base, depth + 1, false));
                        }
                        ExprView::Func { name, arg } =>
                        {
                            if CompiledFuncKind::FromName(name).is_none()
                            {
                                return Err(CompileError::UnknownFunction(name.to_string()));
                            }
                            if !repos.IsValidExprId(arg)
                            {
                                return Err(CompileError::InvalidExprId(arg));
                            }
                            if state[arg] == 1
                            {
                                return Err(CompileError::CycleDetected(arg));
                            }
                            stack.Push((arg, depth + 1, false));
                        }
                        ExprView::Cond { .. } | ExprView::Logic { .. } =>
                        {
                            return Err(CompileError::UnsupportedLazyNode(exprId));
                        }
                    }
                }
            }

            rootOps.Push(exprToOp[rootExpr]);
            rIdx += 1;
        }

        let varCount = repos.SzVar();
        let varNames = Buff::<String>::FromDispenser(varCount, |v| repos.VarNameAt(v).to_string());
        let opCount = ops.Size();
        return Ok(Self {
            _Ops:       ops.IntoBuff(),
            _Operands:  operands.IntoBuff(),
            _RootOps:   rootOps.IntoBuff(),
            _VarCount:  varCount,
            _VarNames:  varNames,
            _ExecCount: opCount,
        });
    }

    #[inline]
    pub fn OpCount(&self) -> u32
    {
        return self._ExecCount;
    }

    #[inline]
    pub fn RootCount(&self) -> u32
    {
        return self._RootOps.Size();
    }

    #[inline]
    pub fn VarCount(&self) -> u32
    {
        return self._VarCount;
    }

    pub fn VarNameAt(&self, varId: u32) -> Option<&str>
    {
        if varId < self._VarNames.Size()
        {
            return Some(self._VarNames[varId].as_str());
        }
        return None;
    }

    pub fn AllocWorkspace(&self) -> CompiledWorkspace
    {
        return CompiledWorkspace::New(self);
    }
}

//-------------------------------------------------------------------------------------------------

/// Preallocated primal, tangent, and adjoint workspace with explicit reset and
/// input-generation rules for zero-allocation repeated evaluation and AD sweeps.
pub struct CompiledWorkspace
{
    _Primal:            Buff<f64>,
    _Tangent:           Buff<f64>,
    _Adjoint:           Buff<f64>,
    _InputBindings:     Buff<f64>,
    _RootOutputs:       Buff<f64>,
    _InputGeneration:   u64,
    _PrimalGeneration:  u64,
    _TangentGeneration: u64,
    _AdjointGeneration: u64,
}

impl CompiledWorkspace
{
    pub fn New(program: &CompiledProgram) -> Self
    {
        let opCount = program.OpCount();
        let varCount = program.VarCount();
        let rootCount = program.RootCount();
        return Self {
            _Primal:            Buff::FromDispenser(opCount, |_| 0.0),
            _Tangent:           Buff::FromDispenser(opCount, |_| 0.0),
            _Adjoint:           Buff::FromDispenser(opCount, |_| 0.0),
            _InputBindings:     Buff::FromDispenser(varCount, |_| f64::NAN),
            _RootOutputs:       Buff::FromDispenser(rootCount, |_| 0.0),
            _InputGeneration:   1,
            _PrimalGeneration:  0,
            _TangentGeneration: 0,
            _AdjointGeneration: 0,
        };
    }

    #[inline]
    pub fn InputGeneration(&self) -> u64
    {
        return self._InputGeneration;
    }

    #[inline]
    pub fn PrimalGeneration(&self) -> u64
    {
        return self._PrimalGeneration;
    }

    #[inline]
    pub fn TangentGeneration(&self) -> u64
    {
        return self._TangentGeneration;
    }

    #[inline]
    pub fn AdjointGeneration(&self) -> u64
    {
        return self._AdjointGeneration;
    }

    #[inline]
    pub fn RootOutputs(&self) -> Arr<'_, f64>
    {
        return self._RootOutputs.Arr();
    }

    pub fn SetInput(&mut self, varId: u32, val: f64) -> Result<u64, EvalError>
    {
        if varId >= self._InputBindings.Size()
        {
            return Err(EvalError::UndefinedVariable(varId));
        }
        self._InputBindings[varId] = val;
        self._InputGeneration = self._InputGeneration.saturating_add(1);
        return Ok(self._InputGeneration);
    }

    pub fn SetInputs(&mut self, bindings: Arr<'_, f64>) -> Result<u64, EvalError>
    {
        let varCount = self._InputBindings.Size();
        let copyCount = bindings.Size().min(varCount);
        let mut v = 0u32;
        while v < copyCount
        {
            self._InputBindings[v] = bindings[v];
            v += 1;
        }
        while v < varCount
        {
            self._InputBindings[v] = f64::NAN;
            v += 1;
        }
        self._InputGeneration = self._InputGeneration.saturating_add(1);
        return Ok(self._InputGeneration);
    }

    pub fn ResetPrimal(&mut self)
    {
        let mut i = 0u32;
        while i < self._Primal.Size()
        {
            self._Primal[i] = 0.0;
            i += 1;
        }
        self._PrimalGeneration = 0;
    }

    pub fn ResetTangent(&mut self)
    {
        let mut i = 0u32;
        while i < self._Tangent.Size()
        {
            self._Tangent[i] = 0.0;
            i += 1;
        }
        self._TangentGeneration = 0;
    }

    pub fn ResetAdjoint(&mut self)
    {
        let mut i = 0u32;
        while i < self._Adjoint.Size()
        {
            self._Adjoint[i] = 0.0;
            i += 1;
        }
        self._AdjointGeneration = 0;
    }

    /// Evaluates all compiled operations once in topological order, writing root values
    /// into `_RootOutputs` with zero heap allocations.
    pub fn Evaluate(&mut self, program: &CompiledProgram) -> Result<Arr<'_, f64>, EvalError>
    {
        let opCount = program._Ops.Size();
        let mut i = 0u32;
        while i < opCount
        {
            let val = match program._Ops[i]
            {
                CompiledOp::Const(v) => v,
                CompiledOp::Var { varId, nonZero } =>
                {
                    if varId >= self._InputBindings.Size()
                    {
                        return Err(EvalError::UndefinedVariable(varId));
                    }
                    let v = self._InputBindings[varId];
                    if v.is_nan()
                    {
                        let name = program.VarNameAt(varId).unwrap_or("?");
                        return Err(EvalError::DomainError(format!(
                            "Variable '{}' (ID {}) is uninitialized or bound to NaN",
                            name, varId
                        )));
                    }
                    if nonZero && (v == 0.0 || v == -0.0)
                    {
                        return Err(EvalError::AssumptionViolated {
                            var_id: varId,
                            reason: "NonZero variable cannot be bound to zero",
                        });
                    }
                    v
                }
                CompiledOp::Sum {
                    start,
                    addCount,
                    subCount,
                } =>
                {
                    let mut total = 0.0f64;
                    let mut k = 0u32;
                    while k < addCount
                    {
                        total += self._Primal[program._Operands[start + k]];
                        k += 1;
                    }
                    let mut s = 0u32;
                    while s < subCount
                    {
                        total -= self._Primal[program._Operands[start + addCount + s]];
                        s += 1;
                    }
                    total
                }
                CompiledOp::Prod {
                    start,
                    numCount,
                    denCount,
                } =>
                {
                    let mut numer = 1.0f64;
                    let mut denom = 1.0f64;
                    let mut k = 0u32;
                    while k < numCount
                    {
                        numer *= self._Primal[program._Operands[start + k]];
                        k += 1;
                    }
                    let mut d = 0u32;
                    while d < denCount
                    {
                        let factor = self._Primal[program._Operands[start + numCount + d]];
                        if factor == 0.0 || factor == -0.0
                        {
                            return Err(EvalError::DivisionByZero);
                        }
                        denom *= factor;
                        d += 1;
                    }
                    if denom == 0.0 || denom == -0.0
                    {
                        return Err(EvalError::DivisionByZero);
                    }
                    numer / denom
                }
                CompiledOp::Pow { base, exp } =>
                {
                    let b = self._Primal[base];
                    let e = self._Primal[exp];
                    if b == 0.0 && e < 0.0
                    {
                        return Err(EvalError::DivisionByZero);
                    }
                    if b < 0.0 && e.fract() != 0.0
                    {
                        return Err(EvalError::DomainError(format!(
                            "Negative base {} cannot be raised to non-integer power {}",
                            b, e
                        )));
                    }
                    b.powf(e)
                }
                CompiledOp::Func { kind, arg } =>
                {
                    let a = self._Primal[arg];
                    match kind
                    {
                        CompiledFuncKind::Abs => a.abs(),
                        CompiledFuncKind::Sqrt =>
                        {
                            if a < 0.0
                            {
                                return Err(EvalError::DomainError(format!(
                                    "sqrt of negative value {}",
                                    a
                                )));
                            }
                            a.sqrt()
                        }
                        CompiledFuncKind::Exp => a.exp(),
                        CompiledFuncKind::Ln =>
                        {
                            if a <= 0.0
                            {
                                return Err(EvalError::DomainError(format!(
                                    "log of non-positive value {}",
                                    a
                                )));
                            }
                            a.ln()
                        }
                        CompiledFuncKind::Log10 =>
                        {
                            if a <= 0.0
                            {
                                return Err(EvalError::DomainError(format!(
                                    "log10 of non-positive value {}",
                                    a
                                )));
                            }
                            a.log10()
                        }
                        CompiledFuncKind::Sin => a.sin(),
                        CompiledFuncKind::Cos => a.cos(),
                        CompiledFuncKind::Tan => a.tan(),
                        CompiledFuncKind::Sinh => a.sinh(),
                        CompiledFuncKind::Cosh => a.cosh(),
                        CompiledFuncKind::Tanh => a.tanh(),
                        CompiledFuncKind::Floor => a.floor(),
                        CompiledFuncKind::Ceil => a.ceil(),
                        CompiledFuncKind::Round => a.round(),
                    }
                }
            };
            self._Primal[i] = val;
            i += 1;
        }

        let rootCount = program._RootOps.Size();
        let mut r = 0u32;
        while r < rootCount
        {
            self._RootOutputs[r] = self._Primal[program._RootOps[r]];
            r += 1;
        }
        self._PrimalGeneration = self._InputGeneration;
        return Ok(self._RootOutputs.Arr());
    }

    /// LS5: Computes a forward Jacobian-vector product `J * v` using `seedTangent` (len `varCount`)
    /// and writes the resulting root tangents into `outJvp` (len `rootCount`).
    pub fn ForwardJvp(
        &mut self,
        program: &CompiledProgram,
        seedTangent: Arr<'_, f64>,
        outJvp: &mut MutArr<'_, f64>,
    ) -> Result<(), AdError>
    {
        if self._PrimalGeneration != self._InputGeneration
        {
            return Err(AdError::StalePrimal {
                input_gen:  self._InputGeneration,
                primal_gen: self._PrimalGeneration,
            });
        }
        if seedTangent.Size() != program._VarCount
        {
            return Err(AdError::DimensionMismatch {
                expected: program._VarCount,
                actual:   seedTangent.Size(),
            });
        }
        if outJvp.Size() != program._RootOps.Size()
        {
            return Err(AdError::DimensionMismatch {
                expected: program._RootOps.Size(),
                actual:   outJvp.Size(),
            });
        }

        self.ResetTangent();
        let opCount = program._Ops.Size();
        let mut i = 0u32;
        while i < opCount
        {
            let dVal = match program._Ops[i]
            {
                CompiledOp::Const(_) => 0.0,
                CompiledOp::Var { varId, .. } => seedTangent[varId],
                CompiledOp::Sum {
                    start,
                    addCount,
                    subCount,
                } =>
                {
                    let mut sum = 0.0f64;
                    let mut k = 0u32;
                    while k < addCount
                    {
                        sum += self._Tangent[program._Operands[start + k]];
                        k += 1;
                    }
                    let mut s = 0u32;
                    while s < subCount
                    {
                        sum -= self._Tangent[program._Operands[start + addCount + s]];
                        s += 1;
                    }
                    sum
                }
                CompiledOp::Prod {
                    start,
                    numCount,
                    denCount,
                } =>
                {
                    // d(N/D) = (N' * D - N * D') / D^2, computing N' and D' without dividing by 0.
                    let mut dNumer = 0.0f64;
                    let mut numer = 1.0f64;
                    let mut k = 0u32;
                    while k < numCount
                    {
                        let opIdx = program._Operands[start + k];
                        numer *= self._Primal[opIdx];
                        let t = self._Tangent[opIdx];
                        if t != 0.0
                        {
                            let mut other = 1.0f64;
                            let mut p = 0u32;
                            while p < numCount
                            {
                                if p != k
                                {
                                    other *= self._Primal[program._Operands[start + p]];
                                }
                                p += 1;
                            }
                            dNumer += t * other;
                        }
                        k += 1;
                    }
                    if denCount == 0
                    {
                        dNumer
                    }
                    else
                    {
                        let mut denom = 1.0f64;
                        let mut dDenom = 0.0f64;
                        let mut d = 0u32;
                        while d < denCount
                        {
                            let opIdx = program._Operands[start + numCount + d];
                            let val = self._Primal[opIdx];
                            denom *= val;
                            let t = self._Tangent[opIdx];
                            if t != 0.0
                            {
                                let mut other = 1.0f64;
                                let mut q = 0u32;
                                while q < denCount
                                {
                                    if q != d
                                    {
                                        other *=
                                            self._Primal[program._Operands[start + numCount + q]];
                                    }
                                    q += 1;
                                }
                                dDenom += t * other;
                            }
                            d += 1;
                        }
                        (dNumer * denom - numer * dDenom) / (denom * denom)
                    }
                }
                CompiledOp::Pow { base, exp } =>
                {
                    let u = self._Primal[base];
                    let v = self._Primal[exp];
                    let du = self._Tangent[base];
                    let dv = self._Tangent[exp];
                    if du == 0.0 && dv == 0.0
                    {
                        0.0
                    }
                    else if dv == 0.0
                    {
                        if v == 0.0
                        {
                            0.0
                        }
                        else if v == 1.0
                        {
                            du
                        }
                        else if u == 0.0 && v < 1.0
                        {
                            return Err(AdError::Eval(EvalError::DivisionByZero));
                        }
                        else
                        {
                            v * u.powf(v - 1.0) * du
                        }
                    }
                    else
                    {
                        if u <= 0.0
                        {
                            return Err(AdError::Eval(EvalError::DomainError(format!(
                                "Logarithm in power derivative requires positive base, got {}",
                                u
                            ))));
                        }
                        let y = self._Primal[i];
                        y * (dv * u.ln() + v * du / u)
                    }
                }
                CompiledOp::Func { kind, arg } =>
                {
                    let du = self._Tangent[arg];
                    if du == 0.0
                    {
                        0.0
                    }
                    else
                    {
                        let u = self._Primal[arg];
                        let y = self._Primal[i];
                        let df = match kind
                        {
                            CompiledFuncKind::Sqrt =>
                            {
                                if y == 0.0
                                {
                                    return Err(AdError::Eval(EvalError::DivisionByZero));
                                }
                                0.5 / y
                            }
                            CompiledFuncKind::Exp => y,
                            CompiledFuncKind::Ln => 1.0 / u,
                            CompiledFuncKind::Sin => u.cos(),
                            CompiledFuncKind::Cos => -u.sin(),
                            CompiledFuncKind::Tan =>
                            {
                                let c = u.cos();
                                1.0 / (c * c)
                            }
                            CompiledFuncKind::Sinh => u.cosh(),
                            CompiledFuncKind::Cosh => u.sinh(),
                            CompiledFuncKind::Tanh => 1.0 - y * y,
                            other => return Err(AdError::UnsupportedAdFunction(other.Name())),
                        };
                        df * du
                    }
                }
            };
            self._Tangent[i] = dVal;
            i += 1;
        }

        let rootCount = program._RootOps.Size();
        let mut r = 0u32;
        while r < rootCount
        {
            outJvp[r] = self._Tangent[program._RootOps[r]];
            r += 1;
        }
        self._TangentGeneration = self._PrimalGeneration;
        return Ok(());
    }

    /// LS5: Computes a reverse vector-Jacobian product `J^T * u` using `rootAdjoints` (len `rootCount`)
    /// and writes the accumulated variable gradients into `outVjp` (len `varCount`).
    pub fn ReverseVjp(
        &mut self,
        program: &CompiledProgram,
        rootAdjoints: Arr<'_, f64>,
        outVjp: &mut MutArr<'_, f64>,
    ) -> Result<(), AdError>
    {
        if self._PrimalGeneration != self._InputGeneration
        {
            return Err(AdError::StalePrimal {
                input_gen:  self._InputGeneration,
                primal_gen: self._PrimalGeneration,
            });
        }
        if rootAdjoints.Size() != program._RootOps.Size()
        {
            return Err(AdError::DimensionMismatch {
                expected: program._RootOps.Size(),
                actual:   rootAdjoints.Size(),
            });
        }
        if outVjp.Size() != program._VarCount
        {
            return Err(AdError::DimensionMismatch {
                expected: program._VarCount,
                actual:   outVjp.Size(),
            });
        }

        self.ResetAdjoint();
        let mut v = 0u32;
        while v < outVjp.Size()
        {
            outVjp[v] = 0.0;
            v += 1;
        }

        // Seed root adjoints (accumulating if the same op appears in multiple roots).
        let rootCount = program._RootOps.Size();
        let mut r = 0u32;
        while r < rootCount
        {
            self._Adjoint[program._RootOps[r]] += rootAdjoints[r];
            r += 1;
        }

        let mut i = program._Ops.Size();
        while i > 0
        {
            i -= 1;
            let adj = self._Adjoint[i];
            if adj == 0.0
            {
                continue;
            }
            match program._Ops[i]
            {
                CompiledOp::Const(_) => {}
                CompiledOp::Var { varId, .. } =>
                {
                    outVjp[varId] += adj;
                }
                CompiledOp::Sum {
                    start,
                    addCount,
                    subCount,
                } =>
                {
                    let mut k = 0u32;
                    while k < addCount
                    {
                        self._Adjoint[program._Operands[start + k]] += adj;
                        k += 1;
                    }
                    let mut s = 0u32;
                    while s < subCount
                    {
                        self._Adjoint[program._Operands[start + addCount + s]] -= adj;
                        s += 1;
                    }
                }
                CompiledOp::Prod {
                    start,
                    numCount,
                    denCount,
                } =>
                {
                    let mut denom = 1.0f64;
                    let mut d = 0u32;
                    while d < denCount
                    {
                        denom *= self._Primal[program._Operands[start + numCount + d]];
                        d += 1;
                    }
                    let mut k = 0u32;
                    while k < numCount
                    {
                        let mut other = 1.0f64;
                        let mut p = 0u32;
                        while p < numCount
                        {
                            if p != k
                            {
                                other *= self._Primal[program._Operands[start + p]];
                            }
                            p += 1;
                        }
                        self._Adjoint[program._Operands[start + k]] += adj * (other / denom);
                        k += 1;
                    }
                    let y = self._Primal[i];
                    let mut d = 0u32;
                    while d < denCount
                    {
                        let opIdx = program._Operands[start + numCount + d];
                        let factor = self._Primal[opIdx];
                        self._Adjoint[opIdx] -= adj * (y / factor);
                        d += 1;
                    }
                }
                CompiledOp::Pow { base, exp } =>
                {
                    let u = self._Primal[base];
                    let v = self._Primal[exp];
                    let y = self._Primal[i];
                    if v != 0.0
                    {
                        if v == 1.0
                        {
                            self._Adjoint[base] += adj;
                        }
                        else if u == 0.0 && v < 1.0
                        {
                            return Err(AdError::Eval(EvalError::DivisionByZero));
                        }
                        else
                        {
                            self._Adjoint[base] += adj * v * u.powf(v - 1.0);
                        }
                    }
                    if !matches!(program._Ops[exp], CompiledOp::Const(_))
                    {
                        if u <= 0.0
                        {
                            return Err(AdError::Eval(EvalError::DomainError(format!(
                                "Logarithm in power derivative requires positive base, got {}",
                                u
                            ))));
                        }
                        self._Adjoint[exp] += adj * y * u.ln();
                    }
                }
                CompiledOp::Func { kind, arg } =>
                {
                    let u = self._Primal[arg];
                    let y = self._Primal[i];
                    let df = match kind
                    {
                        CompiledFuncKind::Sqrt =>
                        {
                            if y == 0.0
                            {
                                return Err(AdError::Eval(EvalError::DivisionByZero));
                            }
                            0.5 / y
                        }
                        CompiledFuncKind::Exp => y,
                        CompiledFuncKind::Ln => 1.0 / u,
                        CompiledFuncKind::Sin => u.cos(),
                        CompiledFuncKind::Cos => -u.sin(),
                        CompiledFuncKind::Tan =>
                        {
                            let c = u.cos();
                            1.0 / (c * c)
                        }
                        CompiledFuncKind::Sinh => u.cosh(),
                        CompiledFuncKind::Cosh => u.sinh(),
                        CompiledFuncKind::Tanh => 1.0 - y * y,
                        other => return Err(AdError::UnsupportedAdFunction(other.Name())),
                    };
                    self._Adjoint[arg] += adj * df;
                }
            }
        }
        self._AdjointGeneration = self._PrimalGeneration;
        return Ok(());
    }

    /// LS5: Assembles the full dense `m x n` row-major Jacobian via `n` forward JVP sweeps.
    /// Reuses scratch buffers `seedBuf` (`varCount`), `jvpBuf` (`m`), avoiding heap allocations.
    pub fn AssembleJacobianForward(
        &mut self,
        program: &CompiledProgram,
        activeVarIds: Arr<'_, u32>,
        mut seedBuf: MutArr<'_, f64>,
        mut jvpBuf: MutArr<'_, f64>,
        mut outJac: MutArr<'_, f64>,
    ) -> Result<(), AdError>
    {
        let m = program._RootOps.Size();
        let n = activeVarIds.Size();
        if outJac.Size() != m * n
        {
            return Err(AdError::DimensionMismatch {
                expected: m * n,
                actual:   outJac.Size(),
            });
        }
        if self._PrimalGeneration != self._InputGeneration
        {
            self.Evaluate(program)?;
        }

        let mut v = 0u32;
        while v < seedBuf.Size()
        {
            seedBuf[v] = 0.0;
            v += 1;
        }

        let mut col = 0u32;
        while col < n
        {
            let varId = activeVarIds[col];
            seedBuf[varId] = 1.0;
            self.ForwardJvp(program, seedBuf.Arr(), &mut jvpBuf)?;
            seedBuf[varId] = 0.0;

            let mut row = 0u32;
            while row < m
            {
                outJac[row * n + col] = jvpBuf[row];
                row += 1;
            }
            col += 1;
        }
        return Ok(());
    }

    /// LS5: Assembles the full dense `m x n` row-major Jacobian via `m` reverse VJP sweeps.
    /// Reuses scratch buffers `seedBuf` (`m`), `vjpBuf` (`varCount`), avoiding heap allocations.
    pub fn AssembleJacobianReverse(
        &mut self,
        program: &CompiledProgram,
        activeVarIds: Arr<'_, u32>,
        mut seedBuf: MutArr<'_, f64>,
        mut vjpBuf: MutArr<'_, f64>,
        mut outJac: MutArr<'_, f64>,
    ) -> Result<(), AdError>
    {
        let m = program._RootOps.Size();
        let n = activeVarIds.Size();
        if outJac.Size() != m * n
        {
            return Err(AdError::DimensionMismatch {
                expected: m * n,
                actual:   outJac.Size(),
            });
        }
        if self._PrimalGeneration != self._InputGeneration
        {
            self.Evaluate(program)?;
        }

        let mut r = 0u32;
        while r < seedBuf.Size()
        {
            seedBuf[r] = 0.0;
            r += 1;
        }

        let mut row = 0u32;
        while row < m
        {
            seedBuf[row] = 1.0;
            self.ReverseVjp(program, seedBuf.Arr(), &mut vjpBuf)?;
            seedBuf[row] = 0.0;

            let mut col = 0u32;
            while col < n
            {
                outJac[row * n + col] = vjpBuf[activeVarIds[col]];
                col += 1;
            }
            row += 1;
        }
        return Ok(());
    }
}

//-------------------------------------------------------------------------------------------------
