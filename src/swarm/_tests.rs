use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test };
// mod.rs ---------------------------------------------------------------------------------------------------------
use	crate::heist::atelier::Atelier;
use	crate::silo::Buff;
use	crate::swarm::cpu::ComputeDevice;
use	crate::swarm::engine::SwarmEngine;
use	crate::swarm::ops::{ StandardOp, StandardOpEntryPoint, StandardOpKernelSource, StandardOpLabel };
use	crate::swarm::traits::{ BackendKind, BufferUsage, CpuBuffer, KernelSourceKind, SwarmErrorKind, WorkgroupDim };
use	crate::symph::compshade::Collatz;

//-------------------------------------------------------------------------------------------------
// Opt-in hardware test: SEGUE_GPU_TEST=1 cargo run -- -t ViewportGpu
jeeves_test!( Swarm, ViewportGpu, |ctx| {
    if std::env::var( "SEGUE_GPU_TEST").as_deref() != Ok( "1") {
        jeeves_println!( 
            ctx,
            "GPU test skipped; set SEGUE_GPU_TEST=1 to exercise an adapter."
        );
        return;
    }
    use	crate::fleck::{ ParsePts, ParseWaveObj, geometry::GeometryAsset };
    use	crate::swarm::viewport::{ RenderMode, ViewFrame, ViewportRenderer };
    use	glam::{ Mat4, Vec3 };
    use	iced::futures::executor::block_on;
    use	std::sync::Arc;
    let  	instance = wgpu::Instance::default();
    let  	adapter = block_on( instance.request_adapter( &wgpu::RequestAdapterOptions::default()))
        .expect( "GPU adapter required for opted-in test");
    println!( "GPU validation adapter: {:?}", adapter.get_info());
    let  	( device, queue) =
        block_on( adapter.request_device( &wgpu::DeviceDescriptor::default())).unwrap();
    device.push_error_scope( wgpu::ErrorFilter::Validation);
    let  	mut renderer = ViewportRenderer::New( &device, wgpu::TextureFormat::Rgba8UnormSrgb);
    assert!( 
        block_on( device.pop_error_scope()).is_none(),
        "GPU shaders/pipelines must validate"
    );
    let  	cloud = Arc::new( 
        GeometryAsset::FromPts( ParsePts( "0 0 1 0 255 0\n0 0 -1 255 0 0\n").unwrap()).unwrap(),
    );
    let  	mesh = Arc::new( 
        GeometryAsset::FromObj( ParseWaveObj( "v -1 -1 0\nv 1 -1 0\nv 0 1 0\nf 1 2 3\n").unwrap())
            .unwrap(),
    );
    let  	matrix = ( Mat4::perspective_rh( 0.8, 1.0, 0.01, 20.0)
        * Mat4::look_at_rh( Vec3::new( 0.0, 0.0, 4.0), Vec3::ZERO, Vec3::Y))
    .to_cols_array();
    let  	frame = |region, mode| {
        ViewFrame::New( 
            matrix,
            [64, 64],
            region,
            [0.0, 0.0, 0.0, 1.0],
            mode,
            0,
            20.0,
        )
    };
    renderer
        .Prepare( 
            1,
            &cloud,
            &device,
            &queue,
            frame( [0.0, 0.0, 0.5, 1.0], RenderMode::Points),
        )
        .unwrap();
    renderer
        .Prepare( 
            2,
            &mesh,
            &device,
            &queue,
            frame( [0.5, 0.0, 0.5, 1.0], RenderMode::Solid),
        )
        .unwrap();
    let  	zoomed = ( Mat4::perspective_rh( 0.8, 1.0, 0.01, 20.0)
        * Mat4::look_at_rh( Vec3::new( 0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y))
    .to_cols_array();
    renderer
        .Prepare( 
            1,
            &cloud,
            &device,
            &queue,
            ViewFrame::New( 
                zoomed,
                [64, 64],
                [0.0, 0.0, 0.5, 1.0],
                [0.0; 4],
                RenderMode::Points,
                0,
                20.0,
            ),
        )
        .unwrap();
    jeeves_assert_eq!( 
        ctx,
        renderer.UploadCount(),
        2,
        "Camera movement must reuse geometry"
    );
    let  	oversized = ViewFrame::New( 
        matrix,
        [device.limits().max_texture_dimension_2d + 1, 64],
        [0.0, 0.0, 1.0, 1.0],
        [0.0; 4],
        RenderMode::Points,
        0,
        3.0,
    );
    jeeves_assert!( 
        ctx,
        renderer
            .Prepare( 3, &cloud, &device, &queue, oversized)
            .is_err()
    );
    jeeves_assert_eq!( 
        ctx,
        renderer.ResidentViews(),
        2,
        "Invalid targets must not allocate a new view"
    );
    let  	target = device.create_texture( &wgpu::TextureDescriptor {
        label: Some( "GPU verification"),
        size: wgpu::Extent3d {
            width: 128,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let  	readback = device.create_buffer( &wgpu::BufferDescriptor {
        label: None,
        size: 128 * 64 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let  	view = target.create_view( &Default::default());
    let  	mut encoder = device.create_command_encoder( &Default::default());
    renderer.Render( 1, &mut encoder, &view, [0, 0, 64, 64]);
    renderer.Render( 2, &mut encoder, &view, [64, 0, 64, 64]);
    encoder.copy_texture_to_buffer( 
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some( 512),
                rows_per_image: Some( 64),
            },
        },
        wgpu::Extent3d {
            width: 128,
            height: 64,
            depth_or_array_layers: 1,
        },
    );
    queue.submit( [encoder.finish()]);
    let  	( sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice( ..)
        .map_async( wgpu::MapMode::Read, move |result| {
            let  	_ = sender.send( result);
        });
    device
        .poll( wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some( std::time::Duration::from_secs( 20)),
        })
        .unwrap();
    receiver
        .recv_timeout( std::time::Duration::from_secs( 20))
        .unwrap()
        .unwrap();
    let  	pixels = readback.slice( ..).get_mapped_range();
    let  	left = ( 32 * 128 + 32) * 4;
    let  	right = ( 32 * 128 + 96) * 4;
    jeeves_assert!( 
        ctx,
        pixels[left + 1] > 200 && pixels[left] < 20,
        "Nearest green point must occlude the red point drawn after it"
    );
    jeeves_assert!( 
        ctx,
        pixels[right] > 80 && pixels[right + 2] > 80,
        "Second viewport must display its mesh independently"
    );
    jeeves_assert!( 
        ctx,
        pixels[0] < 5 && pixels[1] < 5,
        "Background remains clear"
    );
    drop( pixels);
    readback.unmap();
    renderer
        .Prepare( 
            1,
            &cloud,
            &device,
            &queue,
            ViewFrame::New( 
                matrix,
                [96, 48],
                [0.0, 0.0, 1.0, 1.0],
                [0.0; 4],
                RenderMode::Points,
                0,
                3.0,
            ),
        )
        .unwrap();
    jeeves_assert_eq!( 
        ctx,
        renderer.UploadCount(),
        2,
        "Resizing must reuse geometry"
    );
    drop( cloud);
    renderer.Trim();
    jeeves_assert_eq!( ctx, renderer.ResidentViews(), 1);
    drop( mesh);
    renderer.Trim();
    jeeves_assert_eq!( ctx, renderer.ResidentViews(), 0);
});

//-------------------------------------------------------------------------------------------------
// Swarm Tests
jeeves_test!( Swarm, CpuBufferLifecycle, |ctx| {
    let  	buf = CpuBuffer::New( 
        "test_buf",
        64,
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
        BackendKind::Cpu,
    );
    jeeves_assert_eq!( ctx, buf.Size(), 64);
    jeeves_assert_eq!( ctx, buf.Label(), "test_buf");
    let  	data: [u8; 4] = [10, 20, 30, 40];
    let  	write_res = buf.Write( ( &data).into());
    jeeves_assert!( ctx, write_res.is_ok());
    let  	mut read_back = buf.Read();
    jeeves_assert!( ctx, ( read_back.Cap() as usize) >= 4);
    jeeves_assert_eq!( ctx, read_back[0], 10);
    jeeves_assert_eq!( ctx, read_back[1], 20);
    jeeves_assert_eq!( ctx, read_back[2], 30);
    jeeves_assert_eq!( ctx, read_back[3], 40);
    // Verify Read() returns an independent copy
    read_back[0] = 99;
    let  	second_read = buf.Read();
    jeeves_assert_eq!( ctx, second_read[0], 10);
});
jeeves_test!( Swarm, CpuDeviceDoubleOp, |ctx| {
    let  	dev = ComputeDevice::WithWorkers( 1);
    const COUNT: usize = 64;
    let  	values = Buff::FromDispenser( COUNT as u32, |i| ( i + 1) as f32);
    let  	bytes: &[u8] = values.CastArr().into();
    let  	buf = dev.CreateBufferInit( 
        "data",
        bytes.into(),
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
    );
    let  	source = StandardOpKernelSource( StandardOp::Double, BackendKind::Cpu)
        .expect( "CPU source creation failed");
    let  	kernel = dev
        .CompileKernel( StandardOpLabel( StandardOp::Double), "main", &source)
        .expect( "Kernel compilation failed");
    let  	err = dev.Dispatch( &kernel, ( &[&buf]).into(), WorkgroupDim::Linear( 1));
    jeeves_assert!( ctx, err.is_ok());
    let  	result_bytes = buf.Read();
    let  	result_floats: &[f32] = result_bytes.CastArrFrom::< f32>().into();
    for ( i, &val) in result_floats.iter().enumerate() {
        jeeves_assert_eq!( ctx, val, ( ( i + 1) * 2) as f32);
    }
});
jeeves_test!( Swarm, CpuDeviceDoubleOpParallel, |ctx| {
    Atelier::Reset( 4);
    const COUNT: usize = 128;
    let  	values = Buff::FromDispenser( COUNT as u32, |i| ( i + 1) as f32);
    let  	bytes: &[u8] = values.CastArr().into();
    let  	buf = ComputeDevice::New().CreateBufferInit( 
        "data",
        bytes.into(),
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
    );
    let  	err = ComputeDevice::New().Dispatch( 
        &ComputeDevice::DoubleKernel(),
        ( &[&buf]).into(),
        WorkgroupDim::Linear( 2),
    );
    jeeves_assert!( ctx, err.is_ok());
    let  	result_bytes = buf.Read();
    let  	result_floats: &[f32] = result_bytes.CastArrFrom::< f32>().into();
    for ( i, &val) in result_floats.iter().enumerate() {
        jeeves_assert_eq!( ctx, val, ( ( i + 1) * 2) as f32);
    }
});
jeeves_test!( Swarm, CpuDeviceVectorAddOp, |ctx| {
    let  	dev = ComputeDevice::WithWorkers( 1);
    const COUNT: usize = 64;
    let  	a = Buff::FromDispenser( COUNT as u32, |i| i as f32);
    let  	b = Buff::FromDispenser( COUNT as u32, |i| ( i * 10) as f32);
    let  	c = Buff::FromDispenser( COUNT as u32, |_| 0.0f32);
    let  	a_bytes: &[u8] = a.CastArr().into();
    let  	b_bytes: &[u8] = b.CastArr().into();
    let  	c_bytes: &[u8] = c.CastArr().into();
    let  	buf_a = dev.CreateBufferInit( "a", a_bytes.into(), BufferUsage::Storage());
    let  	buf_b = dev.CreateBufferInit( "b", b_bytes.into(), BufferUsage::Storage());
    let  	buf_c = dev.CreateBufferInit( 
        "c",
        c_bytes.into(),
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
    );
    let  	source = StandardOpKernelSource( StandardOp::VectorAdd, BackendKind::Cpu)
        .expect( "CPU source creation failed");
    let  	kernel = dev
        .CompileKernel( StandardOpLabel( StandardOp::VectorAdd), "main", &source)
        .expect( "Kernel compilation failed");
    let  	err = dev.Dispatch( 
        &kernel,
        ( &[&buf_a, &buf_b, &buf_c]).into(),
        WorkgroupDim::Linear( 1),
    );
    jeeves_assert!( ctx, err.is_ok());
    let  	result_bytes = buf_c.Read();
    let  	result_floats: &[f32] = result_bytes.CastArrFrom::< f32>().into();
    for ( i, &val) in result_floats.iter().enumerate() {
        jeeves_assert_eq!( ctx, val, ( i * 11) as f32);
    }
});
jeeves_test!( Swarm, SwarmEngineCollatzAndParallelDispatch, |ctx| {
    // Reset Atelier with 4 worker threads for parallel SIMT dispatch
    Atelier::Reset( 4);
    let  	engine = SwarmEngine::New( BackendKind::Cpu);
    const COUNT: usize = 128;
    let  	in_vals = Buff::FromDispenser( COUNT as u32, |i| ( i % 10) + 1);
    let  	out_vals = Buff::FromDispenser( COUNT as u32, |_| 0u32);
    let  	in_bytes: &[u8] = in_vals.CastArr().into();
    let  	out_bytes: &[u8] = out_vals.CastArr().into();
    let  	in_buf = engine
        .Device()
        .CreateBufferInit( "in", in_bytes.into(), BufferUsage::Storage());
    let  	out_buf = engine.Device().CreateBufferInit( 
        "out",
        out_bytes.into(),
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
    );
    let  	err = engine.ExecuteOp( 
        StandardOp::Collatz,
        ( &[&in_buf, &out_buf]).into(),
        WorkgroupDim::Linear( 2),
    );
    jeeves_assert!( ctx, err.is_ok());
    let  	res_bytes = out_buf.Read();
    let  	res_u32: &[u32] = res_bytes.CastArrFrom::< u32>().into();
    for ( i, &val) in res_u32.iter().enumerate() {
        jeeves_assert_eq!( ctx, val, Collatz( in_vals[i as u32]));
    }
});
jeeves_test!( Swarm, SwarmUnsupportedBackend, |ctx| {
    let  	gpu_dev = ComputeDevice::WithBackend( BackendKind::RustGpu, 0);
    let  	buf = gpu_dev.CreateBuffer( "gpu_buf", 64, BufferUsage::Storage());
    let  	kernel = ComputeDevice::DoubleKernel();
    let  	err = gpu_dev.Dispatch( &kernel, ( &[&buf]).into(), WorkgroupDim::Linear( 1));
    jeeves_assert!( ctx, err.is_err());
    let  	err_val = err.unwrap_err();
    jeeves_assert_eq!( ctx, err_val._Kind, SwarmErrorKind::UnsupportedBackend);
    let  	cuda_dev = ComputeDevice::WithBackend( BackendKind::CudaOxide, 0);
    let  	err_cuda = cuda_dev.Synchronize();
    jeeves_assert!( ctx, err_cuda.is_err());
    let  	cuda_val = err_cuda.unwrap_err();
    jeeves_assert_eq!( ctx, cuda_val._Kind, SwarmErrorKind::UnsupportedBackend);
});

jeeves_test!( Swarm, SwarmExplicitBackendContract, |ctx| {
    let  	engine = SwarmEngine::New( BackendKind::Cpu);
    let  	device = ComputeDevice::WithWorkers( 1);
    let  	input = [1.0f32, 2.0, 3.0, 4.0];
    let  	bytes: &[u8] = bytemuck::cast_slice( &input);
    let  	buffer = device.CreateBufferInit( "double", bytes.into(), BufferUsage::Storage());
    let  	err = engine.ExecuteWith( &device, StandardOp::Double, ( &[&buffer]).into(), WorkgroupDim::Linear( 1));
    jeeves_assert!( ctx, err.is_ok());
    let  	result_bytes = buffer.Read();
    let  	result: &[f32] = result_bytes.CastArrFrom::< f32>().into();
    jeeves_assert_eq!( ctx, result[0], 2.0);
    jeeves_assert_eq!( ctx, result[3], 8.0);
});

jeeves_test!( Swarm, DroveDoubleArtifact, |ctx| {
    let   source = StandardOpKernelSource( StandardOp::Double, BackendKind::RustGpu)
        .expect( "Drove Double SPIR-V source missing");
    jeeves_assert_eq!( ctx, source._Kind, KernelSourceKind::SpirV);
    jeeves_assert!( ctx, !source._ByteCode.IsEmpty());
    jeeves_assert_eq!( ctx,
        StandardOpEntryPoint( StandardOp::Double, BackendKind::RustGpu),
        "compute::double_cs"
    );
    match StandardOpKernelSource( StandardOp::VectorAdd, BackendKind::RustGpu) {
        Err( error) => jeeves_assert_eq!( ctx, error._Kind, SwarmErrorKind::CompilationError),
        Ok( _) => jeeves_assert!( ctx, false),
    }
});

//-------------------------------------------------------------------------------------------------
// Console & Example Tests
jeeves_test!( Swarm, SwarmConsoleReport, Console, |ctx| {
    let  	engine = SwarmEngine::Auto();
    jeeves_println!( 
        ctx,
        "         [Swarm] Auto backend: {}, Workers: {}",
        engine.Backend(),
        engine.Device().WorkerCount()
    );
    jeeves_assert_eq!( ctx, engine.Backend(), BackendKind::Cpu);
});
jeeves_test!( Swarm, SwarmVectorAddExample, Example, |ctx| {
    let  	engine = SwarmEngine::Auto();
    let  	a_data = [1.0f32, 2.0, 3.0, 4.0];
    let  	b_data = [10.0f32, 20.0, 30.0, 40.0];
    let  	c_data = [0.0f32; 4];
    let  	a_bytes: &[u8] = bytemuck::cast_slice( &a_data);
    let  	b_bytes: &[u8] = bytemuck::cast_slice( &b_data);
    let  	c_bytes: &[u8] = bytemuck::cast_slice( &c_data);
    let  	buf_a = engine
        .Device()
        .CreateBufferInit( "a", a_bytes.into(), BufferUsage::Storage());
    let  	buf_b = engine
        .Device()
        .CreateBufferInit( "b", b_bytes.into(), BufferUsage::Storage());
    let  	buf_c = engine.Device().CreateBufferInit( 
        "c",
        c_bytes.into(),
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
    );
    let  	res = engine.ExecuteOp( 
        StandardOp::VectorAdd,
        ( &[&buf_a, &buf_b, &buf_c]).into(),
        WorkgroupDim::Linear( 1),
    );
    jeeves_assert!( ctx, res.is_ok());
    let  	out_bytes = buf_c.Read();
    let  	out_floats: &[f32] = out_bytes.CastArrFrom::< f32>().into();
    jeeves_println!( 
        ctx,
        "         [Example] VectorAdd result: [{:.1}, {:.1}, {:.1}, {:.1}]",
        out_floats[0],
        out_floats[1],
        out_floats[2],
        out_floats[3]
    );
    jeeves_assert_eq!( ctx, out_floats[3], 44.0);
});
