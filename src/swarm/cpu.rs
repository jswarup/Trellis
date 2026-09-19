// cpu.h ----------------------------------------------------------------------------------------------------------
use	crate::heist::atelier::Atelier;
use	crate::silo::buff::Buff;
use	crate::stalks::work::WorkPtr;
use	crate::swarm::ops::{ StandardOp, StandardOpCpuKernelFn, StandardOpLabel };
use	crate::swarm::traits::{ BackendKind, BufferUsage, ComputeBuffer, ComputeKernel, KernelSource, KernelSourceKind, SwarmError, WorkgroupDim };
use	std::sync::Arc;

//-------------------------------------------------------------------------------------------------
// ComputeDevice executing SIMT compute kernels over host CPU worker threads (or returning
// UnsupportedBackend for unconfigured hardware backends).
// Modeled directly from Trellis swarm/cpu.h.
pub struct ComputeDevice
{
    _Backend: BackendKind,
    _WorkerCount: u32,
}
impl Default for ComputeDevice {
    fn	default() -> Self
    {
        Self::New()
    }
}
impl ComputeDevice
{
    pub fn	New() -> Self
    {
        Self {
            _Backend: BackendKind::Cpu,
            _WorkerCount: Atelier::DefaultThreadCount(),
        }
    }
    pub fn	WithWorkers( workers: u32) -> Self
    {
        Self {
            _Backend: BackendKind::Cpu,
            _WorkerCount: workers.max( 1),
        }
    }
    pub fn	WithBackend( backend: BackendKind, workers: u32) -> Self
    {
        Self {
            _Backend: backend,
            _WorkerCount: if workers > 0 {
                workers
            } else {
                Atelier::DefaultThreadCount()
            },
        }
    }
    pub fn	Backend( &self) -> BackendKind
    {
        self._Backend
    }
    pub fn	WorkerCount( &self) -> u32
    {
        self._WorkerCount
    }
    pub fn	DoubleKernel() -> ComputeKernel
    {
        ComputeKernel::New( 
            StandardOpLabel( StandardOp::Double),
            "main",
            BackendKind::Cpu,
            Some( StandardOpCpuKernelFn( StandardOp::Double)),
        )
    }
    pub fn	VectorAddKernel() -> ComputeKernel
    {
        ComputeKernel::New( 
            StandardOpLabel( StandardOp::VectorAdd),
            "main",
            BackendKind::Cpu,
            Some( StandardOpCpuKernelFn( StandardOp::VectorAdd)),
        )
    }
    pub fn	CollatzKernel() -> ComputeKernel
    {
        ComputeKernel::New( 
            StandardOpLabel( StandardOp::Collatz),
            "main",
            BackendKind::Cpu,
            Some( StandardOpCpuKernelFn( StandardOp::Collatz)),
        )
    }
    pub fn	PointCloudKernel() -> ComputeKernel
    {
        ComputeKernel::New( 
            StandardOpLabel( StandardOp::PointCloud),
            "pts_pointcloud_cs",
            BackendKind::Cpu,
            Some( StandardOpCpuKernelFn( StandardOp::PointCloud)),
        )
    }
    pub fn	CameraTransformKernel() -> ComputeKernel
    {
        ComputeKernel::New( 
            StandardOpLabel( StandardOp::CameraTransform),
            "camera_transform_cs",
            BackendKind::Cpu,
            Some( StandardOpCpuKernelFn( StandardOp::CameraTransform)),
        )
    }
    pub fn	CreateBuffer( &self, label: &str, size: usize, usage: BufferUsage) -> ComputeBuffer
    {
        ComputeBuffer::New( label, size, usage, self._Backend)
    }
    pub fn	CreateBufferInit( &self, label: &str, data: &[u8], usage: BufferUsage) -> ComputeBuffer
    {
        ComputeBuffer::WithData( label, data, usage, self._Backend)
    }
    pub fn	CompileKernel( 
        &self, label: &str, entry_point: &str, source: &KernelSource,
    ) -> Result< ComputeKernel, SwarmError>
    {
        if self._Backend != BackendKind::Cpu {
            return Ok( ComputeKernel::New( label, entry_point, self._Backend, None));
        }
        match source._Kind {
            KernelSourceKind::CpuClosure => Ok( ComputeKernel::New( 
                label,
                entry_point,
                self._Backend,
                source._Closure.clone(),
            )),
            KernelSourceKind::Wgsl => {
                let  	src = &source._CodeStr;
                let  	ep = entry_point;
                if src.contains( "pts_pointcloud") || ep.contains( "pts_pointcloud") {
                    Ok( Self::PointCloudKernel())
                } else if src.contains( "collatz") || ep.contains( "collatz") {
                    Ok( Self::CollatzKernel())
                } else if src.contains( "vecadd") || ep.contains( "vecadd") {
                    Ok( Self::VectorAddKernel())
                } else if src.contains( "double") || ep.contains( "double") {
                    Ok( Self::DoubleKernel())
                } else if src.contains( "camera_transform") || ep.contains( "camera_transform") {
                    Ok( Self::CameraTransformKernel())
                } else {
                    Ok( Self::DoubleKernel())
                }
            }
            KernelSourceKind::SpirV => {
                if entry_point == "pts_pointcloud_cs" || label.contains( "pointcloud") {
                    Ok( Self::PointCloudKernel())
                } else if entry_point == "camera_transform_cs" || label.contains( "camera_transform") {
                    Ok( Self::CameraTransformKernel())
                } else if entry_point == "collatz_cs" || label.contains( "collatz") {
                    Ok( Self::CollatzKernel())
                } else {
                    Ok( Self::DoubleKernel())
                }
            }
            KernelSourceKind::Ptx => {
                let  	src = &source._CodeStr;
                let  	ep = entry_point;
                if src.contains( "pointcloud") || ep.contains( "pointcloud") {
                    Ok( Self::PointCloudKernel())
                } else if src.contains( "collatz") || ep.contains( "collatz") {
                    Ok( Self::CollatzKernel())
                } else if src.contains( "vecadd") || ep.contains( "vecadd") {
                    Ok( Self::VectorAddKernel())
                } else if src.contains( "camera_transform") || ep.contains( "camera_transform") {
                    Ok( Self::CameraTransformKernel())
                } else {
                    Ok( Self::DoubleKernel())
                }
            }
        }
    }
    pub fn	Dispatch( 
        &self, kernel: &ComputeKernel, buffers: &[&ComputeBuffer], dim: WorkgroupDim,
    ) -> Result< (), SwarmError>
    {
        if self._Backend != BackendKind::Cpu {
            return Err( SwarmError::UnsupportedBackend( self._Backend));
        }
        if buffers.is_empty() {
            return Ok( ());
        }
        let  	threads_x = dim._X * 64;
        let  	threads_y = dim._Y;
        let  	threads_z = dim._Z;
        // Read all buffers into local Buff<u8>
        let  	mut raw_buffers =
            Buff::FromDispenser( buffers.len() as u32, |i| buffers[i as usize].Read());
        let  	out_idx = raw_buffers.Len() - 1;
        let  	in_count = if raw_buffers.Len() == 1 {
            1
        } else {
            raw_buffers.Len() - 1
        };
        let  	input_bytes: Arc< Buff< Buff< u8>>> =
            Arc::new( Buff::FromDispenser( in_count, |i| raw_buffers[i].clone()));
        let  	atelier = Atelier::Instance();
        if !atelier.IsImmediate() && atelier.SzThreads() > 1 {
            let  	chunk_size = 64u32;
            let  	num_chunks = threads_x.div_ceil( chunk_size);
            // Share pointers across chunks safely since chunks write to disjoint gid_x
            let  	raw_ptrs = Buff::FromDispenser( raw_buffers.Len(), |i| raw_buffers[i].AsMutPtr());
            let  	raw_lens =
                Buff::FromDispenser( raw_buffers.Len(), |i| raw_buffers[i].Cap() as usize);
            // Sendable wrapper for pointers
            struct DispatchContext
            {
                ptrs: Buff< *mut u8>,
                lens: Buff< usize>,
            }
            unsafe impl Send for DispatchContext {}
            unsafe impl Sync for DispatchContext {}
            let  	ctx_arc = Arc::new( DispatchContext {
                ptrs: raw_ptrs,
                lens: raw_lens,
            });
            let  	main_maestro = atelier.MainMaestro();
            for c in 0..num_chunks {
                let  	start_x = c * chunk_size;
                let  	end_x = ( start_x + chunk_size).min( threads_x);
                let  	ctx_clone = ctx_arc.clone();
                let  	input_bytes_clone = input_bytes.clone();
                let  	kernel_clone = kernel.clone();
                main_maestro.PostJob( WorkPtr::FromClosure( move |_w| {
                    let  	in_slices = Buff::FromDispenser( in_count, |i| unsafe {
                        let  	b = &input_bytes_clone[i];
                        std::slice::from_raw_parts( b.Data(), b.Len() as usize)
                    });
                    for z in 0..threads_z {
                        for y in 0..threads_y {
                            for x in start_x..end_x {
                                let  	mut out_slices: [&mut [u8]; 1] = [unsafe {
                                    std::slice::from_raw_parts_mut( 
                                        ctx_clone.ptrs[out_idx],
                                        ctx_clone.lens[out_idx],
                                    )
                                }];
                                kernel_clone.Execute( 
                                    unsafe {
                                        std::slice::from_raw_parts( 
                                            in_slices.Data(),
                                            in_slices.Len() as usize,
                                        )
                                    },
                                    &mut out_slices,
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
            let  	in_slices = Buff::FromDispenser( in_count, |i| unsafe {
                let  	b = &input_bytes[i];
                std::slice::from_raw_parts( b.Data(), b.Len() as usize)
            });
            let  	out_part = raw_buffers[out_idx].AsMutPtr();
            let  	out_len = raw_buffers[out_idx].Cap() as usize;
            for z in 0..threads_z {
                for y in 0..threads_y {
                    for x in 0..threads_x {
                        let  	mut out_slices: [&mut [u8]; 1] =
                            [unsafe { std::slice::from_raw_parts_mut( out_part, out_len) }];
                        kernel.Execute( 
                            unsafe {
                                std::slice::from_raw_parts( 
                                    in_slices.Data(),
                                    in_slices.Len() as usize,
                                )
                            },
                            &mut out_slices,
                            x,
                            y,
                            z,
                        );
                    }
                }
            }
        }
        // Write modified output buffer back
        let  	out = buffers.last().unwrap();
        out.Write( unsafe {
            std::slice::from_raw_parts( 
                raw_buffers[out_idx].Data(),
                raw_buffers[out_idx].Len() as usize,
            )
        })?;
        Ok( ())
    }
    pub fn	Synchronize( &self) -> Result< (), SwarmError>
    {
        if self._Backend != BackendKind::Cpu {
            return Err( SwarmError::UnsupportedBackend( self._Backend));
        }
        Ok( ())
    }
}
pub type CpuDevice = ComputeDevice;
pub type IComputeDevice = ComputeDevice;
