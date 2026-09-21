use	crate::{ jeeves_assert_eq, jeeves_test };
use	crate::flock::CpuOutputPartition;

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
