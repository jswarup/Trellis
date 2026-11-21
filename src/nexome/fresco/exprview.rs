// exprview.rs ------------------------------------------------------------------------------------
//! Borrowed expression descriptions, independent of the repository's node storage.

use crate::fresco::{LogicOp, PolyExpr};
use crate::silo::Arr;
use core::fmt;

//-------------------------------------------------------------------------------------------------

/// Ordered children, with common operands followed by flipped operands.
#[derive(Clone, Copy)]
pub struct PolyView<'a>
{
    _Children: Arr<'a, u32>,
    _CoSize:   u32,
}

impl<'a> PolyView<'a>
{
    pub fn FromParts(children: Arr<'a, u32>, coSize: u32) -> Self
    {
        assert!(coSize <= children.Size(), "Common child count exceeds child count");
        return Self { _Children: children, _CoSize: coSize };
    }

    pub fn Children(&self) -> Arr<'a, u32> { return self._Children; }

    pub fn SizeChild(&self) -> u32 { return self._Children.Size(); }

    pub fn CommonSize(&self) -> u32 { return self._CoSize; }

    pub fn Child(&self, index: u32) -> u32 { return self._Children[index]; }

    pub fn IsFlip(&self, index: u32) -> bool { return index >= self._CoSize; }

    pub fn PositiveChildren(&self) -> Arr<'a, u32>
    { return self._Children.Slice(0, self._CoSize); }

    pub fn NegativeChildren(&self) -> Arr<'a, u32>
    { return self._Children.Slice(self._CoSize, self._Children.Size() - self._CoSize); }
}

impl<'a> From<&'a PolyExpr> for PolyView<'a>
{
    fn from(poly: &'a PolyExpr) -> Self
    { return Self::FromParts(poly.Children().Arr(), poly.CommonSize()); }
}

//-------------------------------------------------------------------------------------------------

/// Inspection copies scalar data and IDs while borrowing names and child arrays.
/// Child IDs, variable IDs and arities are validated by the consuming algorithm.
#[derive(Clone, Copy)]
pub enum ExprView<'a>
{
    Real(f64),
    Var(u32),
    Sum(PolyView<'a>),
    Prod(PolyView<'a>),
    Pow(PolyView<'a>),
    Func { name: &'a str, arg: u32 },
    Logic { op: LogicOp, left: u32, right: u32 },
    Cond { condition: u32, ifTrue: u32, ifFalse: u32 },
}

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExprReadError
{
    InvalidId(u32),
    EmptyNode(u32),
    UnsupportedNode(u32),
}

impl fmt::Display for ExprReadError
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        return match self {
            Self::InvalidId(id) => write!(formatter, "Invalid expression ID {id}"),
            Self::EmptyNode(id) => write!(formatter, "Empty expression ID {id}"),
            Self::UnsupportedNode(id) => write!(formatter, "Unsupported expression node ID {id}"),
        };
    }
}

impl std::error::Error for ExprReadError {}

//-------------------------------------------------------------------------------------------------
