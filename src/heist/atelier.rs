// atelier.rs ------------------------------------------------------------------------------------------------------
use crate::heist::maestro::{Maestro, MaestroContext};
use crate::silo::USeg;
use crate::silo::arr::{Arr, MutArr};
use crate::silo::buff::Buff;
use crate::silo::stash::Stash;
use crate::stalks::work::{SpinMutex, Spinlock, WorkPtr};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
pub const K_JOB_CAPACITY: usize = 65536;
static LIFECYCLE_LOCK: Spinlock = Spinlock::New();
static GLOBAL_ATELIER: RwLock<Option<Arc<Atelier>>> = RwLock::new(None);

//-------------------------------------------------------------------------------------------------
// AtelierState — internal state shared across worker threads and Maestros.
pub struct AtelierState
{
    _SzThreads:              u32,
    _Failed:                 AtomicBool,
    _SzSchedJob:             AtomicU32,
    _SzSchedRunnables:       AtomicU32,
    _SzLaunchWorkersStarted: AtomicU32,
    _NextEnqueueSeq:         AtomicU64,
    _JobSeq:                 Buff<AtomicU64>,
    _SzPreds:                Buff<AtomicU16>,
    _SuccIds:                Buff<AtomicU16>,
    _JobPlacements:          Buff<AtomicU16>,
    _JobBuff:                Buff<SpinMutex<Option<WorkPtr>>>,
    _FreeJobStash:           SpinMutex<Stash<u16>>,
    _Terminal:               AtomicU16,
    _Maestros:               Buff<Maestro>,
}
impl AtelierState
{
    pub fn New(threads: u32) -> Arc<Self>
    {
        let maestro_count = if threads == 0 { 1 } else { threads };
        let maestros = Buff::FromDispenser(maestro_count, Maestro::New);
        let sz_preds = Buff::FromDispenser(K_JOB_CAPACITY as u32, |_| AtomicU16::new(0));
        let succ_ids = Buff::FromDispenser(K_JOB_CAPACITY as u32, |_| AtomicU16::new(0));
        let job_placements = Buff::FromDispenser(K_JOB_CAPACITY as u32, |_| AtomicU16::new(0));
        let job_seq = Buff::FromDispenser(K_JOB_CAPACITY as u32, |_| AtomicU64::new(0));
        let job_buff = Buff::FromDispenser(K_JOB_CAPACITY as u32, |_| SpinMutex::New(None));
        let mut free_stash = Stash::WithCapacity(K_JOB_CAPACITY as u32);
        for i in 1..K_JOB_CAPACITY as u32 {
            free_stash.PushBack(i as u16);
        }
        let state = Arc::new(Self { _SzThreads:              threads,
                                    _Failed:                 AtomicBool::new(false),
                                    _SzSchedJob:             AtomicU32::new(0),
                                    _SzSchedRunnables:       AtomicU32::new(0),
                                    _SzLaunchWorkersStarted: AtomicU32::new(0),
                                    _NextEnqueueSeq:         AtomicU64::new(1),
                                    _JobSeq:                 job_seq,
                                    _SzPreds:                sz_preds,
                                    _SuccIds:                succ_ids,
                                    _JobPlacements:          job_placements,
                                    _JobBuff:                job_buff,
                                    _FreeJobStash:           SpinMutex::New(free_stash),
                                    _Terminal:               AtomicU16::new(0),
                                    _Maestros:               maestros, });
        state._Maestros.Traverse(|m| {
                           m.SetState(Arc::downgrade(&state));
                       });
        let terminal = state.ConstructJob(0, 0, WorkPtr::FromClosure(|_| {}));
        state._Terminal.store(terminal, Ordering::SeqCst);
        state._Maestros[0].SetCurSuccId(terminal);
        state
    }
    #[inline]
    pub fn Terminal(&self) -> u16 { self._Terminal.load(Ordering::Relaxed) }
    #[inline]
    pub fn SzThreads(&self) -> u32 { self._SzThreads }
    #[inline]
    pub fn IsImmediate(&self) -> bool { self._SzThreads == 0 }
    #[inline]
    pub fn SzSchedJob(&self) -> u32 { self._SzSchedJob.load(Ordering::Acquire) }
    #[inline]
    pub fn AddSchedJob(&self, count: u32) { self._SzSchedJob.fetch_add(count, Ordering::SeqCst); }
    #[inline]
    pub fn SubSchedJob(&self, count: u32) { self._SzSchedJob.fetch_sub(count, Ordering::SeqCst); }
    #[inline]
    pub fn SzSchedRunnables(&self) -> u32 { self._SzSchedRunnables.load(Ordering::Acquire) }
    #[inline]
    pub fn AddSchedRunnables(&self, count: u32) { self._SzSchedRunnables.fetch_add(count, Ordering::SeqCst); }
    #[inline]
    pub fn SubSchedRunnables(&self, count: u32) { self._SzSchedRunnables.fetch_sub(count, Ordering::SeqCst); }
    #[inline]
    pub fn Maestro(&self, idx: u32) -> &Maestro { &self._Maestros[idx] }
    #[inline]
    pub fn Maestros(&self) -> Arr<'_, Maestro> { self._Maestros.Arr() }
    #[inline]
    pub fn SzMaestros(&self) -> u32 { self._Maestros.Len() }
    #[inline]
    pub fn SuccId(&self, job_id: u16) -> u16 { self._SuccIds[job_id as u32].load(Ordering::SeqCst) }
    #[inline]
    pub fn SetSuccId(&self, job_id: u16, succ_id: u16) { self._SuccIds[job_id as u32].store(succ_id, Ordering::SeqCst); }
    #[inline]
    pub fn SzPred(&self, job_id: u16) -> u16 { self._SzPreds[job_id as u32].load(Ordering::SeqCst) }
    #[inline]
    pub fn DecrementPred(&self, succ_id: u16) -> u16 { self._SzPreds[succ_id as u32].fetch_sub(1, Ordering::SeqCst) }
    #[inline]
    pub fn EnqueueToWorker(&self, target_worker: u32, job_id: u16, required: bool)
    {
        let target_idx = if target_worker < self._SzThreads.max(1) { target_worker } else { 0 };
        if required {
            self._SzSchedJob.fetch_add(1, Ordering::SeqCst);
            self._Maestros[target_idx].EnqueueRequiredJob(job_id);
        } else {
            self._Maestros[target_idx].EnqueueJob(job_id);
        }
    }
    #[inline]
    pub fn PublishSuccessor(&self, succ_id: u16)
    {
        let prev = self._SzPreds[succ_id as u32].fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            let placement = self.GetJobPlacement(succ_id);
            self.EnqueueByPlacement(succ_id, placement);
        }
    }
    pub fn SubmitJobWithPlacement(&self, target_worker: u32, succ_id: u16, placement: crate::heist::placement::ChorePlacement, job: WorkPtr)
    {
        let target_idx = if target_worker < self._SzThreads.max(1) { target_worker } else { 0 };
        let job_id = self.AllocJobOrPanic(target_idx);
        if job_id != 0 {
            self.ResetJobSlot(job_id);
            *self._JobBuff[job_id as u32].Lock() = Some(job);
            if succ_id != 0 {
                self._SuccIds[job_id as u32].store(succ_id, Ordering::SeqCst);
            }
            self.SetJobPlacement(job_id, placement);
            self.EnqueueByPlacement(job_id, placement);
        }
    }
    pub fn AllocJob(&self, maestro_idx: u32) -> u16
    {
        let maestro = &self._Maestros[maestro_idx];
        // Both allocation and recycling acquire cache before the global free list.
        if let Some(id) = maestro.PopJobCache() {
            return id;
        }
        let mut free = self._FreeJobStash.Lock();
        if free.Size() == 0 {
            return 0;
        }
        let count = free.Size().min(64);
        USeg::FromLen(count).Traverse(|_| {
                                if let Some(id) = free.Pop() {
                                    maestro.PushJobCache(id);
                                }
                            });
        maestro.PopJobCache().unwrap_or(0)
    }
    #[inline]
    pub fn TryAllocJob(&self, maestro_idx: u32) -> Option<u16>
    {
        let id = self.AllocJob(maestro_idx);
        if id != 0 { Some(id) } else { None }
    }
    pub fn ResetJobSlot(&self, job_id: u16)
    {
        self._SuccIds[job_id as u32].store(0, Ordering::SeqCst);
        self._SzPreds[job_id as u32].store(0, Ordering::SeqCst);
        self._JobPlacements[job_id as u32].store(0, Ordering::SeqCst);
        self._JobSeq[job_id as u32].store(0, Ordering::Relaxed);
        *self._JobBuff[job_id as u32].Lock() = None;
    }
    #[inline]
    pub fn StampJobSeq(&self, job_id: u16) -> u64
    {
        let seq = self._NextEnqueueSeq.fetch_add(1, Ordering::Relaxed);
        self._JobSeq[job_id as u32].store(seq, Ordering::Relaxed);
        seq
    }
    #[inline]
    pub fn GetJobSeq(&self, job_id: u16) -> u64
    {
        self._JobSeq[job_id as u32].load(Ordering::Relaxed)
    }
    #[inline]
    pub fn SetJobPlacement(&self, job_id: u16, placement: crate::heist::placement::ChorePlacement)
    {
        self._JobPlacements[job_id as u32].store(placement.ToPackedU16(), Ordering::SeqCst);
    }
    #[inline]
    pub fn GetJobPlacement(&self, job_id: u16) -> crate::heist::placement::ChorePlacement
    {
        crate::heist::placement::ChorePlacement::FromPackedU16(
            self._JobPlacements[job_id as u32].load( Ordering::SeqCst),
        )
    }
    pub fn EnqueueByPlacement(&self, job_id: u16,
                              placement: crate::heist::placement::ChorePlacement)
    {
        self._SzSchedJob.fetch_add(1, Ordering::SeqCst);
        self.StampJobSeq(job_id);
        let target_worker = placement.TargetWorker().unwrap_or(0);
        let target_idx = if target_worker < self._SzThreads.max(1) {
            target_worker
        } else {
            0
        };
        if placement.IsRequired() {
            self._Maestros[target_idx].EnqueueRequiredJob(job_id);
        } else {
            self._SzSchedRunnables.fetch_add(1, Ordering::SeqCst);
            self._Maestros[target_idx].EnqueRunJob(job_id);
        }
    }
    pub fn FreeJob(&self, maestro_idx: u32, job_id: u16)
    {
        self.ResetJobSlot(job_id);
        let maestro = &self._Maestros[maestro_idx];
        maestro.FlushTempQueue(self);
        if maestro.JobCacheSize() < 256 {
            maestro.PushJobCache(job_id);
        } else {
            let mut free = self._FreeJobStash.Lock();
            free.PushBack(job_id);
        }
    }
    pub fn SetSucc(&self, job_id: u16, succ_id: u16)
    {
        self._SuccIds[job_id as u32].store(succ_id, Ordering::SeqCst);
        self._SzPreds[succ_id as u32].fetch_add(1, Ordering::SeqCst);
    }
    pub fn InitJobSlot(&self, job_id: u16, job: WorkPtr)
    {
        self._SuccIds[job_id as u32].store(0, Ordering::SeqCst);
        self._SzPreds[job_id as u32].store(0, Ordering::SeqCst);
        self._JobPlacements[job_id as u32].store(0, Ordering::SeqCst);
        self.StampJobSeq(job_id);
        *self._JobBuff[job_id as u32].Lock() = Some(job);
    }
    pub(super) fn AllocJobOrPanic(&self, maestro_idx: u32) -> u16
    {
        let job_id = self.AllocJob(maestro_idx);
        if job_id == 0 {
            self._Failed.store(true, Ordering::Release);
            panic!("Heist job capacity exhausted; use TryConstructJob for bounded submission");
        }
        return job_id;
    }
    pub fn ConstructJob(&self, maestro_idx: u32, succ_id: u16, job: WorkPtr) -> u16
    {
        let job_id = self.AllocJobOrPanic(maestro_idx);
        self.InitJobSlot(job_id, job);
        if succ_id != 0 {
            self.SetSucc(job_id, succ_id);
        }
        job_id
    }
    pub fn TryConstructJob(&self, maestro_idx: u32, succ_id: u16, job: WorkPtr) -> Option<u16>
    {
        let job_id = self.TryAllocJob(maestro_idx)?;
        self.InitJobSlot(job_id, job);
        if succ_id != 0 {
            self.SetSucc(job_id, succ_id);
        }
        Some(job_id)
    }
    pub fn ConstructEnqueArr(&self, maestro_idx: u32, succ_id: u16, buff: Buff<u16>) -> u16
    {
        self.ConstructJob(maestro_idx,
                          succ_id,
                          WorkPtr::FromClosure(move |worker| {
                              for i in 0..buff.Cap() {
                                  let job_id = buff[i];
                                  if job_id != 0 {
                                      worker.EnqueueJobId(job_id);
                                  }
                              }
                          }))
    }
    pub fn GrabJob(&self, idx: u32, steal_seed: &mut u32) -> u16
    {
        let thief = &self._Maestros[idx];
        thief.IncStealAttempts();
        let sz = self._Maestros.Len();
        const KNUTH_MULT_HASH: u32 = 2654435761;
        *steal_seed = steal_seed.wrapping_mul(KNUTH_MULT_HASH).wrapping_add(1);
        let start = *steal_seed % sz;
        let mut found = 0;
        USeg::FromLen(sz).Span(|offset| {
            // Add only within the nonwrapping part of this circular segment.
            let maestro_idx = if offset < sz - start { start + offset } else { offset - (sz - start) };
            if maestro_idx != idx {
                found = self._Maestros[maestro_idx].PopStealJob();
            }
            return found == 0;
        });
        if found != 0 {
            thief.IncStealSuccesses();
        }
        return found;
    }
    pub fn ExecuteLoop(state: &Arc<AtelierState>, maestro_idx: u32)
    {
        let _failure = ExecutionFailureGuard { _State: state };
        let maestro = &state._Maestros[maestro_idx];
        if maestro_idx != 0 {
            state._SzLaunchWorkersStarted
                 .fetch_add(1, Ordering::Release);
        }
        maestro.FlushTempQueue(state);
        let mut job_id = 0u16;
        let mut steal_seed = maestro_idx;
        while !state._Failed.load(Ordering::Acquire) && state._SzSchedJob.load(Ordering::Acquire) != 0 {
            while job_id != 0 && !state._Failed.load(Ordering::Acquire) {
                let is_required = state.GetJobPlacement(job_id).IsRequired();
                let succ_id = state._SuccIds[job_id as u32].load(Ordering::Acquire);
                maestro.SetCurSuccId(succ_id);
                let job_opt = state._JobBuff[job_id as u32].Lock().take();
                if let Some(mut job) = job_opt {
                    let mut ctx = MaestroContext::New(maestro, state);
                    job.DoWork(&mut ctx);
                }
                maestro.IncProcessed();
                state.FreeJob(maestro_idx, job_id);
                let succ_id = maestro.CurSuccId();
                if succ_id != 0 {
                    let prev_pred = state._SzPreds[succ_id as u32].fetch_sub(1, Ordering::SeqCst);
                    if prev_pred == 1 {
                        let placement = state.GetJobPlacement(succ_id);
                        if placement.CanExecuteOn(maestro_idx) {
                            job_id = succ_id;
                            state._SzSchedJob.fetch_add(1, Ordering::SeqCst);
                            if !placement.IsRequired() {
                                state._SzSchedRunnables.fetch_add(1, Ordering::SeqCst);
                            }
                        } else {
                            state.EnqueueByPlacement(succ_id, placement);
                            job_id = 0;
                        }
                    } else {
                        job_id = 0;
                    }
                } else {
                    job_id = 0;
                }
                if !is_required {
                    state._SzSchedRunnables.fetch_sub(1, Ordering::SeqCst);
                }
                state._SzSchedJob.fetch_sub(1, Ordering::SeqCst);
            }
            job_id = maestro.PopLocalJob(state);
            if job_id == 0 && state._SzThreads >= 2 {
                job_id = state.GrabJob(maestro_idx, &mut steal_seed);
            }

            if job_id == 0 {
                maestro.IncYields();
                std::hint::spin_loop();
                std::thread::yield_now();
            }
        }
    }
}

