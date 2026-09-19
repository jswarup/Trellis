use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test };
// mod.rs ---------------------------------------------------------------------------------------------------------
use	crate::silo::arr::{ Arr, MutArr };
use	crate::silo::buff::Buff;
use	crate::silo::cast::{ IAllocRawExt, IArrExt, ICastExt, IConstPtrAtExt, IConstPtrRefExt, IPtrAtExt, IPtrRefExt, MutAliasPtr };
use	crate::silo::dset::DisjointSet;
use	crate::silo::fifo::Fifo;
use	crate::silo::stash::Stash;
use	crate::silo::stk::Stk;
use	crate::silo::useg::USeg;
use	std::sync::atomic::AtomicU32;

//-------------------------------------------------------------------------------------------------
// USeg Tests
jeeves_test!( Silo, USegBasic, |ctx| {
    let  	s = USeg::WithLen( 10, 5);
    jeeves_assert_eq!( ctx, s.First(), 10);
    jeeves_assert_eq!( ctx, s.Last(), 14);
    jeeves_assert_eq!( ctx, s.Len(), 5);
    jeeves_assert_eq!( ctx, s.Begin(), 10);
    jeeves_assert_eq!( ctx, s.End(), 15);
    jeeves_assert_eq!( ctx, s.Mid(), 12);
    jeeves_assert!( ctx, s.Contains( 12));
    jeeves_assert!( ctx, !s.Contains( 15));
});
jeeves_test!( Silo, USegOverlap, |ctx| {
    let  	a = USeg::New( 0, 10);
    let  	b = USeg::New( 5, 15);
    let  	c = USeg::New( 11, 20);
    jeeves_assert!( ctx, a.Overlaps( &b));
    jeeves_assert!( ctx, !a.Overlaps( &c));
    let  	inter = a.Intersect( &b);
    jeeves_assert_eq!( ctx, inter.First(), 5);
    jeeves_assert_eq!( ctx, inter.Last(), 10);
});
jeeves_test!( Silo, USegSnip, |ctx| {
    let  	s = USeg::WithLen( 10, 10);                                // [10, 19]
    let  	left_snipped = s.LSnip( 3);                                // [13, 19]
    jeeves_assert_eq!( ctx, left_snipped.First(), 13);
    jeeves_assert_eq!( ctx, left_snipped.Len(), 7);
    let  	right_snipped = s.RSnip( 4);                               // [10, 15]
    jeeves_assert_eq!( ctx, right_snipped.First(), 10);
    jeeves_assert_eq!( ctx, right_snipped.Last(), 15);
    jeeves_assert_eq!( ctx, right_snipped.Len(), 6);
});
jeeves_test!( Silo, USegTraverseSpan, |ctx| {
    let  	s = USeg::New( 1, 5);
    let  	mut sum = 0;
    s.Traverse( |val| {
        sum += val;
    });
    jeeves_assert_eq!( ctx, sum, 15);
    let  	all_positive = s.Span( |val| val > 0);
    jeeves_assert!( ctx, all_positive);
});
jeeves_test!( Silo, USegBinarySearch, |ctx| {
    let  	vals = [10u32, 20, 30, 40, 50];
    let  	seg = USeg::FromLen( 5);
    jeeves_assert_eq!( ctx, seg.BinarySearch( |i| vals[i as usize].cmp( &10)), Ok( 0));
    jeeves_assert_eq!( ctx, seg.BinarySearch( |i| vals[i as usize].cmp( &30)), Ok( 2));
    jeeves_assert_eq!( ctx, seg.BinarySearch( |i| vals[i as usize].cmp( &50)), Ok( 4));
    jeeves_assert_eq!( ctx, seg.BinarySearch( |i| vals[i as usize].cmp( &5)), Err( 0));
    jeeves_assert_eq!( ctx, seg.BinarySearch( |i| vals[i as usize].cmp( &25)), Err( 2));
    jeeves_assert_eq!( ctx, seg.BinarySearch( |i| vals[i as usize].cmp( &55)), Err( 5));
    let  	emptySeg = USeg::Empty();
    jeeves_assert_eq!( 
        ctx,
        emptySeg.BinarySearch( |_| std::cmp::Ordering::Equal),
        Err( 0)
    );
});

