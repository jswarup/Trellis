use crate::{jeeves_assert, jeeves_assert_eq, jeeves_assert_ne, jeeves_println, jeeves_test};
// mod.rs ---------------------------------------------------------------------------------------------------------
use crate::stalks::work::{IWorker, Spinlock, WorkPtr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
struct MockWorker
{
    index: u32,
    executed: u32,
}
impl IWorker for MockWorker
{
    fn  PostJob( &mut self, mut job: WorkPtr)
    {
        job.DoWork( self);
        self.executed += 1;
    }
    fn  WorkerIndex( &self) -> u32
    {
        self.index
    }
}

//-------------------------------------------------------------------------------------------------

// Stalks Tests
jeeves_test!( Stalks, SpinlockMutualExclusion, |ctx| {
    let  lock = Arc::new( Spinlock::New());
    let  counter = Arc::new( AtomicU32::new( 0));
    let  lock_clone = lock.clone();
    let  counter_clone = counter.clone();
    let  handle = std::thread::spawn( move || {
        for _ in 0..1000
        {
            let  _guard = lock_clone.Lock();
            counter_clone.fetch_add( 1, Ordering::Relaxed);
        }
    });
    for _ in 0..1000
    {
        let  _guard = lock.Lock();
        counter.fetch_add( 1, Ordering::Relaxed);
    }
    handle.join().unwrap();
    jeeves_assert_eq!( ctx, counter.load( Ordering::SeqCst), 2000);
});
jeeves_test!( Stalks, WorkPtrExecution, |ctx| {
    let  mut worker = MockWorker {
        index: 0,
        executed: 0,
    };
    let  ran = Arc::new( AtomicU32::new( 0));
    let  ran_clone = ran.clone();
    let  job = WorkPtr::FromClosure( move |_w| {
        ran_clone.store( 42, Ordering::SeqCst);
    });
    worker.PostJob( job);
    jeeves_assert_eq!( ctx, ran.load( Ordering::SeqCst), 42);
    jeeves_assert_eq!( ctx, worker.executed, 1);
});
jeeves_test!( Stalks, WorkerSeedExample, Example, |ctx| {
    jeeves_println!( ctx, "         [Example] Stalks worker scaffold operational");
    let  mut worker = MockWorker {
        index: 1,
        executed: 0,
    };
    let  job = WorkPtr::FromFn( |w| {
        assert_eq!( w.WorkerIndex(), 1);
    });
    worker.PostJob( job);
    jeeves_assert_eq!( ctx, worker.executed, 1);
});
