//-- diff.rs ---------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::fresco::{ ExprEntry, ExprRepos, FuncExpr, PowExpr, ProdExpr, RealExpr, SumExpr, VarExpr };
use	crate::silo::{ Arr, Stash, USeg };

//--------------------------------------------------------------------------------------------------
/// Computes the exact symbolic derivative of an expression with respect to a target variable.
pub fn	Differentiate( repos: &mut ExprRepos, exprId: u32, varId: u32) -> Result< u32, String>
{
    if !repos.IsValidExprId( exprId)
    {
        return Err( format!( "Invalid expression ID {exprId}"));
    }

    // Extract clone of entry to avoid borrowing repos mutably while inspecting
    let  	entry = match repos.ExprEntryAt( exprId)
    {
        ExprEntry::Empty => return Err( format!( "Empty expression ID {exprId}")),
        ExprEntry::Expr( e) => e.clone(),
    };

    let  	any = entry.Any();

    // 1. Real constant: d/dx (c) = 0
    if any.is::< RealExpr>()
    {
        return Ok( repos.RealCreate( 0.0));
    }

    // 2. Variable: d/dx (x) = 1, d/dx (y) = 0
    if let Some( var) = any.downcast_ref::< VarExpr>()
    {
        let  	isTarget = var.VarIndex() == varId;
        return Ok( repos.RealCreate( if isTarget { 1.0 } else { 0.0 }));
    }

    // 3. Sum / Difference
    if let Some( sum) = any.downcast_ref::< SumExpr>()
    {
        let  	poly = sum.Poly().clone();
        return DiffSum( repos, &poly, varId);
    }

    // 4. Product / Quotient
    if let Some( prod) = any.downcast_ref::< ProdExpr>()
    {
        let  	poly = prod.Poly().clone();
        return DiffProd( repos, &poly, varId);
    }

    // 5. Power
    if let Some( pow) = any.downcast_ref::< PowExpr>()
    {
        let  	poly = pow.Poly().clone();
        return DiffPow( repos, &poly, varId);
    }

    // 6. Function (sin, cos, exp, ln, sqrt)
    if let Some( func) = any.downcast_ref::< FuncExpr>()
    {
        let  	name = func.Name().to_string();
        let  	arg = func.Arg();
        return DiffFunc( repos, &name, arg, varId);
    }

    Err( format!( "Symbolic differentiation not supported for node ID {exprId}"))
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
        let  	dChild = Differentiate( repos, poly.Child( i), varId)?;
        addsStash.Push( dChild);
        i += 1;
    }
    while i < size
    {
        let  	dChild = Differentiate( repos, poly.Child( i), varId)?;
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
        return Differentiate( repos, poly.Child( start), varId);
    }

    let  	mut termsStash = Stash::New();
    let  	mut i = 0u32;
    while i < count
    {
        let  	childId = poly.Child( start + i);
        let  	dChild = Differentiate( repos, childId, varId)?;

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
fn	DiffPow(
    repos: &mut ExprRepos, poly: &crate::fresco::PolyExpr, varId: u32,
) -> Result< u32, String>
{
    let  	base = poly.Child( 0);
    let  	exp = poly.Child( 1);

    let  	dBase = Differentiate( repos, base, varId)?;
    let  	dExp = Differentiate( repos, exp, varId)?;

    // Check if exponent is constant real: d/dx (u^n) = n * u^(n-1) * u'
    let  	expEntry = repos.ExprEntryAt( exp).clone();
    if let ExprEntry::Expr( e) = expEntry
        && let Some( real) = e.Any().downcast_ref::< RealExpr>()
    {
        let  	nVal = real.Value();
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
    let  	dArg = Differentiate( repos, arg, varId)?;

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
