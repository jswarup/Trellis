// varexpr.rs -------------------------------------------------------------------------------------
use	crate::fresco::{ BaseExpr, TensorShape };
use	core::any::Any;

//-------------------------------------------------------------------------------------------------

#[derive( Copy, Clone, Debug, PartialEq, Eq)]
pub enum VarKind
{
    Scalar  = 0,
    Prime   = 1,
    Control = 2,
    Bridge  = 3,
    NonZero = 4,
}

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Debug)]
pub struct VarAttrib
{
    _Name:         String,
    _DepExpr:      u32,
    _AggrIndex:    u32,
    _SubVarOffset: u32,
    _SubVarStart:  u32,
    _Flags:        u32,
    _Shape:        TensorShape,
}

//-------------------------------------------------------------------------------------------------

impl VarAttrib
{
    pub fn	New( name: String) -> Self
    {
        Self {
            _Name:         name,
            _DepExpr:      u32::MAX,
            _AggrIndex:    u32::MAX,
            _SubVarOffset: 0,
            _SubVarStart:  u32::MAX,
            _Flags:        0,
            _Shape:        TensorShape::Scalar(),
        }
    }

    pub fn	WithShape( name: String, shape: TensorShape) -> Self
    {
        Self {
            _Name:         name,
            _DepExpr:      u32::MAX,
            _AggrIndex:    u32::MAX,
            _SubVarOffset: 0,
            _SubVarStart:  u32::MAX,
            _Flags:        0,
            _Shape:        shape,
        }
    }

    pub fn	SubVar( name: String, aggrIndex: u32, offset: u32) -> Self
    {
        Self {
            _Name:         name,
            _DepExpr:      u32::MAX,
            _AggrIndex:    aggrIndex,
            _SubVarOffset: offset,
            _SubVarStart:  u32::MAX,
            _Flags:        0,
            _Shape:        TensorShape::Scalar(),
        }
    }

    pub fn	SubTensorVar( name: String, aggrIndex: u32, offset: u32, shape: TensorShape) -> Self
    {
        Self {
            _Name:         name,
            _DepExpr:      u32::MAX,
            _AggrIndex:    aggrIndex,
            _SubVarOffset: offset,
            _SubVarStart:  u32::MAX,
            _Flags:        0,
            _Shape:        shape,
        }
    }

    pub fn	Name( &self) -> &str
    { &self._Name }

    pub fn	DepExpr( &self) -> u32
    { self._DepExpr }

    pub fn	AggrIndex( &self) -> u32
    { self._AggrIndex }

    pub fn	SetAggrIndex( &mut self, aggrIndex: u32)
    { self._AggrIndex = aggrIndex; }

    pub fn	SubVarOffset( &self) -> u32
    { self._SubVarOffset }

    pub fn	SetSubVarOffset( &mut self, offset: u32)
    { self._SubVarOffset = offset; }

    pub fn	SubVarStart( &self) -> u32
    { self._SubVarStart }

    pub fn	SetSubVarStart( &mut self, start: u32)
    { self._SubVarStart = start; }

    pub fn	Shape( &self) -> &TensorShape
    { &self._Shape }

    pub fn	SetShape( &mut self, shape: TensorShape)
    { self._Shape = shape; }

    pub fn	IsAggregate( &self) -> bool
    { !self._Shape.IsScalar() }

    pub fn	IsRootAggregate( &self) -> bool
    { !self._Shape.IsScalar() && self._AggrIndex == u32::MAX }

    pub fn	IsSubTensor( &self) -> bool
    { !self._Shape.IsScalar() && self._AggrIndex != u32::MAX }

    pub fn	IsLeafSubVar( &self) -> bool
    { self._Shape.IsScalar() && self._AggrIndex != u32::MAX }

    pub fn	IsSubVar( &self) -> bool
    { self._AggrIndex != u32::MAX }

    pub fn	IsIndependent( &self) -> bool
    { !self.IsAggregate() && !self.IsSubVar() && self._DepExpr == u32::MAX }

    pub fn	IsDependent( &self) -> bool
    { !self.IsAggregate() && !self.IsSubVar() && self._DepExpr != u32::MAX }

    pub fn	Flags( &self) -> u32
    { self._Flags }

    pub fn	HasBits( &self, kind: VarKind) -> bool
    { self._Flags & ( 1 << kind as u32) != 0 }

    pub fn	SetBit( &mut self, kind: VarKind, enabled: bool)
    {
        let  	mask = 1 << kind as u32;
        if enabled
        {
            self._Flags |= mask;
        }
        else
        {
            self._Flags &= !mask;
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl Default for VarAttrib
{
    fn	default() -> Self
    { Self::New( String::new()) }
}

//-------------------------------------------------------------------------------------------------

#[derive( Clone)]
pub struct VarExpr
{
    _VarIndex: u32,
}

//-------------------------------------------------------------------------------------------------

impl VarExpr
{
    pub fn	New( varIndex: impl Into< u32>) -> Self
    { Self { _VarIndex: varIndex.into() } }

    pub fn	VarIndex( &self) -> u32
    { self._VarIndex }
}

//-------------------------------------------------------------------------------------------------

impl BaseExpr for VarExpr
{
    fn	CloneBox( &self) -> Box< dyn BaseExpr>
    { Box::new( self.clone()) }

    fn	Any( &self) -> &dyn Any
    { self }
}

//-------------------------------------------------------------------------------------------------

crate::ImplFluxSource!( VarAttrib, _Name, _DepExpr, _AggrIndex, _SubVarOffset, _SubVarStart, _Flags, _Shape);
crate::ImplFluxSourceTyped!( VarExpr, "VarExpr", _VarIndex);

//-------------------------------------------------------------------------------------------------
