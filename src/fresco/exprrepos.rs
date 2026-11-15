// exprrepos.rs -----------------------------------------------------------------------------------
use	crate::flux::{
    fluximport::{ ObjImp, ObjValidate },
    FieldExp, FieldImp, IFluxExportSource, IFluxImportSource,
};
use	crate::fresco::{
    CondExpr, ExprShard, FuncExpr, ITermNode, LogicExpr, LogicOp, PolyExpr, PowExpr, ProdExpr, RealExpr, SumExpr, Term,
    VarAttrib, VarExpr, VarKind,
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

#[derive( Default)]
struct ExprNodeBuilder
{
    _Type:     String,
    _Value:    f64,
    _VarIndex: u32,
    _Poly:     PolyExpr,
    _Op:       LogicOp,
    _Left:     u32,
    _Right:    u32,
    _Cond:     u32,
    _IfTrue:   u32,
    _IfFalse:  u32,
    _Name:     String,
    _Arg:      u32,
}

fn construct_expr_from_builder( b: &ExprNodeBuilder) -> Option< ExprEntry>
{
    match b._Type.as_str()
    {
        "RealExpr" => Some( ExprEntry::Expr( Box::new( RealExpr::New( b._Value)))),
        "VarExpr" => Some( ExprEntry::Expr( Box::new( VarExpr::New( b._VarIndex)))),
        "SumExpr" =>
        {
            let  	coSize = b._Poly.CommonSize();
            Some( ExprEntry::Expr( Box::new( SumExpr::New( b._Poly.Children().clone(), coSize))))
        }
        "ProdExpr" =>
        {
            let  	coSize = b._Poly.CommonSize();
            Some( ExprEntry::Expr( Box::new( ProdExpr::New( b._Poly.Children().clone(), coSize))))
        }
        "PowExpr" =>
        {
            let  	coSize = b._Poly.CommonSize();
            Some( ExprEntry::Expr( Box::new( PowExpr::New( b._Poly.Children().clone(), coSize))))
        }
        "LogicExpr" => Some( ExprEntry::Expr( Box::new( LogicExpr::New( b._Op, b._Left, b._Right)))),
        "CondExpr" => Some( ExprEntry::Expr( Box::new( CondExpr::New( b._Cond, b._IfTrue, b._IfFalse)))),
        "FuncExpr" => Some( ExprEntry::Expr( Box::new( FuncExpr::New( b._Name.clone(), b._Arg)))),
        _ => None,
    }
}

//-------------------------------------------------------------------------------------------------

impl IFluxImportSource for ExprEntry
{
    fn	FetchFieldImp< 'a>( &'a mut self, field: &mut FieldImp< 'a>)
    {
        let  	mut builder = Box::new( ExprNodeBuilder::default());
        let  	builder_ptr: *mut ExprNodeBuilder = &mut *builder;
        let  	target_ptr = self as *mut Self;

        let  	cb: ObjImp< 'a> = Box::new( move |key, item| {
            let  	b = unsafe { &mut *builder_ptr };
            match key
            {
                "Type" => { b._Type.FetchFieldImp( item); true }
                "_Value" => { b._Value.FetchFieldImp( item); true }
                "_VarIndex" => { b._VarIndex.FetchFieldImp( item); true }
                "_Poly" => { b._Poly.FetchFieldImp( item); true }
                "_Op" => { b._Op.FetchFieldImp( item); true }
                "_Left" => { b._Left.FetchFieldImp( item); true }
                "_Right" => { b._Right.FetchFieldImp( item); true }
                "_Cond" => { b._Cond.FetchFieldImp( item); true }
                "_IfTrue" => { b._IfTrue.FetchFieldImp( item); true }
                "_IfFalse" => { b._IfFalse.FetchFieldImp( item); true }
                "_Name" => { b._Name.FetchFieldImp( item); true }
                "_Arg" => { b._Arg.FetchFieldImp( item); true }
                _ => false,
            }
        });

        let  	validate: ObjValidate< 'a> = Box::new( move || {
            let  	target = unsafe { &mut *target_ptr };
            if let  	Some( entry) = construct_expr_from_builder( &builder)
            {
                *target = entry;
                true
            }
            else
            {
                false
            }
        });

        *field = FieldImp::ObjValidated( cb, validate);
    }
}

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExprCheckpoint
{
    pub _ExprCount:        u32,
    pub _VarCount:         u32,
    pub _SymbolIndexCount: u32,
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
    pub const K_MAX_TREE_DEPTH: u32 = 128;

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
            _ExprCount:        self._Exprs.Size(),
            _VarCount:         self._VarAttribs.Size(),
            _SymbolIndexCount: self._SymbolIndex.Size(),
        }
    }

    pub fn	Rollback( &mut self, cp: ExprCheckpoint)
    {
        self._Exprs.Resize( cp._ExprCount, |_| ExprEntry::Empty);
        self._VarAttribs.Resize( cp._VarCount, |_| VarAttrib::New( String::new()));
        self._SymbolIndex.Resize( cp._SymbolIndexCount, |_| 0);
    }

    pub fn	RebuildSymbolIndex( &mut self)
    {
        self._SymbolIndex.Clear();
        let  	varCount = self._VarAttribs.Size();
        let  	mut v = 0u32;
        while v < varCount
        {
            let  	name = self._VarAttribs[v].Name().to_string();
            let  	mut low = 0u32;
            let  	mut high = self._SymbolIndex.Size();
            while low < high
            {
                let  	mid = low + ( high - low) / 2;
                let  	varId = self._SymbolIndex[mid];
                let  	candName = self._VarAttribs[varId].Name();
                if candName < name.as_str() {
                    low = mid + 1;
                } else {
                    high = mid;
                }
            }
            self._SymbolIndex.Push( v);
            let  	mut i = self._SymbolIndex.Size() - 1;
            while i > low
            {
                self._SymbolIndex[i] = self._SymbolIndex[i - 1];
                i -= 1;
            }
            self._SymbolIndex[low] = v;
            v += 1;
        }
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

    pub fn	FindVar( &self, name: &str) -> u32
    {
        let  	mut low = 0u32;
        let  	mut high = self._SymbolIndex.Size();
        while low < high
        {
            let  	mid = low + ( high - low) / 2;
            let  	varId = self._SymbolIndex[mid];
            let  	candidateName = self._VarAttribs[varId].Name();
            match candidateName.cmp( name)
            {
                core::cmp::Ordering::Equal => return varId,
                core::cmp::Ordering::Less => low = mid + 1,
                core::cmp::Ordering::Greater => high = mid,
            }
        }
        u32::MAX
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

    pub fn	SetVarKind(
        &mut self,
        varIndex: impl Into< u32>,
        kind: VarKind,
        enabled: bool,
    ) -> Result< (), &'static str>
    {
        let  	idx = varIndex.into();
        if idx >= self._VarAttribs.Size()
        {
            return Err( "Variable index out of bounds");
        }
        self._VarAttribs[idx].SetBit( kind, enabled);
        Ok(())
    }

    pub fn	IsValidExprId( &self, id: u32) -> bool
    { id < self._Exprs.Size() }

    pub fn	IsValidVarId( &self, id: u32) -> bool
    { id < self._VarAttribs.Size() }

    pub(crate) fn	ExprEntryAt( &self, id: u32) -> &ExprEntry
    { &self._Exprs[id] }

    pub fn	TryAt< T: BaseExpr>( &self, index: u32) -> Option< &T>
    {
        if index >= self._Exprs.Size()
        {
            return None;
        }
        match &self._Exprs[index]
        {
            ExprEntry::Expr( expression) => expression.Any().downcast_ref::< T>(),
            ExprEntry::Empty => None,
        }
    }

    pub fn	CheckedPowerCreate(
        &mut self,
        base: impl Into< u32>,
        exp: impl Into< u32>,
    ) -> Result< u32, &'static str>
    {
        let  	b = base.into();
        let  	e = exp.into();
        if !self.IsValidExprId( b) || !self.IsValidExprId( e)
        {
            return Err( "Power operand expression ID out of bounds");
        }
        Ok( self.PowerCreate( b, e))
    }

    pub fn	CheckedLogicCreate(
        &mut self,
        op: LogicOp,
        left: impl Into< u32>,
        right: impl Into< u32>,
    ) -> Result< u32, &'static str>
    {
        if op == LogicOp::Not
        {
            return Err( "Binary logic operation cannot be Not");
        }
        let  	l = left.into();
        let  	r = right.into();
        if !self.IsValidExprId( l) || !self.IsValidExprId( r)
        {
            return Err( "Logic operand expression ID out of bounds");
        }
        Ok( self.LogicCreate( op, l, r))
    }

    pub fn	CheckedLogicUnaryCreate(
        &mut self,
        op: LogicOp,
        operand: impl Into< u32>,
    ) -> Result< u32, &'static str>
    {
        if op != LogicOp::Not
        {
            return Err( "Unary logic operation must be Not");
        }
        let  	o = operand.into();
        if !self.IsValidExprId( o)
        {
            return Err( "Logic unary operand expression ID out of bounds");
        }
        Ok( self.LogicUnaryCreate( op, o))
    }

    pub fn	CheckedCondCreate(
        &mut self,
        cond: impl Into< u32>,
        ifTrue: impl Into< u32>,
        ifFalse: impl Into< u32>,
    ) -> Result< u32, &'static str>
    {
        let  	c = cond.into();
        let  	t = ifTrue.into();
        let  	f = ifFalse.into();
        if !self.IsValidExprId( c) || !self.IsValidExprId( t) || !self.IsValidExprId( f)
        {
            return Err( "Conditional operand expression ID out of bounds");
        }
        Ok( self.CondCreate( c, t, f))
    }

    pub fn	CheckedFuncCreate(
        &mut self,
        name: String,
        arg: impl Into< u32>,
    ) -> Result< u32, &'static str>
    {
        if name.is_empty()
        {
            return Err( "Function name cannot be empty");
        }
        let  	a = arg.into();
        if !self.IsValidExprId( a)
        {
            return Err( "Function argument expression ID out of bounds");
        }
        Ok( self.FuncCreate( name, a))
    }

    pub fn	PostTermTree( &mut self, root: &dyn ITermNode) -> u32
    {
        let  	mut expressions = Stash::New();
        Self::TraverseTree( self, root, 0, &mut expressions);
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
        depth: u32,
        isPositive: bool,
        posTerms: &mut Stash< u32>,
        negTerms: &mut Stash< u32>,
    )
    {
        assert!( depth < Self::K_MAX_TREE_DEPTH, "Term tree nesting exceeds maximum supported depth");
        match node.Op()
        {
            BinOp::Sum =>
            {
                Self::CollectAdditive( repos, node.Child( 0), depth + 1, isPositive, posTerms, negTerms);
                Self::CollectAdditive( repos, node.Child( 1), depth + 1, isPositive, posTerms, negTerms);
            }
            BinOp::Sub =>
            {
                Self::CollectAdditive( repos, node.Child( 0), depth + 1, isPositive, posTerms, negTerms);
                Self::CollectAdditive( repos, node.Child( 1), depth + 1, !isPositive, posTerms, negTerms);
            }
            _ =>
            {
                let  	mut exprs = Stash::New();
                Self::TraverseTree( repos, node, depth + 1, &mut exprs);
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
        depth: u32,
        isNumerator: bool,
        numFactors: &mut Stash< u32>,
        denFactors: &mut Stash< u32>,
    )
    {
        assert!( depth < Self::K_MAX_TREE_DEPTH, "Term tree nesting exceeds maximum supported depth");
        match node.Op()
        {
            BinOp::Prod =>
            {
                Self::CollectMultiplicative( repos, node.Child( 0), depth + 1, isNumerator, numFactors, denFactors);
                Self::CollectMultiplicative( repos, node.Child( 1), depth + 1, isNumerator, numFactors, denFactors);
            }
            BinOp::Div =>
            {
                Self::CollectMultiplicative( repos, node.Child( 0), depth + 1, isNumerator, numFactors, denFactors);
                Self::CollectMultiplicative( repos, node.Child( 1), depth + 1, !isNumerator, numFactors, denFactors);
            }
            _ =>
            {
                let  	mut exprs = Stash::New();
                Self::TraverseTree( repos, node, depth + 1, &mut exprs);
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

    fn	TraverseTree( repos: &mut Self, node: &dyn ITermNode, depth: u32, expressions: &mut Stash< u32>)
    {
        assert!( depth < Self::K_MAX_TREE_DEPTH, "Term tree nesting exceeds maximum supported depth");
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
                Self::CollectAdditive( repos, node, depth + 1, true, &mut posTerms, &mut negTerms);
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
                Self::CollectMultiplicative( repos, node, depth + 1, true, &mut numFactors, &mut denFactors);
                let  	coSize = numFactors.Size();
                let  	mut children = Stash::WithCapacity( numFactors.Size() + denFactors.Size());
                numFactors.USeg().Traverse( |idx| children.Push( numFactors[idx]));
                denFactors.USeg().Traverse( |idx| children.Push( denFactors[idx]));
                repos.Store( Box::new( ProdExpr::New( children.IntoBuff(), coSize)))
            }
            BinOp::Pow =>
            {
                let  	mut leftExprs = Stash::New();
                Self::TraverseTree( repos, node.Child( 0), depth + 1, &mut leftExprs);
                let  	left = leftExprs.Pop().expect( "Missing base in pow term");

                let  	mut rightExprs = Stash::New();
                Self::TraverseTree( repos, node.Child( 1), depth + 1, &mut rightExprs);
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

impl IFluxExportSource for ExprRepos
{
    fn	FetchFieldExp< 'a>( &'a self, field: &mut FieldExp< 'a>)
    {
        let  	mut step = 0u32;
        let  	repos = self;
        *field = FieldExp::Obj( Box::new( move |key, item| {
            if step == 0 {
                *key = "_Exprs".to_string();
                *item = FieldExp::FluxSource( &repos._Exprs);
                step += 1;
                return true;
            }
            if step == 1 {
                *key = "_VarAttribs".to_string();
                *item = FieldExp::FluxSource( &repos._VarAttribs);
                step += 1;
                return true;
            }
            false
        }));
    }
}

//-------------------------------------------------------------------------------------------------

impl IFluxImportSource for ExprRepos
{
    fn	FetchFieldImp< 'a>( &'a mut self, field: &mut FieldImp< 'a>)
    {
        let  	repos_ptr = self as *mut Self;
        let  	cb: ObjImp< 'a> = Box::new( move |key, item| {
            let  	repos = unsafe { &mut *repos_ptr };
            match key
            {
                "_Exprs" => {
                    repos._Exprs.FetchFieldImp( item);
                    true
                }
                "_VarAttribs" => {
                    repos._VarAttribs.FetchFieldImp( item);
                    true
                }
                _ => false,
            }
        });
        let  	validate: ObjValidate< 'a> = Box::new( move || {
            let  	repos = unsafe { &mut *repos_ptr };
            repos.RebuildSymbolIndex();
            true
        });
        *field = FieldImp::ObjValidated( cb, validate);
    }
}

//-------------------------------------------------------------------------------------------------
