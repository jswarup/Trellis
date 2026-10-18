use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test };
// mod.rs ---------------------------------------------------------------------------------------------------------
use	crate::heist::atelier::Atelier;
use	crate::heist::choretree::Chore;
use	crate::silo::USeg;
use	crate::stalks::work::WorkPtr;
use	std::sync::Arc;
use	std::sync::Barrier;
use	std::sync::atomic::{ AtomicBool, AtomicI32, AtomicU32, Ordering };

jeeves_test!( Heist, WorkerPanicCancelsJoinsAndPoisonsPool, |ctx| {
    use crate::heist::ChorePlacement;
    USeg::FromLen( 2).Traverse( |panicWorker| {
        let atelier = Atelier::New( 3);
        let started = Arc::new( AtomicBool::new( false));
        let finished = Arc::new( AtomicBool::new( false));
        let start = started.clone();
        let finish = finished.clone();
        atelier.MainMaestro().PostWithPlacement( ChorePlacement::Require( 2), move |_| {
            start.store( true, Ordering::Release);
            std::thread::sleep( std::time::Duration::from_millis( 20));
            finish.store( true, Ordering::Release);
        });
        atelier.MainMaestro().PostWithPlacement( ChorePlacement::Require( panicWorker), move |_| {
            while !started.load( Ordering::Acquire) { std::thread::yield_now(); }
            panic!( "Intentional worker failure");
        });
        let failed = std::panic::catch_unwind( std::panic::AssertUnwindSafe( || atelier.DoLaunch()));
        jeeves_assert!( ctx, failed.is_err());
        jeeves_assert!( ctx, finished.load( Ordering::Acquire));
        let retry = std::panic::catch_unwind( std::panic::AssertUnwindSafe( || atelier.DoLaunch()));
        jeeves_assert!( ctx, retry.is_err());
    });
});

jeeves_test!( Heist, RunnableAccountingPrecedesQueuePublication, |ctx| {
    use crate::heist::ChorePlacement;
    let atelier = Atelier::New( 1);
    let state = &atelier.state;
    let job = state.ConstructJob( 0, 0, WorkPtr::FromClosure( |_| {}));
    let queue = state._Maestros[0]._RunQueue.Lock();
    std::thread::scope( |scope| {
        let worker = scope.spawn( || state.EnqueueByPlacement( job, ChorePlacement::Any));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs( 2);
        while state._SzSchedRunnables.load( Ordering::Acquire) == 0 && std::time::Instant::now() < deadline {
            std::thread::yield_now();
        }
        jeeves_assert_eq!( ctx, state._SzSchedRunnables.load( Ordering::Acquire), 1);
        jeeves_assert_eq!( ctx, state._SzSchedJob.load( Ordering::Acquire), 1);
        drop( queue);
        worker.join().unwrap();
    });
    atelier.DoLaunch();
    jeeves_assert_eq!( ctx, state._SzSchedRunnables.load( Ordering::Acquire), 0);
    jeeves_assert_eq!( ctx, state._SzSchedJob.load( Ordering::Acquire), 0);
});

jeeves_test!( Heist, ConcurrentCacheRefillAndRecyclePreserveUniqueSlots, |ctx| {
    use crate::silo::Buff;
    let atelier = Atelier::New( 1);
    let state = &atelier.state;
    let used = Buff::FromDispenser( crate::heist::atelier::K_JOB_CAPACITY as u32, |_| AtomicBool::new( false));
    let valid = AtomicBool::new( true);
    std::thread::scope( |scope| {
        USeg::FromLen( 4).Traverse( |_| {
            let used = &used;
            let valid = &valid;
            scope.spawn( move || {
                USeg::FromLen( 8).Traverse( |_| {
                    let slots = Buff::FromDispenser( 320, |_| {
                        let slot = state.AllocJob( 0);
                        if slot == 0 || used[slot as u32].swap( true, Ordering::SeqCst) {
                            valid.store( false, Ordering::Relaxed);
                        }
                        slot
                    });
                    slots.USeg().Traverse( |index| {
                        let slot = slots[index];
                        used[slot as u32].store( false, Ordering::SeqCst);
                        state.FreeJob( 0, slot);
                    });
                });
            });
        });
    });
    jeeves_assert!( ctx, valid.load( Ordering::Relaxed));
});