// Cancel peers before a panicking worker leaves its execution scope.
struct ExecutionFailureGuard<'a>
{
    _State: &'a AtelierState,
}

impl Drop for ExecutionFailureGuard<'_>
{
    fn drop(&mut self)
    {
        if std::thread::panicking() {
            self._State._Failed.store(true, Ordering::Release);
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Atelier — singleton work-stealing job pool, DAG dependency resolver, and multi-threaded orchestrator.
// Modeled directly from Trellis heist/atelier.h.
pub struct Atelier
{
    _State:      Arc<AtelierState>,
    _LaunchLock: Spinlock,
}
impl Atelier
{
    pub fn New(threads: u32) -> Self
    {
        Self { _State:      AtelierState::New(threads),
               _LaunchLock: Spinlock::New(), }
    }
    #[inline]
    pub fn State(&self) -> &Arc<AtelierState> { &self._State }
    #[inline]
    pub fn IsImmediate(&self) -> bool { self._State.IsImmediate() }
    #[inline]
    pub fn SzThreads(&self) -> u32 { self._State.SzThreads() }
    #[inline]
    pub fn MainMaestro(&self) -> &Maestro { self._State.Maestro(0) }
    #[inline]
    pub fn Maestros(&self) -> Arr<'_, Maestro> { self._State.Maestros() }
    #[inline]
    pub fn Terminal(&self) -> u16 { self._State.Terminal() }
    #[inline]
    pub fn TryAllocJob(&self, maestro_idx: u32) -> Option<u16>
    {
        self._State.TryAllocJob(maestro_idx)
    }
    pub fn TotalStealAttempts(&self) -> u32
    {
        let mut total = 0;
        let maestros = self.Maestros();
        for i in 0..maestros.Len() {
            total += maestros[i].StealAttempts();
        }
        total
    }
    pub fn TotalStealSuccesses(&self) -> u32
    {
        let mut total = 0;
        let maestros = self.Maestros();
        for i in 0..maestros.Len() {
            total += maestros[i].StealSuccesses();
        }
        total
    }
    /// Execute bounded range partitions within a caller-owned thread scope.
    /// The action may borrow caller storage because every spawned task joins
    /// before this method returns. This does not use the legacy WorkPtr queue.
    pub fn ForEachScopedRange<F>(&self, total: u32, action: F)
        where F: Fn(u32, u32, u32) + Sync
    {
        crate::heist::scoped::ScopedExecutor::New(self._State.SzThreads())
            .ForEachRange(total, action);
    }
    /// Transfer disjoint mutable partitions to a bounded caller-owned scope.
    /// The views are moved, never aliased, and all workers join before return.
    pub fn ForEachScopedMut<'a, T, F>(&self, values: MutArr<'a, T>, action: F)
        where T: Send,
              F: Fn(u32, MutArr<'a, T>) + Sync
    {
        crate::heist::scoped::ScopedExecutor::New(self._State.SzThreads())
            .ForEachMut(values, |worker, _start, partition| action(worker, partition));
    }
    pub fn DoLaunch(&self)
    {
        let _guard = self._LaunchLock.Lock();
        if self._State.IsImmediate() {
            return;
        }
        let state = &self._State;
        assert!(!state._Failed.load(Ordering::Acquire), "Cannot relaunch a failed Atelier; create a new instance");
        for i in 0..state.SzMaestros() {
            state.Maestro(i).FlushTempQueue(state);
        }
        if state.SzThreads() == 1 {
            AtelierState::ExecuteLoop(state, 0);
            return;
        }
        let worker_count = state.SzMaestros().saturating_sub(1);
        state._SzLaunchWorkersStarted.store(0, Ordering::Release);
        std::thread::scope(|scope| {
            // This also cancels already-started workers if thread creation fails.
            let _failure = ExecutionFailureGuard { _State: state };
            let mut handles = Buff::FromDispenser(worker_count, |i| {
                let idx = i + 1;
                Some(scope.spawn(move || AtelierState::ExecuteLoop(state, idx)))
            });
            while state._SzLaunchWorkersStarted.load(Ordering::Acquire) < worker_count {
                std::thread::yield_now();
            }
            AtelierState::ExecuteLoop(state, 0);
            handles.TraverseMut(|handle| {
                if let Some(handle) = handle.take()
                    && let Err(payload) = handle.join()
                {
                    std::panic::resume_unwind(payload);
                }
            });
        });
    }
    pub fn DefaultThreadCount() -> u32
    {
        std::thread::available_parallelism().map(|n| n.get() as u32)
                                            .unwrap_or(1)
    }
    pub fn LifecycleLock() -> &'static Spinlock { &LIFECYCLE_LOCK }
    pub fn Reset(threads: u32) -> Arc<Atelier>
    {
        let _guard = Self::LifecycleLock().Lock();
        let mut write_guard = GLOBAL_ATELIER.write().unwrap();
        let inst = Arc::new(Atelier::New(threads));
        *write_guard = Some(inst.clone());
        inst
    }
    pub fn Boot(threads: u32) -> Arc<Atelier> { Self::Reset(threads) }
    pub fn Instance() -> Arc<Atelier>
    {
        {
            let read_guard = GLOBAL_ATELIER.read().unwrap();
            if let Some(ref inst) = *read_guard {
                return inst.clone();
            }
        }
        let mut write_guard = GLOBAL_ATELIER.write().unwrap();
        if let Some(ref inst) = *write_guard {
            return inst.clone();
        }
        let inst = Arc::new(Atelier::New(Self::DefaultThreadCount()));
        *write_guard = Some(inst.clone());
        inst
    }
}
