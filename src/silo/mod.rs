// mod.rs ---------------------------------------------------------------------------------------------------------
#[cfg( feature = "tests")]
pub mod _tests;
pub mod arr;
pub mod buff;
pub mod cast;
pub mod dset;
pub mod fifo;
pub mod stash;
pub mod stk;
pub mod traits;
pub mod useg;
pub use	arr::{ Arr, MutArr };
pub use	buff::Buff;
pub use	cast::{ IAllocRawExt, IArrExt, ICastExt, IConstPtrAtExt, IConstPtrMutRefExt, IConstPtrRefExt, IMutArrExt, IPtrAtExt, IPtrRefExt, MutAliasPtr };
pub use	dset::DisjointSet;
pub use	fifo::Fifo;
pub use	stash::Stash;
pub use	stk::Stk;
pub use	traits::{ IArr, IArrMut };
pub use	useg::USeg;
