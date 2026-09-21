// cpu.h ----------------------------------------------------------------------------------------------------------------------

use crate::heist::atelier::Atelier;
use crate::silo::{Arr, Buff, MutArr, USeg};
use crate::stalks::work::WorkPtr;
use crate::flock::StandardOpCpuKernelFn;
use crate::swarm::ops::{StandardOp, StandardOpLabel};
use crate::swarm::traits::{
    BackendKind, BufferUsage, ComputeBuffer, ComputeKernel, KernelSource, KernelSourceKind,
    SwarmError, WorkgroupDim,
};
use crate::swarm::backend::IComputeBackend;
use std::sync::Arc;

//-----------------------------------------------------------------------------------------------------------------------------

// ComputeDevice executing SIMT compute kernels over host CPU worker threads (or returning
// UnsupportedBackend for unconfigured hardware backends).
// Modeled directly from Trellis swarm/cpu.h.
pub struct ComputeDevice {
    _Backend: BackendKind,
    _WorkerCount: u32,
}

//-----------------------------------------------------------------------------------------------------------------------------

impl Default for ComputeDevice {
    fn default() -> Self {
        Self::New()
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl ComputeDevice {
    pub fn New() -> Self {
        Self {
            _Backend: BackendKind::Cpu,
            _WorkerCount: Atelier::DefaultThreadCount(),
        }
    }
    pub fn WithWorkers(workers: u32) -> Self {
        Self {
            _Backend: BackendKind::Cpu,
            _WorkerCount: workers.max(1),
        }
    }
    pub fn WithBackend(backend: BackendKind, workers: u32) -> Self {
        Self {
            _Backend: backend,
            _WorkerCount: if workers > 0 {
                workers
            } else {
                Atelier::DefaultThreadCount()
            },
        }
    }
    pub fn Backend(&self) -> BackendKind {
        self._Backend
    }
    pub fn WorkerCount(&self) -> u32 {
        self._WorkerCount
    }
    // Arbitrary legacy kernels receive whole buffers. They remain serial until
    // Dispatch accepts an exclusive bounded output span for each worker.
    #[inline]
    fn SupportsParallelDispatch(&self) -> bool {
        false
    }
    pub fn DoubleKernel() -> ComputeKernel {
        ComputeKernel::Standard(
            StandardOpLabel(StandardOp::Double),
            StandardOp::Double,
            "main",
            BackendKind::Cpu,
            Some(StandardOpCpuKernelFn(StandardOp::Double)),
        )
    }
    pub fn VectorAddKernel() -> ComputeKernel {
        ComputeKernel::Standard(
            StandardOpLabel(StandardOp::VectorAdd),
            StandardOp::VectorAdd,
            "main",
            BackendKind::Cpu,
            Some(StandardOpCpuKernelFn(StandardOp::VectorAdd)),
        )
    }
    pub fn CollatzKernel() -> ComputeKernel {
        ComputeKernel::Standard(
            StandardOpLabel(StandardOp::Collatz),
            StandardOp::Collatz,
            "main",
            BackendKind::Cpu,
            Some(StandardOpCpuKernelFn(StandardOp::Collatz)),
        )
    }
    pub fn PointCloudKernel() -> ComputeKernel {
        ComputeKernel::Standard(
            StandardOpLabel(StandardOp::PointCloud),
            StandardOp::PointCloud,
            "pts_pointcloud_cs",
            BackendKind::Cpu,
            Some(StandardOpCpuKernelFn(StandardOp::PointCloud)),
        )
    }
    pub fn CameraTransformKernel() -> ComputeKernel {
        ComputeKernel::Standard(
            StandardOpLabel(StandardOp::CameraTransform),
            StandardOp::CameraTransform,
            "camera_transform_cs",
            BackendKind::Cpu,
            Some(StandardOpCpuKernelFn(StandardOp::CameraTransform)),
        )
    }
    pub fn CreateBuffer(&self, label: &str, size: usize, usage: BufferUsage) -> ComputeBuffer {
        ComputeBuffer::New(label, size, usage, self._Backend)
    }
    pub fn CreateBufferInit(
        &self, label: &str, data: Arr<'_, u8>, usage: BufferUsage,
    ) -> ComputeBuffer {
        ComputeBuffer::WithData(label, data, usage, self._Backend)
    }
    pub fn CompileKernel(
        &self, label: &str, entry_point: &str, source: &KernelSource,
    ) -> Result<ComputeKernel, SwarmError> {
        if self._Backend != BackendKind::Cpu {
            return Ok(ComputeKernel::New(label, entry_point, self._Backend, None));
        }
        match source._Kind {
            KernelSourceKind::CpuClosure => match source.StandardOp() {
                Some( op) => Ok( ComputeKernel::Standard(
                    label, op, entry_point, self._Backend, source._Closure.clone(),
                )),
                None => Ok( ComputeKernel::New(
                    label, entry_point, self._Backend, source._Closure.clone(),
                )),
            },
            KernelSourceKind::Wgsl => {
                let src = &source._CodeStr;
                let ep = entry_point;
                if src.contains("pts_pointcloud") || ep.contains("pts_pointcloud") {
                    Ok(Self::PointCloudKernel())
                } else if src.contains("collatz") || ep.contains("collatz") {
                    Ok(Self::CollatzKernel())
                } else if src.contains("vecadd") || ep.contains("vecadd") {
                    Ok(Self::VectorAddKernel())
                } else if src.contains("double") || ep.contains("double") {
                    Ok(Self::DoubleKernel())
                } else if src.contains("camera_transform") || ep.contains("camera_transform") {
                    Ok(Self::CameraTransformKernel())
                } else {
                    Ok(Self::DoubleKernel())
                }
            }
            KernelSourceKind::SpirV => {
                if entry_point == "pts_pointcloud_cs" || label.contains("pointcloud") {
                    Ok(Self::PointCloudKernel())
                } else if entry_point == "camera_transform_cs" || label.contains("camera_transform")
                {
                    Ok(Self::CameraTransformKernel())
                } else if entry_point == "collatz_cs" || label.contains("collatz") {
                    Ok(Self::CollatzKernel())
                } else {
                    Ok(Self::DoubleKernel())
                }
            }
            KernelSourceKind::Ptx => {
                let src = &source._CodeStr;
                let ep = entry_point;
                if src.contains("pointcloud") || ep.contains("pointcloud") {
                    Ok(Self::PointCloudKernel())
                } else if src.contains("collatz") || ep.contains("collatz") {
                    Ok(Self::CollatzKernel())
                } else if src.contains("vecadd") || ep.contains("vecadd") {
                    Ok(Self::VectorAddKernel())
                } else if src.contains("camera_transform") || ep.contains("camera_transform") {
                    Ok(Self::CameraTransformKernel())
                } else {
                    Ok(Self::DoubleKernel())
                }
            }
        }
    }
    fn DispatchStandardDouble(
        &self, buffers: Arr<'_, &ComputeBuffer>, dim: WorkgroupDim,
    ) -> Result<(), SwarmError> {
        if buffers.Len() != 1 {
            return Err( SwarmError::ExecutionError( "Double requires exactly one read-write buffer"));
        }
        if dim._Y != 1 || dim._Z != 1 {
            return Err( SwarmError::ExecutionError( "Double requires linear CPU dispatch dimensions"));
        }
        let  	invocations = dim._X.checked_mul( 64).ok_or_else( || {
            SwarmError::ExecutionError( "CPU X workgroup count overflows invocation range")
        })?;
        let  	buffer = buffers[0];
        if !buffer.Size().is_multiple_of( std::mem::size_of::< f32>()) {
            return Err( SwarmError::BufferError( "Double requires an f32-aligned buffer size"));
        }
        buffer.WithMut( |mut raw| {
            let  	mut values = raw.CastMutArr::< f32>();
            USeg::FromLen( invocations.min( values.Len())).Traverse( |idx| values[idx] *= 2.0);
        })?;
        Ok( ())
    }
    pub fn Dispatch(
        &self, kernel: &ComputeKernel, buffers: Arr<'_, &ComputeBuffer>, dim: WorkgroupDim,
    ) -> Result<(), SwarmError> {
        if self._Backend != BackendKind::Cpu {
            return Err(SwarmError::UnsupportedBackend(self._Backend));
        }
        if kernel.StandardOp() == Some( StandardOp::Double) {
            return self.DispatchStandardDouble( buffers, dim);
        }
        if buffers.IsEmpty() {
            return Ok(());
        }
        let threads_x = dim._X.checked_mul( 64).ok_or_else( || {
            SwarmError::ExecutionError( "CPU X workgroup count overflows invocation range")
        })?;
        let threads_y = dim._Y;
        let threads_z = dim._Z;
        // Read all buffers into local Buff<u8>
        let mut raw_buffers = Buff::FromDispenser(buffers.Len(), |i| buffers[i].Read());
        let out_idx = raw_buffers.Len() - 1;
        let in_count = if raw_buffers.Len() == 1 {
            1
        } else {
            raw_buffers.Len() - 1
        };
        let input_bytes: Arc<Buff<Buff<u8>>> =
            Arc::new(Buff::FromDispenser(in_count, |i| raw_buffers[i].clone()));
        let atelier = if self.SupportsParallelDispatch() {
            Some( Atelier::Instance())
        } else {
            None
        };
        if let Some( atelier) = atelier
            && !atelier.IsImmediate()
            && atelier.SzThreads() > 1
        {
            let chunk_size = 64u32;
            let num_chunks = threads_x.div_ceil(chunk_size);
            // Share pointers across chunks safely since chunks write to disjoint gid_x
            let raw_ptrs =
                Buff::FromDispenser(raw_buffers.Len(), |i| raw_buffers[i].MutArr().Data());
            let raw_lens = Buff::FromDispenser(raw_buffers.Len(), |i| raw_buffers[i].Cap());
            // Sendable wrapper for pointers
            struct DispatchContext {
                ptrs: Buff<*mut u8>,
                lens: Buff<u32>,
            }
            unsafe impl Send for DispatchContext {}
            unsafe impl Sync for DispatchContext {}
            let ctx_arc = Arc::new(DispatchContext {
                ptrs: raw_ptrs,
                lens: raw_lens,
            });
            let main_maestro = atelier.MainMaestro();
            for c in 0..num_chunks {
                let start_x = c * chunk_size;
                let end_x = (start_x + chunk_size).min(threads_x);
                let ctx_clone = ctx_arc.clone();
                let input_bytes_clone = input_bytes.clone();
                let kernel_clone = kernel.clone();
                main_maestro.PostJob(WorkPtr::FromClosure(move |_w| {
                    let in_slices = Buff::FromDispenser(in_count, |i| {
                        let b = &input_bytes_clone[i];
                        b.Arr()
                    });
                    for z in 0..threads_z {
                        for y in 0..threads_y {
                            for x in start_x..end_x {
                                let mut out_slices = [unsafe { MutArr::New(
                                    ctx_clone.ptrs[out_idx],
                                    ctx_clone.lens[out_idx],
                                ) }];
                                kernel_clone.Execute(
                                    in_slices.Arr(),
                                    (&mut out_slices).into(),
                                    x,
                                    y,
                                    z,
                                );
                            }
                        }
                    }
                }));
            }
            atelier.DoLaunch();
        } else {
            let in_slices = Buff::FromDispenser(in_count, |i| {
                let b = &input_bytes[i];
                b.Arr()
            });
            let outPart = raw_buffers[out_idx].MutArr();
            for z in 0..threads_z {
                for y in 0..threads_y {
                    for x in 0..threads_x {
                        let mut out_slices = [unsafe { outPart.Alias() }];
                        kernel.Execute(in_slices.Arr(), (&mut out_slices).into(), x, y, z);
                    }
                }
            }
        }
        // Write modified output buffer back
        let out = buffers.Last().unwrap();
        out.Write(raw_buffers[out_idx].Arr())?;
        Ok(())
    }
    pub fn Synchronize(&self) -> Result<(), SwarmError> {
        if self._Backend != BackendKind::Cpu {
            return Err(SwarmError::UnsupportedBackend(self._Backend));
        }
        Ok(())
    }
}
pub type CpuDevice = ComputeDevice;
pub type IComputeDevice = ComputeDevice;

impl IComputeBackend for ComputeDevice
{
    type Buffer = ComputeBuffer;
    type Kernel = ComputeKernel;

    fn Backend(&self) -> BackendKind
    {
        self.Backend()
    }
    fn CreateBuffer(
        &self, label: &str, size: usize, usage: BufferUsage,
    ) -> Result<Self::Buffer, SwarmError>
    {
        Ok( ComputeDevice::CreateBuffer( self, label, size, usage))
    }
    fn CreateBufferInit(
        &self, label: &str, data: Arr<'_, u8>, usage: BufferUsage,
    ) -> Result<Self::Buffer, SwarmError>
    {
        Ok( ComputeDevice::CreateBufferInit( self, label, data, usage))
    }
    fn CompileKernel(
        &self, label: &str, entry_point: &str, source: &KernelSource,
    ) -> Result<Self::Kernel, SwarmError>
    {
        ComputeDevice::CompileKernel( self, label, entry_point, source)
    }
    fn Dispatch(
        &self, kernel: &Self::Kernel, buffers: Arr<'_, &Self::Buffer>, dim: WorkgroupDim,
    ) -> Result<(), SwarmError>
    {
        ComputeDevice::Dispatch( self, kernel, buffers, dim)
    }
    fn Synchronize(&self) -> Result<(), SwarmError>
    {
        ComputeDevice::Synchronize( self)
    }
}

//-----------------------------------------------------------------------------------------------------------------------------