//-------------------------------------------------------------------------------------------------
// Buff Tests (Fixed Capacity, 16 bytes)
jeeves_test!( Silo, BuffBasic, |ctx| {
    let  	data = [100, 200, 300];
    let  	b = Buff::FromArr( Arr::New( data.as_ptr(), data.len() as u32));
    jeeves_assert!( ctx, !b.IsEmpty());
    jeeves_assert_eq!( ctx, b.Cap(), 3);
    jeeves_assert_eq!( ctx, b.Size(), 3);
    jeeves_assert_eq!( ctx, b[0], 100);
    jeeves_assert_eq!( ctx, b[1], 200);
    jeeves_assert_eq!( ctx, b[2], 300);
    // Buff produces an Arr view into Buff[0, _Cap]
    let  	arr = b.AsArr();
    jeeves_assert_eq!( ctx, arr.Size(), 3);
    jeeves_assert_eq!( ctx, arr[1], 200);
});
jeeves_test!( Silo, BuffFromDispenser, |ctx| {
    let  	b = Buff::FromDispenser( 5, |i| ( i + 1) * 10);
    jeeves_assert_eq!( ctx, b.Cap(), 5);
    jeeves_assert_eq!( ctx, b[0], 10);
    jeeves_assert_eq!( ctx, b[4], 50);
    let  	mut sum = 0;
    b.Traverse( |&val| {
        sum += val;
    });
    jeeves_assert_eq!( ctx, sum, 150);
});

//-------------------------------------------------------------------------------------------------
// Arr & MutArr Tests (Non-owning borrowed views)
jeeves_test!( Silo, ArrSlicingAndSnipping, |ctx| {
    let  	data = [1, 2, 3, 4, 5, 6];
    let  	arr = Arr::New( data.as_ptr(), data.len() as u32);
    jeeves_assert_eq!( ctx, arr.Size(), 6);
    jeeves_assert_eq!( ctx, arr.First(), Some( &1));
    jeeves_assert_eq!( ctx, arr.Last(), Some( &6));
    let  	left = arr.LSnip( 2);                                      // items 2..6 => [3, 4, 5, 6]
    jeeves_assert_eq!( ctx, left.Size(), 4);
    jeeves_assert_eq!( ctx, left[0], 3);
    let  	right = arr.RSnip( 2);                                     // items 0..4 => [1, 2, 3, 4]
    jeeves_assert_eq!( ctx, right.Size(), 4);
    jeeves_assert_eq!( ctx, right[3], 4);
    let  	mid = arr.Slice( 2, 3);                                    // items 2..5 => [3, 4, 5]
    jeeves_assert_eq!( ctx, mid.Size(), 3);
    jeeves_assert_eq!( ctx, mid[0], 3);
    jeeves_assert_eq!( ctx, mid[2], 5);
});
jeeves_test!( Silo, MutArrMutations, |ctx| {
    let  	mut data = [10, 20, 30, 40];
    let  	mut mut_arr = MutArr::New( data.as_mut_ptr(), data.len() as u32);
    mut_arr.Swap( 0, 3);
    jeeves_assert_eq!( ctx, mut_arr[0], 40);
    jeeves_assert_eq!( ctx, mut_arr[3], 10);
    mut_arr.SetAt( 1, 99);
    jeeves_assert_eq!( ctx, mut_arr[1], 99);
});

//-------------------------------------------------------------------------------------------------
// Stash Tests (Dynamic filling & Buff extraction)
jeeves_test!( Silo, StashDynamicAndExtractBuff, |ctx| {
    let  	mut stash = Stash::< i32>::New();
    jeeves_assert!( ctx, stash.IsEmpty());
    USeg::FromLen( 10).Traverse( |i| {
        stash.Push( ( i as i32 + 1) * 5);
    });
    jeeves_assert_eq!( ctx, stash.Size(), 10);
    jeeves_assert!( ctx, stash.Capacity() >= 10);
    jeeves_assert_eq!( ctx, stash[0], 5);
    jeeves_assert_eq!( ctx, stash[9], 50);
    // Extract built Buff from Stash (shrinkToFit = true)
    let  	buff = stash.ExtractBuff();
    jeeves_assert_eq!( ctx, buff.Cap(), 10);
    jeeves_assert_eq!( ctx, buff[0], 5);
    jeeves_assert_eq!( ctx, buff[9], 50);
    // Buff view into Buff[0, _Cap]
    let  	arr = buff.AsArr();
    jeeves_assert_eq!( ctx, arr.Size(), 10);
});
jeeves_test!( Silo, StashPopBack, |ctx| {
    let  	mut stash = Stash::< i32>::WithCapacity( 4);
    stash.Push( 11);
    stash.Push( 22);
    stash.Push( 33);
    jeeves_assert_eq!( ctx, stash.Size(), 3);
    let  	popped = stash.Pop();
    jeeves_assert_eq!( ctx, popped, Some( 33));
    jeeves_assert_eq!( ctx, stash.Size(), 2);
});

