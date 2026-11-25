// eval.rs -----------------------------------------------------------------------------------------
use crate::fresco::{ExprReadError, ExprRepos, ExprView, LogicOp, TensorExpr, VarKind};
use crate::silo::{Arr, Buff, Stash};
use core::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum EvalError
{
    DivisionByZero,
    DomainError(String),
    UndefinedVariable(u32),
    UndefinedVariableName(String),
    InvalidNode(u32),
    RecursionLimitExceeded,
    AssumptionViolated
    {
        var_id: u32,
        reason: &'static str,
    },
    UnknownFunction(String),
    UnsupportedNode(u32),
    NodeBudgetExceeded
    {
        limit:   u32,
        visited: u32,
    },
    EdgeBudgetExceeded
    {
        limit:     u32,
        traversed: u32,
    },
    SharedVisitBudgetExceeded
    {
        limit:         u32,
        shared_visits: u32,
    },
    ResultBudgetExceeded
    {
        limit:    u32,
        computed: u32,
    },
    DeadlineExceeded,
    Cancelled,
}

//-------------------------------------------------------------------------------------------------

impl fmt::Display for EvalError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Self::DivisionByZero => write!(f, "Division by zero"),
            Self::DomainError(msg) => write!(f, "Domain error: {}", msg),
            Self::UndefinedVariable(id) => write!(f, "Undefined variable ID {}", id),
            Self::UndefinedVariableName(name) => write!(f, "Undefined variable '{}'", name),
            Self::InvalidNode(id) => write!(f, "Invalid expression node ID {}", id),
            Self::RecursionLimitExceeded => write!(f, "Evaluator recursion limit exceeded"),
            Self::AssumptionViolated { var_id, reason } =>
            {
                write!(f, "Assumption violated for variable ID {}: {}", var_id, reason)
            }
            Self::UnknownFunction(name) => write!(f, "Unknown function '{}'", name),
            Self::UnsupportedNode(id) => write!(f, "Unsupported expression node ID {}", id),
            Self::NodeBudgetExceeded { limit, visited } =>
            {
                write!(f, "Node budget exceeded: visited {} > limit {}", visited, limit)
            }
            Self::EdgeBudgetExceeded { limit, traversed } =>
            {
                write!(f, "Edge budget exceeded: traversed {} > limit {}", traversed, limit)
            }
            Self::SharedVisitBudgetExceeded { limit, shared_visits } =>
            {
                write!(f, "Shared-node visit budget exceeded: visited {} > limit {}", shared_visits, limit)
            }
            Self::ResultBudgetExceeded { limit, computed } =>
            {
                write!(f, "Result budget exceeded: computed {} > limit {}", computed, limit)
            }
            Self::DeadlineExceeded => write!(f, "Evaluation deadline exceeded"),
            Self::Cancelled => write!(f, "Evaluation cancelled"),
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Configurable resource limits and cooperative cancellation boundaries for evaluation.
#[derive(Clone, Debug)]
pub struct EvalLimits<'a>
{
    pub _MaxDepth:        u32,
    pub _MaxNodes:        u32,
    pub _MaxEdges:        u32,
    pub _MaxResults:      u32,
    pub _MaxSharedVisits: u32,
    pub _Deadline:        Option<Instant>,
    pub _CancelFlag:      Option<&'a AtomicBool>,
}

