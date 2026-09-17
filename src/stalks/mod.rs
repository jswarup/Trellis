// mod.rs ---------------------------------------------------------------------------------------------------------
pub mod work;
pub mod coro;
#[cfg( feature = "tests")]
pub mod _test;
pub use work::{IWorker, Spinlock, SpinlockGuard, WorkPtr};
pub use coro::{Coro, CoroRes, CoroYielder, ICoro};
