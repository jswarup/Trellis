//-- wgpu_engine.rs ---------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::silo::buff::Buff;
use	crate::silo::stash::Stash;
use	crate::swarm::ops::{ StandardOp, StandardOpDescriptor, StandardOpWgsl };
use	crate::swarm::traits::SwarmError;
use	iced::futures::executor::block_on;
use	wgpu::util::DeviceExt;

//--------------------------------------------------------------------------------------------------
/// GPU compute engine executing compute shaders via native WebGPU (wgpu).
pub struct WgpuComputeEngine
{
    _Device:      wgpu::Device,
    _Queue:       wgpu::Queue,
    _AdapterName: String,
}

impl WgpuComputeEngine
{
    /// Attempts to initialize the WGPU compute engine with default adapter.
    pub fn	TryNew() -> Option< Self>
    {
        let  	instance = wgpu::Instance::default();
        let  	adapter = block_on(
            instance.request_adapter( &wgpu::RequestAdapterOptions::default()),
        ).ok()?;
        let  	adapterInfo = adapter.get_info();
        let  	name = adapterInfo.name.clone();
        let  	( device, queue) = block_on(
            adapter.request_device( &wgpu::DeviceDescriptor::default()),
        ).ok()?;
        Some( Self {
            _Device:      device,
            _Queue:       queue,
            _AdapterName: name,
        })
    }

    pub fn	New() -> Result< Self, SwarmError>
    {
        Self::TryNew().ok_or_else( || SwarmError::DeviceUnavailable( "No compatible WGPU adapter"))
    }

    pub fn	WithDevice( device: wgpu::Device, queue: wgpu::Queue, name: String) -> Self
    {
        Self {
            _Device:      device,
            _Queue:       queue,
            _AdapterName: name,
        }
    }

    #[inline]
    pub fn	Device( &self) -> &wgpu::Device
    {
        &self._Device
    }

    #[inline]
    pub fn	Queue( &self) -> &wgpu::Queue
    {
        &self._Queue
    }

    #[inline]
    pub fn	AdapterName( &self) -> &str
    {
        &self._AdapterName
    }

    /// Creates an uninitialized GPU storage buffer.
    pub fn	CreateBuffer( &self, label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer
    {
        self._Device.create_buffer( &wgpu::BufferDescriptor {
            label:              Some( label),
            size,
            usage,
            mapped_at_creation: false,
        })
    }

    /// Creates an initialized GPU buffer from raw byte slice.
    pub fn	CreateBufferInit(
        &self, label: &str, data: &[u8], usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer
    {
        self._Device.create_buffer_init( &wgpu::util::BufferInitDescriptor {
            label:    Some( label),
            contents: data,
            usage,
        })
    }

    /// Compiles a WGSL compute shader and creates its compute pipeline.
    pub fn	CompileComputePipeline(
        &self, label: &str, wgslSource: &str, entryPoint: &str,
    ) -> Result< wgpu::ComputePipeline, SwarmError>
    {
        let  	shader = self._Device.create_shader_module( wgpu::ShaderModuleDescriptor {
            label:  Some( label),
            source: wgpu::ShaderSource::Wgsl( wgslSource.into()),
        });
        let  	pipeline = self._Device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label:               Some( label),
                layout:              None,
                module:              &shader,
                entry_point:         Some( entryPoint),
                compilation_options: Default::default(),
                cache:               None,
            },
        );
        Ok( pipeline)
    }

    /// Compiles the compute pipeline corresponding to a StandardOp.
    pub fn	CompileStandardOp( &self, op: StandardOp) -> Result< wgpu::ComputePipeline, SwarmError>
    {
        let  	desc = StandardOpDescriptor::ForOp( op);
        let  	wgsl = StandardOpWgsl( op);
        self.CompileComputePipeline( desc.label, wgsl, desc.wgsl_entry_point)
    }

    /// Dispatches a compute pass with the given pipeline, bind group, and workgroups.
    pub fn	DispatchCompute(
        &self, pipeline: &wgpu::ComputePipeline, bindGroup: &wgpu::BindGroup,
        workgroups: ( u32, u32, u32),
    )
    {
        let  	mut encoder = self._Device.create_command_encoder( &Default::default());
        {
            let  	mut pass = encoder.begin_compute_pass( &wgpu::ComputePassDescriptor::default());
            pass.set_pipeline( pipeline);
            pass.set_bind_group( 0, bindGroup, &[]);
            pass.dispatch_workgroups( workgroups.0, workgroups.1, workgroups.2);
        }
        self._Queue.submit( [encoder.finish()]);
    }

    /// Synchronously reads back buffer bytes via an intermediate staging buffer.
    pub fn	ReadbackBuffer(
        &self, source: &wgpu::Buffer, size: u64,
    ) -> Result< Buff< u8>, SwarmError>
    {
        let  	staging = self.CreateBuffer(
            "Readback staging",
            size,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let  	mut encoder = self._Device.create_command_encoder( &Default::default());
        encoder.copy_buffer_to_buffer( source, 0, &staging, 0, size);
        self._Queue.submit( [encoder.finish()]);

        let  	( sender, receiver) = std::sync::mpsc::channel();
        staging.slice( ..).map_async( wgpu::MapMode::Read, move |res| {
            let  	_ = sender.send( res);
        });
        self._Device.poll( wgpu::PollType::Wait {
            submission_index: None,
            timeout:          Some( std::time::Duration::from_secs( 15)),
        }).map_err( |_| SwarmError::ExecutionError( "GPU poll timeout"))?;

        receiver.recv_timeout( std::time::Duration::from_secs( 15))
            .map_err( |_| SwarmError::ExecutionError( "GPU readback timeout"))?
            .map_err( |_| SwarmError::ExecutionError( "GPU device lost during readback"))?;

        let  	mapped = staging.slice( ..).get_mapped_range();
        let  	mut stash = Stash::WithCapacity( size as u32);
        let  	mut i = 0usize;
        while i < mapped.len()
        {
            stash.Push( mapped[i]);
            i += 1;
        }
        drop( mapped);
        staging.unmap();
        Ok( stash.IntoBuff())
    }

    /// Reads back typed u32 data from a GPU buffer.
    pub fn	ReadbackU32(
        &self, source: &wgpu::Buffer, count: usize,
    ) -> Result< Buff< u32>, SwarmError>
    {
        let  	byteSize = ( count * 4) as u64;
        let  	bytes = self.ReadbackBuffer( source, byteSize)?;
        let  	mut stash = Stash::WithCapacity( count as u32);
        let  	mut idx = 0u32;
        let  	countU32 = count as u32;
        while idx < countU32
        {
            let  	offset = idx * 4;
            let  	val = u32::from_ne_bytes( [
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            stash.Push( val);
            idx += 1;
        }
        Ok( stash.IntoBuff())
    }
}
