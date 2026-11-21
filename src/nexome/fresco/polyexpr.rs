// polyexpr.rs -----------------------------------------------------------------------------------
use crate::flux::{FieldImp, FluxError, IFluxImportSource, IObjectImport};
use crate::fresco::{
    BaseExpr, ExprEntry, ExprRepos, PowExpr, ProdExpr, RealExpr, SumExpr, VarExpr,
};
use crate::silo::{Arr, Buff, Stash};
use core::any::Any;

//-------------------------------------------------------------------------------------------------

#[derive( Clone)]
pub struct PolyExpr
{
    _Children: Buff< u32>,
    _CoSize:   u32,
}

//-------------------------------------------------------------------------------------------------

impl PolyExpr
{
    pub fn	New() -> Self
    {
        Self {
            _Children: Buff::New(),
            _CoSize:   0,
        }
    }

    pub fn	FromParts( coSize: u32, children: Buff< u32>) -> Self
    {
        assert!( coSize <= children.Cap(), "Common child count exceeds child count");
        Self {
            _Children: children,
            _CoSize:   coSize,
        }
    }

    pub fn	SizeChild( &self) -> u32
    { self._Children.Cap() }

    pub fn	Child( &self, index: u32) -> u32
    { self._Children[index] }

    pub fn	CommonSize( &self) -> u32
    { self._CoSize }

    pub fn	Children( &self) -> &Buff< u32>
    { &self._Children }

    pub fn	IntoChildren( self) -> Buff< u32>
    { self._Children }

    pub fn	IsFlip( &self, index: u32) -> bool
    { index >= self._CoSize }
}

//-------------------------------------------------------------------------------------------------

impl Default for PolyExpr
{
    fn	default() -> Self
    { Self::New() }
}

//-------------------------------------------------------------------------------------------------

impl BaseExpr for PolyExpr
{
    fn	CloneBox( &self) -> Box< dyn BaseExpr>
    { Box::new( self.clone()) }

    fn	Any( &self) -> &dyn Any
    { self }
}

//-------------------------------------------------------------------------------------------------

crate::ImplFluxExportSource!( PolyExpr, _Children, _CoSize);

//-------------------------------------------------------------------------------------------------

struct PolyExprImport<'a>
{
    _Target:   &'a mut PolyExpr,
    _Children: Stash<u32>,
    _CoSize:   u32,
}

impl IObjectImport for PolyExprImport<'_>
{
    fn Field(&mut self, key: &str) -> Result<FieldImp<'_>, FluxError>
    {
        return match key {
            "_Children" => Ok(FieldImp::FluxSource(&mut self._Children)),
            "_CoSize" => Ok(FieldImp::FluxSource(&mut self._CoSize)),
            _ => Err(FluxError::TypeMismatch),
        };
    }

    fn Commit(self: Box<Self>) -> Result<(), FluxError>
    {
        let Self { _Target: target,
                   _Children: children,
                   _CoSize: coSize, } = *self;
        if coSize > children.Size() {
            return Err(FluxError::TypeMismatch);
        }
        *target = PolyExpr::FromParts(coSize, children.IntoBuff());
        return Ok(());
    }
}

impl IFluxImportSource for PolyExpr
{
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>)
    {
        let coSize = self._CoSize;
        *field = FieldImp::Obj(Box::new(PolyExprImport { _Target:   self,
                                                         _Children: Stash::New(),
                                                         _CoSize:   coSize, }));
    }
}

//-------------------------------------------------------------------------------------------------
/// Individual monomial term in a normalized polynomial: coeff * prod(var_i ^ power_i).
#[derive( Clone, Debug, PartialEq)]
pub struct Monomial
{
    _Coeff:  f64,
    _Powers: Buff< ( u32, i32)>,
}

impl Monomial
{
    pub fn	New( coeff: f64, powers: Buff< ( u32, i32)>) -> Self
    {
        Self {
            _Coeff:  coeff,
            _Powers: powers,
        }
    }

    #[inline]
    pub const fn	Coeff( &self) -> f64
    {
        self._Coeff
    }

    #[inline]
    pub const fn	Powers( &self) -> &Buff< ( u32, i32)>
    {
        &self._Powers
    }

    pub fn	TotalDegree( &self) -> i32
    {
        let  	mut deg = 0i32;
        self._Powers.Traverse( |p| deg += p.1);
        deg
    }

    pub fn	HasSamePowers( &self, other: &Monomial) -> bool
    {
        if self._Powers.Size() != other._Powers.Size()
        {
            return false;
        }
        let  	len = self._Powers.Size();
        let  	mut i = 0u32;
        while i < len
        {
            if self._Powers[i] != other._Powers[i]
            {
                return false;
            }
            i += 1;
        }
        true
    }

