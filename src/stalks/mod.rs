// mod.rs ---------------------------------------------------------------------------------------------------------
pub mod work;
#[cfg( feature = "tests")]
pub mod _test;
pub use work::{IWorker, Spinlock, SpinlockGuard, WorkPtr};
