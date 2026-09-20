// varexpr.rs -----------------------------------------------------------------------------------------------------
use	crate::fresco::BaseExpr;
use	core::any::Any;
#[derive( Copy, Clone, Debug, PartialEq, Eq)]
pub enum VarKind { Scalar = 0, Prime = 1, Control = 2, Bridge = 3 }
#[derive( Clone, Debug)]
pub struct VarAttrib
{
    _Name: String,
    _DepExpr: Option< u32>,
    _AggrIndex: Option< u32>,
    _Flags: u32,
}
impl VarAttrib
{
    pub fn	New( name: String) -> Self
    {
        Self { _Name: name, _DepExpr: None, _AggrIndex: None, _Flags: 0 }
    }
    pub fn	Name( &self) -> &str
    { &self._Name }
    pub fn	IsAggregate( &self) -> bool
    { self._AggrIndex.is_some() }
    pub fn	IsIndependent( &self) -> bool
    { !self.IsAggregate() && self._DepExpr.is_none() }
    pub fn	IsDependent( &self) -> bool
    { !self.IsAggregate() && self._DepExpr.is_some() }
    pub fn	HasBits( &self, kind: VarKind) -> bool
    { self._Flags & ( 1 << kind as u32) != 0 }
}
impl Default for VarAttrib {
    fn	default() -> Self
    { Self::New( String::new()) }
}
#[derive( Clone)]
pub struct VarExpr
{ _VarIndex: u32 }
impl VarExpr
{
    pub fn	New( varIndex: impl Into< u32>) -> Self
    { Self
    { _VarIndex: varIndex.into() } }
    pub fn	VarIndex( &self) -> u32
    { self._VarIndex }
}
impl BaseExpr for VarExpr {
    fn	CloneBox( &self) -> Box< dyn BaseExpr>
    { Box::new( self.clone()) }
    fn	Any( &self) -> &dyn Any
    { self }
}
crate::ImplFluxSource!( VarAttrib, _Name, _DepExpr, _AggrIndex, _Flags);
crate::ImplFluxSourceTyped!( VarExpr, "VarExpr", _VarIndex);

//-------------------------------------------------------------------------------------------------
