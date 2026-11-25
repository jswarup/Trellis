//-- diff.rs ---------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use crate::fresco::{ExprReadError, ExprRepos, ExprView, PolyExpr, TensorExpr, TensorShape};
use crate::silo::{Arr, Buff, Stash, USeg};

//--------------------------------------------------------------------------------------------------
/// Computes the exact symbolic derivative of an expression with respect to a target variable.
///
/// The operation is transactional: if differentiation fails at any subexpression, all
/// partially constructed derivative nodes are rolled back cleanly.
pub fn Differentiate(repos: &mut ExprRepos, exprId: u32, varId: u32) -> Result<u32, String>
{
    return repos.WithTransaction( |r| DifferentiateInner( r, exprId, varId));
}

fn DifferentiateInner(repos: &mut ExprRepos, exprId: u32, varId: u32) -> Result<u32, String>
{
    let node = repos.Inspect( exprId).map_err( |error| match error {
        ExprReadError::InvalidId( _) => format!( "Invalid expression ID {exprId}"),
        ExprReadError::EmptyNode( _) => format!( "Empty expression ID {exprId}"),
        ExprReadError::UnsupportedNode( _) => format!( "Symbolic differentiation not supported for node ID {exprId}"),
    })?;
    return match node {
        ExprView::Real(_) => Ok(repos.RealCreate(0.0)),
        ExprView::Var(varIndex) => Ok(repos.RealCreate(if varIndex == varId { 1.0 } else { 0.0 })),
        ExprView::Sum(poly) => {
            // Snapshot child IDs once before appending derivative nodes to the repository.
            let poly = PolyExpr::FromParts(poly.CommonSize(), Buff::FromArr(poly.Children()));
            DiffSum(repos, &poly, varId)
        }
        ExprView::Prod(poly) => {
            let poly = PolyExpr::FromParts(poly.CommonSize(), Buff::FromArr(poly.Children()));
            DiffProd(repos, &poly, varId)
        }
        ExprView::Pow(poly) => {
            if poly.SizeChild() < 2 {
                return Err(format!("Power expression missing operands at node ID {exprId}"));
            }
            let base = poly.Child(0);
            let exponent = poly.Child(1);
            DiffPow(repos, base, exponent, varId)
        }
        ExprView::Func { name, arg } => {
            let name = name.to_string();
            DiffFunc(repos, &name, arg, varId)
        }
        _ => Err(format!("Symbolic differentiation not supported for node ID {exprId}")),
    };
}

//--------------------------------------------------------------------------------------------------
fn	DiffSum(
    repos: &mut ExprRepos, poly: &crate::fresco::PolyExpr, varId: u32,
) -> Result< u32, String>
{
    let  	coSize = poly.CommonSize();
    let  	size = poly.SizeChild();

    let  	mut addsStash = Stash::New();
    let  	mut subsStash = Stash::New();

    let  	mut i = 0u32;
    while i < coSize
    {
        let  	dChild = DifferentiateInner( repos, poly.Child( i), varId)?;
        addsStash.Push( dChild);
        i += 1;
    }
    while i < size
    {
        let  	dChild = DifferentiateInner( repos, poly.Child( i), varId)?;
        subsStash.Push( dChild);
        i += 1;
    }

    let  	adds = addsStash.IntoBuff();
    let  	subs = subsStash.IntoBuff();
    Ok( repos.SumCreate( adds.Arr(), subs.Arr()))
}

//--------------------------------------------------------------------------------------------------
fn	DiffProd(
    repos: &mut ExprRepos, poly: &crate::fresco::PolyExpr, varId: u32,
) -> Result< u32, String>
{
    let  	coSize = poly.CommonSize();
    let  	size = poly.SizeChild();

    if size == 0
    {
        return Ok( repos.RealCreate( 0.0));
    }

    let  	numChild = coSize;
    let  	denChild = size - coSize;

    // Pure product: d/dx (u0 * u1 * ... * un-1) = sum( u_i' * prod(u_j, j!=i) )
    if denChild == 0
    {
        return DiffPureProduct( repos, poly, 0, numChild, varId);
    }

    // Quotient: d/dx (N / D) = (N' * D - N * D') / D^2
    let  	nExpr = if numChild == 1 {
        poly.Child( 0)
    } else {
        let  	mut numStash = Stash::New();
        USeg::FromLen( numChild).Traverse( |idx| numStash.Push( poly.Child( idx)));
        let  	numBuff = numStash.IntoBuff();
        repos.ProdCreate( numBuff.Arr(), Arr::Empty())
    };

    let  	dExpr = if denChild == 1 {
        poly.Child( coSize)
    } else {
        let  	mut denStash = Stash::New();
        USeg::FromLen( denChild).Traverse( |idx| denStash.Push( poly.Child( coSize + idx)));
        let  	denBuff = denStash.IntoBuff();
        repos.ProdCreate( denBuff.Arr(), Arr::Empty())
    };

    let  	dnExpr = DiffPureProduct( repos, poly, 0, numChild, varId)?;
    let  	ddExpr = DiffPureProduct( repos, poly, coSize, denChild, varId)?;

    let  	term1 = repos.MultCreate( dnExpr, dExpr);
    let  	term2 = repos.MultCreate( nExpr, ddExpr);
    let  	numRes = repos.DiffCreate( term1, term2);

    let  	two = repos.RealCreate( 2.0);
    let  	denRes = repos.PowerCreate( dExpr, two);

    Ok( repos.DivCreate( numRes, denRes))
}

