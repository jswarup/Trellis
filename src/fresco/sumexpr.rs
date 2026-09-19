// sumexpr.rs -----------------------------------------------------------------------------------------------------
use crate::fresco::{BaseExpr, PolyExpr};
use crate::silo::Buff;
use core::any::Any;

#[derive(Clone)]
pub struct SumExpr { _Poly: PolyExpr }

impl SumExpr {
    pub fn New(children: Buff<u32>, addCount: u32) -> Self {
        Self { _Poly: PolyExpr::FromParts(addCount, children) }
    }
    pub fn Poly(&self) -> &PolyExpr { &self._Poly }
}

impl BaseExpr for SumExpr {
    fn CloneBox(&self) -> Box<dyn BaseExpr> { Box::new(self.clone()) }
    fn AsAny(&self) -> &dyn Any { self }
}

crate::ImplFluxSourceTyped!(SumExpr, "SumExpr", _Poly);

//-------------------------------------------------------------------------------------------------
