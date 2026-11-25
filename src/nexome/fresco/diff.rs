//-- diff.rs ---------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use crate::fresco::{ExprReadError, ExprRepos, ExprView, PolyExpr, TensorExpr, TensorShape};
use crate::silo::{Arr, Buff, Stash, USeg};

//--------------------------------------------------------------------------------------------------

/// Resource limits for bounded symbolic differentiation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffLimits
{
    pub _MaxDepth: u32,
    pub _MaxNodes: u32,
}

impl Default for DiffLimits
{
    fn default() -> Self
    {
        Self {
            _MaxDepth: ExprRepos::K_MAX_TREE_DEPTH,
            _MaxNodes: 100_000,
        }
    }
}

//--------------------------------------------------------------------------------------------------

struct DiffCtx<'a>
{
    _VarId:   u32,
    _Limits:  &'a DiffLimits,
    _Visited: u32,
    _Memo:    Buff<u32>,
}

impl<'a> DiffCtx<'a>
{
    fn New(varId: u32, initialExprCount: u32, limits: &'a DiffLimits) -> Self
    {
        Self {
            _VarId:   varId,
            _Limits:  limits,
            _Visited: 0,
            _Memo:    Buff::FromDispenser(initialExprCount, |_| u32::MAX),
        }
    }
}

//--------------------------------------------------------------------------------------------------

#[inline]
fn IsZeroExpr(repos: &ExprRepos, id: u32) -> bool
{
    return matches!(repos.Inspect(id), Ok(ExprView::Real(v)) if v == 0.0);
}

//--------------------------------------------------------------------------------------------------
/// Computes the exact symbolic derivative of an expression with respect to a target variable.
///
/// The operation is transactional: if differentiation fails at any subexpression, all
/// partially constructed derivative nodes are rolled back cleanly.
pub fn Differentiate(repos: &mut ExprRepos, exprId: u32, varId: u32) -> Result<u32, String>
{
    return DifferentiateWithLimits(repos, exprId, varId, &DiffLimits::default());
}

//--------------------------------------------------------------------------------------------------
/// Computes the symbolic derivative subject to explicit depth and node budgets.
pub fn DifferentiateWithLimits(
    repos: &mut ExprRepos,
    exprId: u32,
    varId: u32,
    limits: &DiffLimits,
) -> Result<u32, String>
{
    let initialSize = repos.Size();
    return repos.WithTransaction(|r| {
        let mut dctx = DiffCtx::New(varId, initialSize, limits);
        return DifferentiateInner(r, exprId, 0, &mut dctx);
    });
}

//--------------------------------------------------------------------------------------------------
/// Computes a row-major `m x n` grid of symbolic derivatives for `exprIds` with respect to
/// scalar variable IDs `varIds`, sharing memoization per variable across all expressions.
pub fn DifferentiateManyWithLimits(
    repos: &mut ExprRepos,
    exprIds: Arr<'_, u32>,
    varIds: Arr<'_, u32>,
    limits: &DiffLimits,
) -> Result<Buff<u32>, String>
{
    let m = exprIds.Size();
    let n = varIds.Size();
    let initialSize = repos.Size();
    return repos.WithTransaction(|r| {
        let mut out = Buff::<u32>::FromDispenser(m * n, |_| u32::MAX);
        let mut j = 0u32;
        while j < n
        {
            let mut dctx = DiffCtx::New(varIds[j], initialSize, limits);
            let mut i = 0u32;
            while i < m
            {
                let dExpr = DifferentiateInner(r, exprIds[i], 0, &mut dctx)?;
                out[i * n + j] = dExpr;
                i += 1;
            }
            j += 1;
        }
        return Ok(out);
    });
}

//--------------------------------------------------------------------------------------------------
fn DifferentiateInner(
    repos: &mut ExprRepos,
    exprId: u32,
    depth: u32,
    dctx: &mut DiffCtx<'_>,
) -> Result<u32, String>
{
    if depth > dctx._Limits._MaxDepth
    {
        return Err(format!(
            "Symbolic differentiation depth exceeds limit of {}",
            dctx._Limits._MaxDepth
        ));
    }
    if exprId < dctx._Memo.Size() && dctx._Memo[exprId] != u32::MAX
    {
        return Ok(dctx._Memo[exprId]);
    }
    dctx._Visited = dctx._Visited.saturating_add(1);
    if dctx._Visited > dctx._Limits._MaxNodes
    {
        return Err(format!(
            "Symbolic differentiation node budget exceeds limit of {}",
            dctx._Limits._MaxNodes
        ));
    }

    let node = repos.Inspect(exprId).map_err(|error| match error {
        ExprReadError::InvalidId(_) => format!("Invalid expression ID {exprId}"),
        ExprReadError::EmptyNode(_) => format!("Empty expression ID {exprId}"),
        ExprReadError::UnsupportedNode(_) => {
            format!("Symbolic differentiation not supported for node ID {exprId}")
        }
    })?;

    let result = match node
    {
        ExprView::Real(_) => Ok(repos.RealCreate(0.0)),
        ExprView::Var(varIndex) => {
            Ok(repos.RealCreate(if varIndex == dctx._VarId { 1.0 } else { 0.0 }))
        }
        ExprView::Sum(poly) =>
        {
            let poly = PolyExpr::FromParts(poly.CommonSize(), Buff::FromArr(poly.Children()));
            DiffSum(repos, &poly, depth, dctx)
        }
        ExprView::Prod(poly) =>
        {
            let poly = PolyExpr::FromParts(poly.CommonSize(), Buff::FromArr(poly.Children()));
            DiffProd(repos, &poly, depth, dctx)
        }
        ExprView::Pow(poly) =>
        {
            if poly.SizeChild() < 2
            {
                return Err(format!("Power expression missing operands at node ID {exprId}"));
            }
            let base = poly.Child(0);
            let exponent = poly.Child(1);
            DiffPow(repos, base, exponent, depth, dctx)
        }
        ExprView::Func { name, arg } =>
        {
            let name = name.to_string();
            DiffFunc(repos, &name, arg, depth, dctx)
        }
        _ => Err(format!(
            "Symbolic differentiation not supported for node ID {exprId}"
        )),
    }?;

    if exprId < dctx._Memo.Size()
    {
        dctx._Memo[exprId] = result;
    }
    return Ok(result);
}

//--------------------------------------------------------------------------------------------------
fn DiffSum(
    repos: &mut ExprRepos,
    poly: &PolyExpr,
    depth: u32,
    dctx: &mut DiffCtx<'_>,
) -> Result<u32, String>
{
    let coSize = poly.CommonSize();
    let size = poly.SizeChild();

    let mut addsStash = Stash::New();
    let mut subsStash = Stash::New();

    let mut i = 0u32;
    while i < coSize
    {
        let dChild = DifferentiateInner(repos, poly.Child(i), depth + 1, dctx)?;
        if !IsZeroExpr(repos, dChild)
        {
            addsStash.Push(dChild);
        }
        i += 1;
    }
    while i < size
    {
        let dChild = DifferentiateInner(repos, poly.Child(i), depth + 1, dctx)?;
        if !IsZeroExpr(repos, dChild)
        {
            subsStash.Push(dChild);
        }
        i += 1;
    }

    if addsStash.Size() == 0 && subsStash.Size() == 0
    {
        return Ok(repos.RealCreate(0.0));
    }
    let adds = addsStash.IntoBuff();
    let subs = subsStash.IntoBuff();
    return Ok(repos.SumCreate(adds.Arr(), subs.Arr()));
}

//--------------------------------------------------------------------------------------------------
fn DiffProd(
    repos: &mut ExprRepos,
    poly: &PolyExpr,
    depth: u32,
    dctx: &mut DiffCtx<'_>,
) -> Result<u32, String>
{
    let coSize = poly.CommonSize();
    let size = poly.SizeChild();

    if size == 0
    {
        return Ok(repos.RealCreate(0.0));
    }

    let numChild = coSize;
    let denChild = size - coSize;

    // Pure product: d/dx (u0 * u1 * ... * un-1) = sum( u_i' * prod(u_j, j!=i) )
    if denChild == 0
    {
        return DiffPureProduct(repos, poly, 0, numChild, depth, dctx);
    }

    let dnExpr = DiffPureProduct(repos, poly, 0, numChild, depth, dctx)?;
    let ddExpr = DiffPureProduct(repos, poly, coSize, denChild, depth, dctx)?;

    let dnZero = IsZeroExpr(repos, dnExpr);
    let ddZero = IsZeroExpr(repos, ddExpr);

    if dnZero && ddZero
    {
        return Ok(repos.RealCreate(0.0));
    }

    let dExpr = if denChild == 1
    {
        poly.Child(coSize)
    }
    else
    {
        let mut denStash = Stash::New();
        USeg::FromLen(denChild).Traverse(|idx| denStash.Push(poly.Child(coSize + idx)));
        let denBuff = denStash.IntoBuff();
        repos.ProdCreate(denBuff.Arr(), Arr::Empty())
    };

    if ddZero
    {
        return Ok(repos.DivCreate(dnExpr, dExpr));
    }

    let nExpr = if numChild == 1
    {
        poly.Child(0)
    }
    else
    {
        let mut numStash = Stash::New();
        USeg::FromLen(numChild).Traverse(|idx| numStash.Push(poly.Child(idx)));
        let numBuff = numStash.IntoBuff();
        repos.ProdCreate(numBuff.Arr(), Arr::Empty())
    };

    let term1 = repos.MultCreate(dnExpr, dExpr);
    let term2 = repos.MultCreate(nExpr, ddExpr);
    let numRes = if dnZero
    {
        let zero = repos.RealCreate(0.0);
        repos.DiffCreate(zero, term2)
    }
    else
    {
        repos.DiffCreate(term1, term2)
    };

    let two = repos.RealCreate(2.0);
    let denSq = repos.PowerCreate(dExpr, two);
    return Ok(repos.DivCreate(numRes, denSq));
}

//--------------------------------------------------------------------------------------------------
fn DiffPureProduct(
    repos: &mut ExprRepos,
    poly: &PolyExpr,
    start: u32,
    count: u32,
    depth: u32,
    dctx: &mut DiffCtx<'_>,
) -> Result<u32, String>
{
    if count == 0
    {
        return Ok(repos.RealCreate(0.0));
    }
    if count == 1
    {
        return DifferentiateInner(repos, poly.Child(start), depth + 1, dctx);
    }

    let mut termsStash = Stash::New();
    let mut i = 0u32;
    while i < count
    {
        let childId = poly.Child(start + i);
        let dChild = DifferentiateInner(repos, childId, depth + 1, dctx)?;
        if !IsZeroExpr(repos, dChild)
        {
            let mut factorsStash = Stash::New();
            factorsStash.Push(dChild);

            let mut j = 0u32;
            while j < count
            {
                if j != i
                {
                    factorsStash.Push(poly.Child(start + j));
                }
                j += 1;
            }

            let factorsBuff = factorsStash.IntoBuff();
            let term = repos.ProdCreate(factorsBuff.Arr(), Arr::Empty());
            termsStash.Push(term);
        }
        i += 1;
    }

    if termsStash.Size() == 0
    {
        return Ok(repos.RealCreate(0.0));
    }
    if termsStash.Size() == 1
    {
        return Ok(termsStash[0]);
    }
    let termsBuff = termsStash.IntoBuff();
    return Ok(repos.SumCreate(termsBuff.Arr(), Arr::Empty()));
}

//--------------------------------------------------------------------------------------------------
fn DiffPow(
    repos: &mut ExprRepos,
    base: u32,
    exp: u32,
    depth: u32,
    dctx: &mut DiffCtx<'_>,
) -> Result<u32, String>
{
    // Specialize constant real exponent before introducing domain-restricting nodes.
    if let Ok(ExprView::Real(nVal)) = repos.Inspect(exp)
    {
        if nVal == 0.0
        {
            return Ok(repos.RealCreate(0.0));
        }
        let dBase = DifferentiateInner(repos, base, depth + 1, dctx)?;
        if IsZeroExpr(repos, dBase)
        {
            return Ok(repos.RealCreate(0.0));
        }
        if nVal == 1.0
        {
            return Ok(dBase);
        }
        if nVal == 2.0
        {
            let two = repos.RealCreate(2.0);
            let t1 = repos.MultCreate(two, base);
            return Ok(repos.MultCreate(t1, dBase));
        }
        let nConst = repos.RealCreate(nVal);
        let nMinus1 = repos.RealCreate(nVal - 1.0);
        let uPow = repos.PowerCreate(base, nMinus1);
        let t1 = repos.MultCreate(nConst, uPow);
        return Ok(repos.MultCreate(t1, dBase));
    }

    let dBase = DifferentiateInner(repos, base, depth + 1, dctx)?;
    let dExp = DifferentiateInner(repos, exp, depth + 1, dctx)?;

    let baseZero = IsZeroExpr(repos, dBase);
    let expZero = IsZeroExpr(repos, dExp);
    if baseZero && expZero
    {
        return Ok(repos.RealCreate(0.0));
    }

    // General power: u^v * (v' * ln(u) + v * u' / u)
    let orig = repos.PowerCreate(base, exp);
    if expZero
    {
        let vUPrime = repos.MultCreate(exp, dBase);
        let part2 = repos.DivCreate(vUPrime, base);
        return Ok(repos.MultCreate(orig, part2));
    }
    let lnU = repos.FuncCreate("ln".to_string(), base);
    let part1 = repos.MultCreate(dExp, lnU);
    if baseZero
    {
        return Ok(repos.MultCreate(orig, part1));
    }

    let vUPrime = repos.MultCreate(exp, dBase);
    let part2 = repos.DivCreate(vUPrime, base);
    let innerSum = repos.AddCreate(part1, part2);
    return Ok(repos.MultCreate(orig, innerSum));
}

//--------------------------------------------------------------------------------------------------
fn DiffFunc(
    repos: &mut ExprRepos,
    name: &str,
    arg: u32,
    depth: u32,
    dctx: &mut DiffCtx<'_>,
) -> Result<u32, String>
{
    let dArg = DifferentiateInner(repos, arg, depth + 1, dctx)?;
    if IsZeroExpr(repos, dArg)
    {
        return Ok(repos.RealCreate(0.0));
    }

    let fPrime = match name
    {
        "sin" => repos.FuncCreate("cos".to_string(), arg),
        "cos" =>
        {
            let sinArg = repos.FuncCreate("sin".to_string(), arg);
            let zero = repos.RealCreate(0.0);
            repos.DiffCreate(zero, sinArg)
        }
        "exp" => repos.FuncCreate("exp".to_string(), arg),
        "ln" =>
        {
            let one = repos.RealCreate(1.0);
            repos.DivCreate(one, arg)
        }
        "sqrt" =>
        {
            let sqrtArg = repos.FuncCreate("sqrt".to_string(), arg);
            let two = repos.RealCreate(2.0);
            let twoSqrt = repos.MultCreate(two, sqrtArg);
            let one = repos.RealCreate(1.0);
            repos.DivCreate(one, twoSqrt)
        }
        other => return Err(format!("Unsupported function for differentiation: {other}")),
    };

    return Ok(repos.MultCreate(fPrime, dArg));
}

//--------------------------------------------------------------------------------------------------
/// Computes the gradient vector of a scalar expression with respect to a vector variable or vector subvar.
pub fn DifferentiateVector(
    repos: &mut ExprRepos,
    exprId: u32,
    vecVarId: u32,
) -> Result<TensorExpr, String>
{
    if !repos.IsValidVarId(vecVarId)
    {
        return Err(format!("Invalid variable ID {vecVarId}"));
    }
    let shape = *repos.VarAttrAt(vecVarId).Shape();
    if !shape.IsVector()
    {
        return Err(format!(
            "DifferentiateVector requires a vector variable, got shape {:?}",
            shape
        ));
    }

    let len = shape.Dim(0);
    let initialSize = repos.Size();
    let limits = DiffLimits::default();
    return repos.WithTransaction(|r| {
        let mut stash = Stash::WithCapacity(len);
        let mut k = 0u32;
        while k < len
        {
            let leafId = r.SubVarLinear(vecVarId, k).map_err(|e| e.to_string())?;
            let mut dctx = DiffCtx::New(leafId, initialSize, &limits);
            let dExpr = DifferentiateInner(r, exprId, 0, &mut dctx)?;
            stash.Push(dExpr);
            k += 1;
        }
        return TensorExpr::New(shape, stash.IntoBuff()).map_err(|e| e.to_string());
    });
}

//--------------------------------------------------------------------------------------------------
/// Computes the Jacobian matrix J_ij = d(r_i) / d(x_j) of vector residuals with respect to a vector variable.
fn ComputeJacobianElements(
    repos: &mut ExprRepos,
    residuals: &TensorExpr,
    vecVarId: u32,
    m: u32,
    n: u32,
) -> Result<Buff<u32>, String>
{
    let initialSize = repos.Size();
    let limits = DiffLimits::default();
    return repos.WithTransaction(|r| {
        let mut out = Buff::<u32>::FromDispenser(m * n, |_| u32::MAX);
        let mut j = 0u32;
        while j < n
        {
            let leafId = r.SubVarLinear(vecVarId, j).map_err(|e| e.to_string())?;
            let mut dctx = DiffCtx::New(leafId, initialSize, &limits);
            let mut i = 0u32;
            while i < m
            {
                let rExpr = residuals.ElemLinear(i).map_err(|e| e.to_string())?;
                let dExpr = DifferentiateInner(r, rExpr, 0, &mut dctx)?;
                out[i * n + j] = dExpr;
                i += 1;
            }
            j += 1;
        }
        return Ok(out);
    });
}

//--------------------------------------------------------------------------------------------------
/// Computes the Jacobian matrix J_ij = d(r_i) / d(x_j) of vector residuals with respect to a vector variable.
pub fn Jacobian(
    repos: &mut ExprRepos,
    residuals: &TensorExpr,
    vecVarId: u32,
) -> Result<TensorExpr, String>
{
    if !residuals.Shape().IsVector()
    {
        return Err(format!(
            "Jacobian requires vector residuals, got shape {:?}",
            residuals.Shape()
        ));
    }
    if !repos.IsValidVarId(vecVarId)
    {
        return Err(format!("Invalid variable ID {vecVarId}"));
    }
    let varShape = *repos.VarAttrAt(vecVarId).Shape();
    if !varShape.IsVector()
    {
        return Err(format!(
            "Jacobian requires a vector state variable, got shape {:?}",
            varShape
        ));
    }
    let m = residuals.Dim(0);
    let n = varShape.Dim(0);

    // 1. Prevalidate all residual expression IDs before constructing any derivative nodes
    let mut r = 0u32;
    while r < m
    {
        let rExpr = residuals.ElemLinear(r).map_err(|e| e.to_string())?;
        if !repos.IsValidExprId(rExpr)
        {
            return Err(format!(
                "Invalid residual expression ID {rExpr} at index {r}"
            ));
        }
        r += 1;
    }

    // 2. Compute elements with transaction protection
    let elems = ComputeJacobianElements(repos, residuals, vecVarId, m, n)?;
    let matShape = TensorShape::Matrix(m, n).map_err(|e| e.to_string())?;
    return TensorExpr::New(matShape, elems).map_err(|e| e.to_string());
}

//--------------------------------------------------------------------------------------------------
