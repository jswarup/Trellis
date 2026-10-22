// funcexpr.rs ------------------------------------------------------------------------------------
use	crate::fresco::BaseExpr;
use	core::any::Any;

//-------------------------------------------------------------------------------------------------

#[derive( Clone)]
pub struct FuncExpr
{
    _Name: String,
    _Arg:  u32,
}

//-------------------------------------------------------------------------------------------------

impl FuncExpr
{
    pub fn	New( name: String, arg: impl Into< u32>) -> Self
    {
        Self {
            _Name: name,
            _Arg:  arg.into(),
        }
    }

    pub fn	Name( &self) -> &str
    { &self._Name }

    pub fn	Arg( &self) -> u32
    { self._Arg }
}

//-------------------------------------------------------------------------------------------------

impl BaseExpr for FuncExpr
{
    fn	CloneBox( &self) -> Box< dyn BaseExpr>
    { Box::new( self.clone()) }

    fn	Any( &self) -> &dyn Any
    { self }
}

//-------------------------------------------------------------------------------------------------

crate::ImplFluxSourceTyped!( FuncExpr, "FuncExpr", _Name, _Arg);

//-------------------------------------------------------------------------------------------------
