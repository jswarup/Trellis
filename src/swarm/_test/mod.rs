use crate::{jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test};
// mod.rs ---------------------------------------------------------------------------------------------------------
use crate::heist::atelier::Atelier;
use crate::silo::buff::Buff;
use crate::swarm::cpu::ComputeDevice;
use crate::swarm::engine::SwarmEngine;
use crate::swarm::ops::{StandardOp, StandardOpKernelSource, StandardOpLabel};
use crate::swarm::traits::{BackendKind, BufferUsage, CpuBuffer, SwarmErrorKind, WorkgroupDim};
use crate::symph::compshade::Collatz;

//-------------------------------------------------------------------------------------------------

// Swarm Tests
jeeves_test!( Swarm, CpuBufferLifecycle, |ctx| {
    let  buf = CpuBuffer::New(
        "test_buf",
        64,
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
        BackendKind::Cpu,
    );
    jeeves_assert_eq!( ctx, buf.Size(), 64);
    jeeves_assert_eq!( ctx, buf.Label(), "test_buf");
    let  data: [u8; 4] = [10, 20, 30, 40];
    let  write_res = buf.Write( &data);
    jeeves_assert!( ctx, write_res.is_ok());
    let  mut read_back = buf.Read();
    jeeves_assert!( ctx, ( read_back.Cap() as usize) >= 4);
    jeeves_assert_eq!( ctx, read_back[0], 10);
    jeeves_assert_eq!( ctx, read_back[1], 20);
    jeeves_assert_eq!( ctx, read_back[2], 30);
    jeeves_assert_eq!( ctx, read_back[3], 40);
    // Verify Read() returns an independent copy
    read_back[0] = 99;
    let  second_read = buf.Read();
    jeeves_assert_eq!( ctx, second_read[0], 10);
});
jeeves_test!( Swarm, CpuDeviceDoubleOp, |ctx| {
    let  dev = ComputeDevice::WithWorkers( 1);
    const COUNT: usize = 64;
    let  values = Buff::FromDispenser( COUNT as u32, |i| ( i + 1) as f32);
    let  bytes = unsafe {
        std::slice::from_raw_parts(
            values.AsPtr() as *const u8,
            COUNT * std::mem::size_of::<f32>(),
        )
    };
    let  buf = dev.CreateBufferInit(
        "data",
        bytes,
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
    );
    let  source = StandardOpKernelSource( StandardOp::Double, BackendKind::Cpu);
    let  kernel = dev
        .CompileKernel( StandardOpLabel( StandardOp::Double), "main", &source)
        .expect( "Kernel compilation failed");
    let  err = dev.Dispatch( &kernel, &[&buf], WorkgroupDim::Linear( 1));
    jeeves_assert!( ctx, err.is_ok());
    let  result_bytes = buf.Read();
    let  result_floats =
        unsafe { std::slice::from_raw_parts( result_bytes.AsPtr() as *const f32, COUNT) };
    for ( i, &val) in result_floats.iter().enumerate()
    {
        jeeves_assert_eq!( ctx, val, ( ( i + 1) * 2) as f32);
    }
});
jeeves_test!( Swarm, CpuDeviceDoubleOpParallel, |ctx| {
    Atelier::Reset( 4);
    const COUNT: usize = 128;
    let  values = Buff::FromDispenser( COUNT as u32, |i| ( i + 1) as f32);
    let  bytes = unsafe { std::slice::from_raw_parts( values.AsPtr() as *const u8, COUNT * 4) };
    let  buf = ComputeDevice::New().CreateBufferInit(
        "data",
        bytes,
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
    );
    let  err = ComputeDevice::New().Dispatch(
        &ComputeDevice::DoubleKernel(),
        &[&buf],
        WorkgroupDim::Linear( 2),
    );
    jeeves_assert!( ctx, err.is_ok());
    let  result_bytes = buf.Read();
    let  result_floats =
        unsafe { std::slice::from_raw_parts( result_bytes.AsPtr() as *const f32, COUNT) };
    for ( i, &val) in result_floats.iter().enumerate()
    {
        jeeves_assert_eq!( ctx, val, ( ( i + 1) * 2) as f32);
    }
});
jeeves_test!( Swarm, CpuDeviceVectorAddOp, |ctx| {
    let  dev = ComputeDevice::WithWorkers( 1);
    const COUNT: usize = 64;
    let  a = Buff::FromDispenser( COUNT as u32, |i| i as f32);
    let  b = Buff::FromDispenser( COUNT as u32, |i| ( i * 10) as f32);
    let  c = Buff::FromDispenser( COUNT as u32, |_| 0.0f32);
    let  a_bytes = unsafe { std::slice::from_raw_parts( a.AsPtr() as *const u8, COUNT * 4) };
    let  b_bytes = unsafe { std::slice::from_raw_parts( b.AsPtr() as *const u8, COUNT * 4) };
    let  c_bytes = unsafe { std::slice::from_raw_parts( c.AsPtr() as *const u8, COUNT * 4) };
    let  buf_a = dev.CreateBufferInit( "a", a_bytes, BufferUsage::Storage());
    let  buf_b = dev.CreateBufferInit( "b", b_bytes, BufferUsage::Storage());
    let  buf_c = dev.CreateBufferInit(
        "c",
        c_bytes,
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
    );
    let  source = StandardOpKernelSource( StandardOp::VectorAdd, BackendKind::Cpu);
    let  kernel = dev
        .CompileKernel( StandardOpLabel( StandardOp::VectorAdd), "main", &source)
        .expect( "Kernel compilation failed");
    let  err = dev.Dispatch( &kernel, &[&buf_a, &buf_b, &buf_c], WorkgroupDim::Linear( 1));
    jeeves_assert!( ctx, err.is_ok());
    let  result_bytes = buf_c.Read();
    let  result_floats =
        unsafe { std::slice::from_raw_parts( result_bytes.AsPtr() as *const f32, COUNT) };
    for ( i, &val) in result_floats.iter().enumerate()
    {
        jeeves_assert_eq!( ctx, val, ( i * 11) as f32);
    }
});
jeeves_test!( Swarm, SwarmEngineCollatzAndParallelDispatch, |ctx| {
    // Reset Atelier with 4 worker threads for parallel SIMT dispatch
    Atelier::Reset( 4);
    let  engine = SwarmEngine::New( BackendKind::Cpu);
    const COUNT: usize = 128;
    let  in_vals = Buff::FromDispenser( COUNT as u32, |i| ( i % 10) + 1);
    let  out_vals = Buff::FromDispenser( COUNT as u32, |_| 0u32);
    let  in_bytes = unsafe { std::slice::from_raw_parts( in_vals.AsPtr() as *const u8, COUNT * 4) };
    let  out_bytes = unsafe { std::slice::from_raw_parts( out_vals.AsPtr() as *const u8, COUNT * 4) };
    let  in_buf = engine
        .Device()
        .CreateBufferInit( "in", in_bytes, BufferUsage::Storage());
    let  out_buf = engine.Device().CreateBufferInit(
        "out",
        out_bytes,
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
    );
    let  err = engine.ExecuteOp(
        StandardOp::Collatz,
        &[&in_buf, &out_buf],
        WorkgroupDim::Linear( 2),
    );
    jeeves_assert!( ctx, err.is_ok());
    let  res_bytes = out_buf.Read();
    let  res_u32 = unsafe { std::slice::from_raw_parts( res_bytes.AsPtr() as *const u32, COUNT) };
    for ( i, &val) in res_u32.iter().enumerate()
    {
        jeeves_assert_eq!( ctx, val, Collatz( in_vals[i as u32]));
    }
});
jeeves_test!( Swarm, SwarmUnsupportedBackend, |ctx| {
    let  gpu_dev = ComputeDevice::WithBackend( BackendKind::RustGpu, 0);
    let  buf = gpu_dev.CreateBuffer( "gpu_buf", 64, BufferUsage::Storage());
    let  kernel = ComputeDevice::DoubleKernel();
    let  err = gpu_dev.Dispatch( &kernel, &[&buf], WorkgroupDim::Linear( 1));
    jeeves_assert!( ctx, err.is_err());
    let  err_val = err.unwrap_err();
    jeeves_assert_eq!( ctx, err_val._Kind, SwarmErrorKind::UnsupportedBackend);
    let  cuda_dev = ComputeDevice::WithBackend( BackendKind::CudaOxide, 0);
    let  err_cuda = cuda_dev.Synchronize();
    jeeves_assert!( ctx, err_cuda.is_err());
    let  cuda_val = err_cuda.unwrap_err();
    jeeves_assert_eq!( ctx, cuda_val._Kind, SwarmErrorKind::UnsupportedBackend);
});

