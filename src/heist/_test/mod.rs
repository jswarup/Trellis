use crate::{jeeves_assert, jeeves_assert_eq, jeeves_assert_ne, jeeves_println, jeeves_test};
// mod.rs ---------------------------------------------------------------------------------------------------------
use crate::heist::atelier::Atelier;
use crate::heist::choretree::Chore;
use crate::stalks::work::WorkPtr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

//-------------------------------------------------------------------------------------------------

// MaestroOps Tests
jeeves_test!( Heist, MaestroOps, |ctx| {
    let  atelier = Atelier::Reset( 4);
    let  maestros = atelier.Maestros();
    maestros[2].SetCurSuccId( 42);
    jeeves_assert_eq!( ctx, maestros[2].MaestroIndex(), 2);
    jeeves_assert_eq!( ctx, maestros[2].CurSuccId(), 42);
    // Test local run queue push and pop
    maestros[1].EnqueRunJob( 123);
    let  popped_id = maestros[1].PopJob();
    jeeves_assert_eq!( ctx, popped_id, 123);
    jeeves_assert_eq!( ctx, maestros[1].PopJob(), 0);
});

//-------------------------------------------------------------------------------------------------

// AtelierLaunch Tests
jeeves_test!( Heist, AtelierLaunchImmediate, |ctx| {
    // Immediate mode (0 threads)
    let  atelier = Atelier::Reset( 0);
    jeeves_assert!( ctx, atelier.IsImmediate());
    jeeves_assert_eq!( ctx, atelier.SzThreads(), 0);
    let  main_maestro = atelier.MainMaestro();
    let  executed = Arc::new( AtomicBool::new( false));
    let  exec_clone = executed.clone();
    main_maestro.Post( move |_w| {
        exec_clone.store( true, Ordering::SeqCst);
    });
    jeeves_assert!( ctx, executed.load( Ordering::SeqCst));
    atelier.DoLaunch();                                                // Safe no-op
});
jeeves_test!( Heist, AtelierLaunchQueued, |ctx| {
    for sz_threads in [1u32, 4u32]
    {
        let  count = Arc::new( AtomicI32::new( 0));
        let  atelier = Atelier::Reset( sz_threads);
        jeeves_assert!( ctx, !atelier.IsImmediate());
        jeeves_assert_eq!( ctx, atelier.SzThreads(), sz_threads);
        let  main_maestro = atelier.MainMaestro();
        let  count_clone = count.clone();
        let  job_id = main_maestro.ConstructJob(
            0,
            WorkPtr::FromClosure( move |w| {
                count_clone.fetch_add( 1, Ordering::SeqCst);
                let  count_child = count_clone.clone();
                let  child = w.WorkerIndex();                          // worker index
                let  _ = child;
                // Post child job to worker
                w.PostJob( WorkPtr::FromClosure( move |_| {
                    count_child.fetch_add( 10, Ordering::SeqCst);
                }));
            }),
        );
        main_maestro.EnqueueJob( job_id);
        atelier.DoLaunch();
        jeeves_assert_eq!( ctx, count.load( Ordering::SeqCst), 11);
    }
});

//-------------------------------------------------------------------------------------------------

// AtelierResetAfterLaunch Tests
jeeves_test!( Heist, AtelierResetAfterLaunch, |ctx| {
    for _ in 0..4
    {
        let  completed = Arc::new( AtomicU32::new( 0));
        let  atelier = Atelier::Reset( 4);
        let  main_maestro = atelier.MainMaestro();
        let  comp_clone = completed.clone();
        let  job_id = main_maestro.ConstructJob(
            0,
            WorkPtr::FromClosure( move |_w| {
                comp_clone.fetch_add( 1, Ordering::Relaxed);
            }),
        );
        main_maestro.EnqueueJob( job_id);
        atelier.DoLaunch();
        jeeves_assert_eq!( ctx, completed.load( Ordering::SeqCst), 1);
        Atelier::Reset( 0);
        jeeves_assert!( ctx, Atelier::Instance().IsImmediate());
    }
});

//-------------------------------------------------------------------------------------------------

