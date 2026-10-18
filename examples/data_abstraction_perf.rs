// data_abstraction_perf.rs ------------------------------------------------------------------------
#![allow( non_snake_case)]

use    std::{ hint::black_box, time::Instant };
use    trellis::silo::{ MutArr, USeg };
use    trellis::swarm::cpu::ComputeDevice;
use    trellis::swarm::traits::{ BufferUsage, KernelSource, WorkgroupDim };
use    trellis::zephyr::shm::{ ShmPacket, ShmRingBuffer };

//-------------------------------------------------------------------------------------------------

fn    Measure( label: &str, mut action: impl FnMut())
{
    USeg::FromLen( 10_000).Traverse( |_| action());
    let    mut samples = [0_u128; 9];
    USeg::FromLen( 9).Traverse( |sample| {
        let    start = Instant::now();
        USeg::FromLen( 200_000).Traverse( |_| action());
        samples[sample as usize] = start.elapsed().as_nanos();
    });
    MutArr::from( &mut samples).QSort( |left, right| left < right);
    println!( "{}: median {:.2} ns/op", label, samples[4] as f64 / 200_000.0);
}

fn    main()
{
    println!( "KernelSource: {} bytes", std::mem::size_of::< KernelSource>());
    let    source = KernelSource::Wgsl( "@compute @workgroup_size(64) fn main() {}");
    Measure( "source_clone", || {
        black_box( source.clone());
    });
    let    device = ComputeDevice::WithWorkers( 1);
    let    values = [1.0_f32; 64];
    let    a = device.CreateBufferInit( "a", bytemuck::cast_slice( &values).into(), BufferUsage::Storage());
    let    b = device.CreateBufferInit( "b", bytemuck::cast_slice( &values).into(), BufferUsage::Storage());
    let    out = device.CreateBufferInit( "out", bytemuck::cast_slice( &values).into(), BufferUsage::Storage());
    let    kernel = ComputeDevice::VectorAddKernel();
    Measure( "vector_add_64", || {
        device.Dispatch( black_box( &kernel), ( &[&a, &b, &out]).into(), WorkgroupDim::Linear( 1)).unwrap();
    });
    let    input = [1_u32; 64];
    let    a = device.CreateBufferInit( "collatz", bytemuck::cast_slice( &input).into(), BufferUsage::Storage());
    let    kernel = ComputeDevice::CollatzKernel();
    Measure( "collatz_64", || {
        device.Dispatch( black_box( &kernel), ( &[&a, &out]).into(), WorkgroupDim::Linear( 1)).unwrap();
    });
    let    mut ring = ShmRingBuffer::New( 1000);
    let    packet = ShmPacket::New( 0, 0, ( &[42_u8; 16]).into());
    Measure( "ring_push_pop", || {
        ring.PushPacket( black_box( &packet)).unwrap();
        black_box( ring.PopPacket().unwrap());
    });
}

//-------------------------------------------------------------------------------------------------