//-------------------------------------------------------------------------------------------------
// Stk Tests (Atomic Stack View)
jeeves_test!( Silo, StkAtomicStackOps, |ctx| {
    let  	mut storage = [0i32; 8];
    let  	size_atomic = AtomicU32::new( 0);
    let  	mut_arr = MutArr::New( storage.as_mut_ptr(), storage.len() as u32);
    let  	stk = Stk::Create( &size_atomic, mut_arr);
    jeeves_assert_eq!( ctx, stk.Size(), 0);
    jeeves_assert_eq!( ctx, stk.Capacity(), 8);
    jeeves_assert_eq!( ctx, stk.SzVoid(), 8);
    jeeves_assert!( ctx, stk.Push( 101));
    jeeves_assert!( ctx, stk.Push( 202));
    jeeves_assert_eq!( ctx, stk.Size(), 2);
    jeeves_assert_eq!( ctx, stk.SzVoid(), 6);
    let  	mut out = 0;
    jeeves_assert!( ctx, stk.Pop( &mut out));
    jeeves_assert_eq!( ctx, out, 202);
    jeeves_assert_eq!( ctx, stk.Size(), 1);
});

//-------------------------------------------------------------------------------------------------
// DisjointSet Tests (Backed by Stash)
jeeves_test!( Silo, DsetUnionFind, |ctx| {
    let  	mut dset = DisjointSet::WithCapacity( 10);
    jeeves_assert_eq!( ctx, dset.Size(), 10);
    let  	not_same = !dset.Same( 1, 2);
    jeeves_assert!( ctx, not_same);
    dset.Union( 1, 2);
    dset.Union( 2, 3);
    dset.Union( 4, 5);
    let  	same_1_3 = dset.Same( 1, 3);
    let  	same_4_5 = dset.Same( 4, 5);
    let  	not_same_1_5 = !dset.Same( 1, 5);
    jeeves_assert!( ctx, same_1_3);
    jeeves_assert!( ctx, same_4_5);
    jeeves_assert!( ctx, not_same_1_5);
    dset.Union( 3, 4);
    let  	same_1_5 = dset.Same( 1, 5);
    jeeves_assert!( ctx, same_1_5);
});
jeeves_test!( Silo, DsetGrow, |ctx| {
    let  	mut dset = DisjointSet::WithCapacity( 4);
    dset.Grow( 4);
    jeeves_assert_eq!( ctx, dset.Size(), 8);
    dset.Union( 3, 7);
    let  	same_3_7 = dset.Same( 3, 7);
    jeeves_assert!( ctx, same_3_7);
});

//-------------------------------------------------------------------------------------------------
// Fifo Tests
jeeves_test!( Silo, FifoOperations, |ctx| {
    let  	mut q = Fifo::< i32, 4>::New();
    jeeves_assert!( ctx, q.IsEmpty());
    let  	not_full = !q.IsFull();
    jeeves_assert!( ctx, not_full);
    let  	p1 = q.PushBack( 10);
    let  	p2 = q.PushBack( 20);
    let  	p3 = q.PushBack( 30);
    let  	p4 = q.PushBack( 40);
    jeeves_assert!( ctx, p1);
    jeeves_assert!( ctx, p2);
    jeeves_assert!( ctx, p3);
    jeeves_assert!( ctx, p4);
    let  	is_full = q.IsFull();
    jeeves_assert!( ctx, is_full);
    let  	p5 = q.PushBack( 50);
    let  	p5_failed = !p5;
    jeeves_assert!( ctx, p5_failed);                                   // Should fail, capacity 4
    jeeves_assert_eq!( ctx, q.Front(), Some( &10));
    jeeves_assert_eq!( ctx, q.PopFront(), Some( 10));
    jeeves_assert_eq!( ctx, q.PopFront(), Some( 20));
    // Circular wrap
    let  	p6 = q.PushBack( 60);
    let  	p7 = q.PushBack( 70);
    jeeves_assert!( ctx, p6);
    jeeves_assert!( ctx, p7);
    jeeves_assert_eq!( ctx, q.Size(), 4);
    jeeves_assert_eq!( ctx, q.PopFront(), Some( 30));
    jeeves_assert_eq!( ctx, q.PopFront(), Some( 40));
    jeeves_assert_eq!( ctx, q.PopFront(), Some( 60));
    jeeves_assert_eq!( ctx, q.PopFront(), Some( 70));
    jeeves_assert!( ctx, q.IsEmpty());
});

