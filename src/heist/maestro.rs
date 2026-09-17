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
pub struct Maestro {
    pub _Index: u32,
    pub _SzProcessed: AtomicU32,
    pub _CurSuccId: AtomicU16,
    pub _RunQueue: SpinMutex<Stash<u16>>,
    pub _TempQueue: SpinMutex<Stash<u16>>,
    pub _JobCache: SpinMutex<Stash<u16>>,
    pub _State: SpinMutex<Option<Weak<AtelierState>>>,
}
impl Maestro {
    pub fn New(index: u32) -> Self {
        Self {
            _Index: index,
            _SzProcessed: AtomicU32::new(0),
            _CurSuccId: AtomicU16::new(0),
            _RunQueue: SpinMutex::New(Stash::WithCapacity(1024)),
            _TempQueue: SpinMutex::New(Stash::WithCapacity(64)),
            _JobCache: SpinMutex::New(Stash::WithCapacity(256)),
            _State: SpinMutex::New(None),
        }
    }
    #[inline]
    pub fn Index(&self) -> u32 {
        self._Index
    }
    #[inline]
    pub fn MaestroIndex(&self) -> u32 {
        self._Index
    }
    #[inline]
    pub fn CurSuccId(&self) -> u16 {
        self._CurSuccId.load(Ordering::Acquire)
    }
    #[inline]
    pub fn SetCurSuccId(&self, val: u16) {
        self._CurSuccId.store(val, Ordering::Release);
    }
    pub fn SetState(&self, state: Weak<AtelierState>) {
        *self._State.Lock() = Some(state);
    }
    pub fn State(&self) -> Option<Arc<AtelierState>> {
        self._State.Lock().as_ref().and_then(|w| w.upgrade())
    }
    pub fn EnqueRunJob(&self, job_id: u16) {
        self._RunQueue.Lock().PushBack(job_id);
    }
    pub fn EnqueueJob(&self, job_id: u16) {
        self._TempQueue.Lock().PushBack(job_id);
    }
    pub fn PopJob(&self) -> u16 {
        self._RunQueue.Lock().Pop().unwrap_or(0)
    }
    pub fn FlushTempQueue(&self, state: &AtelierState) {
        let mut temp = self._TempQueue.Lock();
        let mut run_q = self._RunQueue.Lock();
        while let Some(id) = temp.Pop() {
            if id != 0 {
                state._SzSchedJob.fetch_add(1, Ordering::SeqCst);
                run_q.PushBack(id);
            }
        }
    }
    pub fn ConstructJob(&self, succ_id: u16, job: WorkPtr) -> u16 {
        if let Some(state) = self.State() {
            state.ConstructJob(self._Index, succ_id, job)
        } else {
            0
        }
    }
    pub fn ConstructEnqueArr(&self, succ_id: u16, buff: Buff<u16>) -> u16 {
        if let Some(state) = self.State() {
            state.ConstructEnqueArr(self._Index, succ_id, buff)
        } else {
            0
        }
    }
    pub fn PostJob(&self, job: WorkPtr) {
        if let Some(state) = self.State() {
            if state._SzThreads == 0 {
                let mut ctx = MaestroContext {
                    maestro: self,
                    state: &state,
                };
                let mut j = job;
                j.DoWork(&mut ctx);
                return;
            }
            let succ_id = self.CurSuccId();
            let job_id = state.ConstructJob(self._Index, succ_id, job);
            self.EnqueueJob(job_id);
        }
    }
    pub fn Post<F>(&self, f: F)
    where
        F: FnOnce(&mut dyn IWorker) + Send + 'static,
    {
        self.PostJob(WorkPtr::FromClosure(f));
    }
    pub fn PostChoreTree(&self, node: &ChoreNode) {
        let mut tails = Stash::WithCapacity(64);
        let head = PostChoreNode(node, self, &mut tails);
        if let Some(state) = self.State() {
            let succ_id = self.CurSuccId();
            while let Some(tail) = tails.Pop() {
                state.SetSucc(tail, succ_id);
            }
            self.EnqueueJob(head);
        }
    }
}
impl IWorker for Maestro {
    fn PostJob(&mut self, job: WorkPtr) {
        (*self).PostJob(job);
    }
    fn WorkerIndex(&self) -> u32 {
        self._Index
    }
    fn EnqueueJobId(&mut self, job_id: u16) {
        self.EnqueueJob(job_id);
    }
}

//-------------------------------------------------------------------------------------------------

// MaestroContext — borrowed execution context passed into WorkPtr::DoWork during loop execution.
pub struct MaestroContext<'a> {
    pub maestro: &'a Maestro,
    pub state: &'a AtelierState,
}
impl<'a> IWorker for MaestroContext<'a> {
    fn PostJob(&mut self, mut job: WorkPtr) {
        if self.state._SzThreads == 0 {
            job.DoWork(self);
            return;
        }
        let succ_id = self.maestro.CurSuccId();
        let job_id = self.state.ConstructJob(self.maestro.Index(), succ_id, job);
        self.maestro.EnqueueJob(job_id);
    }
    fn WorkerIndex(&self) -> u32 {
        self.maestro.Index()
    }
    fn EnqueueJobId(&mut self, job_id: u16) {
        self.maestro.EnqueueJob(job_id);
    }
}
