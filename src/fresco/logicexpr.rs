// logicexpr.rs -----------------------------------------------------------------------------------
use	crate::flux::{ FieldExp, FieldImp, IFluxExportSource, IFluxImportSink, IFluxImportSource };
use	crate::fresco::BaseExpr;
use	core::any::Any;

//-------------------------------------------------------------------------------------------------

#[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum LogicOp
{
    #[default]
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

//-------------------------------------------------------------------------------------------------

impl LogicOp
{
    pub fn	AsStr( &self) -> &'static str
    {
        match self
        {
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::And => "and",
            Self::Or => "or",
        }
    }

    pub fn	FromStr( s: &str) -> Option< Self>
    {
        match s
        {
            "==" | "=" => Some( Self::Eq),
            "!=" => Some( Self::Ne),
            "<" => Some( Self::Lt),
            "<=" => Some( Self::Le),
            ">" => Some( Self::Gt),
            ">=" => Some( Self::Ge),
            "and" | "AND" => Some( Self::And),
            "or" | "OR" => Some( Self::Or),
            _ => None,
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl IFluxExportSource for LogicOp
{
    fn	FetchFieldExp< 'a>( &self, field: &mut FieldExp< 'a>)
    {
        *field = FieldExp::Str( self.AsStr());
    }
}

//-------------------------------------------------------------------------------------------------

impl IFluxImportSource for LogicOp
{
    fn	FetchFieldImp< 'a>( &'a mut self, field: &mut FieldImp< 'a>)
    {
        *field = FieldImp::FluxSink( self);
    }
}

//-------------------------------------------------------------------------------------------------

impl IFluxImportSink for LogicOp
{
    fn	FromFieldImp( &mut self, field: FieldImp) -> bool
    {
        match field
        {
            FieldImp::Str( s) =>
            {
                if let  	Some( op) = Self::FromStr( s)
                {
                    *self = op;
                    true
                }
                else
                {
                    false
                }
            }
            FieldImp::String( s) =>
            {
                if let  	Some( op) = Self::FromStr( s)
                {
                    *self = op;
                    true
                }
                else
                {
                    false
                }
            }
            _ => false,
        }
    }
}

//-------------------------------------------------------------------------------------------------

#[derive( Clone)]
pub struct LogicExpr
{
    _Op:    LogicOp,
    _Left:  u32,
    _Right: u32,
}

//-------------------------------------------------------------------------------------------------

impl LogicExpr
{
    pub fn	New( op: LogicOp, left: impl Into< u32>, right: impl Into< u32>) -> Self
    {
        Self {
            _Op:    op,
            _Left:  left.into(),
            _Right: right.into(),
        }
    }

    pub fn	Op( &self) -> LogicOp
    { self._Op }

    pub fn	Left( &self) -> u32
    { self._Left }

    pub fn	Right( &self) -> u32
    { self._Right }
}

//-------------------------------------------------------------------------------------------------

impl BaseExpr for LogicExpr
{
    fn	CloneBox( &self) -> Box< dyn BaseExpr>
    { Box::new( self.clone()) }

    fn	Any( &self) -> &dyn Any
    { self }
}

//-------------------------------------------------------------------------------------------------

crate::ImplFluxSourceTyped!( LogicExpr, "LogicExpr", _Op, _Left, _Right);

//-------------------------------------------------------------------------------------------------
