// mod.rs ---------------------------------------------------------------------------------------------------------
pub mod arr;
pub mod buff;
pub mod cast;
pub mod dset;
pub mod fifo;
pub mod seg;
pub mod stash;
pub mod stk;
pub mod traits;
#[cfg( feature = "tests")]
pub mod _test;
pub use arr::{Arr, MutArr};
pub use buff::Buff;
pub use cast::{
    IAllocRawExt, ICastExt, IConstPtrAtExt, IConstPtrExt, IConstPtrMutRefExt, IConstPtrRefExt,
    IArrExt, IMutArrExt, IPtrAtExt, IPtrExt, IPtrRefExt, IVoidPtrExt,
    MutAliasPtr,
};
pub use dset::DisjointSet;
pub use fifo::Fifo;
pub use seg::{Seg, USeg};
pub use stash::Stash;
pub use stk::Stk;
pub use traits::{IArr, IArrMut};
