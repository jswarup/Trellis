// maestro.rs ------------------------------------------------------------------------------------------------------
use crate::heist::atelier::AtelierState;
use crate::heist::choretree::{ChoreNode, PostChoreNode};
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
    pub _Index:            u32,
    pub _SzProcessed:      AtomicU32,
    pub _SzStealAttempts:  AtomicU32,
    pub _SzStealSuccesses: AtomicU32,
    pub _SzYields:         AtomicU32,
    pub _CurSuccId:        AtomicU16,
    pub _RunQueue:         SpinMutex<Stash<u16>>,
    pub _RequiredQueue:    SpinMutex<Stash<u16>>,
    pub _TempQueue:        SpinMutex<Stash<u16>>,
    pub _JobCache:         SpinMutex<Stash<u16>>,
    pub _State:            SpinMutex<Option<Weak<AtelierState>>>,
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
               _State:            SpinMutex::New(None), }
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
    pub fn CurSuccId(&self) -> u16 { self._CurSuccId.load(Ordering::Acquire) }
    #[inline]
    pub fn SetCurSuccId(&self, val: u16) { self._CurSuccId.store(val, Ordering::Release); }
    pub fn SetState(&self, state: Weak<AtelierState>) { *self._State.Lock() = Some(state); }
    pub fn State(&self) -> Option<Arc<AtelierState>>
    {
        self._State.Lock().as_ref().and_then(|w| w.upgrade())
    }
    pub fn EnqueRunJob(&self, job_id: u16) { self._RunQueue.Lock().PushBack(job_id); }
    pub fn EnqueueRequiredJob(&self, job_id: u16) { self._RequiredQueue.Lock().PushBack(job_id); }
    pub fn EnqueueJob(&self, job_id: u16) { self._TempQueue.Lock().PushBack(job_id); }
    pub fn PopRunJob(&self) -> u16 { self._RunQueue.Lock().Pop().unwrap_or(0) }
    pub fn PopStealJob(&self) -> u16 { self._RunQueue.Lock().Pop().unwrap_or(0) }
    pub fn PopRequiredJob(&self) -> u16 { self._RequiredQueue.Lock().Pop().unwrap_or(0) }
    pub fn PopJob(&self) -> u16
    {
        let id = self.PopRunJob();
        if id != 0 { id } else { self.PopRequiredJob() }
    }
    pub fn FlushTempQueue(&self, state: &AtelierState)
    {
        let mut temp = self._TempQueue.Lock();
        let mut run_q = self._RunQueue.Lock();
        let mut req_q = self._RequiredQueue.Lock();
        while let Some(id) = temp.Pop() {
            if id != 0 {
                state._SzSchedJob.fetch_add(1, Ordering::SeqCst);
                if state.GetJobPlacement(id).IsRequired() {
                    req_q.PushBack(id);
                } else {
                    run_q.PushBack(id);
                    state._SzSchedRunnables.fetch_add(1, Ordering::SeqCst);
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
            if state._SzThreads == 0 {
                let mut ctx = MaestroContext { maestro: self,
                                               state:   &state, };
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
            if state._SzThreads == 0 {
                let mut ctx = MaestroContext { maestro: self,
                                               state:   &state, };
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
            if state._SzThreads == 0 {
                let mut ctx = MaestroContext { maestro: self,
                                               state:   &state, };
                let mut j = job;
                j.DoWork(&mut ctx);
                return;
            }
            let succ_id = self.CurSuccId();
            let target_worker = placement.TargetWorker().unwrap_or(self._Index);
            let target_idx = if target_worker < state._SzThreads.max(1) {
                target_worker
            } else {
                0
            };
            let job_id = state.ConstructJob(target_idx, succ_id, job);
            state.SetJobPlacement(job_id, placement);
            if placement.IsRequired() {
                state._Maestros[target_idx].EnqueueRequiredJob(job_id);
                state._SzSchedJob.fetch_add(1, Ordering::SeqCst);
            } else {
                state._Maestros[target_idx].EnqueueJob(job_id);
            }
        }
    }
    pub fn PostWithPlacement<F>(&self, placement: crate::heist::placement::ChorePlacement, f: F)
        where F: FnOnce(&mut dyn IWorker) + Send + 'static
    {
        self.PostJobWithPlacement(WorkPtr::FromClosure(f), placement);
    }
    pub fn PostChoreTree(&self, node: &ChoreNode)
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
            let target_idx = if target_worker < state._SzThreads.max(1) {
                target_worker
            } else {
                0
            };
            state._Maestros[target_idx].EnqueueJob(head);
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
            let target_idx = if target_worker < state._SzThreads.max(1) {
                target_worker
            } else {
                0
            };
            let job_id = state.AllocJob(target_idx);
            if job_id != 0 {
                state.ResetJobSlot(job_id);
                *state._JobBuff[job_id as u32].Lock() = Some(job);
                if succ_id != 0 {
                    state._SuccIds[job_id as u32].store(succ_id, Ordering::SeqCst);
                }
                state.SetJobPlacement(job_id, placement);
                if placement.IsRequired() {
                    state._Maestros[target_idx].EnqueueRequiredJob(job_id);
                } else {
                    state._Maestros[target_idx].EnqueRunJob(job_id);
                    state._SzSchedRunnables.fetch_add(1, Ordering::SeqCst);
                }
                state._SzSchedJob.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
    fn PublishSuccessor(&mut self, succ_id: u16)
    {
        if let Some(state) = self.State() {
            let prev = state._SzPreds[succ_id as u32].fetch_sub(1, Ordering::SeqCst);
            if prev == 1 {
                let placement = state.GetJobPlacement(succ_id);
                state.EnqueueByPlacement(succ_id, placement);
            }
        }
    }
    fn CurSuccId(&self) -> u16 { (*self).CurSuccId() }
    fn SetCurSuccId(&mut self, val: u16) { (*self).SetCurSuccId(val); }
}

//-------------------------------------------------------------------------------------------------
// MaestroContext — borrowed execution context passed into WorkPtr::DoWork during loop execution.
pub struct MaestroContext<'a>
{
    pub maestro: &'a Maestro,
    pub state:   &'a AtelierState,
}
impl<'a> IWorker for MaestroContext<'a>
{
    fn PostJob(&mut self, mut job: WorkPtr)
    {
        if self.state._SzThreads == 0 {
            job.DoWork(self);
            return;
        }
        let succ_id = self.maestro.CurSuccId();
        let job_id = self.state.ConstructJob(self.maestro.Index(), succ_id, job);
        self.maestro.EnqueueJob(job_id);
    }
    fn WorkerIndex(&self) -> u32 { self.maestro.Index() }
    fn EnqueueJobId(&mut self, job_id: u16) { self.maestro.EnqueueJob(job_id); }
    fn PostJobWithPlacement(&mut self, mut job: WorkPtr, placement_packed: u16, succ_id: u16)
    {
        if self.state._SzThreads == 0 {
            job.DoWork(self);
            return;
        }
        let placement = crate::heist::placement::ChorePlacement::FromPackedU16(placement_packed);
        let target_worker = placement.TargetWorker().unwrap_or(self.maestro.Index());
        let target_idx = if target_worker < self.state._SzThreads.max(1) {
            target_worker
        } else {
            0
        };
        let job_id = self.state.AllocJob(target_idx);
        if job_id != 0 {
            self.state.ResetJobSlot(job_id);
            *self.state._JobBuff[job_id as u32].Lock() = Some(job);
            if succ_id != 0 {
                self.state._SuccIds[job_id as u32].store(succ_id, Ordering::SeqCst);
            }
            self.state.SetJobPlacement(job_id, placement);
            if placement.IsRequired() {
                self.state._Maestros[target_idx].EnqueueRequiredJob(job_id);
            } else {
                self.state._Maestros[target_idx].EnqueRunJob(job_id);
                self.state._SzSchedRunnables.fetch_add(1, Ordering::SeqCst);
            }
            self.state._SzSchedJob.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn PublishSuccessor(&mut self, succ_id: u16)
    {
        let prev = self.state._SzPreds[succ_id as u32].fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            let placement = self.state.GetJobPlacement(succ_id);
            self.state.EnqueueByPlacement(succ_id, placement);
        }
    }
    fn CurSuccId(&self) -> u16 { self.maestro.CurSuccId() }
    fn SetCurSuccId(&mut self, val: u16) { self.maestro.SetCurSuccId(val); }
}
