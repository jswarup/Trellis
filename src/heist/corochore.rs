use crate::heist::choretree::{ChoreNode, ChoreTarget, ErasedCoro};
use crate::stalks::coro::{Coro, CoroRes, CoroYielder, ICoro};
use crate::stalks::work::{IWorker, WorkPtr};

#[derive(Copy, Clone)]
pub struct WorkerFatPtr {
    pub _Ptr: *mut (dyn IWorker + 'static),
}
unsafe impl Send for WorkerFatPtr {}
unsafe impl Sync for WorkerFatPtr {}

pub struct CoroChore {
    pub _DocStr: &'static str,
    pub _Target: ChoreTarget,
    pub _Weight: u32,
    pub _Closure: fn(CoroYielder<'_, WorkerFatPtr, ()>, WorkerFatPtr),
}

impl CoroChore {
    pub fn New(f: fn(CoroYielder<'_, WorkerFatPtr, ()>, WorkerFatPtr)) -> Self {
        Self {
            _DocStr: "",
            _Target: ChoreTarget::Cpu,
            _Weight: 1,
            _Closure: f,
        }
    }
    pub fn NewDoc(
        docStr: &'static str,
        f: fn(CoroYielder<'_, WorkerFatPtr, ()>, WorkerFatPtr),
    ) -> Self {
        Self {
            _DocStr: docStr,
            _Target: ChoreTarget::Cpu,
            _Weight: 1,
            _Closure: f,
        }
    }
    pub fn WithWeight(mut self, weight: u32) -> Self {
        self._Weight = weight;
        self
    }
}

pub fn coro_job_func(mut coro: Coro<WorkerFatPtr, (), ()>, worker: &mut dyn IWorker) {
    let worker_ptr = WorkerFatPtr {
        _Ptr: unsafe {
            std::mem::transmute::<&mut dyn IWorker, *mut (dyn IWorker + 'static)>(worker)
        },
    };
    match coro.Resume(worker_ptr) {
        CoroRes::Yield(_) => {
            worker.PostJob(WorkPtr::FromClosure(move |w| {
                coro_job_func(coro, w);
            }));
        }
        CoroRes::Done(_) => {}
    }
}

impl Into<ChoreNode> for CoroChore {
    fn into(self) -> ChoreNode {
        ChoreNode::Coro(ErasedCoro {
            _DocStr: self._DocStr,
            _Closure: self._Closure,
        })
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