//--------------------------------------------------------------------------------------------------
fn	DiffPureProduct(
    repos: &mut ExprRepos, poly: &crate::fresco::PolyExpr,
    start: u32, count: u32, varId: u32,
) -> Result< u32, String>
{
    if count == 0
    {
        return Ok( repos.RealCreate( 0.0));
    }
    if count == 1
    {
        return DifferentiateInner( repos, poly.Child( start), varId);
    }

    let  	mut termsStash = Stash::New();
    let  	mut i = 0u32;
    while i < count
    {
        let  	childId = poly.Child( start + i);
        let  	dChild = DifferentiateInner( repos, childId, varId)?;

        let  	mut factorsStash = Stash::New();
        factorsStash.Push( dChild);

        let  	mut j = 0u32;
        while j < count
        {
            if j != i
            {
                factorsStash.Push( poly.Child( start + j));
            }
            j += 1;
        }

        let  	factorsBuff = factorsStash.IntoBuff();
        let  	term = repos.ProdCreate( factorsBuff.Arr(), Arr::Empty());
        termsStash.Push( term);
        i += 1;
    }

    let  	termsBuff = termsStash.IntoBuff();
    Ok( repos.SumCreate( termsBuff.Arr(), Arr::Empty()))
}

//--------------------------------------------------------------------------------------------------
fn DiffPow(repos: &mut ExprRepos, base: u32, exp: u32, varId: u32) -> Result<u32, String>
{
    let  	dBase = DifferentiateInner( repos, base, varId)?;
    let  	dExp = DifferentiateInner( repos, exp, varId)?;

    // Check if exponent is constant real: d/dx (u^n) = n * u^(n-1) * u'
    if let Ok(ExprView::Real(nVal)) = repos.Inspect(exp) {
        let  	nConst = repos.RealCreate( nVal);
        let  	nMinus1 = repos.RealCreate( nVal - 1.0);
        let  	uPow = repos.PowerCreate( base, nMinus1);
        let  	t1 = repos.MultCreate( nConst, uPow);
        return Ok( repos.MultCreate( t1, dBase));
    }

    // General power: u^v * (v' * ln(u) + v * u' / u)
    let  	orig = repos.PowerCreate( base, exp);
    let  	lnU = repos.FuncCreate( "ln".to_string(), base);
    let  	part1 = repos.MultCreate( dExp, lnU);

    let  	vUPrime = repos.MultCreate( exp, dBase);
    let  	part2 = repos.DivCreate( vUPrime, base);

    let  	innerSum = repos.AddCreate( part1, part2);
    Ok( repos.MultCreate( orig, innerSum))
}

//--------------------------------------------------------------------------------------------------
fn	DiffFunc(
    repos: &mut ExprRepos, name: &str, arg: u32, varId: u32,
) -> Result< u32, String>
{
    let  	dArg = DifferentiateInner( repos, arg, varId)?;

    let  	fPrime = match name
    {
        "sin" => repos.FuncCreate( "cos".to_string(), arg),
        "cos" =>
        {
            let  	sinArg = repos.FuncCreate( "sin".to_string(), arg);
            let  	zero = repos.RealCreate( 0.0);
            repos.DiffCreate( zero, sinArg)
        }
        "exp" => repos.FuncCreate( "exp".to_string(), arg),
        "ln" =>
        {
            let  	one = repos.RealCreate( 1.0);
            repos.DivCreate( one, arg)
        }
        "sqrt" =>
        {
            let  	sqrtArg = repos.FuncCreate( "sqrt".to_string(), arg);
            let  	two = repos.RealCreate( 2.0);
            let  	twoSqrt = repos.MultCreate( two, sqrtArg);
            let  	one = repos.RealCreate( 1.0);
            repos.DivCreate( one, twoSqrt)
        }
        other => return Err( format!( "Unsupported function for differentiation: {other}")),
    };

    Ok( repos.MultCreate( fPrime, dArg))
}