jeeves_test!( Heist, StealScanWrapsWithoutOverflow, |ctx| {
    let atelier = Atelier::New( 3);
    let state = &atelier.state;
    let job = state.ConstructJob( 1, 0, WorkPtr::FromClosure( |_| {}));
    state._Maestros[1].EnqueRunJob( job);
    let mut seed = 0xE2E9_A15E_u32;
    jeeves_assert_eq!( ctx, seed.wrapping_mul( 2654435761).wrapping_add( 1), u32::MAX);
    jeeves_assert_eq!( ctx, state.GrabJob( 0, &mut seed), job);
    state.FreeJob( 0, job);
});

jeeves_test!( Heist, InfallibleConstructionRejectsExhaustion, |ctx| {
    let atelier = Atelier::New( 1);
    while atelier.state.TryAllocJob( 0).is_some() {}
    let result = std::panic::catch_unwind( std::panic::AssertUnwindSafe( || {
        atelier.state.ConstructJob( 0, 0, WorkPtr::FromClosure( |_| {}));
    }));
    jeeves_assert!( ctx, result.is_err());
    jeeves_assert!( ctx, atelier.state.TryConstructJob( 0, 0, WorkPtr::FromClosure( |_| {})).is_none());
    let relaunch = std::panic::catch_unwind( std::panic::AssertUnwindSafe( || atelier.DoLaunch()));
    jeeves_assert!( ctx, relaunch.is_err());
});

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
// Balanced partitions and scoped completion
jeeves_test!( Heist, ScopedPartitionBoundaries, |ctx| {
    use crate::heist::ScopedExecutor;
    let totals = [0, 1, 2, 3, 7, 8, 9, 63, 64, 65, 96, 192, u32::MAX - 1, u32::MAX];
    let budgets = [0, 1, 2, 3, 4, 8, 16];
    USeg::FromLen(totals.len() as u32).Traverse(|t| {
        USeg::FromLen(budgets.len() as u32).Traverse(|b| {
            let total = totals[t as usize];
            let executor = ScopedExecutor::New(budgets[b as usize]);
            let count = executor.WorkerCount(total);
            let mut next = 0u32;
            let mut firstLen = 0u32;
            USeg::FromLen(count).Traverse(|worker| {
                let partition = executor.Partition(total, worker);
                jeeves_assert!(ctx, !partition.IsEmpty());
                jeeves_assert_eq!(ctx, partition.First(), next);
                if worker == 0 {
                    firstLen = partition.Size();
                }
                jeeves_assert!(ctx, firstLen - partition.Size() <= 1);
                next += partition.Size();
            });
            jeeves_assert_eq!(ctx, next, total);
            jeeves_assert!(ctx, executor.Partition(total, count).IsEmpty());
        });
    });
});

jeeves_test!( Heist, ScopedMutMatchesBalancedRanges, |ctx| {
    use crate::heist::ScopedExecutor;
    let totals = [0, 1, 8, 9, 96];
    let budgets = [0, 1, 3, 8];
    USeg::FromLen(totals.len() as u32).Traverse(|t| {
        USeg::FromLen(budgets.len() as u32).Traverse(|b| {
            let total = totals[t as usize];
            let executor = ScopedExecutor::New(budgets[b as usize]);
            let mut values = [u32::MAX; 96];
            let valid = AtomicBool::new(true);
            executor.ForEachMut((&mut values[..total as usize]).into(),
                |worker, start, mut partition| {
                    let expected = executor.Partition(total, worker);
                    if expected.First() != start || expected.Size() != partition.Len() {
                        valid.store(false, Ordering::Relaxed);
                    }
                    partition.USeg().Traverse(|i| {
                        *partition.GetMut(i).unwrap() = worker;
                    });
                });
            jeeves_assert!(ctx, valid.load(Ordering::Relaxed));
            USeg::FromLen(executor.WorkerCount(total)).Traverse(|worker| {
                executor.Partition(total, worker).Traverse(|index| {
                    jeeves_assert_eq!(ctx, values[index as usize], worker);
                });
            });
            USeg::WithLen(total, 96 - total).Traverse(|index| {
                jeeves_assert_eq!(ctx, values[index as usize], u32::MAX);
            });
        });
    });
});