//-------------------------------------------------------------------------------------------------
// Console Tests
jeeves_test!( Silo, BuffLayoutConsole, Console, |ctx| {
    let  	b = Buff::FromDispenser( 8, |i| ( i as u64) * 10);
    jeeves_println!( 
        ctx,
        "         [Console] Buff: Cap={}, Ptr={:?}",
        b.Cap(),
        b.Data()
    );
    jeeves_assert_eq!( ctx, b.Cap(), 8);
    let  	arr = b.AsArr();
    jeeves_println!( 
        ctx,
        "         [Console] Arr view into Buff[0, _Cap]: Size={}",
        arr.Size()
    );
    jeeves_assert_eq!( ctx, arr.Size(), 8);
});
jeeves_test!( Silo, StashBuildConsole, Console, |ctx| {
    let  	mut stash = Stash::< u32>::WithCapacity( 4);
    USeg::FromLen( 6).Traverse( |i| stash.Push( i * 11));
    jeeves_println!( 
        ctx,
        "         [Console] Stash: Size={}, Buff.Cap={}",
        stash.Size(),
        stash.Capacity()
    );
    jeeves_assert_eq!( ctx, stash.Size(), 6);
    let  	buff = stash.ExtractBuff();
    jeeves_println!( ctx, "         [Console] Extracted Buff: Cap={}", buff.Cap());
    jeeves_assert_eq!( ctx, buff.Cap(), 6);
});

//-------------------------------------------------------------------------------------------------
// Example Tests
jeeves_test!( Silo, StashBuildBuffExample, Example, |ctx| {
    jeeves_println!( 
        ctx,
        "         [Example] Using Stash locally to build and extract a Buff:"
    );
    let  	mut stash = Stash::< u32>::New();
    USeg::New( 1, 5).Traverse( |i| {
        stash.Push( i * 7);
    });
    let  	buff = stash.ExtractBuff();
    let  	arr = buff.AsArr();
    arr.USeg().Traverse( |i| {
        jeeves_println!( ctx, "           arr[{}] = {}", i, arr[i]);
    });
    jeeves_assert_eq!( ctx, arr[4], 35);
});
jeeves_test!( Silo, StkUsageExample, Example, |ctx| {
    jeeves_println!( ctx, "         [Example] Stk lock-free atomic stack view:");
    let  	mut data = [0u32; 4];
    let  	size = AtomicU32::new( 0);
    let  	stk = Stk::Create( &size, MutArr::New( data.as_mut_ptr(), data.len() as u32));
    stk.Push( 77);
    stk.Push( 88);
    jeeves_println!( ctx, "           stk size = {}", stk.Size());
    let  	mut val = 0;
    stk.Pop( &mut val);
    jeeves_println!( ctx, "           popped = {}", val);
    jeeves_assert_eq!( ctx, val, 88);
});
jeeves_test!( Silo, DsetUsageExample, Example, |ctx| {
    jeeves_println!( 
        ctx,
        "         [Example] DisjointSet (Union-Find) clustering:"
    );
    let  	mut d = DisjointSet::WithCapacity( 5);
    d.Union( 0, 2);
    d.Union( 2, 4);
    jeeves_println!( ctx, "           Same(0, 4) = {}", d.Same( 0, 4));
    let  	same_0_4 = d.Same( 0, 4);
    jeeves_assert!( ctx, same_0_4);
});
jeeves_test!( Silo, FifoUsageExample, Example, |ctx| {
    jeeves_println!( 
        ctx,
        "         [Example] Fifo circular ring buffer streaming:"
    );
    let  	mut q = Fifo::< &'static str, 3>::New();
    q.PushBack( "alpha");
    q.PushBack( "beta");
    q.PushBack( "gamma");
    while let  	Some( msg) = q.PopFront() {
        jeeves_println!( ctx, "           dequeued: {}", msg);
    }
    jeeves_assert!( ctx, q.IsEmpty());
});