//-------------------------------------------------------------------------------------------------

// Console & Example Tests
jeeves_test!( Swarm, SwarmConsoleReport, Console, |ctx| {
    let  engine = SwarmEngine::Auto();
    jeeves_println!(
        ctx,
        "         [Swarm] Auto backend: {}, Workers: {}",
        engine.Backend(),
        engine.Device().WorkerCount()
    );
    jeeves_assert_eq!( ctx, engine.Backend(), BackendKind::Cpu);
});
jeeves_test!( Swarm, SwarmVectorAddExample, Example, |ctx| {
    let  engine = SwarmEngine::Auto();
    let  a_data = [1.0f32, 2.0, 3.0, 4.0];
    let  b_data = [10.0f32, 20.0, 30.0, 40.0];
    let  c_data = [0.0f32; 4];
    let  a_bytes = unsafe { std::slice::from_raw_parts( a_data.as_ptr() as *const u8, 16) };
    let  b_bytes = unsafe { std::slice::from_raw_parts( b_data.as_ptr() as *const u8, 16) };
    let  c_bytes = unsafe { std::slice::from_raw_parts( c_data.as_ptr() as *const u8, 16) };
    let  buf_a = engine
        .Device()
        .CreateBufferInit( "a", a_bytes, BufferUsage::Storage());
    let  buf_b = engine
        .Device()
        .CreateBufferInit( "b", b_bytes, BufferUsage::Storage());
    let  buf_c = engine.Device().CreateBufferInit(
        "c",
        c_bytes,
        BufferUsage::Storage() | BufferUsage::ReadWrite(),
    );
    let  res = engine.ExecuteOp(
        StandardOp::VectorAdd,
        &[&buf_a, &buf_b, &buf_c],
        WorkgroupDim::Linear( 1),
    );
    jeeves_assert!( ctx, res.is_ok());
    let  out_bytes = buf_c.Read();
    let  out_floats = unsafe { std::slice::from_raw_parts( out_bytes.AsPtr() as *const f32, 4) };
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