//--------------------------------------------------------------------------------------------------
/// Computes the gradient vector of a scalar expression with respect to a vector variable or vector subvar.
pub fn	DifferentiateVector(
    repos: &mut ExprRepos,
    exprId: u32,
    vecVarId: u32,
) -> Result< TensorExpr, String>
{
    if !repos.IsValidExprId( exprId)
    {
        return Err( format!( "Invalid expression ID {exprId}"));
    }
    if !repos.IsValidVarId( vecVarId)
    {
        return Err( format!( "Invalid variable ID {vecVarId}"));
    }
    let  	shape = *repos.VarAttrAt( vecVarId).Shape();
    if !shape.IsVector()
    {
        return Err( format!( "DifferentiateVector requires a vector variable, got shape {:?}", shape));
    }

    let  	len = shape.Dim( 0);
    return repos.WithTransaction( |r| {
        let  	mut stash = Stash::WithCapacity( len);
        let  	mut k = 0u32;
        while k < len
        {
            let  	leafId = r.SubVarLinear( vecVarId, k).map_err( |e| e.to_string())?;
            let  	dExpr = DifferentiateInner( r, exprId, leafId)?;
            stash.Push( dExpr);
            k += 1;
        }
        return TensorExpr::New( shape, stash.IntoBuff()).map_err( |e| e.to_string());
    });
}

//--------------------------------------------------------------------------------------------------
/// Computes the Jacobian matrix J_ij = d(r_i) / d(x_j) of vector residuals with respect to a vector variable.
fn	ComputeJacobianElements(
    repos: &mut ExprRepos,
    residuals: &TensorExpr,
    vecVarId: u32,
    m: u32,
    n: u32,
) -> Result< Buff< u32>, String>
{
    return repos.WithTransaction( |r| {
        let  	mut stash = Stash::WithCapacity( m * n);
        let  	mut i = 0u32;
        while i < m
        {
            let  	rExpr = residuals.ElemLinear( i).map_err( |e| e.to_string())?;
            let  	mut j = 0u32;
            while j < n
            {
                let  	leafId = r.SubVarLinear( vecVarId, j).map_err( |e| e.to_string())?;
                let  	dExpr = DifferentiateInner( r, rExpr, leafId)?;
                stash.Push( dExpr);
                j += 1;
            }
            i += 1;
        }
        return Ok( stash.IntoBuff());
    });
}

//--------------------------------------------------------------------------------------------------
/// Computes the Jacobian matrix J_ij = d(r_i) / d(x_j) of vector residuals with respect to a vector variable.
pub fn	Jacobian(
    repos: &mut ExprRepos,
    residuals: &TensorExpr,
    vecVarId: u32,
) -> Result< TensorExpr, String>
{
    if !residuals.Shape().IsVector()
    {
        return Err( format!( "Jacobian requires vector residuals, got shape {:?}", residuals.Shape()));
    }
    if !repos.IsValidVarId( vecVarId)
    {
        return Err( format!( "Invalid variable ID {vecVarId}"));
    }
    let  	varShape = *repos.VarAttrAt( vecVarId).Shape();
    if !varShape.IsVector()
    {
        return Err( format!( "Jacobian requires a vector state variable, got shape {:?}", varShape));
    }
    let  	m = residuals.Dim( 0);
    let  	n = varShape.Dim( 0);

    // 1. Prevalidate all residual expression IDs before constructing any derivative nodes
    let  	mut r = 0u32;
    while r < m
    {
        let  	rExpr = residuals.ElemLinear( r).map_err( |e| e.to_string())?;
        if !repos.IsValidExprId( rExpr)
        {
            return Err( format!( "Invalid residual expression ID {rExpr} at index {r}"));
        }
        r += 1;
    }

    // 2. Compute elements with checkpoint/rollback protection
    let  	elems = ComputeJacobianElements( repos, residuals, vecVarId, m, n)?;
    let  	matShape = TensorShape::Matrix( m, n).map_err( |e| e.to_string())?;
    TensorExpr::New( matShape, elems).map_err( |e| e.to_string())
}
