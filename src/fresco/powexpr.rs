// powexpr.rs -----------------------------------------------------------------------------------------------------
use crate::fresco::{BaseExpr, PolyExpr};
use crate::silo::Buff;
use core::any::Any;

#[derive(Clone)]
pub struct PowExpr { _Poly: PolyExpr }

impl PowExpr {
    pub fn New(children: Buff<u32>, baseCount: u32) -> Self {
        Self { _Poly: PolyExpr::FromParts(baseCount, children) }
    }
    pub fn Poly(&self) -> &PolyExpr { &self._Poly }
}

impl BaseExpr for PowExpr {
    fn CloneBox(&self) -> Box<dyn BaseExpr> { Box::new(self.clone()) }
    fn AsAny(&self) -> &dyn Any { self }
}

crate::ImplFluxSourceTyped!(PowExpr, "PowExpr", _Poly);

//-------------------------------------------------------------------------------------------------
