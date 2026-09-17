use crate::{jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test};
// mod.rs ---------------------------------------------------------------------------------------------------------
use crate::silo::arr::{Arr, MutArr};
use crate::silo::buff::Buff;
use crate::silo::dset::DisjointSet;
use crate::silo::fifo::Fifo;
use crate::silo::seg::{Seg, USeg};
use crate::silo::stash::Stash;
use crate::silo::stk::Stk;
use std::sync::atomic::AtomicU32;

//-------------------------------------------------------------------------------------------------

// Seg Tests
jeeves_test!( Silo, SegBasic, |ctx| {
    let  s = Seg::WithLen( 10, 5);
    jeeves_assert_eq!( ctx, s.First(), 10);
    jeeves_assert_eq!( ctx, s.Last(), 14);
    jeeves_assert_eq!( ctx, s.Len(), 5);
    jeeves_assert_eq!( ctx, s.Begin(), 10);
    jeeves_assert_eq!( ctx, s.End(), 15);
    jeeves_assert_eq!( ctx, s.Mid(), 12);
    jeeves_assert!( ctx, s.Contains( 12));
    jeeves_assert!( ctx, !s.Contains( 15));
});
jeeves_test!( Silo, SegOverlap, |ctx| {
    let  a = Seg::New( 0, 10);
    let  b = Seg::New( 5, 15);
    let  c = Seg::New( 11, 20);
    jeeves_assert!( ctx, a.Overlaps( &b));
    jeeves_assert!( ctx, !a.Overlaps( &c));
    let  inter = a.Intersect( &b);
    jeeves_assert_eq!( ctx, inter.First(), 5);
    jeeves_assert_eq!( ctx, inter.Last(), 10);
});
jeeves_test!( Silo, SegSnip, |ctx| {
    let  s = Seg::WithLen( 10, 10);                                    // [10, 19]
    let  left_snipped = s.LSnip( 3);                                   // [13, 19]
    jeeves_assert_eq!( ctx, left_snipped.First(), 13);
    jeeves_assert_eq!( ctx, left_snipped.Len(), 7);
    let  right_snipped = s.RSnip( 4);                                  // [10, 15]
    jeeves_assert_eq!( ctx, right_snipped.First(), 10);
    jeeves_assert_eq!( ctx, right_snipped.Last(), 15);
    jeeves_assert_eq!( ctx, right_snipped.Len(), 6);
});
jeeves_test!( Silo, SegTraverseSpan, |ctx| {
    let  s = Seg::New( 1, 5);
    let  mut sum = 0;
    s.Traverse( |val| {
        sum += val;
    });
    jeeves_assert_eq!( ctx, sum, 15);
    let  all_positive = s.Span( |val| val > 0);
    jeeves_assert!( ctx, all_positive);
});

//-------------------------------------------------------------------------------------------------

// Buff Tests (Fixed Capacity, 16 bytes)
jeeves_test!( Silo, BuffBasic, |ctx| {
    let  data = [100, 200, 300];
    let  b = Buff::FromSlice( &data);
    jeeves_assert!( ctx, !b.IsEmpty());
    jeeves_assert_eq!( ctx, b.Cap(), 3);
    jeeves_assert_eq!( ctx, b.Size(), 3);
    jeeves_assert_eq!( ctx, b[0], 100);
    jeeves_assert_eq!( ctx, b[1], 200);
    jeeves_assert_eq!( ctx, b[2], 300);
    // Buff produces an Arr view into Buff[0, _Cap]
    let  arr = b.AsArr();
    jeeves_assert_eq!( ctx, arr.Size(), 3);
    jeeves_assert_eq!( ctx, arr[1], 200);
});
jeeves_test!( Silo, BuffFromDispenser, |ctx| {
    let  b = Buff::FromDispenser( 5, |i| ( i + 1) * 10);
    jeeves_assert_eq!( ctx, b.Cap(), 5);
    jeeves_assert_eq!( ctx, b[0], 10);
    jeeves_assert_eq!( ctx, b[4], 50);
    let  mut sum = 0;
    b.Traverse( |&val| {
        sum += val;
    });
    jeeves_assert_eq!( ctx, sum, 150);
});

//-------------------------------------------------------------------------------------------------

