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
