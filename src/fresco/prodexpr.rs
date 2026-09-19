// prodexpr.rs ----------------------------------------------------------------------------------------------------
use crate::fresco::{BaseExpr, PolyExpr};
use crate::silo::Buff;
use core::any::Any;

#[derive(Clone)]
pub struct ProdExpr { _Poly: PolyExpr }

impl ProdExpr {
    pub fn New(children: Buff<u32>, numerCount: u32) -> Self {
        Self { _Poly: PolyExpr::FromParts(numerCount, children) }
    }
    pub fn Poly(&self) -> &PolyExpr { &self._Poly }
}

impl BaseExpr for ProdExpr {
    fn CloneBox(&self) -> Box<dyn BaseExpr> { Box::new(self.clone()) }
    fn AsAny(&self) -> &dyn Any { self }
}

crate::ImplFluxSourceTyped!(ProdExpr, "ProdExpr", _Poly);

//-------------------------------------------------------------------------------------------------
