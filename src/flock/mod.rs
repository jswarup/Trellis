// mod.rs ---------------------------------------------------------------------------------------------------------
//! CPU-equivalent compute kernels used when a GPU device is unavailable.
pub mod kernel;
pub use	kernel::{ CpuKernelFn, StandardOpCpuKernelFn };

//-------------------------------------------------------------------------------------------------
