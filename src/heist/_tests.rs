use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test };
// mod.rs ---------------------------------------------------------------------------------------------------------
use	crate::heist::atelier::Atelier;
use	crate::heist::choretree::Chore;
use	crate::silo::USeg;
use	crate::stalks::work::WorkPtr;
use	std::sync::Arc;
use	std::sync::Barrier;
use	std::sync::atomic::{ AtomicBool, AtomicI32, AtomicU32, Ordering };

//-------------------------------------------------------------------------------------------------
// MaestroOps Tests
jeeves_test!( Heist, MaestroOps, |ctx| {
    let  	atelier = Atelier::Reset( 4);
    let  	maestros = atelier.Maestros();
    maestros[2].SetCurSuccId( 42);
    jeeves_assert_eq!( ctx, maestros[2].MaestroIndex(), 2);
    jeeves_assert_eq!( ctx, maestros[2].CurSuccId(), 42);
    // Test local run queue push and pop
    maestros[1].EnqueRunJob( 123);
    let  	popped_id = maestros[1].PopJob();
    jeeves_assert_eq!( ctx, popped_id, 123);
    jeeves_assert_eq!( ctx, maestros[1].PopJob(), 0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, ReusedJobClearsSuccessor, |ctx| {
    let  	atelier = Atelier::Reset( 1);
    let  	state = &atelier.state;
    let  	first = state.ConstructJob( 0, 7, WorkPtr::FromClosure( |_| {}));
    state.FreeJob( 0, first);
    let  	reused = state.ConstructJob( 0, 0, WorkPtr::FromClosure( |_| {}));
    jeeves_assert_eq!( ctx, reused, first);
    jeeves_assert_eq!( ctx, state._SuccIds[reused as u32].load( Ordering::SeqCst), 0);
});

//-------------------------------------------------------------------------------------------------
// AtelierLaunch Tests
jeeves_test!( Heist, AtelierLaunchImmediate, |ctx| {
    // Immediate mode (0 threads)
    let  	atelier = Atelier::Reset( 0);
    jeeves_assert!( ctx, atelier.IsImmediate());
    jeeves_assert_eq!( ctx, atelier.SzThreads(), 0);
    let  	main_maestro = atelier.MainMaestro();
    let  	executed = Arc::new( AtomicBool::new( false));
    let  	exec_clone = executed.clone();
    main_maestro.Post( move |_w| {
        exec_clone.store( true, Ordering::SeqCst);
    });
    jeeves_assert!( ctx, executed.load( Ordering::SeqCst));
    atelier.DoLaunch();                                                // Safe no-op
});
jeeves_test!( Heist, AtelierLaunchQueued, |ctx| {
    for sz_threads in [1u32, 4u32] {
        let  	count = Arc::new( AtomicI32::new( 0));
        let  	atelier = Atelier::Reset( sz_threads);
        jeeves_assert!( ctx, !atelier.IsImmediate());
        jeeves_assert_eq!( ctx, atelier.SzThreads(), sz_threads);
        let  	main_maestro = atelier.MainMaestro();
        let  	count_clone = count.clone();
        let  	job_id = main_maestro.ConstructJob( 
            0,
            WorkPtr::FromClosure( move |w| {
                count_clone.fetch_add( 1, Ordering::SeqCst);
                let  	count_child = count_clone.clone();
                let  	child = w.WorkerIndex();                       // worker index
                let  	_ = child;
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

jeeves_test!( Heist, IndependentAteliersLaunchConcurrently, |ctx| {
    let  	first = Arc::new( Atelier::New( 1));
    let  	second = Arc::new( Atelier::New( 1));
    let  	active = Arc::new( AtomicU32::new( 0));
    let  	peak = Arc::new( AtomicU32::new( 0));
    for atelier in [&first, &second] {
        let  	active = active.clone();
        let  	peak = peak.clone();
        atelier.MainMaestro().Post( move |_| {
            let  	now = active.fetch_add( 1, Ordering::SeqCst) + 1;
            peak.fetch_max( now, Ordering::SeqCst);
            std::thread::sleep( std::time::Duration::from_millis( 20));
            active.fetch_sub( 1, Ordering::SeqCst);
        });
    }
    let  	start = Arc::new( Barrier::new( 3));
    std::thread::scope( |scope| {
        let  	first_start = start.clone();
        let  	first_atelier = first.clone();
        scope.spawn( move || {
            first_start.wait();
            first_atelier.DoLaunch();
        });
        let  	second_start = start.clone();
        let  	second_atelier = second.clone();
        scope.spawn( move || {
            second_start.wait();
            second_atelier.DoLaunch();
        });
        start.wait();
    });
    jeeves_assert_eq!( ctx, peak.load( Ordering::SeqCst), 2);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, ScopedRangeBorrowsCallerDataAcrossWorkers, |ctx| {
    let  	atelier = Atelier::New( 3);
    let  	values = std::array::from_fn::< _, 96, _>( |idx| idx as u32 + 1);
    let  	sum = AtomicU32::new( 0);
    let  	workers = AtomicU32::new( 0);
    atelier.ForEachScopedRange( values.len() as u32, |worker, start, end| {
        let  	mut partial = 0u32;
        for index in start..end {
            partial += values[index as usize];
        }
        workers.fetch_or( 1 << worker, Ordering::SeqCst);
        sum.fetch_add( partial, Ordering::SeqCst);
    });
    jeeves_assert_eq!( ctx, sum.load( Ordering::SeqCst), 4656);
    jeeves_assert_eq!( ctx, workers.load( Ordering::SeqCst), 0b111);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, ScopedMutTransfersDisjointCallerDataAcrossWorkers, |ctx| {
    let  	atelier = Atelier::New( 3);
    let  	mut values = [0u32; 96];
    let  	workers = AtomicU32::new( 0);
    atelier.ForEachScopedMut( ( &mut values).into(), |worker, mut partition| {
        workers.fetch_or( 1 << worker, Ordering::SeqCst);
        partition.USeg().Traverse( |index| {
            *partition.GetMut( index).unwrap() = worker + 1;
        });
    });
    jeeves_assert_eq!( ctx, workers.load( Ordering::SeqCst), 0b111);
    USeg::FromLen( values.len() as u32).Traverse( |index| {
        jeeves_assert!( ctx, values[index as usize] > 0);
    });
});

//-------------------------------------------------------------------------------------------------
// AtelierResetAfterLaunch Tests
jeeves_test!( Heist, AtelierResetAfterLaunch, |ctx| {
    for _ in 0..4 {
        let  	completed = Arc::new( AtomicU32::new( 0));
        let  	atelier = Atelier::Reset( 4);
        let  	main_maestro = atelier.MainMaestro();
        let  	comp_clone = completed.clone();
        let  	job_id = main_maestro.ConstructJob( 
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
    let  	trace_idx = Arc::new( AtomicI32::new( 0));
    let  	a_done = Arc::new( AtomicBool::new( false));
    let  	c_done = Arc::new( AtomicBool::new( false));
    let  	seq_order_ok = Arc::new( AtomicBool::new( true));
    let  	t_a = trace_idx.clone();
    let  	a_d = a_done.clone();
    let  	a = Chore::FromClosure( "A", move |_w| {
        t_a.fetch_add( 1, Ordering::SeqCst);
        a_d.store( true, Ordering::Release);
    });
    let  	t_b = trace_idx.clone();
    let  	a_check = a_done.clone();
    let  	seq_b = seq_order_ok.clone();
    let  	b = Chore::FromClosure( "B", move |_w| {
        if !a_check.load( Ordering::Acquire) {
            seq_b.store( false, Ordering::Relaxed);
        }
        t_b.fetch_add( 2, Ordering::SeqCst);
    });
    let  	t_c = trace_idx.clone();
    let  	c_d = c_done.clone();
    let  	c = Chore::FromClosure( "C", move |_w| {
        t_c.fetch_add( 4, Ordering::SeqCst);
        c_d.store( true, Ordering::Release);
    });
    let  	t_d = trace_idx.clone();
    let  	c_check = c_done.clone();
    let  	seq_d = seq_order_ok.clone();
    let  	d = Chore::FromClosure( "D", move |_w| {
        if !c_check.load( Ordering::Acquire) {
            seq_d.store( false, Ordering::Relaxed);
        }
        t_d.fetch_add( 8, Ordering::SeqCst);
    });
    let  	t_e = trace_idx.clone();
    let  	e = Chore::FromClosure( "E", move |_w| {
        t_e.fetch_add( 10, Ordering::SeqCst);
    });
    // ChoreTree DAG: (a >> b) | (c >> d) | e
    let  	chore_tree = ( a >> b) | ( c >> d) | e;
    let  	atelier = Atelier::Reset( 4);
    let  	main_maestro = atelier.MainMaestro();
    main_maestro.PostChoreTree( &chore_tree);
    atelier.DoLaunch();
    jeeves_assert_eq!( ctx, trace_idx.load( Ordering::SeqCst), 25);
    jeeves_assert!( ctx, seq_order_ok.load( Ordering::SeqCst));
});

//-------------------------------------------------------------------------------------------------
// WorkStealing Tests
jeeves_test!( Heist, WorkStealing, |ctx| {
    const JOB_COUNT: u32 = 256;
    let  	completed_count = Arc::new( AtomicU32::new( 0));
    let  	atelier = Atelier::Reset( 3);
    let  	main_maestro = atelier.MainMaestro();
    for _ in 0..JOB_COUNT {
        let  	comp_clone = completed_count.clone();
        let  	job_id = main_maestro.ConstructJob( 
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
    let  	maestros = atelier.Maestros();
    let  	mut stolen_jobs = 0;
    for i in 1..maestros.Len() {
        stolen_jobs += maestros[i]._SzProcessed.load( Ordering::Relaxed);
    }
    let  	total_processed = stolen_jobs + maestros[0]._SzProcessed.load( Ordering::Relaxed);
    jeeves_assert_eq!( ctx, maestros.Len(), 3);
    jeeves_assert!( ctx, stolen_jobs > 0);
    jeeves_assert!( ctx, total_processed >= JOB_COUNT);
});

//-------------------------------------------------------------------------------------------------
// Console & Example Tests
jeeves_test!( Heist, HeistConsoleReport, Console, |ctx| {
    let  	atelier = Atelier::Reset( 4);
    let  	maestros = atelier.Maestros();
    jeeves_println!( 
        ctx,
        "         [Heist] Booted pool with {} Maestros",
        maestros.Len()
    );
    for i in 0..maestros.Len() {
        jeeves_println!( 
            ctx,
            "           Thread {}: processed={}",
            i,
            maestros[i]._SzProcessed.load( Ordering::Relaxed)
        );
    }
    jeeves_assert_eq!( ctx, maestros.Len(), 4);
});
jeeves_test!( Heist, HeistDAGExecutionExample, Example, |ctx| {
    let  	flag = Arc::new( AtomicBool::new( false));
    let  	flag_clone = flag.clone();
    let  	a = Chore::FromClosure( "Step1", move |_w| {});
    let  	b = Chore::FromClosure( "Step2", move |_w| {
        flag_clone.store( true, Ordering::SeqCst);
    });
    let  	tree = a >> b;
    let  	atelier = Atelier::Reset( 2);
    atelier.MainMaestro().PostChoreTree( &tree);
    atelier.DoLaunch();
    jeeves_println!( 
        ctx,
        "         [Example] Heist DAG sequential chore tree executed successfully"
    );
    jeeves_assert!( ctx, flag.load( Ordering::SeqCst));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, SpawnQuellCpuBasic, |ctx| {
    let  	mut buff = crate::silo::buff::Buff::FromDispenser( 10000, |_| 1u32);
    let  	spawn_quell = crate::CpuSpawnQuell!( 
        buff.MutArr(),
        |mut chunk, _w| {
            for i in 0..chunk.Size() {
                let  	val = chunk.Arr()[i];
                chunk.GetMut( i).unwrap().clone_from( &( val * 2));
            }
        },
        |_all, _w| {}
    );
    let  	atelier = Atelier::Reset( 4);
    atelier.MainMaestro().PostChoreTree( &spawn_quell.into());
    atelier.DoLaunch();
    let  	mut total_sum = 0u32;
    for i in 0..buff.Len() {
        total_sum += buff[i];
    }
    jeeves_assert_eq!( ctx, total_sum, 20000);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, HeistTryAllocJobExhaustion, |ctx| {
    let  	atelier = Atelier::Reset( 1);
    let  	state = &atelier.state;
    // Drain free stash and maestro 0 cache
    let  	mut drained = Vec::new();
    while let  	Some( id) = state.TryAllocJob( 0) {
        drained.push( id);
    }
    // Now pool is exhausted, TryAllocJob must return None without hanging
    jeeves_assert_eq!( ctx, state.TryAllocJob( 0), None);
    jeeves_assert_eq!( ctx, atelier.TryAllocJob( 0), None);
    // Return one job and verify it can be reallocated
    if let  	Some( returned_id) = drained.pop() {
        state.FreeJob( 0, returned_id);
        let  	reallocated = state.TryAllocJob( 0);
        jeeves_assert_eq!( ctx, reallocated, Some( returned_id));
        drained.push( returned_id);
    }
    // Return all drained jobs so atelier state is clean
    for id in drained {
        state.FreeJob( 0, id);
    }
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, HeistStealMetricsRecorded, |ctx| {
    let  	atelier = Atelier::Reset( 4);
    let  	main_maestro = atelier.MainMaestro();
    // Post enough jobs to trigger work stealing among the 4 threads
    for _ in 0..128 {
        let  	job_id = main_maestro.ConstructJob( 
            0,
            WorkPtr::FromClosure( |_| {
                std::thread::yield_now();
            }),
        );
        main_maestro.EnqueueJob( job_id);
    }
    atelier.DoLaunch();
    // Verify steal attempts and successes are tracked
    let  	attempts = atelier.TotalStealAttempts();
    let  	successes = atelier.TotalStealSuccesses();
    jeeves_assert!( ctx, attempts > 0);
    jeeves_assert!( ctx, successes <= attempts);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, HeistSlotRecyclingClearsFields, |ctx| {
    let  	atelier = Atelier::Reset( 1);
    let  	state = &atelier.state;
    let  	job_id = state.ConstructJob( 0, 42, WorkPtr::FromClosure( |_| {}));
    jeeves_assert_eq!( 
        ctx,
        state._SuccIds[job_id as u32].load( Ordering::SeqCst),
        42
    );
    // Free the job: FreeJob must immediately reset _SuccIds, _SzPreds, and _JobBuff
    state.FreeJob( 0, job_id);
    jeeves_assert_eq!( ctx, state._SuccIds[job_id as u32].load( Ordering::SeqCst), 0);
    jeeves_assert_eq!( ctx, state._SzPreds[job_id as u32].load( Ordering::SeqCst), 0);
});
