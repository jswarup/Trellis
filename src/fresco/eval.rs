// eval.rs -----------------------------------------------------------------------------------------
use	crate::fresco::{
    CondExpr, ExprEntry, ExprRepos, FuncExpr, LogicExpr, LogicOp, PowExpr, ProdExpr, RealExpr, SumExpr,
    VarExpr, VarKind,
};
use	crate::silo::{ Arr, Buff };
use	core::fmt;

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Debug, PartialEq)]
pub enum EvalError
{
    DivisionByZero,
    DomainError( String),
    UndefinedVariable( u32),
    UndefinedVariableName( String),
    InvalidNode( u32),
    RecursionLimitExceeded,
    AssumptionViolated
    {
        var_id: u32,
        reason: &'static str,
    },
    UnknownFunction( String),
    UnsupportedNode( u32),
}

//-------------------------------------------------------------------------------------------------

impl fmt::Display for EvalError
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        match self
        {
            Self::DivisionByZero => write!( f, "Division by zero"),
            Self::DomainError( msg) => write!( f, "Domain error: {}", msg),
            Self::UndefinedVariable( id) => write!( f, "Undefined variable ID {}", id),
            Self::UndefinedVariableName( name) => write!( f, "Undefined variable '{}'", name),
            Self::InvalidNode( id) => write!( f, "Invalid expression node ID {}", id),
            Self::RecursionLimitExceeded => write!( f, "Evaluator recursion limit exceeded"),
            Self::AssumptionViolated { var_id, reason } =>
            {
                write!( f, "Assumption violated for variable ID {}: {}", var_id, reason)
            }
            Self::UnknownFunction( name) => write!( f, "Unknown function '{}'", name),
            Self::UnsupportedNode( id) => write!( f, "Unsupported expression node ID {}", id),
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl ExprRepos
{
    pub fn	Evaluate( &self, root: u32, bindings: Arr< '_, f64>) -> Result< f64, EvalError>
    {
        self.EvaluateWithDepth( root, bindings, 0)
    }

    pub fn	EvaluateNamed(
        &self,
        root: u32,
        namedBindings: Arr< '_, ( &str, f64)>,
    ) -> Result< f64, EvalError>
    {
        let  	varCount = self.SzVar();
        let  	mut values = Buff::< f64>::FromDispenser( varCount, |_| f64::NAN);
        let  	bindCount = namedBindings.Size();
        let  	mut b = 0u32;
        while b < bindCount
        {
            let  	( name, val) = namedBindings[b];
            let  	varId = self.FindVar( name);
            if varId == u32::MAX
            {
                return Err( EvalError::UndefinedVariableName( name.to_string()));
            }
            values[varId] = val;
            b += 1;
        }

        self.Evaluate( root, values.Arr())
    }

    fn	EvaluateWithDepth( &self, id: u32, bindings: Arr< '_, f64>, depth: u32) -> Result< f64, EvalError>
    {
        if depth >= Self::K_MAX_TREE_DEPTH
        {
            return Err( EvalError::RecursionLimitExceeded);
        }
        if !self.IsValidExprId( id)
        {
            return Err( EvalError::InvalidNode( id));
        }

        match self.ExprEntryAt( id)
        {
            ExprEntry::Empty => Err( EvalError::InvalidNode( id)),
            ExprEntry::Expr( expression) =>
            {
                let  	any = expression.Any();
                if let  	Some( real) = any.downcast_ref::< RealExpr>()
                {
                    return Ok( real.Value());
                }
                if let  	Some( var) = any.downcast_ref::< VarExpr>()
                {
                    let  	varIndex = var.VarIndex();
                    if !self.IsValidVarId( varIndex) || varIndex >= bindings.Size()
                    {
                        return Err( EvalError::UndefinedVariable( varIndex));
                    }
                    let  	val = bindings[varIndex];
                    if val.is_nan()
                    {
                        return Err( EvalError::DomainError( format!(
                            "Variable '{}' (ID {}) is uninitialized or bound to NaN",
                            self.VarNameAt( varIndex),
                            varIndex,
                        )));
                    }
                    let  	attrib = self.VarAttrAt( varIndex);
                    if attrib.HasBits( VarKind::NonZero) && ( val == 0.0 || val == -0.0)
                    {
                        return Err( EvalError::AssumptionViolated
                        {
                            var_id: varIndex,
                            reason: "NonZero variable cannot be bound to zero",
                        });
                    }
                    return Ok( val);
                }
                if let  	Some( sum) = any.downcast_ref::< SumExpr>()
                {
                    let  	poly = sum.Poly();
                    let  	coSize = poly.CommonSize();
                    let  	size = poly.SizeChild();
                    let  	mut total = 0.0;
                    let  	mut i = 0u32;
                    while i < coSize
                    {
                        let  	t = self.EvaluateWithDepth( poly.Child( i), bindings, depth + 1)?;
                        total += t;
                        i += 1;
                    }
                    while i < size
                    {
                        let  	t = self.EvaluateWithDepth( poly.Child( i), bindings, depth + 1)?;
                        total -= t;
                        i += 1;
                    }
                    return Ok( total);
                }
                if let  	Some( prod) = any.downcast_ref::< ProdExpr>()
                {
                    let  	poly = prod.Poly();
                    let  	coSize = poly.CommonSize();
                    let  	size = poly.SizeChild();
                    let  	mut numer = 1.0;
                    let  	mut denom = 1.0;
                    let  	mut i = 0u32;
                    while i < coSize
                    {
                        let  	factor = self.EvaluateWithDepth( poly.Child( i), bindings, depth + 1)?;
                        numer *= factor;
                        i += 1;
                    }
                    while i < size
                    {
                        let  	factor = self.EvaluateWithDepth( poly.Child( i), bindings, depth + 1)?;
                        if factor == 0.0 || factor == -0.0
                        {
                            return Err( EvalError::DivisionByZero);
                        }
                        denom *= factor;
                        i += 1;
                    }
                    if denom == 0.0 || denom == -0.0
                    {
                        return Err( EvalError::DivisionByZero);
                    }
                    return Ok( numer / denom);
                }
                if let  	Some( pow) = any.downcast_ref::< PowExpr>()
                {
                    let  	poly = pow.Poly();
                    if poly.SizeChild() < 2
                    {
                        return Err( EvalError::DomainError( "Power expression missing operands".to_string()));
                    }
                    let  	base = self.EvaluateWithDepth( poly.Child( 0), bindings, depth + 1)?;
                    let  	exponent = self.EvaluateWithDepth( poly.Child( 1), bindings, depth + 1)?;
                    if base == 0.0 && exponent < 0.0
                    {
                        return Err( EvalError::DivisionByZero);
                    }
                    if base < 0.0 && exponent.fract() != 0.0
                    {
                        return Err( EvalError::DomainError( format!(
                            "Negative base {} cannot be raised to non-integer power {}",
                            base, exponent,
                        )));
                    }
                    return Ok( base.powf( exponent));
                }
                if let  	Some( cond) = any.downcast_ref::< CondExpr>()
                {
                    let  	c = self.EvaluateWithDepth( cond.Cond(), bindings, depth + 1)?;
                    let  	isTrue = c != 0.0 && !c.is_nan();
                    return if isTrue
                    {
                        self.EvaluateWithDepth( cond.IfTrue(), bindings, depth + 1)
                    }
                    else
                    {
                        self.EvaluateWithDepth( cond.IfFalse(), bindings, depth + 1)
                    };
                }
                if let  	Some( logic) = any.downcast_ref::< LogicExpr>()
                {
                    let  	op = logic.Op();
                    if logic.IsUnary()
                    {
                        if op != LogicOp::Not
                        {
                            return Err( EvalError::DomainError( "Unary logic operator must be Not".to_string()));
                        }
                        let  	val = self.EvaluateWithDepth( logic.Left(), bindings, depth + 1)?;
                        let  	isTrue = val != 0.0 && !val.is_nan();
                        return Ok( if isTrue { 0.0 } else { 1.0 });
                    }
                    match op
                    {
                        LogicOp::And =>
                        {
                            let  	l = self.EvaluateWithDepth( logic.Left(), bindings, depth + 1)?;
                            if l == 0.0 || l.is_nan()
                            {
                                return Ok( 0.0);
                            }
                            let  	r = self.EvaluateWithDepth( logic.Right(), bindings, depth + 1)?;
                            return Ok( if r != 0.0 && !r.is_nan() { 1.0 } else { 0.0 });
                        }
                        LogicOp::Or =>
                        {
                            let  	l = self.EvaluateWithDepth( logic.Left(), bindings, depth + 1)?;
                            if l != 0.0 && !l.is_nan()
                            {
                                return Ok( 1.0);
                            }
                            let  	r = self.EvaluateWithDepth( logic.Right(), bindings, depth + 1)?;
                            return Ok( if r != 0.0 && !r.is_nan() { 1.0 } else { 0.0 });
                        }
                        LogicOp::Eq =>
                        {
                            let  	l = self.EvaluateWithDepth( logic.Left(), bindings, depth + 1)?;
                            let  	r = self.EvaluateWithDepth( logic.Right(), bindings, depth + 1)?;
                            return Ok( if l == r { 1.0 } else { 0.0 });
                        }
                        LogicOp::Ne =>
                        {
                            let  	l = self.EvaluateWithDepth( logic.Left(), bindings, depth + 1)?;
                            let  	r = self.EvaluateWithDepth( logic.Right(), bindings, depth + 1)?;
                            return Ok( if l != r { 1.0 } else { 0.0 });
                        }
                        LogicOp::Lt =>
                        {
                            let  	l = self.EvaluateWithDepth( logic.Left(), bindings, depth + 1)?;
                            let  	r = self.EvaluateWithDepth( logic.Right(), bindings, depth + 1)?;
                            return Ok( if l < r { 1.0 } else { 0.0 });
                        }
                        LogicOp::Le =>
                        {
                            let  	l = self.EvaluateWithDepth( logic.Left(), bindings, depth + 1)?;
                            let  	r = self.EvaluateWithDepth( logic.Right(), bindings, depth + 1)?;
                            return Ok( if l <= r { 1.0 } else { 0.0 });
                        }
                        LogicOp::Gt =>
                        {
                            let  	l = self.EvaluateWithDepth( logic.Left(), bindings, depth + 1)?;
                            let  	r = self.EvaluateWithDepth( logic.Right(), bindings, depth + 1)?;
                            return Ok( if l > r { 1.0 } else { 0.0 });
                        }
                        LogicOp::Ge =>
                        {
                            let  	l = self.EvaluateWithDepth( logic.Left(), bindings, depth + 1)?;
                            let  	r = self.EvaluateWithDepth( logic.Right(), bindings, depth + 1)?;
                            return Ok( if l >= r { 1.0 } else { 0.0 });
                        }
                        LogicOp::Not => unreachable!(),
                    }
                }
                if let  	Some( func) = any.downcast_ref::< FuncExpr>()
                {
                    let  	arg = self.EvaluateWithDepth( func.Arg(), bindings, depth + 1)?;
                    return match func.Name()
                    {
                        "abs" => Ok( arg.abs()),
                        "sqrt" =>
                        {
                            if arg < 0.0
                            {
                                Err( EvalError::DomainError( format!( "sqrt of negative value {}", arg)))
                            }
                            else
                            {
                                Ok( arg.sqrt())
                            }
                        }
                        "exp" => Ok( arg.exp()),
                        "ln" | "log" =>
                        {
                            if arg <= 0.0
                            {
                                Err( EvalError::DomainError( format!( "log of non-positive value {}", arg)))
                            }
                            else
                            {
                                Ok( arg.ln())
                            }
                        }
                        "log10" =>
                        {
                            if arg <= 0.0
                            {
                                Err( EvalError::DomainError( format!( "log10 of non-positive value {}", arg)))
                            }
                            else
                            {
                                Ok( arg.log10())
                            }
                        }
                        "sin" => Ok( arg.sin()),
                        "cos" => Ok( arg.cos()),
                        "tan" => Ok( arg.tan()),
                        "sinh" => Ok( arg.sinh()),
                        "cosh" => Ok( arg.cosh()),
                        "tanh" => Ok( arg.tanh()),
                        "floor" => Ok( arg.floor()),
                        "ceil" => Ok( arg.ceil()),
                        "round" => Ok( arg.round()),
                        other => Err( EvalError::UnknownFunction( other.to_string())),
                    };
                }
                Err( EvalError::UnsupportedNode( id))
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------
