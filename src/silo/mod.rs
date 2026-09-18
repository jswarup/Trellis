// mod.rs ---------------------------------------------------------------------------------------------------------
#[cfg(feature = "tests")]
pub mod _tests;
pub mod arr;
pub mod buff;
pub mod cast;
pub mod dset;
pub mod fifo;
pub mod useg;
pub mod stash;
pub mod stk;
pub mod traits;
pub use arr::{Arr, MutArr};
pub use buff::Buff;
pub use cast::{
    IAllocRawExt, IArrExt, ICastExt, IConstPtrAtExt, IConstPtrExt, IConstPtrMutRefExt,
    IConstPtrRefExt, IMutArrExt, IPtrAtExt, IPtrExt, IPtrRefExt, IVoidPtrExt, MutAliasPtr,
};
pub use dset::DisjointSet;
pub use fifo::Fifo;
pub use useg::USeg;
pub use stash::Stash;
pub use stk::Stk;
pub use traits::{IArr, IArrMut};
