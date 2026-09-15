// mod.rs ---------------------------------------------------------------------------------------------------------

pub mod cpu;
pub mod engine;
pub mod ops;
pub mod traits;

#[cfg(feature = "tests")]
pub mod _test;

pub use cpu::{ComputeDevice, CpuDevice, IComputeDevice};
pub use engine::SwarmEngine;
pub use ops::{
    StandardOp, StandardOpCpuKernelFn, StandardOpEntryPoint, StandardOpKernelSource,
    StandardOpLabel, StandardOpPtx, StandardOpWgsl,
};
pub use traits::{
    BackendKind, BufferUsage, ComputeBuffer, ComputeKernel, CpuBuffer, CpuKernel, CpuKernelFn,
    IComputeBuffer, IComputeKernel, KernelSource, KernelSourceKind, SwarmError, SwarmErrorKind,
    WorkgroupDim,
};
