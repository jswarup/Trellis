// exprrepos.rs ---------------------------------------------------------------------------------------------------
use	crate::fresco::{ ITermNode, PowExpr, ProdExpr, RealExpr, SumExpr, Term, VarAttrib, VarExpr };
use	crate::flux::{ FieldExp, IFluxExportSource };
use	crate::silo::{ Arr, Buff, Stash };
use	crate::stalks::BinOp;
use	core::any::Any;
pub trait BaseExpr: Any + IFluxExportSource {
    fn	CloneBox( &self) -> Box< dyn BaseExpr>;
    fn	AsAny( &self) -> &dyn Any;
}
impl Clone for Box< dyn BaseExpr> {
    fn	clone( &self) -> Self
    { self.CloneBox() }
}
#[derive( Default)]
pub enum ExprEntry {
    #[default]
    Empty,
    Expr( Box< dyn BaseExpr>),
}
impl Clone for ExprEntry {
    fn	clone( &self) -> Self
    {
        match self {
            Self::Empty => Self::Empty,
            Self::Expr( expression) => Self::Expr( expression.clone()),
        }
    }
}
impl IFluxExportSource for ExprEntry {
    fn	FetchFieldExp< 'a>(&'a self, field: &mut FieldExp< 'a>) {
        match self {
            Self::Empty => *field = FieldExp::Null,
            Self::Expr( expression) => expression.FetchFieldExp( field),
        }
    }
}
pub struct ExprRepos
{
    _Exprs: Stash< ExprEntry>,
    _VarAttribs: Stash< VarAttrib>,
}
impl ExprRepos
{
    pub fn	New() -> Self
    {
        Self { _Exprs: Stash::New(), _VarAttribs: Stash::New() }
    }
    pub fn	Size( &self) -> u32
    { self._Exprs.Size() }
    pub fn	Store( &mut self, expression: Box< dyn BaseExpr>) -> u32
    {
        let  	index = self.Size();
        self._Exprs.Push( ExprEntry::Expr( expression));
        index
    }
    pub fn	VarCreate( &mut self, name: String, _reuse: bool) -> u32
    {
        let  	varIndex = self._VarAttribs.Size();
        self._VarAttribs.Push( VarAttrib::New( name));
        self.Store( Box::new( VarExpr::New( varIndex)))
    }
    pub fn	RealCreate( &mut self, value: f64) -> u32
    {
        self.Store( Box::new( RealExpr::New( value)))
    }
    pub fn	SumCreate( &mut self, adds: Arr< '_, u32>, subs: Arr<'_, u32>) -> u32
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
    pub fn	ProdCreate( &mut self, numers: Arr< '_, u32>, denoms: Arr<'_, u32>) -> u32
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
    pub fn	PowCreate( &mut self, bases: Arr< '_, u32>, exps: Arr<'_, u32>) -> u32
    {
        self.Store( Box::new( PowExpr::New( Self::Join( bases, exps), bases.Size())))
    }
    pub fn	SzVar( &self) -> u32
    { self._VarAttribs.Size() }
    pub fn	At< T: BaseExpr>( &self, index: u32) -> &T
    {
        match &self._Exprs[index] {
            ExprEntry::Expr( expression) => expression.AsAny().downcast_ref::< T>()
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
    fn	Join( first: Arr< '_, u32>, second: Arr<'_, u32>) -> Buff< u32>
    {
        let  	mut children = Stash::WithCapacity( first.Size() + second.Size());
        first.USeg().Traverse( |index| children.Push( first[index]));
        second.USeg().Traverse( |index| children.Push( second[index]));
        children.IntoBuff()
    }
    fn	CollectTree( repos: &mut Self, node: &dyn ITermNode, parentOp: BinOp, expressions: &mut Stash< u32>)
    {
        if node.Op() == parentOp {
            Self::CollectTree( repos, node.Child( 0), parentOp, expressions);
            Self::CollectTree( repos, node.Child( 1), parentOp, expressions);
        } else {
            Self::TraverseTree( repos, node, expressions);
        }
    }
    fn	TraverseTree( repos: &mut Self, node: &dyn ITermNode, expressions: &mut Stash< u32>)
    {
        let  	operation = node.Op();
        if operation == BinOp::None {
            let  	expression = match node.AsLeaf() {
                Term::Null => panic!( "Null term cannot become an expression"),
                Term::String( value) => repos.VarCreate( value.clone(), false),
                Term::Real( value) => repos.RealCreate( *value),
            };
            expressions.Push( expression);
            return;
        }
        let  	start = expressions.Size();
        Self::CollectTree( repos, node.Child( 0), operation, expressions);
        Self::CollectTree( repos, node.Child( 1), operation, expressions);
        let  	size = expressions.Size() - start;
        let  	children = Buff::FromArr( expressions.AsArr().Slice( start, size));
        expressions.Resize( start, |_| 0);
        let  	expression = match operation {
            BinOp::Sum => repos.Store( Box::new( SumExpr::New( children, size))),
            BinOp::Prod => repos.Store( Box::new( ProdExpr::New( children, size))),
            BinOp::Sub => repos.Store( Box::new( SumExpr::New( children, 1))),
            BinOp::Div => repos.Store( Box::new( ProdExpr::New( children, 1))),
            BinOp::Pow => repos.Store( Box::new( PowExpr::New( children, 1))),
            _ => panic!( "Unsupported term-tree operation"),
        };
        expressions.Push( expression);
    }
}
impl Default for ExprRepos {
    fn	default() -> Self
    { Self::New() }
}
crate::ImplFluxExportSource!( ExprRepos, _Exprs, _VarAttribs);

//-------------------------------------------------------------------------------------------------
