// maestro.rs ------------------------------------------------------------------------------------------------------
use crate::heist::atelier::AtelierState;
use crate::heist::choretree::{ChoreNode, PostChoreNode};
use crate::silo::arr::Arr;
use crate::silo::buff::Buff;
use crate::silo::stash::Stash;
use crate::stalks::work::{IWorker, SpinMutex, WorkPtr};
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::{Arc, Weak};

//-------------------------------------------------------------------------------------------------
// Maestro — per-thread worker context managing job caching, scheduling queues, and work execution.
// Modeled directly from Trellis heist/maestro.h.
pub struct Maestro
{
    _Index:            u32,
    _SzProcessed:      AtomicU32,
    _SzStealAttempts:  AtomicU32,
    _SzStealSuccesses: AtomicU32,
    _SzYields:         AtomicU32,
    _CurSuccId:        AtomicU16,
    _RunQueue:         SpinMutex<Stash<u16>>,
    _RequiredQueue:    SpinMutex<Stash<u16>>,
    _TempQueue:        SpinMutex<Stash<u16>>,
    _JobCache:         SpinMutex<Stash<u16>>,
    _State:            std::sync::OnceLock<Weak<AtelierState>>,
}
impl Maestro
{
    pub fn New(index: u32) -> Self
    {
        Self { _Index:            index,
               _SzProcessed:      AtomicU32::new(0),
               _SzStealAttempts:  AtomicU32::new(0),
               _SzStealSuccesses: AtomicU32::new(0),
               _SzYields:         AtomicU32::new(0),
               _CurSuccId:        AtomicU16::new(0),
               _RunQueue:         SpinMutex::New(Stash::WithCapacity(1024)),
               _RequiredQueue:    SpinMutex::New(Stash::WithCapacity(1024)),
               _TempQueue:        SpinMutex::New(Stash::WithCapacity(64)),
               _JobCache:         SpinMutex::New(Stash::WithCapacity(256)),
               _State:            std::sync::OnceLock::new(), }
    }
    #[inline]
    pub fn Index(&self) -> u32 { self._Index }
    #[inline]
    pub fn MaestroIndex(&self) -> u32 { self._Index }
    #[inline]
    pub fn ProcessedCount(&self) -> u32 { self._SzProcessed.load(Ordering::Relaxed) }
    #[inline]
    pub fn StealAttempts(&self) -> u32 { self._SzStealAttempts.load(Ordering::Relaxed) }
    #[inline]
    pub fn StealSuccesses(&self) -> u32 { self._SzStealSuccesses.load(Ordering::Relaxed) }
    #[inline]
    pub fn YieldCount(&self) -> u32 { self._SzYields.load(Ordering::Relaxed) }
    #[inline]
    pub fn IncStealAttempts(&self) { self._SzStealAttempts.fetch_add(1, Ordering::Relaxed); }
    #[inline]
    pub fn IncStealSuccesses(&self) { self._SzStealSuccesses.fetch_add(1, Ordering::Relaxed); }
    #[inline]
    pub fn IncProcessed(&self) { self._SzProcessed.fetch_add(1, Ordering::Relaxed); }
    #[inline]
    pub fn IncYields(&self) { self._SzYields.fetch_add(1, Ordering::Relaxed); }
    #[inline]
    pub fn PopJobCache(&self) -> Option<u16> { self._JobCache.Lock().Pop() }
    #[inline]
    pub fn PushJobCache(&self, id: u16) { self._JobCache.Lock().PushBack(id); }
    #[inline]
    pub fn JobCacheSize(&self) -> u32 { self._JobCache.Lock().Size() }
    #[inline]
    pub fn WithRunQueueArr<F, R>(&self, f: F) -> R where F: FnOnce(Arr<'_, u16>) -> R
    {
        let q = self._RunQueue.Lock();
        f(q.Arr())
    }
    #[inline]
    pub fn LockRunQueue(&self) -> crate::stalks::work::SpinMutexGuard<'_, Stash<u16>>
    {
        self._RunQueue.Lock()
    }
    #[inline]
    pub fn CurSuccId(&self) -> u16 { self._CurSuccId.load(Ordering::Acquire) }
    #[inline]
    pub fn SetCurSuccId(&self, val: u16) { self._CurSuccId.store(val, Ordering::Release); }
    pub fn SetState(&self, state: Weak<AtelierState>) { let _ = self._State.set(state); }
    pub fn State(&self) -> Option<Arc<AtelierState>>
    {
        self._State.get().and_then(|w| w.upgrade())
    }
    pub fn EnqueRunJob(&self, job_id: u16)
    {
        if let Some(state) = self.State().filter(|s| s.GetJobSeq(job_id) == 0) {
            state.StampJobSeq(job_id);
        }
        self._RunQueue.Lock().PushBack(job_id);
    }
    pub fn EnqueueRequiredJob(&self, job_id: u16)
    {
        if let Some(state) = self.State().filter(|s| s.GetJobSeq(job_id) == 0) {
            state.StampJobSeq(job_id);
        }
        self._RequiredQueue.Lock().PushBack(job_id);
    }
    pub fn EnqueueJob(&self, job_id: u16) { self._TempQueue.Lock().PushBack(job_id); }
    pub fn PopRunJob(&self) -> u16 { self._RunQueue.Lock().Pop().unwrap_or(0) }
    pub fn PopStealJob(&self) -> u16 { self._RunQueue.Lock().Pop().unwrap_or(0) }
    pub fn PopRequiredJob(&self) -> u16 { self._RequiredQueue.Lock().Pop().unwrap_or(0) }
    pub fn PopLocalJob(&self, state: &AtelierState) -> u16
    {
        let mut run_q = self._RunQueue.Lock();
        let mut req_q = self._RequiredQueue.Lock();
        let run_top = run_q.Top();
        let req_top = req_q.Top();
        match (run_top, req_top) {
            (Some(r), Some(q)) => {
                let r_seq = state.GetJobSeq(r);
                let q_seq = state.GetJobSeq(q);
                if q_seq <= r_seq {
                    req_q.Pop().unwrap_or(0)
                } else {
                    run_q.Pop().unwrap_or(0)
                }
            }
            (Some(_), None) => run_q.Pop().unwrap_or(0),
            (None, Some(_)) => {
                if state.SzSchedRunnables() == 0 {
                    req_q.Pop().unwrap_or(0)
                } else {
                    0
                }
            }
            (None, None) => 0,
        }
    }
    pub fn PopJob(&self) -> u16
    {
        let id = self.PopRunJob();
        if id != 0 { id } else { self.PopRequiredJob() }
    }
    pub fn FlushTempQueue(&self, state: &AtelierState)
    {
        let mut temp = self._TempQueue.Lock();
        if temp.Size() == 0 {
            return;
        }
        let mut run_q = self._RunQueue.Lock();
        let mut req_q = self._RequiredQueue.Lock();
        while let Some(id) = temp.Pop() {
            if id != 0 {
                state.AddSchedJob(1);
                if state.GetJobSeq(id) == 0 {
                    state.StampJobSeq(id);
                }
                if state.GetJobPlacement(id).IsRequired() {
                    req_q.PushBack(id);
                } else {
                    run_q.PushBack(id);
                    state.AddSchedRunnables(1);
                }
            }
        }
    }
    pub fn ConstructJob(&self, succ_id: u16, job: WorkPtr) -> u16
    {
        if let Some(state) = self.State() {
            state.ConstructJob(self._Index, succ_id, job)
        } else {
            0
        }
    }
    pub fn ConstructEnqueArr(&self, succ_id: u16, buff: Buff<u16>) -> u16
    {
        if let Some(state) = self.State() {
            state.ConstructEnqueArr(self._Index, succ_id, buff)
        } else {
            0
        }
    }
    pub fn PostJob(&self, job: WorkPtr)
    {
        if let Some(state) = self.State() {
            if state.IsImmediate() {
                let mut ctx = MaestroContext::New(self, &state);
                let mut j = job;
                j.DoWork(&mut ctx);
                return;
            }
            let succ_id = self.CurSuccId();
            let job_id = state.ConstructJob(self._Index, succ_id, job);
            self.EnqueueJob(job_id);
        }
    }
    #[allow(clippy::result_unit_err)] // Queue saturation is the only error state.
    pub fn TryPostJob(&self, job: WorkPtr) -> Result<u16, ()>
    {
        if let Some(state) = self.State() {
            if state.IsImmediate() {
                let mut ctx = MaestroContext::New(self, &state);
                let mut j = job;
                j.DoWork(&mut ctx);
                return Ok(0);
            }
            let succ_id = self.CurSuccId();
            if let Some(job_id) = state.TryConstructJob(self._Index, succ_id, job) {
                self.EnqueueJob(job_id);
                Ok(job_id)
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
    pub fn Post<F>(&self, f: F)
        where F: FnOnce(&mut dyn IWorker) + Send + 'static
    {
        self.PostJob(WorkPtr::FromClosure(f));
    }
    pub fn PostJobWithPlacement(&self, job: WorkPtr,
                                placement: crate::heist::placement::ChorePlacement)
    {
        if let Some(state) = self.State() {
            if state.IsImmediate() {
                let mut ctx = MaestroContext::New(self, &state);
                let mut j = job;
                j.DoWork(&mut ctx);
                return;
            }
            let succ_id = self.CurSuccId();
            let target_worker = placement.TargetWorker().unwrap_or(self._Index);
            let job_id = state.ConstructJob(target_worker, succ_id, job);
            state.SetJobPlacement(job_id, placement);
            state.EnqueueToWorker(target_worker, job_id, placement.IsRequired());
        }
    }
    pub fn PostWithPlacement<F>(&self, placement: crate::heist::placement::ChorePlacement, f: F)
        where F: FnOnce(&mut dyn IWorker) + Send + 'static
    {
        self.PostJobWithPlacement(WorkPtr::FromClosure(f), placement);
    }
    pub fn PostChoreTree(&self, node: ChoreNode)
    {
        let mut tails = Stash::WithCapacity(64);
        let head = PostChoreNode(node, self, &mut tails);
        if let Some(state) = self.State() {
            let succ_id = self.CurSuccId();
            while let Some(tail) = tails.Pop() {
                state.SetSucc(tail, succ_id);
            }
            let placement = state.GetJobPlacement(head);
            let target_worker = placement.TargetWorker().unwrap_or(self._Index);
            state.EnqueueToWorker(target_worker, head, false);
        }
    }
}
impl IWorker for Maestro
{
    fn PostJob(&mut self, job: WorkPtr) { (*self).PostJob(job); }
    fn WorkerIndex(&self) -> u32 { self._Index }
    fn EnqueueJobId(&mut self, job_id: u16) { self.EnqueueJob(job_id); }
    fn PostJobWithPlacement(&mut self, job: WorkPtr, placement_packed: u16, succ_id: u16)
    {
        if let Some(state) = self.State() {
            let placement =
                crate::heist::placement::ChorePlacement::FromPackedU16(placement_packed);
            let target_worker = placement.TargetWorker().unwrap_or(self._Index);
            state.SubmitJobWithPlacement(target_worker, succ_id, placement, job);
        }
    }
    fn PublishSuccessor(&mut self, succ_id: u16)
    {
        if let Some(state) = self.State() {
            state.PublishSuccessor(succ_id);
        }
    }
    fn CurSuccId(&self) -> u16 { (*self).CurSuccId() }
    fn SetCurSuccId(&mut self, val: u16) { (*self).SetCurSuccId(val); }
}

//-------------------------------------------------------------------------------------------------
// MaestroContext — borrowed execution context passed into WorkPtr::DoWork during loop execution.
pub struct MaestroContext<'a>
{
    _Maestro: &'a Maestro,
    _State:   &'a AtelierState,
}
impl<'a> MaestroContext<'a>
{
    #[inline]
    pub fn New(maestro: &'a Maestro, state: &'a AtelierState) -> Self
    {
        Self { _Maestro: maestro,
               _State:   state, }
    }
    #[inline]
    pub fn Maestro(&self) -> &'a Maestro { self._Maestro }
    #[inline]
    pub fn State(&self) -> &'a AtelierState { self._State }
}
impl<'a> IWorker for MaestroContext<'a>
{
    fn PostJob(&mut self, mut job: WorkPtr)
    {
        if self._State.IsImmediate() {
            job.DoWork(self);
            return;
        }
        let succ_id = self._Maestro.CurSuccId();
        let job_id = self._State.ConstructJob(self._Maestro.Index(), succ_id, job);
        self._Maestro.EnqueueJob(job_id);
    }
    fn WorkerIndex(&self) -> u32 { self._Maestro.Index() }
    fn EnqueueJobId(&mut self, job_id: u16) { self._Maestro.EnqueueJob(job_id); }
    fn PostJobWithPlacement(&mut self, mut job: WorkPtr, placement_packed: u16, succ_id: u16)
    {
        if self._State.IsImmediate() {
            job.DoWork(self);
            return;
        }
        let placement = crate::heist::placement::ChorePlacement::FromPackedU16(placement_packed);
        let target_worker = placement.TargetWorker().unwrap_or(self._Maestro.Index());
        self._State.SubmitJobWithPlacement(target_worker, succ_id, placement, job);
    }
    fn PublishSuccessor(&mut self, succ_id: u16)
    {
        self._State.PublishSuccessor(succ_id);
    }
    fn CurSuccId(&self) -> u16 { self._Maestro.CurSuccId() }
    fn SetCurSuccId(&mut self, val: u16) { self._Maestro.SetCurSuccId(val); }
}