    pub fn	Multiply( &self, other: &Monomial) -> Monomial
    {
        let  	coeff = self._Coeff * other._Coeff;
        let  	mut merged = Stash::New();
        let  	mut i = 0u32;
        let  	mut j = 0u32;
        let  	lenI = self._Powers.Size();
        let  	lenJ = other._Powers.Size();

        while i < lenI && j < lenJ
        {
            let  	pI = self._Powers[i];
            let  	pJ = other._Powers[j];
            if pI.0 == pJ.0
            {
                let  	sumP = pI.1 + pJ.1;
                if sumP != 0
                {
                    merged.Push( ( pI.0, sumP));
                }
                i += 1;
                j += 1;
            }
            else if pI.0 < pJ.0
            {
                merged.Push( pI);
                i += 1;
            }
            else
            {
                merged.Push( pJ);
                j += 1;
            }
        }
        while i < lenI
        {
            merged.Push( self._Powers[i]);
            i += 1;
        }
        while j < lenJ
        {
            merged.Push( other._Powers[j]);
            j += 1;
        }

        Monomial::New( coeff, merged.IntoBuff())
    }
}

//-------------------------------------------------------------------------------------------------
/// Canonical normalized polynomial: sum of monomials in graded lexicographical order.
#[derive( Clone, Debug, Default, PartialEq)]
pub struct CanonicalPolynomial
{
    _Terms: Buff< Monomial>,
}

impl CanonicalPolynomial
{
    pub fn	New() -> Self
    {
        Self {
            _Terms: Buff::New(),
        }
    }

    pub fn	Constant( c: f64) -> Self
    {
        if c.abs() < 1e-12
        {
            Self::New()
        }
        else
        {
            let  	mut stash = Stash::New();
            stash.Push( Monomial::New( c, Buff::New()));
            Self {
                _Terms: stash.IntoBuff(),
            }
        }
    }

    pub fn	Variable( varId: u32, power: i32) -> Self
    {
        let  	mut powStash = Stash::New();
        powStash.Push( ( varId, power));
        let  	mut stash = Stash::New();
        stash.Push( Monomial::New( 1.0, powStash.IntoBuff()));
        Self {
            _Terms: stash.IntoBuff(),
        }
    }

    #[inline]
    pub const fn	Terms( &self) -> &Buff< Monomial>
    {
        &self._Terms
    }

    #[inline]
    pub fn	TermCount( &self) -> u32
    {
        self._Terms.Size()
    }

    pub fn	Degree( &self) -> i32
    {
        let  	mut maxDeg = 0i32;
        self._Terms.Traverse( |m| {
            let  	d = m.TotalDegree();
            if d > maxDeg
            {
                maxDeg = d;
            }
        });
        maxDeg
    }

    pub fn	Add( &self, other: &CanonicalPolynomial) -> CanonicalPolynomial
    {
        let  	mut stash = Stash::New();
        self._Terms.Traverse( |m| stash.Push( m.clone()));
        other._Terms.Traverse( |m| stash.Push( m.clone()));
        let  	mut result = CanonicalPolynomial {
            _Terms: stash.IntoBuff(),
        };
        result.Normalize();
        result
    }

    pub fn	Subtract( &self, other: &CanonicalPolynomial) -> CanonicalPolynomial
    {
        let  	mut stash = Stash::New();
        self._Terms.Traverse( |m| stash.Push( m.clone()));
        other._Terms.Traverse( |m| {
            stash.Push( Monomial::New( -m._Coeff, m._Powers.clone()));
        });
        let  	mut result = CanonicalPolynomial {
            _Terms: stash.IntoBuff(),
        };
        result.Normalize();
        result
    }

    pub fn	Multiply( &self, other: &CanonicalPolynomial) -> CanonicalPolynomial
    {
        let  	mut stash = Stash::New();
        self._Terms.Traverse( |m1| {
            other._Terms.Traverse( |m2| {
                stash.Push( m1.Multiply( m2));
            });
        });
        let  	mut result = CanonicalPolynomial {
            _Terms: stash.IntoBuff(),
        };
        result.Normalize();
        result
    }

    /// Combines like terms and sorts in graded lexicographical order.
    pub fn	Normalize( &mut self)
    {
        if self._Terms.Size() <= 1
        {
            if self._Terms.Size() == 1 && self._Terms[0]._Coeff.abs() < 1e-12
            {
                self._Terms = Buff::New();
            }
            return;
        }

        // 1. Group like terms
        let  	count = self._Terms.Size();
        let  	mut combined = Stash::New();
        let  	mut used = Buff::FromDispenser( count, |_| false);

        let  	mut i = 0u32;
        while i < count
        {
            if used[i]
            {
                i += 1;
                continue;
            }
            let  	mut coeff = self._Terms[i]._Coeff;
            let  	powers = self._Terms[i]._Powers.clone();

            let  	mut j = i + 1;
            while j < count
            {
                if !used[j] && self._Terms[i].HasSamePowers( &self._Terms[j])
                {
                    coeff += self._Terms[j]._Coeff;
                    used[j] = true;
                }
                j += 1;
            }

            if coeff.abs() >= 1e-12
            {
                combined.Push( Monomial::New( coeff, powers));
            }
            i += 1;
        }

        // 2. Sort by total degree descending
        let  	cLen = combined.Size();
        let  	mut a = 0u32;
        while a < cLen
        {
            let  	mut b = a + 1;
            while b < cLen
            {
                let  	degA = combined[a].TotalDegree();
                let  	degB = combined[b].TotalDegree();
                if degB > degA
                {
                    let  	temp = combined[a].clone();
                    combined[a] = combined[b].clone();
                    combined[b] = temp;
                }
                b += 1;
            }
            a += 1;
        }

        self._Terms = combined.IntoBuff();
    }

