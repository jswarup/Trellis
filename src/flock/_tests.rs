use	crate::{ jeeves_assert_eq, jeeves_test };
use	crate::flock::CpuOutputPartition;
use	crate::heist::Atelier;
use	std::sync::atomic::{ AtomicU32, Ordering };

//-------------------------------------------------------------------------------------------------

jeeves_test!( Flock, CpuOutputPartitionSplitsDisjointRanges, |ctx| {
    let  	mut values = [0u32; 6];
    let  	output = CpuOutputPartition::New( 10, ( &mut values).into());
    let  	( mut left, mut right) = output.SplitAt( 2);
    jeeves_assert_eq!( ctx, left.GlobalBase(), 10);
    jeeves_assert_eq!( ctx, left.Count(), 2);
    jeeves_assert_eq!( ctx, right.GlobalBase(), 12);
    jeeves_assert_eq!( ctx, right.Count(), 4);
    left.ForEach( |global, value| *value = global);
    right.ForEach( |global, value| *value = global);
    jeeves_assert_eq!( ctx, values, [10, 11, 12, 13, 14, 15]);
});

jeeves_test!( Flock, CpuOutputPartitionExecutesInScopedHeistWorkers, |ctx| {
    let  	atelier = Atelier::New( 3);
    let  	mut values = [0u32; 96];
    let  	workers = AtomicU32::new( 0);
    CpuOutputPartition::New( 0, ( &mut values).into()).ExecuteScoped( &atelier, |worker, mut partition| {
        workers.fetch_or( 1 << worker, Ordering::Relaxed);
        partition.ForEach( |index, value| *value = index + 1);
    });
    jeeves_assert_eq!( ctx, workers.load( Ordering::Relaxed), 0b111);
    jeeves_assert_eq!( ctx, values[0], 1);
    jeeves_assert_eq!( ctx, values[95], 96);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Flock, PointKernelsRejectOutOfBoundsInvocations, |ctx| {
    use crate::flock::StandardOpCpuKernelFn;
    use crate::silo::{ Arr, MutArr, USeg };
    use crate::symph::{ CameraUniforms, StandardOp };

    let points      = [1.0_f32, 2.0, 3.0];
    let camera      = CameraUniforms::default().Values();
    let inputs: [Arr< '_, u8>; 2] = [
        bytemuck::cast_slice( &points).into(),
        bytemuck::cast_slice( &camera).into(),
    ];
    let operations  = [StandardOp::PointCloud, StandardOp::CameraTransform];
    let invocations = [1_u32, 1 << 30, 1 << 31, u32::MAX];
    USeg::FromLen( 2).Traverse( |op| {
        let kernel  = StandardOpCpuKernelFn( operations[op as usize]);
        USeg::FromLen( 4).Traverse( |invocation| {
            let mut values = [7.0_f32; 6];
            {
                let mut outputs: [MutArr< '_, u8>; 1] = [
                    bytemuck::cast_slice_mut( &mut values).into(),
                ];
                kernel( ( &inputs).into(), ( &mut outputs).into(),
                    invocations[invocation as usize], 0, 0);
            }
            jeeves_assert_eq!( ctx, values, [7.0_f32; 6]);
        });
    });
});

//-------------------------------------------------------------------------------------------------
