//-- worker.rs -------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::heist::scheduler::PriorityScheduler;
use	std::sync::Arc;

//--------------------------------------------------------------------------------------------------
/// Worker thread context executing tasks scheduled across multi-priority queues.
pub struct PriorityWorker
{
    _WorkerIndex:    u32,
    _Scheduler:      Arc< PriorityScheduler>,
    _ExecutedCount:  u32,
    _CancelledCount: u32,
}

impl PriorityWorker
{
    pub fn	New( workerIndex: u32, scheduler: Arc< PriorityScheduler>) -> Self
    {
        Self {
            _WorkerIndex:    workerIndex,
            _Scheduler:      scheduler,
            _ExecutedCount:  0,
            _CancelledCount: 0,
        }
    }

    #[inline]
    pub const fn	WorkerIndex( &self) -> u32
    {
        self._WorkerIndex
    }

    #[inline]
    pub const fn	ExecutedCount( &self) -> u32
    {
        self._ExecutedCount
    }

    #[inline]
    pub const fn	CancelledCount( &self) -> u32
    {
        self._CancelledCount
    }

    /// Attempts to steal and execute a single job. Returns true if work was attempted.
    pub fn	StepOne( &mut self) -> bool
    {
        let  	Some( job) = self._Scheduler.StealNext( self._WorkerIndex) else {
            return false;
        };
        if job.Execute()
        {
            self._ExecutedCount += 1;
        }
        else
        {
            self._CancelledCount += 1;
        }
        true
    }

    /// Runs jobs continuously until no pending or stealable jobs remain.
    pub fn	RunUntilEmpty( &mut self) -> u32
    {
        let  	startCount = self._ExecutedCount;
        while self.StepOne() {}
        self._ExecutedCount - startCount
    }
}