//-------------------------------------------------------------------------------------------------
// Cast Extension Tests
jeeves_test!( Silo, CastTraits, |ctx| {
    // 1. ICastExt: value transmute
    let  	val_u32: u32 = 0x12345678;
    let  	val_i32: i32 = val_u32.Cast();
    jeeves_assert_eq!( ctx, val_i32, 0x12345678i32);
    // 2. ISliceExt -> IArrExt: CastArr & CastArrFrom
    let  	u32_slice: [u32; 2] = [0xAABBCCDD, 0x11223344];
    let  	u32_arr = Arr::New( u32_slice.as_ptr(), 2);
    let  	byte_arr = u32_arr.CastArr();
    jeeves_assert_eq!( ctx, byte_arr.Size(), 8);
    let  	recovered: Arr< '_, u32> = byte_arr.CastArrFrom();
    jeeves_assert_eq!( ctx, recovered[0], u32_slice[0]);
    jeeves_assert_eq!( ctx, recovered[1], u32_slice[1]);
    // 3. IPtrAtExt and IConstPtrAtExt
    let  	mut nums = [10u32, 20, 30];
    let  	raw_const = nums.as_ptr();
    jeeves_assert_eq!( ctx, *raw_const.RefAt( 0), 10);
    jeeves_assert_eq!( ctx, *raw_const.RefAt( 2), 30);
    let  	raw_mut = nums.as_mut_ptr();
    *raw_mut.MutRefAt( 1) = 99;
    jeeves_assert_eq!( ctx, *raw_const.RefAt( 1), 99);
    // 4. IPtrRefExt & IConstPtrRefExt
    jeeves_assert_eq!( ctx, *raw_const.Ref(), 10);
    *raw_mut.MutRef() = 55;
    jeeves_assert_eq!( ctx, *raw_const.Ref(), 55);
    // 5. IAllocRawExt
    let  	raw_heap = 777u32.AllocRaw();
    jeeves_assert_eq!( ctx, *raw_heap.Ref(), 777);
    unsafe {
        let  	_ = Box::from_raw( raw_heap);
    }
    // 6. MutAliasPtr
    let  	mut x = 1234u32;
    let  	alias = MutAliasPtr::NewMut( &mut x);
    jeeves_assert_eq!( ctx, *alias.Ref(), 1234);
    *alias.MutRef() = 5678;
    jeeves_assert_eq!( ctx, *alias.Ref(), 5678);
});
jeeves_test!( Silo, CastUsageExample, Example, |ctx| {
    jeeves_println!( 
        ctx,
        "         [Example] Silo Cast zero-cost pointer and array traits:"
    );
    let  	ints = [100u32, 200, 300];
    let  	ints_arr = Arr::New( ints.as_ptr(), 3);
    let  	bytes = ints_arr.CastArr();
    jeeves_println!( 
        ctx,
        "           ints (3 x u32) cast to byte array: size={}",
        bytes.Size()
    );
    jeeves_assert_eq!( ctx, bytes.Size(), 12);
    let  	recast: Arr< '_, u32> = bytes.CastArrFrom();
    jeeves_println!( ctx, "           recast first item: {}", recast[0]);
    jeeves_assert_eq!( ctx, recast[0], 100);
});
jeeves_test!( Silo, DropCountTracking, |ctx| {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DROP_COUNT: AtomicUsize = AtomicUsize::new( 0);
    struct DropTracker
    {
        _val: u32,
    }
    impl Drop for DropTracker {
        fn	drop( &mut self)
        {
            DROP_COUNT.fetch_add( 1, Ordering::SeqCst);
        }
    }
    DROP_COUNT.store( 0, Ordering::SeqCst);
    {
        let  	mut stash = Stash::< DropTracker>::WithCapacity( 5);
        stash.Push( DropTracker { _val: 1 });
        stash.Push( DropTracker { _val: 2 });
        stash.Push( DropTracker { _val: 3 });
        drop( stash.Pop());
        // 1 element dropped via Pop.
        jeeves_assert_eq!( ctx, DROP_COUNT.load( Ordering::SeqCst), 1);
        let  	_buff = stash.ExtractBuff();
        // Stash is destroyed, but 2 elements are in _buff.
        jeeves_assert_eq!( ctx, DROP_COUNT.load( Ordering::SeqCst), 1);
    }                                                                  // _buff drops here
    // 2 more elements dropped. Total = 3.
    jeeves_assert_eq!( ctx, DROP_COUNT.load( Ordering::SeqCst), 3);
});
jeeves_test!( Silo, PanicUnwindGuard, |ctx| {
    use std::panic;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DROP_COUNT: AtomicUsize = AtomicUsize::new( 0);
    struct DropTracker
    {
        _dummy: u8,
    }
    impl Drop for DropTracker {
        fn	drop( &mut self)
        {
            DROP_COUNT.fetch_add( 1, Ordering::SeqCst);
        }
    }
    DROP_COUNT.store( 0, Ordering::SeqCst);
    let  	result = panic::catch_unwind( || {
        Buff::FromDispenser( 5, |i| {
            if i == 3 {
                panic!( "Intentional panic during construction");
            }
            DropTracker { _dummy: 0 }
        });
    });
    jeeves_assert!( ctx, result.is_err());
    // Only 3 elements were initialized (0, 1, 2) before the panic at 3.
    // The Guard should drop exactly those 3.
    jeeves_assert_eq!( ctx, DROP_COUNT.load( Ordering::SeqCst), 3);
});
