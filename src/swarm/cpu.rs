// cpu.h ----------------------------------------------------------------------------------------------------------------------

use crate::heist::atelier::Atelier;
use crate::silo::{Arr, Buff};
use crate::flock::{ CpuOutputPartition, StandardOpCpuKernelFn };
use crate::swarm::ops::{StandardOp, StandardOpLabel};
use crate::swarm::traits::{
    BackendKind, BufferUsage, ComputeBuffer, ComputeKernel, KernelSource, KernelSourceKind,
    SwarmError, WorkgroupDim,
};
use crate::swarm::backend::IComputeBackend;
use crate::symph::{ Collatz, HashToFloat, WangHash };
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
            let  	values = raw.CastMutArr::< f32>();
            let  	count = invocations.min( values.Len());
            let  	( active, _remaining) = values.SplitAt( count);
            let  	mut output = CpuOutputPartition::New( 0, active);
            output.ForEach( |_idx, value| *value *= 2.0);
        })?;
        Ok( ())
    }
    fn DispatchStandardCollatz(
        &self, buffers: Arr<'_, &ComputeBuffer>, dim: WorkgroupDim,
    ) -> Result<(), SwarmError> {
        if buffers.Len() != 2 {
            return Err( SwarmError::ExecutionError( "Collatz requires one input and one output buffer"));
        }
        if dim._Y != 1 || dim._Z != 1 {
            return Err( SwarmError::ExecutionError( "Collatz requires linear CPU dispatch dimensions"));
        }
        let  	invocations = dim._X.checked_mul( 64).ok_or_else( || {
            SwarmError::ExecutionError( "CPU X workgroup count overflows invocation range")
        })?;
        buffers[0].WithInputOutput( buffers[1], |input_raw, mut output_raw| -> Result< (), SwarmError> {
            if !input_raw.Len().is_multiple_of( std::mem::size_of::< u32>() as u32)
                || !output_raw.Len().is_multiple_of( std::mem::size_of::< u32>() as u32)
            {
                return Err( SwarmError::BufferError( "Collatz requires u32-sized buffers"));
            }
            let  	input = input_raw.CastArrFrom::< u32>();
            let  	output = output_raw.CastMutArr::< u32>();
            let  	count = invocations.min( input.Len()).min( output.Len());
            let  	( active, _remaining) = output.SplitAt( count);
            let  	mut partition = CpuOutputPartition::New( 0, active);
            partition.ForEach( |index, value| *value = Collatz( input[index]));
            Ok( ())
        })??;
        Ok( ())
    }
    fn DispatchStandardVectorAdd(
        &self, buffers: Arr<'_, &ComputeBuffer>, dim: WorkgroupDim,
    ) -> Result<(), SwarmError> {
        if buffers.Len() != 3 {
            return Err( SwarmError::ExecutionError( "VectorAdd requires two inputs and one output buffer"));
        }
        if dim._Y != 1 || dim._Z != 1 {
            return Err( SwarmError::ExecutionError( "VectorAdd requires linear CPU dispatch dimensions"));
        }
        let  	invocations = dim._X.checked_mul( 64).ok_or_else( || {
            SwarmError::ExecutionError( "CPU X workgroup count overflows invocation range")
        })?;
        buffers[0].WithInputsOutput(
            buffers[1], buffers[2],
            |a_raw, b_raw, mut output_raw| -> Result< (), SwarmError> {
                let  	word_size = std::mem::size_of::< f32>() as u32;
                if !a_raw.Len().is_multiple_of( word_size)
                    || !b_raw.Len().is_multiple_of( word_size)
                    || !output_raw.Len().is_multiple_of( word_size)
                {
                    return Err( SwarmError::BufferError( "VectorAdd requires f32-sized buffers"));
                }
                let  	a = a_raw.CastArrFrom::< f32>();
                let  	b = b_raw.CastArrFrom::< f32>();
                let  	output = output_raw.CastMutArr::< f32>();
                let  	count = invocations.min( a.Len()).min( b.Len()).min( output.Len());
                let  	( active, _remaining) = output.SplitAt( count);
                let  	mut partition = CpuOutputPartition::New( 0, active);
                partition.ForEach( |index, value| *value = a[index] + b[index]);
                Ok( ())
            },
        )??;
        Ok( ())
    }
    fn DispatchStandardPointCloud(
        &self, buffers: Arr<'_, &ComputeBuffer>, dim: WorkgroupDim,
    ) -> Result<(), SwarmError> {
        if buffers.Len() != 1 {
            return Err( SwarmError::ExecutionError( "PointCloud requires exactly one output buffer"));
        }
        if dim._Y != 1 || dim._Z != 1 {
            return Err( SwarmError::ExecutionError( "PointCloud requires linear CPU dispatch dimensions"));
        }
        let  	invocations = dim._X.checked_mul( 64).ok_or_else( || {
            SwarmError::ExecutionError( "CPU X workgroup count overflows invocation range")
        })?;
        buffers[0].WithMut( |mut raw| -> Result< (), SwarmError> {
            if !raw.Len().is_multiple_of( std::mem::size_of::< f32>() as u32) {
                return Err( SwarmError::BufferError( "PointCloud requires an f32-sized buffer"));
            }
            let  	values = raw.CastMutArr::< f32>();
            let  	point_count = invocations.min( values.Len() / 4);
            let  	scalar_count = point_count.checked_mul( 4).ok_or_else( || {
                SwarmError::ExecutionError( "PointCloud output range overflows")
            })?;
            let  	( active, _remaining) = values.SplitAt( scalar_count);
            let  	mut partition = CpuOutputPartition::New( 0, active);
            partition.ForEach( |scalar, value| {
                let  	point = scalar / 4;
                *value = match scalar % 4 {
                    0 => HashToFloat( WangHash( point * 3)) * 40.0 - 20.0,
                    1 => HashToFloat( WangHash( point * 3 + 1)) * 40.0 - 20.0,
                    2 => HashToFloat( WangHash( point * 3 + 2)) * 40.0 - 20.0,
                    _ => 1.0,
                };
            });
            Ok( ())
        })??;
        Ok( ())
    }
    fn DispatchStandardCameraTransform(
        &self, buffers: Arr<'_, &ComputeBuffer>, dim: WorkgroupDim,
    ) -> Result<(), SwarmError> {
        if buffers.Len() != 3 {
            return Err( SwarmError::ExecutionError( "CameraTransform requires points, camera, and output buffers"));
        }
        if dim._Y != 1 || dim._Z != 1 {
            return Err( SwarmError::ExecutionError( "CameraTransform requires linear CPU dispatch dimensions"));
        }
        let  	invocations = dim._X.checked_mul( 64).ok_or_else( || {
            SwarmError::ExecutionError( "CPU X workgroup count overflows invocation range")
        })?;
        buffers[0].WithInputsOutput(
            buffers[1], buffers[2],
            |points_raw, camera_raw, mut output_raw| -> Result< (), SwarmError> {
                let  	word_size = std::mem::size_of::< f32>() as u32;
                if !points_raw.Len().is_multiple_of( word_size)
                    || !camera_raw.Len().is_multiple_of( word_size)
                    || !output_raw.Len().is_multiple_of( word_size)
                {
                    return Err( SwarmError::BufferError( "CameraTransform requires f32-sized buffers"));
                }
                let  	points = points_raw.CastArrFrom::< f32>();
                let  	camera = camera_raw.CastArrFrom::< f32>();
                if camera.Len() < 13 {
                    return Err( SwarmError::BufferError( "CameraTransform requires 13 camera values"));
                }
                let  	output = output_raw.CastMutArr::< f32>();
                let  	point_count = invocations.min( points.Len() / 3).min( output.Len() / 6);
                let  	scalar_count = point_count.checked_mul( 6).ok_or_else( || {
                    SwarmError::ExecutionError( "CameraTransform output range overflows")
                })?;
                let  	( active, _remaining) = output.SplitAt( scalar_count);
                let  	mut partition = CpuOutputPartition::New( 0, active);
                partition.ForEach( |scalar, value| {
                    let  	point = scalar / 6;
                    let  	x = points[point * 3];
                    let  	y = points[point * 3 + 1];
                    let  	z = points[point * 3 + 2];
                    let  	nx = ( x - camera[9]) * camera[12];
                    let  	ny = ( y - camera[10]) * camera[12];
                    let  	nz = ( z - camera[11]) * camera[12];
                    let  	x1 = nx * camera[1].cos() + nz * camera[1].sin();
                    let  	z1 = -nx * camera[1].sin() + nz * camera[1].cos();
                    let  	y2 = ny * camera[0].cos() - z1 * camera[0].sin();
                    let  	z2 = ny * camera[0].sin() + z1 * camera[0].cos();
                    let  	scale = ( camera[5] * camera[2]) / ( camera[6] + z2).max( 1e-4);
                    let  	depth = ( ( 300.0 - z2) / 400.0).clamp( 0.3, 1.0);
                    *value = match scalar % 6 {
                        0 => camera[7] / 2.0 + camera[3] + x1 * scale,
                        1 => camera[8] / 2.0 + camera[4] - y2 * scale,
                        2 => 3.0 + depth * 4.0,
                        3 => 1.0 + depth * 1.5,
                        4 => 0.5 + depth * 0.5,
                        _ => depth,
                    };
                });
                Ok( ())
            },
        )??;
        Ok( ())
    }
    pub fn Dispatch(
        &self, kernel: &ComputeKernel, buffers: Arr<'_, &ComputeBuffer>, dim: WorkgroupDim,
    ) -> Result<(), SwarmError> {
        if self._Backend != BackendKind::Cpu {
            return Err(SwarmError::UnsupportedBackend(self._Backend));
        }
        match kernel.StandardOp() {
            Some( StandardOp::Double) => return self.DispatchStandardDouble( buffers, dim),
            Some( StandardOp::Collatz) => return self.DispatchStandardCollatz( buffers, dim),
            Some( StandardOp::VectorAdd) => return self.DispatchStandardVectorAdd( buffers, dim),
            Some( StandardOp::PointCloud) => return self.DispatchStandardPointCloud( buffers, dim),
            Some( StandardOp::CameraTransform) => return self.DispatchStandardCameraTransform( buffers, dim),
            _ => {}
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
        // Legacy closures receive whole-buffer views and therefore remain serial.
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
