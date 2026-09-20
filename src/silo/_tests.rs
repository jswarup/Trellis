use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test };
// mod.rs ---------------------------------------------------------------------------------------------------------
use	crate::silo::arr::{ Arr, MutArr };
use	crate::silo::buff::Buff;
use	crate::silo::cast::{ IAllocRawExt, IArrExt, ICastExt, IConstPtrAtExt, IConstPtrRefExt, IPtrAtExt, IPtrRefExt, MutAliasPtr };
use	crate::silo::cast::IMutArrExt;
use	crate::silo::dset::DisjointSet;
use	crate::silo::fifo::Fifo;
use	crate::silo::stash::Stash;
use	crate::silo::stk::Stk;
use	crate::silo::useg::USeg;
use	std::sync::atomic::AtomicU32;

//-------------------------------------------------------------------------------------------------
// USeg Tests
jeeves_test!( Silo, ArrSliceInterop, |ctx| {
    let  	bytes = [b'a', 0xff, b'z'];
    let  	arr = Arr::from( &bytes);
    let  	slice: &[u8] = arr.into();
    jeeves_assert_eq!( ctx, slice.as_ptr(), bytes.as_ptr());
    jeeves_assert_eq!( ctx, slice, &bytes);
    jeeves_assert!( ctx, std::str::from_utf8( slice).is_err());
    let  	empty: &[u8] = Arr::Empty().into();
    jeeves_assert!( ctx, empty.is_empty());
    let  	mut values = [1u8, 2, 3];
    let  	mutable: &mut [u8] = MutArr::from( &mut values).into();
    mutable[1] = 9;
    jeeves_assert_eq!( ctx, values[1], 9);
    let  	emptyMut: &mut [u8] = MutArr::Empty().into();
    jeeves_assert!( ctx, emptyMut.is_empty());
});
jeeves_test!( Silo, MutArrSort, |ctx| {
    let  	mut values = [String::from( "c"), String::from( "a"), String::from( "b"), String::from( "a")];
    MutArr::from( &mut values).QSort( |a, b| a < b);
    jeeves_assert_eq!( ctx, values, ["a", "a", "b", "c"]);
    MutArr::from( &mut values).QSort( |a, b| a > b);
    jeeves_assert_eq!( ctx, values, ["c", "b", "a", "a"]);
    MutArr::< String>::Empty().QSort( |_, _| panic!( "Empty array must not compare elements"));
});
jeeves_test!( Silo, ArrCopyOperations, |ctx| {
    let  	source = [10u32, 20, 30];
    let  	mut target = [0u32; 3];
    MutArr::from( &mut target).CopyFrom( Arr::from( &source));
    jeeves_assert_eq!( ctx, target, source);
    MutArr::< u32>::Empty().CopyFrom( Arr::Empty());
    let  	mismatch = std::panic::catch_unwind( || {
        let  	mut short = [0u32; 2];
        MutArr::from( &mut short).CopyFrom( Arr::from( &source));
    });
    jeeves_assert!( ctx, mismatch.is_err());
});
jeeves_test!( Silo, ArrUnalignedValues, |ctx| {
    let  	mut storage = [0xAAu8; 10];
    {
        let  	mut whole = MutArr::from( &mut storage);
        let  	mut tail = whole.LSnip( 1);
        let  	bytes = tail.RSnip( 1);
        unsafe {
            bytes.WriteValue( 0, 0x12345678u32);
            bytes.WriteValue( 1, 1.25f32);
            jeeves_assert_eq!( ctx, bytes.Arr().ReadValue::< u32>( 0), 0x12345678);
            jeeves_assert_eq!( ctx, bytes.Arr().ReadValue::< f32>( 1), 1.25);
        }
        let  	outside = std::panic::catch_unwind( || unsafe { bytes.Arr().ReadValue::< u32>( 2) });
        jeeves_assert!( ctx, outside.is_err());
        let  	outsideWrite = std::panic::catch_unwind( || unsafe { bytes.WriteValue( 2, 7u32) });
        jeeves_assert!( ctx, outsideWrite.is_err());
    }
    jeeves_assert_eq!( ctx, storage[0], 0xAA);
    jeeves_assert_eq!( ctx, storage[9], 0xAA);
});
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
    let  	arr = b.Arr();
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
jeeves_test!( Silo, ArrCopyWithoutElementClone, |ctx| {
    struct Value
    {
        _Value:     u32,
    }
    fn	CloneView< T: Clone>( value: &T) -> T
    {
        return value.clone();
    }
    let  	values      = [Value { _Value: 7 }];
    let  	view        = Arr::from( &values);
    let  	copied      = view;
    let  	cloned      = CloneView( &view);
    jeeves_assert_eq!( ctx, view[0]._Value, 7);
    jeeves_assert_eq!( ctx, copied[0]._Value, 7);
    jeeves_assert_eq!( ctx, cloned[0]._Value, 7);
});
jeeves_test!( Silo, ArrSliceClipping, |ctx| {
    let  	values      = [10u32, 20, 30, 40];
    let  	view        = Arr::from( &values);
    let  	cases       = Arr::from( &[
        ( 0, 0, 0),
        ( 0, u32::MAX, 4),
        ( 1, 2, 2),
        ( 3, u32::MAX, 1),
        ( 4, 1, 0),
        ( u32::MAX, u32::MAX, 0),
    ]);
    cases.USeg().Traverse( |i| {
        let  	( start, count, expected)    = cases[i];
        let  	sliced                      = view.Slice( start, count);
        jeeves_assert_eq!( ctx, sliced.Len(), expected);
        if expected > 0 {
            jeeves_assert_eq!( ctx, sliced[0], view[start]);
            jeeves_assert_eq!( ctx, sliced[expected - 1], view[start + expected - 1]);
        }
    });
    jeeves_assert_eq!( ctx, view.LSnip( 0).Len(), 4);
    jeeves_assert_eq!( ctx, view.RSnip( 0).Len(), 4);
    jeeves_assert!( ctx, view.LSnip( u32::MAX).IsEmpty());
    jeeves_assert!( ctx, view.RSnip( u32::MAX).IsEmpty());
    jeeves_assert!( ctx, Arr::< u32>::Empty().Slice( 0, u32::MAX).IsEmpty());
    let  	mut mutableValues   = values;
    let  	mut mutable         = MutArr::from( &mut mutableValues);
    cases.USeg().Traverse( |i| {
        let  	( start, count, expected)    = cases[i];
        let  	sliced                      = mutable.Slice( start, count);
        jeeves_assert_eq!( ctx, sliced.Len(), expected);
        if expected > 0 {
            jeeves_assert_eq!( ctx, sliced[0], view[start]);
            jeeves_assert_eq!( ctx, sliced[expected - 1], view[start + expected - 1]);
        }
    });
    jeeves_assert_eq!( ctx, mutable.LSnip( 0).Len(), 4);
    jeeves_assert_eq!( ctx, mutable.RSnip( 0).Len(), 4);
    jeeves_assert!( ctx, mutable.LSnip( u32::MAX).IsEmpty());
    jeeves_assert!( ctx, mutable.RSnip( u32::MAX).IsEmpty());
    jeeves_assert!( ctx, MutArr::< u32>::Empty().Slice( 0, u32::MAX).IsEmpty());
    mutable.Slice( 1, u32::MAX).SetAt( 0, 99);
    jeeves_assert_eq!( ctx, mutableValues, [10, 99, 30, 40]);
});
jeeves_test!( Silo, MutArrSortPreservesOwnershipOnPanic, |ctx| {
    struct Value< 'a>
    {
        _Id:        u32,
        _Drops:     &'a std::cell::Cell<u32>,
    }
    impl Drop for Value< '_> {
        fn	drop( &mut self)
        {
            self._Drops.set( self._Drops.get() + 1);
        }
    }
    let  	drops               = std::cell::Cell::new( 0u32);
    let  	mut values          = Buff::FromDispenser( 5, |i| Value { _Id: 4 - i, _Drops: &drops });
    let  	mut comparisons     = 0u32;
    let  	result              = std::panic::catch_unwind( std::panic::AssertUnwindSafe( || {
        values.MutArr().QSort( |a, b| {
            comparisons += 1;
            if comparisons == 3 {
                panic!( "Intentional comparator panic");
            }
            return a._Id < b._Id;
        });
    }));
    jeeves_assert!( ctx, result.is_err());
    jeeves_assert_eq!( ctx, drops.get(), 0);
    let  	mut seen    = 0u32;
    values.Traverse( |value| seen |= 1 << value._Id);
    jeeves_assert_eq!( ctx, seen, 0b11111);
    values.MutArr().QSort( |a, b| a._Id < b._Id);
    values.Arr().USeg().Traverse( |i| jeeves_assert_eq!( ctx, values[i]._Id, i));
    values.MutArr().QSort( |a, b| a._Id < b._Id);
    values.Arr().USeg().Traverse( |i| jeeves_assert_eq!( ctx, values[i]._Id, i));
    values.MutArr().QSort( |a, b| a._Id > b._Id);
    values.Arr().USeg().Traverse( |i| jeeves_assert_eq!( ctx, values[i]._Id, 4 - i));
    drop( values);
    jeeves_assert_eq!( ctx, drops.get(), 5);
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
    let  	arr = buff.Arr();
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
        b.Arr().Data()
    );
    jeeves_assert_eq!( ctx, b.Cap(), 8);
    let  	arr = b.Arr();
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
    let  	arr = buff.Arr();
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
jeeves_test!( Silo, ArrCastBoundsAndAlignment, |ctx| {
    let  	values          = [0x11223344u32, 0x55667788];
    let  	view            = Arr::from( &values);
    let  	bytes           = IArrExt::CastArr( &view);
    let  	misaligned      = bytes.Slice( 1, 4);
    let  	alignment       = std::panic::catch_unwind( || {
        let  	_   = misaligned.CastArrFrom::< u32>();
    });
    jeeves_assert!( ctx, alignment.is_err());
    let  	short           = bytes.Slice( 0, 3);
    let  	remainder       = std::panic::catch_unwind( || {
        let  	_   = IArrExt::CastArrFrom::< u32>( &short);
    });
    jeeves_assert!( ctx, remainder.is_err());
    let  	oversized       = Arr::New( std::ptr::NonNull::< u32>::dangling().as_ptr(), u32::MAX);
    let  	overflow        = std::panic::catch_unwind( || {
        let  	_   = oversized.CastArr();
    });
    jeeves_assert!( ctx, overflow.is_err());
    let  	overflow        = std::panic::catch_unwind( || {
        let  	_   = oversized.CastArrFrom::< u32>();
    });
    jeeves_assert!( ctx, overflow.is_err());
    let  	emptyBytes      = Arr::< u8>::Empty();
    let  	empty           = emptyBytes.CastArrFrom::< u32>();
    jeeves_assert!( ctx, empty.IsEmpty());
    jeeves_assert!( ctx, empty.Data().is_aligned());
    let  	mut mutableValues   = values;
    let  	mut mutable         = MutArr::from( &mut mutableValues);
    let  	mut mutableBytes    = IMutArrExt::CastMutArr::< u8>( &mut mutable);
    let  	alignment           = std::panic::catch_unwind( std::panic::AssertUnwindSafe( || {
        let  	mut misaligned      = mutableBytes.Slice( 1, 4);
        let  	_                   = misaligned.CastMutArr::< u32>();
    }));
    jeeves_assert!( ctx, alignment.is_err());
    let  	mut oversized   = MutArr::New( std::ptr::NonNull::< u32>::dangling().as_ptr(), u32::MAX);
    let  	overflow        = std::panic::catch_unwind( std::panic::AssertUnwindSafe( || {
        let  	_   = oversized.CastMutArr::< u32>();
    }));
    jeeves_assert!( ctx, overflow.is_err());
    let  	mut emptyBytes      = MutArr::< u8>::Empty();
    let  	empty               = emptyBytes.CastMutArr::< u32>();
    jeeves_assert!( ctx, empty.IsEmpty());
    jeeves_assert!( ctx, empty.Data().is_aligned());
});
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
    use	std::sync::atomic::{ AtomicUsize, Ordering };
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
    use	std::panic;
    use	std::sync::atomic::{ AtomicUsize, Ordering };
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