    /// Recursively extracts a canonical polynomial from an expression node.
    pub fn	FromExpr( repos: &ExprRepos, rootId: u32) -> Result< CanonicalPolynomial, String>
    {
        if !repos.IsValidExprId( rootId)
        {
            return Err( format!( "Invalid node ID {rootId}"));
        }

        let  	entry = match repos.ExprEntryAt( rootId)
        {
            ExprEntry::Empty => return Err( format!( "Empty node ID {rootId}")),
            ExprEntry::Expr( e) => e.clone(),
        };

        let  	any = entry.Any();

        if let Some( real) = any.downcast_ref::< RealExpr>()
        {
            return Ok( CanonicalPolynomial::Constant( real.Value()));
        }

        if let Some( var) = any.downcast_ref::< VarExpr>()
        {
            return Ok( CanonicalPolynomial::Variable( var.VarIndex(), 1));
        }

        if let Some( sum) = any.downcast_ref::< SumExpr>()
        {
            let  	poly = sum.Poly();
            let  	coSize = poly.CommonSize();
            let  	size = poly.SizeChild();
            let  	mut result = CanonicalPolynomial::New();

            let  	mut i = 0u32;
            while i < coSize
            {
                let  	childP = Self::FromExpr( repos, poly.Child( i))?;
                result = result.Add( &childP);
                i += 1;
            }
            while i < size
            {
                let  	childP = Self::FromExpr( repos, poly.Child( i))?;
                result = result.Subtract( &childP);
                i += 1;
            }
            return Ok( result);
        }

        if let Some( prod) = any.downcast_ref::< ProdExpr>()
        {
            let  	poly = prod.Poly();
            let  	coSize = poly.CommonSize();
            let  	size = poly.SizeChild();
            if size != coSize
            {
                return Err( "Polynomial normalization does not support rational fractions".to_string());
            }

            let  	mut result = CanonicalPolynomial::Constant( 1.0);
            let  	mut i = 0u32;
            while i < coSize
            {
                let  	childP = Self::FromExpr( repos, poly.Child( i))?;
                result = result.Multiply( &childP);
                i += 1;
            }
            return Ok( result);
        }

        if let Some( pow) = any.downcast_ref::< PowExpr>()
        {
            let  	poly = pow.Poly();
            let  	baseP = Self::FromExpr( repos, poly.Child( 0))?;
            let  	expEntry = repos.ExprEntryAt( poly.Child( 1)).clone();
            if let ExprEntry::Expr( e) = expEntry
                && let Some( realExp) = e.Any().downcast_ref::< RealExpr>()
            {
                let  	val = realExp.Value();
                if val >= 0.0 && ( val.fract() == 0.0)
                {
                    let  	expInt = val as i32;
                    let  	mut result = CanonicalPolynomial::Constant( 1.0);
                    let  	mut k = 0i32;
                    while k < expInt
                    {
                        result = result.Multiply( &baseP);
                        k += 1;
                    }
                    return Ok( result);
                }
            }
        }

        Err( format!( "Node ID {rootId} is not a valid polynomial expression"))
    }

    /// Re-synthesizes the canonical polynomial back into an AST in ExprRepos.
    pub fn	ToExpr( &self, repos: &mut ExprRepos) -> u32
    {
        if self._Terms.IsEmpty()
        {
            return repos.RealCreate( 0.0);
        }

        let  	mut termIds = Stash::New();
        self._Terms.Traverse( |m| {
            let  	cNode = repos.RealCreate( m._Coeff);
            let  	mut factorIds = Stash::New();
            factorIds.Push( cNode);

            m._Powers.Traverse( |&( varId, pow)| {
                let  	vNode = repos.Store( Box::new( VarExpr::New( varId)));
                if pow == 1
                {
                    factorIds.Push( vNode);
                }
                else
                {
                    let  	pNode = repos.RealCreate( pow as f64);
                    let  	powExpr = repos.PowerCreate( vNode, pNode);
                    factorIds.Push( powExpr);
                }
            });

            let  	factorsBuff = factorIds.IntoBuff();
            let  	prodNode = repos.ProdCreate( factorsBuff.Arr(), Arr::Empty());
            termIds.Push( prodNode);
        });

        let  	termBuff = termIds.IntoBuff();
        repos.SumCreate( termBuff.Arr(), Arr::Empty())
    }
}
