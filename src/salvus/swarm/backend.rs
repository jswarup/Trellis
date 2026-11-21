// backend.rs ------------------------------------------------------------------------------------------------------
//! Backend-neutral compute contract used by Swarm.
//!
//! The associated buffer and kernel types are intentional. Rust-GPU uses
//! wgpu-backed resources, while cuda-oxide owns CUDA buffers and streams. A
//! single CPU-shaped buffer type would force either backend to copy through
//! host memory or expose unsafe handles to the rest of the application.

use crate::silo::Arr;
use crate::swarm::ops::{ StandardOp, StandardOpEntryPoint, StandardOpKernelSource };
use crate::swarm::traits::{ BufferUsage, KernelSource, SwarmError, WorkgroupDim };

/// Compute operations that a Swarm backend must provide.
pub trait IComputeBackend
{
    type Buffer;
    type Kernel;

    fn Backend(&self) -> super::BackendKind;
    fn EntryPoint(&self, op: StandardOp) -> &'static str
    {
        StandardOpEntryPoint( op, self.Backend())
    }
    fn Source(&self, op: StandardOp) -> Result<KernelSource, SwarmError>
    {
        StandardOpKernelSource( op, self.Backend())
    }
    fn CreateBuffer(
        &self, label: &str, size: usize, usage: BufferUsage,
    ) -> Result<Self::Buffer, SwarmError>;
    fn CreateBufferInit(
        &self, label: &str, data: Arr<'_, u8>, usage: BufferUsage,
    ) -> Result<Self::Buffer, SwarmError>;
    fn CompileKernel(
        &self, label: &str, entry_point: &str, source: &KernelSource,
    ) -> Result<Self::Kernel, SwarmError>;
    fn Dispatch(
        &self, kernel: &Self::Kernel, buffers: Arr<'_, &Self::Buffer>, dim: WorkgroupDim,
    ) -> Result<(), SwarmError>;
    fn Synchronize(&self) -> Result<(), SwarmError>;
}
