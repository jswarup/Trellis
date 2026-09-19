// polyexpr.rs ----------------------------------------------------------------------------------------------------
use crate::fresco::BaseExpr;
use crate::silo::Buff;
use core::any::Any;

#[derive(Clone)]
pub struct PolyExpr {
    _Children: Buff<u32>,
    _CoSize: u32,
}

impl PolyExpr {
    pub fn New() -> Self {
        Self { _Children: Buff::New(), _CoSize: 0 }
    }

    pub fn FromParts(coSize: u32, children: Buff<u32>) -> Self {
        assert!(coSize <= children.Cap(), "Common child count exceeds child count");
        Self { _Children: children, _CoSize: coSize }
    }

    pub fn SizeChild(&self) -> u32 { self._Children.Cap() }
    pub fn Child(&self, index: u32) -> u32 { self._Children[index] }
    pub fn CommonSize(&self) -> u32 { self._CoSize }
    pub fn IsFlip(&self, index: u32) -> bool { index >= self._CoSize }
}

impl Default for PolyExpr {
    fn default() -> Self { Self::New() }
}

impl BaseExpr for PolyExpr {
    fn CloneBox(&self) -> Box<dyn BaseExpr> { Box::new(self.clone()) }
    fn AsAny(&self) -> &dyn Any { self }
}

crate::ImplFluxSource!(PolyExpr, _Children, _CoSize);

//-------------------------------------------------------------------------------------------------