// ChoreTreeDAG Tests
jeeves_test!( Heist, ChoreTreeDAG, |ctx| {
    let  trace_idx = Arc::new( AtomicI32::new( 0));
    let  a_done = Arc::new( AtomicBool::new( false));
    let  c_done = Arc::new( AtomicBool::new( false));
    let  seq_order_ok = Arc::new( AtomicBool::new( true));
    let  t_a = trace_idx.clone();
    let  a_d = a_done.clone();
    let  a = Chore::FromClosure( "A", move |_w| {
        t_a.fetch_add( 1, Ordering::SeqCst);
        a_d.store( true, Ordering::Release);
    });
    let  t_b = trace_idx.clone();
    let  a_check = a_done.clone();
    let  seq_b = seq_order_ok.clone();
    let  b = Chore::FromClosure( "B", move |_w| {
        if !a_check.load( Ordering::Acquire)
        {
            seq_b.store( false, Ordering::Relaxed);
        }
        t_b.fetch_add( 2, Ordering::SeqCst);
    });
    let  t_c = trace_idx.clone();
    let  c_d = c_done.clone();
    let  c = Chore::FromClosure( "C", move |_w| {
        t_c.fetch_add( 4, Ordering::SeqCst);
        c_d.store( true, Ordering::Release);
    });
    let  t_d = trace_idx.clone();
    let  c_check = c_done.clone();
    let  seq_d = seq_order_ok.clone();
    let  d = Chore::FromClosure( "D", move |_w| {
        if !c_check.load( Ordering::Acquire)
        {
            seq_d.store( false, Ordering::Relaxed);
        }
        t_d.fetch_add( 8, Ordering::SeqCst);
    });
    let  t_e = trace_idx.clone();
    let  e = Chore::FromClosure( "E", move |_w| {
        t_e.fetch_add( 10, Ordering::SeqCst);
    });
    // ChoreTree DAG: (a >> b) | (c >> d) | e
    let  chore_tree = ( a >> b) | ( c >> d) | e;
    let  atelier = Atelier::Reset( 4);
    let  main_maestro = atelier.MainMaestro();
    main_maestro.PostChoreTree( &chore_tree);
    atelier.DoLaunch();
    jeeves_assert_eq!( ctx, trace_idx.load( Ordering::SeqCst), 25);
    jeeves_assert!( ctx, seq_order_ok.load( Ordering::SeqCst));
});

//-------------------------------------------------------------------------------------------------

// WorkStealing Tests
jeeves_test!( Heist, WorkStealing, |ctx| {
    const JOB_COUNT: u32 = 256;
    let  completed_count = Arc::new( AtomicU32::new( 0));
    let  atelier = Atelier::Reset( 4);
    let  main_maestro = atelier.MainMaestro();
    for _ in 0..JOB_COUNT
    {
        let  comp_clone = completed_count.clone();
        let  job_id = main_maestro.ConstructJob(
            0,
            WorkPtr::FromClosure( move |_w| {
                std::thread::yield_now();
                comp_clone.fetch_add( 1, Ordering::Relaxed);
            }),
        );
        main_maestro.EnqueueJob( job_id);
    }
    atelier.DoLaunch();
    jeeves_assert_eq!( ctx, completed_count.load( Ordering::SeqCst), JOB_COUNT);
    let  maestros = atelier.Maestros();
    let  mut stolen_jobs = 0;
    for m in maestros.iter().skip( 1)
    {
        stolen_jobs += m._SzProcessed.load( Ordering::Relaxed);
    }
    let  total_processed = stolen_jobs + maestros[0]._SzProcessed.load( Ordering::Relaxed);
    jeeves_assert!( ctx, total_processed >= JOB_COUNT);
});

//-------------------------------------------------------------------------------------------------

// Console & Example Tests
jeeves_test!( Heist, HeistConsoleReport, Console, |ctx| {
    let  atelier = Atelier::Reset( 4);
    let  maestros = atelier.Maestros();
    jeeves_println!(
        ctx,
        "         [Heist] Booted pool with {} Maestros",
        maestros.len()
    );
    for ( i, m) in maestros.iter().enumerate()
    {
        jeeves_println!(
            ctx,
            "           Thread {}: processed={}",
            i,
            m._SzProcessed.load( Ordering::Relaxed)
        );
    }
    jeeves_assert_eq!( ctx, maestros.len(), 4);
});
jeeves_test!( Heist, HeistDAGExecutionExample, Example, |ctx| {
    let  flag = Arc::new( AtomicBool::new( false));
    let  flag_clone = flag.clone();
    let  a = Chore::FromClosure( "Step1", move |_w| {});
    let  b = Chore::FromClosure( "Step2", move |_w| {
        flag_clone.store( true, Ordering::SeqCst);
    });
    let  tree = a >> b;
    let  atelier = Atelier::Reset( 2);
    atelier.MainMaestro().PostChoreTree( &tree);
    atelier.DoLaunch();
    jeeves_println!(
        ctx,
        "         [Example] Heist DAG sequential chore tree executed successfully"
    );
    jeeves_assert!( ctx, flag.load( Ordering::SeqCst));
});
