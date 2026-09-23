use crate::heist::choretree::{ChoreNode, ChoreTarget, ErasedCoro};
use crate::heist::placement::ChorePlacement;
use crate::stalks::coro::{Coro, CoroRes, CoroYielder, ICoro};
use crate::stalks::work::{IWorker, WorkPtr};
#[derive(Copy, Clone)]
pub struct WorkerFatPtr
{
    pub _Ptr: *mut (dyn IWorker + 'static),
}
unsafe impl Send for WorkerFatPtr {}
unsafe impl Sync for WorkerFatPtr {}
pub struct CoroChore
{
    pub _DocStr:    &'static str,
    pub _Target:    ChoreTarget,
    pub _Weight:    u32,
    pub _Placement: ChorePlacement,
    pub _Closure:   fn(CoroYielder<'_, WorkerFatPtr, ()>, WorkerFatPtr),
}
impl CoroChore
{
    pub fn New(f: fn(CoroYielder<'_, WorkerFatPtr, ()>, WorkerFatPtr)) -> Self
    {
        Self { _DocStr:    "",
               _Target:    ChoreTarget::Cpu,
               _Weight:    1,
               _Placement: ChorePlacement::Any,
               _Closure:   f, }
    }
    pub fn NewDoc(docStr: &'static str, f: fn(CoroYielder<'_, WorkerFatPtr, ()>, WorkerFatPtr))
                  -> Self
    {
        Self { _DocStr:    docStr,
               _Target:    ChoreTarget::Cpu,
               _Weight:    1,
               _Placement: ChorePlacement::Any,
               _Closure:   f, }
    }
    pub fn WithWeight(mut self, weight: u32) -> Self
    {
        self._Weight = weight;
        self
    }
    pub fn Require(mut self, worker: u32) -> Self
    {
        self._Placement = ChorePlacement::Require(worker);
        self
    }
    pub fn Prefer(mut self, worker: u32) -> Self
    {
        self._Placement = ChorePlacement::Prefer(worker);
        self
    }
    pub fn Any(mut self) -> Self
    {
        self._Placement = ChorePlacement::Any;
        self
    }
    pub fn WithPlacement(mut self, placement: ChorePlacement) -> Self
    {
        self._Placement = placement;
        self
    }
    pub fn Then(self, other: impl Into<ChoreNode>) -> ChoreNode
    {
        ChoreNode::from(self).Then(other)
    }
    pub fn Par(self, other: impl Into<ChoreNode>) -> ChoreNode { ChoreNode::from(self).Par(other) }
}
pub fn coro_job_func(mut coro: Coro<WorkerFatPtr, (), ()>, placement: ChorePlacement,
                     mut stored_succ_id: u16, worker: &mut dyn IWorker)
{
    if stored_succ_id == 0 {
        stored_succ_id = worker.CurSuccId();
    }
    let worker_ptr = WorkerFatPtr { _Ptr: unsafe {
                                        std::mem::transmute::<&mut dyn IWorker,
                                                            *mut (dyn IWorker + 'static)>(worker)
                                    }, };
    match coro.Resume(worker_ptr) {
        CoroRes::Yield(_) => {
            // Prevent ExecuteLoop from triggering successor prematurely upon yield
            worker.SetCurSuccId(0);
            worker.PostJobWithPlacement(WorkPtr::FromClosure(move |w| {
                                            coro_job_func(coro, placement, stored_succ_id, w);
                                        }),
                                        placement.ToPackedU16(),
                                        stored_succ_id);
        }
        CoroRes::Done(_) => {
            if stored_succ_id != 0 {
                worker.PublishSuccessor(stored_succ_id);
                worker.SetCurSuccId(0);
            }
        }
    }
}
impl From<CoroChore> for ChoreNode
{
    fn from(val: CoroChore) -> Self
    {
        ChoreNode::Coro(ErasedCoro { _DocStr:    val._DocStr,
                                     _Placement: val._Placement,
                                     _Closure:   val._Closure, })
    }
}
#[macro_export]
macro_rules! CoroChore {
    ( |$yielder:ident, $input:ident| $body:expr ) => {
        $crate::heist::corochore::CoroChore::New(|$yielder, $input| $body)
    };
    ( |$yielder:ident| $body:expr ) => {
        $crate::heist::corochore::CoroChore::New(|$yielder, _| $body)
    };
}