jeeves_test!( Heist, ScopedPanicJoinsBorrowedWorkers, |ctx| {
    use crate::heist::ScopedExecutor;
    let completed = std::array::from_fn::<_, 3, _>(|_| AtomicBool::new(false));
    let result = std::panic::catch_unwind(|| {
        ScopedExecutor::New(3).ForEachRange(3, |worker, _, _| {
            if worker == 0 {
                panic!("intentional scoped worker panic");
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
            completed[worker as usize].store(true, Ordering::Relaxed);
        });
    });
    jeeves_assert!(ctx, result.is_err());
    jeeves_assert!(ctx, completed[1].load(Ordering::Relaxed));
    jeeves_assert!(ctx, completed[2].load(Ordering::Relaxed));
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
    main_maestro.PostChoreTree(chore_tree);
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
    atelier.MainMaestro().PostChoreTree(tree);
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
    atelier.MainMaestro().PostChoreTree(spawn_quell.into());
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

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, HeistRunnablesExhaustedBeforeRequiredCoroutines, |ctx| {
    use crate::heist::ChorePlacement;
    let  	atelier = Atelier::Reset( 2);
    let  	runnables_remaining = Arc::new( AtomicU32::new( 20));
    let  	coros_executed_prematurely = Arc::new( AtomicBool::new( false));

    // Post 20 generic runnables (10 to worker 0, 10 to worker 1)
    for _ in 0..10 {
        let  	rem = runnables_remaining.clone();
        atelier.MainMaestro().PostWithPlacement( ChorePlacement::Any, move |_| {
            std::thread::yield_now();
            rem.fetch_sub( 1, Ordering::SeqCst);
        });
    }
    for _ in 0..10 {
        let  	rem = runnables_remaining.clone();
        atelier.Maestros()[1].PostWithPlacement( ChorePlacement::Any, move |_| {
            std::thread::yield_now();
            rem.fetch_sub( 1, Ordering::SeqCst);
        });
    }

    // Post required coroutine jobs to worker 0 and worker 1
    for w in 0..2 {
        let  	rem = runnables_remaining.clone();
        let  	premature = coros_executed_prematurely.clone();
        atelier.Maestros()[w].PostWithPlacement( ChorePlacement::Require( w), move |_| {
            if rem.load( Ordering::SeqCst) > 0 {
                premature.store( true, Ordering::SeqCst);
            }
        });
    }

    atelier.DoLaunch();
    jeeves_assert_eq!( ctx, runnables_remaining.load( Ordering::SeqCst), 0);
    jeeves_assert!( ctx, !coros_executed_prematurely.load( Ordering::SeqCst));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, HeistRequiredNeverStolen, |ctx| {
    use crate::heist::ChorePlacement;
    let  	atelier = Atelier::Reset( 3);
    let  	worker1_processed = Arc::new( AtomicU32::new( 0));
    let  	thieves_stole_required = Arc::new( AtomicBool::new( false));

    // Post 20 Require(1) jobs directly to Maestro 1
    for _ in 0..20 {
        let  	proc = worker1_processed.clone();
        let  	thief_flag = thieves_stole_required.clone();
        atelier.Maestros()[1].PostWithPlacement( ChorePlacement::Require( 1), move |w| {
            if w.WorkerIndex() != 1 {
                thief_flag.store( true, Ordering::SeqCst);
            } else {
                proc.fetch_add( 1, Ordering::SeqCst);
            }
        });
    }

    atelier.DoLaunch();
    jeeves_assert_eq!( ctx, worker1_processed.load( Ordering::SeqCst), 20);
    jeeves_assert!( ctx, !thieves_stole_required.load( Ordering::SeqCst));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, HeistPreferCanBeStolen, |ctx| {
    use crate::heist::ChorePlacement;
    let  	atelier = Atelier::Reset( 2);
    let  	total_processed = Arc::new( AtomicU32::new( 0));

    // Post 50 Prefer(1) jobs to Maestro 1
    for _ in 0..50 {
        let  	tot = total_processed.clone();
        atelier.Maestros()[1].PostWithPlacement( ChorePlacement::Prefer( 1), move |_w| {
            tot.fetch_add( 1, Ordering::SeqCst);
        });
    }

    atelier.Maestros()[1].FlushTempQueue( &atelier.state);
    let stolen_id = atelier.Maestros()[1].PopStealJob();
    jeeves_assert!( ctx, stolen_id != 0);
    atelier.Maestros()[0].EnqueRunJob( stolen_id);
    atelier.DoLaunch();
    jeeves_assert_eq!( ctx, total_processed.load( Ordering::SeqCst), 50);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Heist, HeistCrossWorkerDirectContinuationRouted, |ctx| {
    use crate::heist::ChorePlacement;
    let  	atelier = Atelier::Reset( 2);
    let  	state = &atelier.state;
    let  	worker_executed_successor = Arc::new( AtomicU32::new( 999));

    // Successor requires worker 1
    let  	wex = worker_executed_successor.clone();
    let  	succ_job = state.ConstructJob(
        1,
        0,
        WorkPtr::FromClosure( move |w| {
            wex.store( w.WorkerIndex(), Ordering::SeqCst);
        }),
    );
    state.SetJobPlacement( succ_job, ChorePlacement::Require( 1));

    // Predecessor runs on worker 0 with succ_job as successor
    let  	pred_job = state.ConstructJob(
        0,
        succ_job,
        WorkPtr::FromClosure( move |_w| {
            // Pred finishes on worker 0
        }),
    );
    state.SetJobPlacement( pred_job, ChorePlacement::Require( 0));

    atelier.MainMaestro().EnqueueRequiredJob( pred_job);
    state._SzSchedJob.fetch_add( 1, Ordering::SeqCst);

    atelier.DoLaunch();
    // Verify successor was routed to and executed on worker 1, NOT inlined on worker 0!
    jeeves_assert_eq!( ctx, worker_executed_successor.load( Ordering::SeqCst), 1);
});

//-------------------------------------------------------------------------------------------------

static PINNED_CORO_YIELD_COUNT: AtomicU32 = AtomicU32::new( 0);
static PINNED_CORO_CORRECT_WORKER: AtomicBool = AtomicBool::new( true);

jeeves_test!( Heist, HeistCoroContinuationPinned, |ctx| {
    PINNED_CORO_YIELD_COUNT.store( 0, Ordering::SeqCst);
    PINNED_CORO_CORRECT_WORKER.store( true, Ordering::SeqCst);
    let  	atelier = Atelier::Reset( 3);

    let  	coro = crate::CoroChore!( |yielder, worker_fat_ptr| {
        for _ in 0..5 {
            let  	w_idx = unsafe { ( *worker_fat_ptr._Ptr).WorkerIndex() };
            if w_idx != 2 {
                PINNED_CORO_CORRECT_WORKER.store( false, Ordering::SeqCst);
            }
            PINNED_CORO_YIELD_COUNT.fetch_add( 1, Ordering::SeqCst);
            let  	_ = yielder.Suspend( ());
        }
    }).Require( 2);

    atelier.MainMaestro().PostChoreTree(coro.into());
    atelier.DoLaunch();

    jeeves_assert_eq!( ctx, PINNED_CORO_YIELD_COUNT.load( Ordering::SeqCst), 5);
    jeeves_assert!( ctx, PINNED_CORO_CORRECT_WORKER.load( Ordering::SeqCst));
});

//-------------------------------------------------------------------------------------------------

static DONE_CORO_YIELD_COUNTER: AtomicU32 = AtomicU32::new( 0);

jeeves_test!( Heist, HeistCoroDoneTriggersSuccessor, |ctx| {
    DONE_CORO_YIELD_COUNTER.store( 0, Ordering::SeqCst);
    let  	atelier = Atelier::Reset( 2);
    let  	successor_executed_after_done = Arc::new( AtomicBool::new( false));

    let  	coro = crate::CoroChore!( |yielder| {
        for _ in 0..3 {
            DONE_CORO_YIELD_COUNTER.fetch_add( 1, Ordering::SeqCst);
            let  	_ = yielder.Suspend( ());
        }
    });

    let  	succ_flag = successor_executed_after_done.clone();
    let  	succ = Chore::FromClosure( "Successor", move |_w| {
        if DONE_CORO_YIELD_COUNTER.load( Ordering::SeqCst) == 3 {
            succ_flag.store( true, Ordering::SeqCst);
        }
    });

    let  	tree = coro.Then( succ);
    atelier.MainMaestro().PostChoreTree(tree);
    atelier.DoLaunch();

    jeeves_assert_eq!( ctx, DONE_CORO_YIELD_COUNTER.load( Ordering::SeqCst), 3);
    jeeves_assert!( ctx, successor_executed_after_done.load( Ordering::SeqCst));
});
