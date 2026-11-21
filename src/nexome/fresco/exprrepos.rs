// exprrepos.rs -----------------------------------------------------------------------------------
use crate::flux::{
    FieldExp, FieldImp, FluxError, IFluxExportSource, IFluxImportSource, IObjectImport,
};
use crate::fresco::{
    CondExpr, ExprReadError, ExprShard, ExprView, FuncExpr, ITermNode, K_MAX_TENSOR_RANK, LogicExpr, LogicOp, PolyExpr,
    PowExpr, ProdExpr, RealExpr, ShapeError, SumExpr, TensorShape, Term, VarAttrib, VarExpr,
    VarKind,
};
use crate::silo::{Arr, Buff, Stash};
use crate::stalks::BinOp;
use core::any::Any;

//-------------------------------------------------------------------------------------------------

pub trait BaseExpr: Any + IFluxExportSource
{
    fn	CloneBox( &self) -> Box< dyn BaseExpr>;

    fn	Any( &self) -> &dyn Any;

    /// Extensions may expose an existing semantic kind; unknown extensions remain unsupported.
    fn View(&self) -> Option<ExprView<'_>> { return None; }
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

fn construct_expr_from_builder(b: ExprNodeBuilder) -> Option<ExprEntry>
{
    match b._Type.as_str()
    {
        "RealExpr" => Some( ExprEntry::Expr( Box::new( RealExpr::New( b._Value)))),
        "VarExpr" => Some( ExprEntry::Expr( Box::new( VarExpr::New( b._VarIndex)))),
        "SumExpr" =>
        {
            let  	coSize = b._Poly.CommonSize();
            Some( ExprEntry::Expr( Box::new( SumExpr::New( b._Poly.IntoChildren(), coSize))))
        }
        "ProdExpr" =>
        {
            let  	coSize = b._Poly.CommonSize();
            Some( ExprEntry::Expr( Box::new( ProdExpr::New( b._Poly.IntoChildren(), coSize))))
        }
        "PowExpr" =>
        {
            let  	coSize = b._Poly.CommonSize();
            Some( ExprEntry::Expr( Box::new( PowExpr::New( b._Poly.IntoChildren(), coSize))))
        }
        "LogicExpr" => Some( ExprEntry::Expr( Box::new( LogicExpr::New( b._Op, b._Left, b._Right)))),
        "CondExpr" => Some( ExprEntry::Expr( Box::new( CondExpr::New( b._Cond, b._IfTrue, b._IfFalse)))),
        "FuncExpr" => Some( ExprEntry::Expr( Box::new( FuncExpr::New( b._Name, b._Arg)))),
        _ => None,
    }
}

//-------------------------------------------------------------------------------------------------

struct ExprEntryImport<'a>
{
    _Target:  &'a mut ExprEntry,
    _Builder: ExprNodeBuilder,
}

impl IObjectImport for ExprEntryImport<'_>
{
    fn Field(&mut self, key: &str) -> Result<FieldImp<'_>, FluxError>
    {
        let b = &mut self._Builder;
        return Ok(match key {
            "Type" => FieldImp::FluxSource(&mut b._Type),
            "_Value" => FieldImp::FluxSource(&mut b._Value),
            "_VarIndex" => FieldImp::FluxSource(&mut b._VarIndex),
            "_Poly" => FieldImp::FluxSource(&mut b._Poly),
            "_Op" => FieldImp::FluxSource(&mut b._Op),
            "_Left" => FieldImp::FluxSource(&mut b._Left),
            "_Right" => FieldImp::FluxSource(&mut b._Right),
            "_Cond" => FieldImp::FluxSource(&mut b._Cond),
            "_IfTrue" => FieldImp::FluxSource(&mut b._IfTrue),
            "_IfFalse" => FieldImp::FluxSource(&mut b._IfFalse),
            "_Name" => FieldImp::FluxSource(&mut b._Name),
            "_Arg" => FieldImp::FluxSource(&mut b._Arg),
            _ => return Err(FluxError::TypeMismatch),
        });
    }