// Arr & MutArr Tests (Non-owning borrowed views)
jeeves_test!( Silo, ArrSlicingAndSnipping, |ctx| {
    let  data = [1, 2, 3, 4, 5, 6];
    let  arr = Arr::FromSlice( &data);
    jeeves_assert_eq!( ctx, arr.Size(), 6);
    jeeves_assert_eq!( ctx, arr.First(), Some( &1));
    jeeves_assert_eq!( ctx, arr.Last(), Some( &6));
    let  left = arr.LSnip( 2);                                         // items 2..6 => [3, 4, 5, 6]
    jeeves_assert_eq!( ctx, left.Size(), 4);
    jeeves_assert_eq!( ctx, left[0], 3);
    let  right = arr.RSnip( 2);                                        // items 0..4 => [1, 2, 3, 4]
    jeeves_assert_eq!( ctx, right.Size(), 4);
    jeeves_assert_eq!( ctx, right[3], 4);
    let  mid = arr.Slice( 2, 3);                                       // items 2..5 => [3, 4, 5]
    jeeves_assert_eq!( ctx, mid.Size(), 3);
    jeeves_assert_eq!( ctx, mid[0], 3);
    jeeves_assert_eq!( ctx, mid[2], 5);
});
jeeves_test!( Silo, MutArrMutations, |ctx| {
    let  mut data = [10, 20, 30, 40];
    let  mut mut_arr = MutArr::FromMutSlice( &mut data);
    mut_arr.Swap( 0, 3);
    jeeves_assert_eq!( ctx, mut_arr[0], 40);
    jeeves_assert_eq!( ctx, mut_arr[3], 10);
    mut_arr.SetAt( 1, 99);
    jeeves_assert_eq!( ctx, mut_arr[1], 99);
});

//-------------------------------------------------------------------------------------------------

// Stash Tests (Dynamic filling & Buff extraction)
jeeves_test!( Silo, StashDynamicAndExtractBuff, |ctx| {
    let  mut stash = Stash::<i32>::New();
    jeeves_assert!( ctx, stash.IsEmpty());
    USeg::FromLen( 10).Traverse( |i| {
        stash.Push( ( i as i32 + 1) * 5);
    });
    jeeves_assert_eq!( ctx, stash.Size(), 10);
    jeeves_assert!( ctx, stash.Capacity() >= 10);
    jeeves_assert_eq!( ctx, stash[0], 5);
    jeeves_assert_eq!( ctx, stash[9], 50);
    // Extract built Buff from Stash (shrinkToFit = true)
    let  buff = stash.ExtractBuff( true);
    jeeves_assert_eq!( ctx, buff.Cap(), 10);
    jeeves_assert_eq!( ctx, buff[0], 5);
    jeeves_assert_eq!( ctx, buff[9], 50);
    // Buff view into Buff[0, _Cap]
    let  arr = buff.AsArr();
    jeeves_assert_eq!( ctx, arr.Size(), 10);
});
jeeves_test!( Silo, StashPopBack, |ctx| {
    let  mut stash = Stash::<i32>::WithCapacity( 4);
    stash.Push( 11);
    stash.Push( 22);
    stash.Push( 33);
    jeeves_assert_eq!( ctx, stash.Size(), 3);
    let  popped = stash.Pop();
    jeeves_assert_eq!( ctx, popped, Some( 33));
    jeeves_assert_eq!( ctx, stash.Size(), 2);
});

//-------------------------------------------------------------------------------------------------

// Stk Tests (Atomic Stack View)
jeeves_test!( Silo, StkAtomicStackOps, |ctx| {
    let  mut storage = [0i32; 8];
    let  size_atomic = AtomicU32::new( 0);
    let  mut_arr = MutArr::FromMutSlice( &mut storage);
    let  stk = Stk::Create( &size_atomic, mut_arr);
    jeeves_assert_eq!( ctx, stk.Size(), 0);
    jeeves_assert_eq!( ctx, stk.Capacity(), 8);
    jeeves_assert_eq!( ctx, stk.SzVoid(), 8);
    jeeves_assert!( ctx, stk.Push( 101));
    jeeves_assert!( ctx, stk.Push( 202));
    jeeves_assert_eq!( ctx, stk.Size(), 2);
    jeeves_assert_eq!( ctx, stk.SzVoid(), 6);
    let  mut out = 0;
    jeeves_assert!( ctx, stk.Pop( &mut out));
    jeeves_assert_eq!( ctx, out, 202);
    jeeves_assert_eq!( ctx, stk.Size(), 1);
});

//-------------------------------------------------------------------------------------------------

// DisjointSet Tests (Backed by Stash)
jeeves_test!( Silo, DsetUnionFind, |ctx| {
    let  mut dset = DisjointSet::WithCapacity( 10);
    jeeves_assert_eq!( ctx, dset.Size(), 10);
    let  not_same = !dset.Same( 1, 2);
    jeeves_assert!( ctx, not_same);
    dset.Union( 1, 2);
    dset.Union( 2, 3);
    dset.Union( 4, 5);
    let  same_1_3 = dset.Same( 1, 3);
    let  same_4_5 = dset.Same( 4, 5);
    let  not_same_1_5 = !dset.Same( 1, 5);
    jeeves_assert!( ctx, same_1_3);
    jeeves_assert!( ctx, same_4_5);
    jeeves_assert!( ctx, not_same_1_5);
    dset.Union( 3, 4);
    let  same_1_5 = dset.Same( 1, 5);
    jeeves_assert!( ctx, same_1_5);
});
jeeves_test!( Silo, DsetGrow, |ctx| {
    let  mut dset = DisjointSet::WithCapacity( 4);
    dset.Grow( 4);
    jeeves_assert_eq!( ctx, dset.Size(), 8);
    dset.Union( 3, 7);
    let  same_3_7 = dset.Same( 3, 7);
    jeeves_assert!( ctx, same_3_7);
});

