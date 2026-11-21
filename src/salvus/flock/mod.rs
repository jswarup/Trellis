// mod.rs ---------------------------------------------------------------------------------------------------------
//! CPU-equivalent compute kernels used when a GPU device is unavailable.
pub mod kernel;
#[cfg( feature = "tests")]
pub mod _tests;
pub use	kernel::{ CpuKernelFn, CpuOutputPartition, StandardOpCpuKernelFn };

//-------------------------------------------------------------------------------------------------