    fn Commit(self: Box<Self>) -> Result<(), FluxError>
    {
        let Self { _Target: target,
                   _Builder: builder, } = *self;
        let entry = construct_expr_from_builder(builder).ok_or(FluxError::TypeMismatch)?;
        *target = entry;
        return Ok(());
    }
}

impl IFluxImportSource for ExprEntry
{
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>)
    {
        *field = FieldImp::Obj(Box::new(ExprEntryImport { _Target:  self,
                                                          _Builder: ExprNodeBuilder::default(), }));
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

    pub fn	FindVarExact( &self, name: &str) -> u32
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

    pub fn	FindVar( &self, name: &str) -> u32
    {
        let  	exact = self.FindVarExact( name);
        if exact != u32::MAX
        {
            return exact;
        }
        if name.contains( "][")
        {
            let  	normalized = name.replace( "][", ",");
            return self.FindVarExact( &normalized);
        }
        u32::MAX
    }

    fn	InsertSymbolIndex( &mut self, varId: u32, name: &str)
    {
        let  	mut low = 0u32;
        let  	mut high = self._SymbolIndex.Size();
        while low < high
        {
            let  	mid = low + ( high - low) / 2;
            let  	candVarId = self._SymbolIndex[mid];
            let  	candName = self._VarAttribs[candVarId].Name();
            if candName < name
            {
                low = mid + 1;
            }
            else
            {
                high = mid;
            }
        }
        self._SymbolIndex.Push( varId);
        let  	mut i = self._SymbolIndex.Size() - 1;
        while i > low
        {
            self._SymbolIndex[i] = self._SymbolIndex[i - 1];
            i -= 1;
        }
        self._SymbolIndex[low] = varId;
    }

    fn	DecodeLevelCoords(
        shape: &TensorShape,
        level: usize,
        p: u32,
    ) -> [u32; K_MAX_TENSOR_RANK]
    {
        let  	dims = shape.DimsArray();
        let  	mut strides = [1u32; K_MAX_TENSOR_RANK];
        if level > 1
        {
            let  	mut j = level - 1;
            while j > 0
            {
                strides[j - 1] = strides[j] * dims[j];
                j -= 1;
            }
        }
        let  	mut coords = [0u32; K_MAX_TENSOR_RANK];
        let  	mut rem = p;
        let  	mut i = 0usize;
        while i < level
        {
            coords[i] = rem / strides[i];
            rem %= strides[i];
            i += 1;
        }
        coords
    }

    fn	FormatSubVarName(
        name: &str,
        shape: &TensorShape,
        level: usize,
        p: u32,
    ) -> String
    {
        let  	coords = Self::DecodeLevelCoords( shape, level, p);
        if level == 1
        {
            return format!( "{}[{}]", name, coords[0]);
        }
        let  	mut s = format!( "{}[{}", name, coords[0]);
        let  	mut i = 1usize;
        while i < level
        {
            s.push_str( &format!( ",{}", coords[i]));
            i += 1;
        }
        s.push( ']');
        s
    }

    fn	ComputeSubVarOffset(
        shape: &TensorShape,
        level: usize,
        p: u32,
    ) -> u32
    {
        let  	coords = Self::DecodeLevelCoords( shape, level, p);
        let  	rootStrides = shape.Strides();
        let  	mut offset = 0u32;
        let  	mut i = 0usize;
        while i < level
        {
            offset += coords[i] * rootStrides[i];
            i += 1;
        }
        offset
    }

    fn	LeafSubVarStart( &self, aggrVarId: u32) -> u32
    {
        let  	attr = self.VarAttrAt( aggrVarId);
        let  	rank = attr.Shape().Rank();
        if rank <= 1
        {
            return aggrVarId + 1;
        }
        let  	dims = attr.Shape().DimsArray();
        let  	mut intermediateCount = 0u32;
        let  	mut levelCount = 1u32;
        let  	mut l = 0usize;
        while l < ( rank - 1) as usize
        {
            levelCount = levelCount.saturating_mul( dims[l]);
            intermediateCount = intermediateCount.saturating_add( levelCount);
            l += 1;
        }
        aggrVarId + 1 + intermediateCount
    }

    pub fn	ScalarVarCreate( &mut self, name: &str) -> Result< u32, ShapeError>
    {
        if self.FindVarExact( name) != u32::MAX
        {
            return Err( ShapeError::DuplicateSymbol( name.to_string()));
        }
        self.VarCreate( name.to_string(), false);
        let  	varId = self.FindVarExact( name);
        Ok( varId)
    }

    pub fn	VectorVarCreate( &mut self, name: &str, len: u32) -> Result< u32, ShapeError>
    {
        let  	shape = TensorShape::Vector( len)?;
        self.TensorVarCreate( name, shape)
    }

    pub fn	MatrixVarCreate( &mut self, name: &str, rows: u32, cols: u32) -> Result< u32, ShapeError>
    {
        let  	shape = TensorShape::Matrix( rows, cols)?;
        self.TensorVarCreate( name, shape)
    }

    fn	ValidateTensorVarNames(
        &self,
        name: &str,
        shape: &TensorShape,
        levelCounts: &[u32; K_MAX_TENSOR_RANK + 1],
    ) -> Result< (), ShapeError>
    {
        if self.FindVar( name) != u32::MAX
        {
            return Err( ShapeError::DuplicateSymbol( name.to_string()));
        }
        let  	rank = shape.Rank() as usize;
        let  	mut lev = 1usize;
        while lev <= rank
        {
            let  	numAtLev = levelCounts[lev];
            let  	mut p = 0u32;
            while p < numAtLev
            {
                let  	subName = Self::FormatSubVarName( name, shape, lev, p);
                if self.FindVar( &subName) != u32::MAX
                {
                    return Err( ShapeError::DuplicateSymbol( subName));
                }
                p += 1;
            }
            lev += 1;
        }
        Ok( ())
    }

    fn	MaterializeTensorLevels(
        &mut self,
        name: &str,
        shape: &TensorShape,
        parentVarId: u32,
        levelCounts: &[u32; K_MAX_TENSOR_RANK + 1],
    ) -> Result< (), ShapeError>
    {
        let  	rank = shape.Rank() as usize;
        let  	dims = shape.DimsArray();
        let  	mut levelStarts = [0u32; K_MAX_TENSOR_RANK + 1];
        levelStarts[0] = parentVarId;
        levelStarts[1] = parentVarId + 1;
        let  	mut s = 1usize;
        while s < rank
        {
            levelStarts[s + 1] = levelStarts[s] + levelCounts[s];
            s += 1;
        }

        let  	mut currLev = 1usize;
        while currLev <= rank
        {
            let  	numAtLev = levelCounts[currLev];
            let  	levShape = shape.SliceShape( currLev)?;
            let  	isLeaf = currLev == rank;
            let  	nextLevStart = if isLeaf { u32::MAX } else { levelStarts[currLev + 1] };
            let  	dCurr = if isLeaf { 0 } else { dims[currLev] };

            let  	mut p = 0u32;
            while p < numAtLev
            {
                let  	subName = Self::FormatSubVarName( name, shape, currLev, p);
                let  	subVarId = self._VarAttribs.Size();
                let  	subOffset = Self::ComputeSubVarOffset( shape, currLev, p);
                let  	childStart = if isLeaf { u32::MAX } else { nextLevStart + p * dCurr };

                let  	mut subAttr = if isLeaf
                {
                    VarAttrib::SubVar( subName.clone(), parentVarId, subOffset)
                }
                else
                {
                    VarAttrib::SubTensorVar( subName.clone(), parentVarId, subOffset, levShape)
                };
                subAttr.SetSubVarStart( childStart);
                self._VarAttribs.Push( subAttr);
                self.InsertSymbolIndex( subVarId, &subName);
                p += 1;
            }
            currLev += 1;
        }
        Ok( ())
    }

    pub fn	TensorVarCreate(
        &mut self,
        name: &str,
        shape: TensorShape,
    ) -> Result< u32, ShapeError>
    {
        let  	rank = shape.Rank();
        if rank == 0
        {
            return Err( ShapeError::RankMismatch { expected: 1, actual: 0 });
        }
        let  	dims = shape.DimsArray();
        let  	mut levelCounts = [0u32; K_MAX_TENSOR_RANK + 1];
        levelCounts[0] = 1;
        let  	mut l = 0usize;
        while l < rank as usize
        {
            levelCounts[l + 1] = levelCounts[l].checked_mul( dims[l]).ok_or( ShapeError::Overflow)?;
            l += 1;
        }

        self.ValidateTensorVarNames( name, &shape, &levelCounts)?;

        let  	parentVarId = self._VarAttribs.Size();
        let  	mut parentAttr = VarAttrib::WithShape( name.to_string(), shape);
        let  	level1Start = parentVarId + 1;
        parentAttr.SetSubVarStart( level1Start);
        self._VarAttribs.Push( parentAttr);
        self.InsertSymbolIndex( parentVarId, name);

        self.MaterializeTensorLevels( name, &shape, parentVarId, &levelCounts)?;
        Ok( parentVarId)
    }

    pub fn	SubVar( &self, parentVarId: u32, indices: Arr< '_, u32>) -> Result< u32, ShapeError>
    {
        if !self.IsValidVarId( parentVarId)
        {
            return Err( ShapeError::RankMismatch { expected: 1, actual: 0 });
        }
        let  	attr = self.VarAttrAt( parentVarId);
        let  	rank = attr.Shape().Rank() as usize;
        let  	idxLen = indices.Size() as usize;
        if idxLen == 0
        {
            return Ok( parentVarId);
        }
        if idxLen > rank
        {
            return Err( ShapeError::RankMismatch {
                expected: rank as u32,
                actual:   idxLen as u32,
            });
        }

        let  	idx0 = indices[0];
        let  	dim0 = attr.Shape().Dim( 0);
        if idx0 >= dim0
        {
            return Err( ShapeError::IndexOutOfBounds {
                axis:  0,
                index: idx0,
                dim:   dim0,
            });
        }
        let  	start = attr.SubVarStart();
        if start == u32::MAX
        {
            return Err( ShapeError::RankMismatch { expected: 1, actual: 0 });
        }
        let  	childSubVar = start + idx0;

        if idxLen == 1
        {
            return Ok( childSubVar);
        }

        self.SubVar( childSubVar, indices.LSnip( 1))
    }

    pub fn	SubVarSlice( &self, parentVarId: u32, indices: &[u32]) -> Result< u32, ShapeError>
    {
        self.SubVar( parentVarId, Arr::New( indices.as_ptr(), indices.len() as u32))
    }

    pub fn	SubVarLinear( &self, parentVarId: u32, flatIndex: u32) -> Result< u32, ShapeError>
    {
        if !self.IsValidVarId( parentVarId)
        {
            return Err( ShapeError::RankMismatch { expected: 1, actual: 0 });
        }
        let  	attr = self.VarAttrAt( parentVarId);
        let  	total = attr.Shape().TotalElements();
        if flatIndex >= total
        {
            return Err( ShapeError::LinearIndexOutOfBounds { index: flatIndex, total });
        }
        if attr.Shape().IsScalar() && attr.IsSubVar() && flatIndex == 0
        {
            return Ok( parentVarId);
        }
        let  	rootId = if attr.IsSubVar() { attr.AggrIndex() } else { parentVarId };
        let  	leafStart = self.LeafSubVarStart( rootId);
        Ok( leafStart + attr.SubVarOffset() + flatIndex)
    }

    pub fn	SubVarExpr( &mut self, parentVarId: u32, indices: Arr< '_, u32>) -> Result< u32, ShapeError>
    {
        let  	subVarId = self.SubVar( parentVarId, indices)?;
        let  	subAttr = self.VarAttrAt( subVarId);
        if !subAttr.Shape().IsScalar()
        {
            return Err( ShapeError::RankMismatch {
                expected: 0,
                actual:   subAttr.Shape().Rank(),
            });
        }
        Ok( self.Store( Box::new( VarExpr::New( subVarId))))
    }

    pub fn	SubVarExprSlice( &mut self, parentVarId: u32, indices: &[u32]) -> Result< u32, ShapeError>
    {
        self.SubVarExpr( parentVarId, Arr::New( indices.as_ptr(), indices.len() as u32))
    }

    pub fn	SubVarLinearExpr( &mut self, parentVarId: u32, flatIndex: u32) -> Result< u32, ShapeError>
    {
        let  	subVarId = self.SubVarLinear( parentVarId, flatIndex)?;
        return Ok( self.Store( Box::new( VarExpr::New( subVarId))));
    }

    pub fn	SubVarShape( &self, varId: u32) -> &TensorShape
    {
        self.VarAttrAt( varId).Shape()
    }

    pub fn	IsSubVar( &self, varId: u32) -> bool
    {
        self.VarAttrAt( varId).IsSubVar()
    }

    pub fn	SubVarOffset( &self, varId: u32) -> u32
    {
        self.VarAttrAt( varId).SubVarOffset()
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

    /// Looks up a node without allocating, cloning it, or exposing its concrete storage type.
    pub fn Inspect(&self, id: u32) -> Result<ExprView<'_>, ExprReadError>
    {
        let Some(entry) = self._Exprs.Arr().Get(id) else {
            return Err(ExprReadError::InvalidId(id));
        };
        return match entry {
            ExprEntry::Empty => Err(ExprReadError::EmptyNode(id)),
            ExprEntry::Expr(expression) => expression.View().ok_or(ExprReadError::UnsupportedNode(id)),
        };
    }

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

struct ExprReposImport<'a>
{
    _Target: &'a mut ExprRepos,
    _Stage:  ExprRepos,
}

impl IObjectImport for ExprReposImport<'_>
{
    fn Field(&mut self, key: &str) -> Result<FieldImp<'_>, FluxError>
    {
        return match key {
            "_Exprs" => Ok(FieldImp::FluxSource(&mut self._Stage._Exprs)),
            "_VarAttribs" => Ok(FieldImp::FluxSource(&mut self._Stage._VarAttribs)),
            _ => Err(FluxError::TypeMismatch),
        };
    }

    fn Commit(self: Box<Self>) -> Result<(), FluxError>
    {
        let Self { _Target: target,
                   _Stage: mut stage, } = *self;
        stage.RebuildSymbolIndex();
        *target = stage;
        return Ok(());
    }
}

impl IFluxImportSource for ExprRepos
{
    fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>)
    {
        // Preserve partial-update semantics while keeping failed imports isolated.
        let stage = ExprRepos { _Exprs:       self._Exprs.clone(),
                                _VarAttribs:  self._VarAttribs.clone(),
                                _SymbolIndex: Stash::New(), };
        *field = FieldImp::Obj(Box::new(ExprReposImport { _Target: self,
                                                          _Stage:  stage, }));
    }
}

//-------------------------------------------------------------------------------------------------