//-------------------------------------------------------------------------------------------------

// Fifo Tests
jeeves_test!( Silo, FifoOperations, |ctx| {
    let  mut q = Fifo::<i32, 4>::New();
    jeeves_assert!( ctx, q.IsEmpty());
    let  not_full = !q.IsFull();
    jeeves_assert!( ctx, not_full);
    let  p1 = q.PushBack( 10);
    let  p2 = q.PushBack( 20);
    let  p3 = q.PushBack( 30);
    let  p4 = q.PushBack( 40);
    jeeves_assert!( ctx, p1);
    jeeves_assert!( ctx, p2);
    jeeves_assert!( ctx, p3);
    jeeves_assert!( ctx, p4);
    let  is_full = q.IsFull();
    jeeves_assert!( ctx, is_full);
    let  p5 = q.PushBack( 50);
    let  p5_failed = !p5;
    jeeves_assert!( ctx, p5_failed);                                   // Should fail, capacity 4
    jeeves_assert_eq!( ctx, q.Front(), Some( &10));
    jeeves_assert_eq!( ctx, q.PopFront(), Some( 10));
    jeeves_assert_eq!( ctx, q.PopFront(), Some( 20));
    // Circular wrap
    let  p6 = q.PushBack( 60);
    let  p7 = q.PushBack( 70);
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
    let  b = Buff::FromDispenser( 8, |i| ( i as u64) * 10);
    jeeves_println!(
        ctx,
        "         [Console] Buff: Cap={}, Ptr={:?}",
        b.Cap(),
        b._Ptr
    );
    jeeves_assert_eq!( ctx, b.Cap(), 8);
    let  arr = b.AsArr();
    jeeves_println!(
        ctx,
        "         [Console] Arr view into Buff[0, _Cap]: Size={}",
        arr.Size()
    );
    jeeves_assert_eq!( ctx, arr.Size(), 8);
});
jeeves_test!( Silo, StashBuildConsole, Console, |ctx| {
    let  mut stash = Stash::<u32>::WithCapacity( 4);
    USeg::FromLen( 6).Traverse( |i| stash.Push( i * 11));
    jeeves_println!(
        ctx,
        "         [Console] Stash: Size={}, Buff.Cap={}",
        stash.Size(),
        stash.Capacity()
    );
    jeeves_assert_eq!( ctx, stash.Size(), 6);
    let  buff = stash.ExtractBuff( true);
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
    let  mut stash = Stash::<u32>::New();
    USeg::New( 1, 5).Traverse( |i| {
        stash.Push( i * 7);
    });
    let  buff = stash.ExtractBuff( true);
    let  arr = buff.AsArr();
    arr.USeg().Traverse( |i| {
        jeeves_println!( ctx, "           arr[{}] = {}", i, arr[i]);
    });
    jeeves_assert_eq!( ctx, arr[4], 35);
});
jeeves_test!( Silo, StkUsageExample, Example, |ctx| {
    jeeves_println!( ctx, "         [Example] Stk lock-free atomic stack view:");
    let  mut data = [0u32; 4];
    let  size = AtomicU32::new( 0);
    let  stk = Stk::Create( &size, MutArr::FromMutSlice( &mut data));
    stk.Push( 77);
    stk.Push( 88);
    jeeves_println!( ctx, "           stk size = {}", stk.Size());
    let  mut val = 0;
    stk.Pop( &mut val);
    jeeves_println!( ctx, "           popped = {}", val);
    jeeves_assert_eq!( ctx, val, 88);
});
jeeves_test!( Silo, DsetUsageExample, Example, |ctx| {
    jeeves_println!(
        ctx,
        "         [Example] DisjointSet (Union-Find) clustering:"
    );
    let  mut d = DisjointSet::WithCapacity( 5);
    d.Union( 0, 2);
    d.Union( 2, 4);
    jeeves_println!( ctx, "           Same(0, 4) = {}", d.Same( 0, 4));
    let  same_0_4 = d.Same( 0, 4);
    jeeves_assert!( ctx, same_0_4);
});
jeeves_test!( Silo, FifoUsageExample, Example, |ctx| {
    jeeves_println!(
        ctx,
        "         [Example] Fifo circular ring buffer streaming:"
    );
    let  mut q = Fifo::<&'static str, 3>::New();
    q.PushBack( "alpha");
    q.PushBack( "beta");
    q.PushBack( "gamma");
    while let  Some( msg) = q.PopFront()
    {
        jeeves_println!( ctx, "           dequeued: {}", msg);
    }
    jeeves_assert!( ctx, q.IsEmpty());
});
