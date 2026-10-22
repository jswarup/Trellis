// condexpr.rs ------------------------------------------------------------------------------------
use	crate::fresco::BaseExpr;
use	core::any::Any;

//-------------------------------------------------------------------------------------------------

#[derive( Clone)]
pub struct CondExpr
{
    _Cond:    u32,
    _IfTrue:  u32,
    _IfFalse: u32,
}

//-------------------------------------------------------------------------------------------------

impl CondExpr
{
    pub fn	New( cond: impl Into< u32>, ifTrue: impl Into< u32>, ifFalse: impl Into< u32>) -> Self
    {
        Self {
            _Cond:    cond.into(),
            _IfTrue:  ifTrue.into(),
            _IfFalse: ifFalse.into(),
        }
    }

    pub fn	Cond( &self) -> u32
    { self._Cond }

    pub fn	IfTrue( &self) -> u32
    { self._IfTrue }

    pub fn	IfFalse( &self) -> u32
    { self._IfFalse }
}

//-------------------------------------------------------------------------------------------------

impl BaseExpr for CondExpr
{
    fn	CloneBox( &self) -> Box< dyn BaseExpr>
    { Box::new( self.clone()) }

    fn	Any( &self) -> &dyn Any
    { self }
}

//-------------------------------------------------------------------------------------------------

crate::ImplFluxSourceTyped!( CondExpr, "CondExpr", _Cond, _IfTrue, _IfFalse);

//-------------------------------------------------------------------------------------------------
