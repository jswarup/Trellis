// exprrepos.rs -----------------------------------------------------------------------------------
use	crate::flux::{ FieldExp, IFluxExportSource };
use	crate::fresco::{
    CondExpr, ExprShard, FuncExpr, ITermNode, LogicExpr, LogicOp, PowExpr, ProdExpr, RealExpr, SumExpr, Term,
    VarAttrib, VarExpr,
};
use	crate::silo::{ Arr, Buff, Stash };
use	crate::stalks::BinOp;
use	core::any::Any;

//-------------------------------------------------------------------------------------------------

pub trait BaseExpr: Any + IFluxExportSource
{
    fn	CloneBox( &self) -> Box< dyn BaseExpr>;

    fn	Any( &self) -> &dyn Any;
}

//-------------------------------------------------------------------------------------------------

impl Clone for Box< dyn BaseExpr>
{
    fn	clone( &self) -> Self
    { self.CloneBox() }
}

//-------------------------------------------------------------------------------------------------

#[derive( Default)]
pub enum ExprEntry
{
    #[default]
    Empty,
    Expr( Box< dyn BaseExpr>),
}

//-------------------------------------------------------------------------------------------------

impl Clone for ExprEntry
{
    fn	clone( &self) -> Self
    {
        match self
        {
            Self::Empty => Self::Empty,
            Self::Expr( expression) => Self::Expr( expression.clone()),
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl IFluxExportSource for ExprEntry
{
    fn	FetchFieldExp< 'a>( &'a self, field: &mut FieldExp< 'a>)
    {
        match self
        {
            Self::Empty => *field = FieldExp::Null,
            Self::Expr( expression) => expression.FetchFieldExp( field),
        }
    }
}

//-------------------------------------------------------------------------------------------------

#[derive( Copy, Clone, Debug)]
pub struct ExprCheckpoint
{
    _ExprCount: u32,
    _VarCount:  u32,
}

//-------------------------------------------------------------------------------------------------

pub struct ExprRepos
{
    _Exprs:       Stash< ExprEntry>,
    _VarAttribs:  Stash< VarAttrib>,
    _SymbolIndex: Stash< u32>,
}

//-------------------------------------------------------------------------------------------------

impl ExprRepos
{
    pub fn	New() -> Self
    {
        Self {
            _Exprs:       Stash::New(),
            _VarAttribs:  Stash::New(),
            _SymbolIndex: Stash::New(),
        }
    }

    pub fn	Checkpoint( &self) -> ExprCheckpoint
    {
        ExprCheckpoint {
            _ExprCount: self._Exprs.Size(),
            _VarCount:  self._VarAttribs.Size(),
        }
    }

    pub fn	Rollback( &mut self, cp: ExprCheckpoint)
    {
        self._Exprs.Resize( cp._ExprCount, |_| ExprEntry::Empty);
        self._VarAttribs.Resize( cp._VarCount, |_| VarAttrib::default());
        let  	sz = self._SymbolIndex.Size();
        let  	mut writeIdx = 0u32;
        let  	mut readIdx = 0u32;
        while readIdx < sz
        {
            let  	id = self._SymbolIndex[readIdx];
            if id < cp._VarCount
            {
                self._SymbolIndex[writeIdx] = id;
                writeIdx += 1;
            }
            readIdx += 1;
        }
        self._SymbolIndex.Resize( writeIdx, |_| 0);
    }

    pub fn	Size( &self) -> u32
    { self._Exprs.Size() }

    pub fn	Store( &mut self, expression: Box< dyn BaseExpr>) -> u32
    {
        let  	index = self.Size();
        self._Exprs.Push( ExprEntry::Expr( expression));
        index
    }

    pub fn	VarCreate( &mut self, name: String, reuse: bool) -> u32
    {
        let  	mut low = 0u32;
        let  	mut high = self._SymbolIndex.Size();
        let  	mut found = None;
        while low < high
        {
            let  	mid = low + ( high - low) / 2;
            let  	varId = self._SymbolIndex[mid];
            let  	candidateName = self._VarAttribs[varId].Name();
            match candidateName.cmp( &name)
            {
                core::cmp::Ordering::Equal =>
                {
                    found = Some( varId);
                    break;
                }
                core::cmp::Ordering::Less => low = mid + 1,
                core::cmp::Ordering::Greater => high = mid,
            }
        }
        let  	varIndex = if reuse
        {
            match found
            {
                Some( idx) => idx,
                None =>
                {
                    let  	newId = self._VarAttribs.Size();
                    self._VarAttribs.Push( VarAttrib::New( name));
                    self._SymbolIndex.Push( newId);
                    let  	mut i = self._SymbolIndex.Size() - 1;
                    while i > low
                    {
                        self._SymbolIndex[i] = self._SymbolIndex[i - 1];
                        i -= 1;
                    }
                    self._SymbolIndex[low] = newId;
                    newId
                }
            }
        }
        else
        {
            let  	newId = self._VarAttribs.Size();
            self._VarAttribs.Push( VarAttrib::New( name));
            if found.is_none()
            {
                self._SymbolIndex.Push( newId);
                let  	mut i = self._SymbolIndex.Size() - 1;
                while i > low
                {
                    self._SymbolIndex[i] = self._SymbolIndex[i - 1];
                    i -= 1;
                }
                self._SymbolIndex[low] = newId;
            }
            newId
        };
        self.Store( Box::new( VarExpr::New( varIndex)))
    }

    pub fn	RealCreate( &mut self, value: f64) -> u32
    {
        self.Store( Box::new( RealExpr::New( value)))
    }

    pub fn	SumCreate( &mut self, adds: Arr< '_, u32>, subs: Arr< '_, u32>) -> u32
    {
        self.Store( Box::new( SumExpr::New( Self::Join( adds, subs), adds.Size())))
    }

    pub fn	AddCreate( &mut self, first: impl Into< u32>, second: impl Into< u32>) -> u32
    {
        let  	children = [first.into(), second.into()];
        self.SumCreate( Arr::from( &children), Arr::Empty())
    }

    pub fn	DiffCreate( &mut self, first: impl Into< u32>, second: impl Into< u32>) -> u32
    {
        let  	adds = [first.into()];
        let  	subs = [second.into()];
        self.SumCreate( Arr::from( &adds), Arr::from( &subs))
    }

    pub fn	ProdCreate( &mut self, numers: Arr< '_, u32>, denoms: Arr< '_, u32>) -> u32
    {
        self.Store( Box::new( ProdExpr::New( Self::Join( numers, denoms), numers.Size())))
    }

    pub fn	MultCreate( &mut self, first: impl Into< u32>, second: impl Into< u32>) -> u32
    {
        let  	children = [first.into(), second.into()];
        self.ProdCreate( Arr::from( &children), Arr::Empty())
    }

    pub fn	DivCreate( &mut self, first: impl Into< u32>, second: impl Into< u32>) -> u32
    {
        let  	numers = [first.into()];
        let  	denoms = [second.into()];
        self.ProdCreate( Arr::from( &numers), Arr::from( &denoms))
    }

    pub fn	PowCreate( &mut self, bases: Arr< '_, u32>, exps: Arr< '_, u32>) -> u32
    {
        self.Store( Box::new( PowExpr::New( Self::Join( bases, exps), bases.Size())))
    }

    pub fn	PowerCreate( &mut self, base: impl Into< u32>, exp: impl Into< u32>) -> u32
    {
        let  	bases = [base.into()];
        let  	exps = [exp.into()];
        self.PowCreate( Arr::from( &bases), Arr::from( &exps))
    }

    pub fn	LogicCreate( &mut self, op: LogicOp, left: impl Into< u32>, right: impl Into< u32>) -> u32
    {
        self.Store( Box::new( LogicExpr::New( op, left, right)))
    }

    pub fn	LogicUnaryCreate( &mut self, op: LogicOp, operand: impl Into< u32>) -> u32
    {
        self.Store( Box::new( LogicExpr::Unary( op, operand)))
    }

    pub fn	CondCreate( &mut self, cond: impl Into< u32>, ifTrue: impl Into< u32>, ifFalse: impl Into< u32>) -> u32
    {
        self.Store( Box::new( CondExpr::New( cond, ifTrue, ifFalse)))
    }

    pub fn	FuncCreate( &mut self, name: String, arg: impl Into< u32>) -> u32
    {
        self.Store( Box::new( FuncExpr::New( name, arg)))
    }

    pub fn	ParseExpr( &mut self, text: &str) -> Result< u32, String>
    {
        ExprShard::Parse( self, text)
    }

    pub fn	SzVar( &self) -> u32
    { self._VarAttribs.Size() }

    pub fn	At< T: BaseExpr>( &self, index: u32) -> &T
    {
        match &self._Exprs[index]
        {
            ExprEntry::Expr( expression) => expression.Any().downcast_ref::< T>()
                .expect( "Expression type mismatch"),
            ExprEntry::Empty => panic!( "Empty expression entry"),
        }
    }

    pub fn	VarNameAt( &self, index: impl Into< u32>) -> &str
    {
        self._VarAttribs[index.into()].Name()
    }

    pub fn	VarAttrAt( &self, index: impl Into< u32>) -> &VarAttrib
    {
        &self._VarAttribs[index.into()]
    }

    pub fn	PostTermTree( &mut self, root: &dyn ITermNode) -> u32
    {
        let  	mut expressions = Stash::New();
        Self::TraverseTree( self, root, &mut expressions);
        expressions.Pop().expect( "Term tree did not produce an expression")
    }

    //---------------------------------------------------------------------------------------------

    fn	Join( first: Arr< '_, u32>, second: Arr< '_, u32>) -> Buff< u32>
    {
        let  	mut children = Stash::WithCapacity( first.Size() + second.Size());
        first.USeg().Traverse( |index| children.Push( first[index]));
        second.USeg().Traverse( |index| children.Push( second[index]));
        children.IntoBuff()
    }

    fn	CollectAdditive(
        repos: &mut Self,
        node: &dyn ITermNode,
        isPositive: bool,
        posTerms: &mut Stash< u32>,
        negTerms: &mut Stash< u32>,
    )
    {
        match node.Op()
        {
            BinOp::Sum =>
            {
                Self::CollectAdditive( repos, node.Child( 0), isPositive, posTerms, negTerms);
                Self::CollectAdditive( repos, node.Child( 1), isPositive, posTerms, negTerms);
            }
            BinOp::Sub =>
            {
                Self::CollectAdditive( repos, node.Child( 0), isPositive, posTerms, negTerms);
                Self::CollectAdditive( repos, node.Child( 1), !isPositive, posTerms, negTerms);
            }
            _ =>
            {
                let  	mut exprs = Stash::New();
                Self::TraverseTree( repos, node, &mut exprs);
                if let  	Some( expr) = exprs.Pop()
                {
                    if isPositive
                    {
                        posTerms.Push( expr);
                    }
                    else
                    {
                        negTerms.Push( expr);
                    }
                }
            }
        }
    }

    fn	CollectMultiplicative(
        repos: &mut Self,
        node: &dyn ITermNode,
        isNumerator: bool,
        numFactors: &mut Stash< u32>,
        denFactors: &mut Stash< u32>,
    )
    {
        match node.Op()
        {
            BinOp::Prod =>
            {
                Self::CollectMultiplicative( repos, node.Child( 0), isNumerator, numFactors, denFactors);
                Self::CollectMultiplicative( repos, node.Child( 1), isNumerator, numFactors, denFactors);
            }
            BinOp::Div =>
            {
                Self::CollectMultiplicative( repos, node.Child( 0), isNumerator, numFactors, denFactors);
                Self::CollectMultiplicative( repos, node.Child( 1), !isNumerator, numFactors, denFactors);
            }
            _ =>
            {
                let  	mut exprs = Stash::New();
                Self::TraverseTree( repos, node, &mut exprs);
                if let  	Some( expr) = exprs.Pop()
                {
                    if isNumerator
                    {
                        numFactors.Push( expr);
                    }
                    else
                    {
                        denFactors.Push( expr);
                    }
                }
            }
        }
    }

    fn	TraverseTree( repos: &mut Self, node: &dyn ITermNode, expressions: &mut Stash< u32>)
    {
        let  	operation = node.Op();
        if operation == BinOp::None
        {
            let  	expression = match node.Leaf()
            {
                Term::Null => panic!( "Null term cannot become an expression"),
                Term::String( value) => repos.VarCreate( value.clone(), false),
                Term::Real( value) => repos.RealCreate( *value),
            };
            expressions.Push( expression);
            return;
        }

        let  	expression = match operation
        {
            BinOp::Sum | BinOp::Sub =>
            {
                let  	mut posTerms = Stash::New();
                let  	mut negTerms = Stash::New();
                Self::CollectAdditive( repos, node, true, &mut posTerms, &mut negTerms);
                let  	coSize = posTerms.Size();
                let  	mut children = Stash::WithCapacity( posTerms.Size() + negTerms.Size());
                posTerms.USeg().Traverse( |idx| children.Push( posTerms[idx]));
                negTerms.USeg().Traverse( |idx| children.Push( negTerms[idx]));
                repos.Store( Box::new( SumExpr::New( children.IntoBuff(), coSize)))
            }
            BinOp::Prod | BinOp::Div =>
            {
                let  	mut numFactors = Stash::New();
                let  	mut denFactors = Stash::New();
                Self::CollectMultiplicative( repos, node, true, &mut numFactors, &mut denFactors);
                let  	coSize = numFactors.Size();
                let  	mut children = Stash::WithCapacity( numFactors.Size() + denFactors.Size());
                numFactors.USeg().Traverse( |idx| children.Push( numFactors[idx]));
                denFactors.USeg().Traverse( |idx| children.Push( denFactors[idx]));
                repos.Store( Box::new( ProdExpr::New( children.IntoBuff(), coSize)))
            }
            BinOp::Pow =>
            {
                let  	mut leftExprs = Stash::New();
                Self::TraverseTree( repos, node.Child( 0), &mut leftExprs);
                let  	left = leftExprs.Pop().expect( "Missing base in pow term");

                let  	mut rightExprs = Stash::New();
                Self::TraverseTree( repos, node.Child( 1), &mut rightExprs);
                let  	right = rightExprs.Pop().expect( "Missing exponent in pow term");

                let  	children = Buff::FromArr( Arr::from( &[left, right]));
                repos.Store( Box::new( PowExpr::New( children, 1)))
            }
            _ => panic!( "Unsupported term-tree operation"),
        };
        expressions.Push( expression);
    }
}

//-------------------------------------------------------------------------------------------------

impl Default for ExprRepos
{
    fn	default() -> Self
    { Self::New() }
}

//-------------------------------------------------------------------------------------------------

crate::ImplFluxExportSource!( ExprRepos, _Exprs, _VarAttribs);

//-------------------------------------------------------------------------------------------------
