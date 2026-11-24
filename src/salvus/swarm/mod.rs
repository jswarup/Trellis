// mod.rs ---------------------------------------------------------------------------------------------------------
#[cfg( feature = "tests")]
pub mod _tests;
pub mod backend;
pub mod cpu;
pub mod engine;
pub mod viewport;
pub mod ops;
pub mod traits;
pub mod wgpu_engine;
pub use	backend::IComputeBackend;
pub use	cpu::{ ComputeDevice, CpuDevice, IComputeDevice };
pub use	engine::SwarmEngine;
pub use	ops::{ BindingRangeRecord, ISwarmBufferRef, OperationReviewRecord, StandardOp, StandardOpCpuKernelFn, StandardOpDescriptor, StandardOpEntryPoint, StandardOpKernelSource, StandardOpLabel, StandardOpPtx, StandardOpWgsl };
pub use	traits::{ BackendKind, BufferUsage, ComputeBuffer, ComputeKernel, CpuBuffer, CpuKernel, CpuKernelFn, IComputeBuffer, IComputeKernel, KernelSource, KernelSourceKind, SwarmError, SwarmErrorKind, WorkgroupDim };
pub use	wgpu_engine::{ WgpuComputeEngine, WgpuKernel };