impl<'a> Default for EvalLimits<'a>
{
    fn default() -> Self
    {
        Self {
            _MaxDepth:        ExprRepos::K_MAX_TREE_DEPTH,
            _MaxNodes:        100_000,
            _MaxEdges:        200_000,
            _MaxResults:      10_000,
            _MaxSharedVisits: 50_000,
            _Deadline:        None,
            _CancelFlag:      None,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Operational work counters recorded during expression evaluation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EvalCounters
{
    pub _NodesVisited:    u32,
    pub _EdgesTraversed:  u32,
    pub _SharedVisits:    u32,
    pub _ResultsComputed: u32,
    pub _PeakDepth:       u32,
}

//-------------------------------------------------------------------------------------------------

/// Evaluated outcome including computed value and execution telemetry.
#[derive(Clone, Debug, PartialEq)]
pub struct EvalOutcome<T>
{
    pub _Value:    T,
    pub _Counters: EvalCounters,
}

//-------------------------------------------------------------------------------------------------

struct EvalState<'s, 'a>
{
    limits:      &'s EvalLimits<'a>,
    counters:    EvalCounters,
    visitCounts: Buff<u32>,
}

//-------------------------------------------------------------------------------------------------

impl ExprRepos
{
    /// Evaluates an expression root subject to explicit budgets and cancellation checkpoints,
    /// retaining work counters on both success and failure.
    pub fn EvaluateOutcomeWithLimits(
        &self,
        root: u32,
        bindings: Arr<'_, f64>,
        limits: &EvalLimits<'_>,
    ) -> EvalOutcome<Result<f64, EvalError>>
    {
        let mut state = EvalState {
            limits,
            counters: EvalCounters::default(),
            visitCounts: Buff::FromDispenser(self.Size(), |_| 0),
        };
        let res = match self.EvaluateNode(root, bindings, 0, &mut state)
        {
            Ok(val) =>
            {
                state.counters._ResultsComputed = state.counters._ResultsComputed.saturating_add(1);
                if state.counters._ResultsComputed > state.limits._MaxResults
                {
                    Err(EvalError::ResultBudgetExceeded {
                        limit:    state.limits._MaxResults,
                        computed: state.counters._ResultsComputed,
                    })
                }
                else
                {
                    Ok(val)
                }
            }
            Err(err) => Err(err),
        };
        return EvalOutcome {
            _Value:    res,
            _Counters: state.counters,
        };
    }

    /// Evaluates an expression root subject to explicit budgets and cancellation checkpoints.
    pub fn EvaluateWithLimits(
        &self,
        root: u32,
        bindings: Arr<'_, f64>,
        limits: &EvalLimits<'_>,
    ) -> Result<EvalOutcome<f64>, EvalError>
    {
        let outcome = self.EvaluateOutcomeWithLimits(root, bindings, limits);
        return match outcome._Value
        {
            Ok(val) => Ok(EvalOutcome {
                _Value:    val,
                _Counters: outcome._Counters,
            }),
            Err(err) => Err(err),
        };
    }

    /// Evaluates an expression root using default resource limits.
    pub fn Evaluate(&self, root: u32, bindings: Arr<'_, f64>) -> Result<f64, EvalError>
    {
        let outcome = self.EvaluateWithLimits(root, bindings, &EvalLimits::default())?;
        return Ok(outcome._Value);
    }

    /// Evaluates an expression root with named variable bindings and explicit limits,
    /// retaining work counters on both success and failure.
    pub fn EvaluateNamedOutcomeWithLimits(
        &self,
        root: u32,
        namedBindings: Arr<'_, (&str, f64)>,
        limits: &EvalLimits<'_>,
    ) -> EvalOutcome<Result<f64, EvalError>>
    {
        let varCount = self.SzVar();
        let mut values = Buff::<f64>::FromDispenser(varCount, |_| f64::NAN);
        let bindCount = namedBindings.Size();
        let mut b = 0u32;
        while b < bindCount
        {
            let (name, val) = namedBindings[b];
            let varId = self.FindVar(name);
            if varId == u32::MAX
            {
                return EvalOutcome {
                    _Value:    Err(EvalError::UndefinedVariableName(name.to_string())),
                    _Counters: EvalCounters::default(),
                };
            }
            values[varId] = val;
            b += 1;
        }

        return self.EvaluateOutcomeWithLimits(root, values.Arr(), limits);
    }

    /// Evaluates an expression root with named variable bindings and explicit limits.
    pub fn EvaluateNamedWithLimits(
        &self,
        root: u32,
        namedBindings: Arr<'_, (&str, f64)>,
        limits: &EvalLimits<'_>,
    ) -> Result<EvalOutcome<f64>, EvalError>
    {
        let outcome = self.EvaluateNamedOutcomeWithLimits(root, namedBindings, limits);
        return match outcome._Value
        {
            Ok(val) => Ok(EvalOutcome {
                _Value:    val,
                _Counters: outcome._Counters,
            }),
            Err(err) => Err(err),
        };
    }

    /// Evaluates an expression root with named variable bindings using default resource limits.
    pub fn EvaluateNamed(
        &self,
        root: u32,
        namedBindings: Arr<'_, (&str, f64)>,
    ) -> Result<f64, EvalError>
    {
        let outcome = self.EvaluateNamedWithLimits(root, namedBindings, &EvalLimits::default())?;
        return Ok(outcome._Value);
    }

    /// Evaluates every element of a tensor expression with explicit resource limits,
    /// retaining work counters on both success and failure.
    pub fn EvaluateTensorOutcomeWithLimits(
        &self,
        tensor: &TensorExpr,
        bindings: Arr<'_, f64>,
        limits: &EvalLimits<'_>,
    ) -> EvalOutcome<Result<Buff<f64>, EvalError>>
    {
        let mut state = EvalState {
            limits,
            counters: EvalCounters::default(),
            visitCounts: Buff::FromDispenser(self.Size(), |_| 0),
        };
        let total = tensor.TotalElements();
        let mut stash = Stash::WithCapacity(total);
        let mut k = 0u32;
        while k < total
        {
            let exprId = tensor._Elems[k];
            let val = match self.EvaluateNode(exprId, bindings, 0, &mut state)
            {
                Ok(v) => v,
                Err(err) =>
                {
                    return EvalOutcome {
                        _Value:    Err(err),
                        _Counters: state.counters,
                    };
                }
            };
            state.counters._ResultsComputed = state.counters._ResultsComputed.saturating_add(1);
            if state.counters._ResultsComputed > state.limits._MaxResults
            {
                return EvalOutcome {
                    _Value:    Err(EvalError::ResultBudgetExceeded {
                        limit:    state.limits._MaxResults,
                        computed: state.counters._ResultsComputed,
                    }),
                    _Counters: state.counters,
                };
            }
            stash.Push(val);
            k += 1;
        }
        return EvalOutcome {
            _Value:    Ok(stash.IntoBuff()),
            _Counters: state.counters,
        };
    }

    /// Evaluates every element of a tensor expression with explicit resource limits.
    pub fn EvaluateTensorWithLimits(
        &self,
        tensor: &TensorExpr,
        bindings: Arr<'_, f64>,
        limits: &EvalLimits<'_>,
    ) -> Result<EvalOutcome<Buff<f64>>, EvalError>
    {
        let outcome = self.EvaluateTensorOutcomeWithLimits(tensor, bindings, limits);
        return match outcome._Value
        {
            Ok(val) => Ok(EvalOutcome {
                _Value:    val,
                _Counters: outcome._Counters,
            }),
            Err(err) => Err(err),
        };
    }

    /// Evaluates every element of a tensor expression using default resource limits.
    pub fn EvaluateTensor(
        &self,
        tensor: &TensorExpr,
        bindings: Arr<'_, f64>,
    ) -> Result<Buff<f64>, EvalError>
    {
        let outcome = self.EvaluateTensorWithLimits(tensor, bindings, &EvalLimits::default())?;
        return Ok(outcome._Value);
    }

    /// Evaluates every element of a tensor with named bindings and explicit limits.
    pub fn EvaluateTensorNamedWithLimits(
        &self,
        tensor: &TensorExpr,
        namedBindings: Arr<'_, (&str, f64)>,
        limits: &EvalLimits<'_>,
    ) -> Result<EvalOutcome<Buff<f64>>, EvalError>
    {
        let varCount = self.SzVar();
        let mut values = Buff::<f64>::FromDispenser(varCount, |_| f64::NAN);
        let bindCount = namedBindings.Size();
        let mut b = 0u32;
        while b < bindCount
        {
            let (name, val) = namedBindings[b];
            let varId = self.FindVar(name);
            if varId == u32::MAX
            {
                return Err(EvalError::UndefinedVariableName(name.to_string()));
            }
            values[varId] = val;
            b += 1;
        }
        return self.EvaluateTensorWithLimits(tensor, values.Arr(), limits);
    }

    /// Evaluates every element of a tensor with named variable bindings using default resource limits.
    pub fn EvaluateTensorNamed(
        &self,
        tensor: &TensorExpr,
        namedBindings: Arr<'_, (&str, f64)>,
    ) -> Result<Buff<f64>, EvalError>
    {
        let outcome = self.EvaluateTensorNamedWithLimits(tensor, namedBindings, &EvalLimits::default())?;
        return Ok(outcome._Value);
    }

    fn StepChild(
        &self,
        childId: u32,
        bindings: Arr<'_, f64>,
        depth: u32,
        state: &mut EvalState<'_, '_>,
    ) -> Result<f64, EvalError>
    {
        state.counters._EdgesTraversed = state.counters._EdgesTraversed.saturating_add(1);
        if state.counters._EdgesTraversed > state.limits._MaxEdges
        {
            return Err(EvalError::EdgeBudgetExceeded {
                limit:     state.limits._MaxEdges,
                traversed: state.counters._EdgesTraversed,
            });
        }
        return self.EvaluateNode(childId, bindings, depth + 1, state);
    }

    fn EvaluateNode(
        &self,
        id: u32,
        bindings: Arr<'_, f64>,
        depth: u32,
        state: &mut EvalState<'_, '_>,
    ) -> Result<f64, EvalError>
    {
        if let Some(flag) = state.limits._CancelFlag
            && flag.load(Ordering::Relaxed)
        {
            return Err(EvalError::Cancelled);
        }
        if let Some(deadline) = state.limits._Deadline
            && Instant::now() >= deadline
        {
            return Err(EvalError::DeadlineExceeded);
        }
        if depth >= state.limits._MaxDepth
        {
            return Err(EvalError::RecursionLimitExceeded);
        }
        state.counters._PeakDepth = state.counters._PeakDepth.max(depth);

        if !self.IsValidExprId(id)
        {
            return Err(EvalError::InvalidNode(id));
        }

        state.counters._NodesVisited = state.counters._NodesVisited.saturating_add(1);
        if state.counters._NodesVisited > state.limits._MaxNodes
        {
            return Err(EvalError::NodeBudgetExceeded {
                limit:   state.limits._MaxNodes,
                visited: state.counters._NodesVisited,
            });
        }

        if id < state.visitCounts.Size()
        {
            let count = state.visitCounts[id];
            if count > 0
            {
                state.counters._SharedVisits = state.counters._SharedVisits.saturating_add(1);
                if state.counters._SharedVisits > state.limits._MaxSharedVisits
                {
                    return Err(EvalError::SharedVisitBudgetExceeded {
                        limit:         state.limits._MaxSharedVisits,
                        shared_visits: state.counters._SharedVisits,
                    });
                }
            }
            state.visitCounts[id] = count.saturating_add(1);
        }

        match self.Inspect(id)
        {
            Err(ExprReadError::InvalidId(_) | ExprReadError::EmptyNode(_)) =>
            {
                return Err(EvalError::InvalidNode(id));
            }
            Err(ExprReadError::UnsupportedNode(_)) =>
            {
                return Err(EvalError::UnsupportedNode(id));
            }
            Ok(node) =>
            {
                match node
                {
                    ExprView::Real(value) =>
                    {
                        return Ok(value);
                    }
                    ExprView::Var(varIndex) =>
                    {
                        if !self.IsValidVarId(varIndex) || varIndex >= bindings.Size()
                        {
                            return Err(EvalError::UndefinedVariable(varIndex));
                        }
                        let val = bindings[varIndex];
                        if val.is_nan()
                        {
                            return Err(EvalError::DomainError(format!(
                                "Variable '{}' (ID {}) is uninitialized or bound to NaN",
                                self.VarNameAt(varIndex),
                                varIndex,
                            )));
                        }
                        let attrib = self.VarAttrAt(varIndex);
                        if attrib.HasBits(VarKind::NonZero) && (val == 0.0 || val == -0.0)
                        {
                            return Err(EvalError::AssumptionViolated {
                                var_id: varIndex,
                                reason: "NonZero variable cannot be bound to zero",
                            });
                        }
                        return Ok(val);
                    }
                    ExprView::Sum(poly) =>
                    {
                        let coSize = poly.CommonSize();
                        let size = poly.SizeChild();
                        let mut total = 0.0;
                        let mut i = 0u32;
                        while i < coSize
                        {
                            let t = self.StepChild(poly.Child(i), bindings, depth, state)?;
                            total += t;
                            i += 1;
                        }
                        while i < size
                        {
                            let t = self.StepChild(poly.Child(i), bindings, depth, state)?;
                            total -= t;
                            i += 1;
                        }
                        return Ok(total);
                    }
                    ExprView::Prod(poly) =>
                    {
                        let coSize = poly.CommonSize();
                        let size = poly.SizeChild();
                        let mut numer = 1.0;
                        let mut denom = 1.0;
                        let mut i = 0u32;
                        while i < coSize
                        {
                            let factor = self.StepChild(poly.Child(i), bindings, depth, state)?;
                            numer *= factor;
                            i += 1;
                        }
                        while i < size
                        {
                            let factor = self.StepChild(poly.Child(i), bindings, depth, state)?;
                            if factor == 0.0 || factor == -0.0
                            {
                                return Err(EvalError::DivisionByZero);
                            }
                            denom *= factor;
                            i += 1;
                        }
                        if denom == 0.0 || denom == -0.0
                        {
                            return Err(EvalError::DivisionByZero);
                        }
                        return Ok(numer / denom);
                    }
                    ExprView::Pow(poly) =>
                    {
                        if poly.SizeChild() < 2
                        {
                            return Err(EvalError::DomainError("Power expression missing operands".to_string()));
                        }
                        let base = self.StepChild(poly.Child(0), bindings, depth, state)?;
                        let exponent = self.StepChild(poly.Child(1), bindings, depth, state)?;
                        if base == 0.0 && exponent < 0.0
                        {
                            return Err(EvalError::DivisionByZero);
                        }
                        if base < 0.0 && exponent.fract() != 0.0
                        {
                            return Err(EvalError::DomainError(format!(
                                "Negative base {} cannot be raised to non-integer power {}",
                                base, exponent,
                            )));
                        }
                        return Ok(base.powf(exponent));
                    }
                    ExprView::Cond { condition, ifTrue, ifFalse } =>
                    {
                        let c = self.StepChild(condition, bindings, depth, state)?;
                        let isTrue = c != 0.0 && !c.is_nan();
                        return if isTrue
                        {
                            self.StepChild(ifTrue, bindings, depth, state)
                        }
                        else
                        {
                            self.StepChild(ifFalse, bindings, depth, state)
                        };
                    }
                    ExprView::Logic { op, left, right } =>
                    {
                        if right == u32::MAX
                        {
                            if op != LogicOp::Not
                            {
                                return Err(EvalError::DomainError("Unary logic operator must be Not".to_string()));
                            }
                            let val = self.StepChild(left, bindings, depth, state)?;
                            let isTrue = val != 0.0 && !val.is_nan();
                            return Ok(if isTrue { 0.0 } else { 1.0 });
                        }
                        match op
                        {
                            LogicOp::And =>
                            {
                                let l = self.StepChild(left, bindings, depth, state)?;
                                if l == 0.0 || l.is_nan()
                                {
                                    return Ok(0.0);
                                }
                                let r = self.StepChild(right, bindings, depth, state)?;
                                return Ok(if r != 0.0 && !r.is_nan() { 1.0 } else { 0.0 });
                            }
                            LogicOp::Or =>
                            {
                                let l = self.StepChild(left, bindings, depth, state)?;
                                if l != 0.0 && !l.is_nan()
                                {
                                    return Ok(1.0);
                                }
                                let r = self.StepChild(right, bindings, depth, state)?;
                                return Ok(if r != 0.0 && !r.is_nan() { 1.0 } else { 0.0 });
                            }
                            LogicOp::Eq =>
                            {
                                let l = self.StepChild(left, bindings, depth, state)?;
                                let r = self.StepChild(right, bindings, depth, state)?;
                                return Ok(if l == r { 1.0 } else { 0.0 });
                            }
                            LogicOp::Ne =>
                            {
                                let l = self.StepChild(left, bindings, depth, state)?;
                                let r = self.StepChild(right, bindings, depth, state)?;
                                return Ok(if l != r { 1.0 } else { 0.0 });
                            }
                            LogicOp::Lt =>
                            {
                                let l = self.StepChild(left, bindings, depth, state)?;
                                let r = self.StepChild(right, bindings, depth, state)?;
                                return Ok(if l < r { 1.0 } else { 0.0 });
                            }
                            LogicOp::Le =>
                            {
                                let l = self.StepChild(left, bindings, depth, state)?;
                                let r = self.StepChild(right, bindings, depth, state)?;
                                return Ok(if l <= r { 1.0 } else { 0.0 });
                            }
                            LogicOp::Gt =>
                            {
                                let l = self.StepChild(left, bindings, depth, state)?;
                                let r = self.StepChild(right, bindings, depth, state)?;
                                return Ok(if l > r { 1.0 } else { 0.0 });
                            }
                            LogicOp::Ge =>
                            {
                                let l = self.StepChild(left, bindings, depth, state)?;
                                let r = self.StepChild(right, bindings, depth, state)?;
                                return Ok(if l >= r { 1.0 } else { 0.0 });
                            }
                            LogicOp::Not => unreachable!(),
                        }
                    }
                    ExprView::Func { name, arg: argId } =>
                    {
                        let arg = self.StepChild(argId, bindings, depth, state)?;
                        return match name
                        {
                            "abs" => Ok(arg.abs()),
                            "sqrt" =>
                            {
                                if arg < 0.0
                                {
                                    Err(EvalError::DomainError(format!("sqrt of negative value {}", arg)))
                                }
                                else
                                {
                                    Ok(arg.sqrt())
                                }
                            }
                            "exp" => Ok(arg.exp()),
                            "ln" | "log" =>
                            {
                                if arg <= 0.0
                                {
                                    Err(EvalError::DomainError(format!("log of non-positive value {}", arg)))
                                }
                                else
                                {
                                    Ok(arg.ln())
                                }
                            }
                            "log10" =>
                            {
                                if arg <= 0.0
                                {
                                    Err(EvalError::DomainError(format!("log10 of non-positive value {}", arg)))
                                }
                                else
                                {
                                    Ok(arg.log10())
                                }
                            }
                            "sin" => Ok(arg.sin()),
                            "cos" => Ok(arg.cos()),
                            "tan" => Ok(arg.tan()),
                            "sinh" => Ok(arg.sinh()),
                            "cosh" => Ok(arg.cosh()),
                            "tanh" => Ok(arg.tanh()),
                            "floor" => Ok(arg.floor()),
                            "ceil" => Ok(arg.ceil()),
                            "round" => Ok(arg.round()),
                            other => Err(EvalError::UnknownFunction(other.to_string())),
                        };
                    }
                }
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------
