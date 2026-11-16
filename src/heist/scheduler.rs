//-- scheduler.rs ----------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::heist::token::{ CancellationToken, PreemptionEpoch };
use	crate::silo::buff::Buff;
use	crate::silo::stash::Stash;
use	crate::silo::useg::USeg;
use	crate::stalks::work::SpinMutex;

pub const K_PRIORITY_COUNT: usize = 4;

//--------------------------------------------------------------------------------------------------
/// Priority classification for scheduled work execution.
#[derive( Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority
{
    High       = 0,
    Normal     = 1,
    Low        = 2,
    Background = 3,
}

impl JobPriority
{
    #[inline]
    pub const fn	Index( &self) -> usize
    {
        *self as usize
    }

    pub const fn	FromIndex( idx: usize) -> Self
    {
        match idx
        {
            0 => Self::High,
            1 => Self::Normal,
            2 => Self::Low,
            _ => Self::Background,
        }
    }
}

//--------------------------------------------------------------------------------------------------
/// A unit of work managed by the priority work-stealing scheduler.
pub struct PriorityJob
{
    _Id:       u64,
    _Priority: JobPriority,
    _Epoch:    u64,
    _Token:    Option< CancellationToken>,
    _Task:     Box< dyn FnOnce() + Send + 'static>,
}

impl PriorityJob
{
    pub fn	New< F>( id: u64, priority: JobPriority, task: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            _Id:       id,
            _Priority: priority,
            _Epoch:    0,
            _Token:    None,
            _Task:     Box::new( task),
        }
    }

    pub fn	WithToken< F>(
        id: u64, priority: JobPriority, token: CancellationToken, task: F,
    ) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let  	epoch = token.Epoch();
        Self {
            _Id:       id,
            _Priority: priority,
            _Epoch:    epoch,
            _Token:    Some( token),
            _Task:     Box::new( task),
        }
    }

    #[inline]
    pub const fn	Id( &self) -> u64
    {
        self._Id
    }

    #[inline]
    pub const fn	Priority( &self) -> JobPriority
    {
        self._Priority
    }

    #[inline]
    pub const fn	Epoch( &self) -> u64
    {
        self._Epoch
    }

    #[inline]
    pub fn	Token( &self) -> Option< &CancellationToken>
    {
        self._Token.as_ref()
    }

    /// Executes the job if not cancelled. Returns true if executed.
    pub fn	Execute( self) -> bool
    {
        if let Some( token) = &self._Token
            && token.ShouldPreempt( self._Epoch)
        {
            return false;
        }
        ( self._Task)();
        true
    }
}

//--------------------------------------------------------------------------------------------------
/// Multi-priority work-stealing scheduler supporting priority queues and preemption.
pub struct PriorityScheduler
{
    _Queues:      Buff< [SpinMutex< Stash< PriorityJob>>; K_PRIORITY_COUNT]>,
    _WorkerCount: u32,
    _Epoch:       PreemptionEpoch,
}

impl PriorityScheduler
{
    pub fn	New( workerCount: u32) -> Self
    {
        let  	count = workerCount.max( 1);
        let  	mut stash = Stash::WithCapacity( count);
        let  	mut i = 0u32;
        while i < count
        {
            stash.Push( [
                SpinMutex::New( Stash::WithCapacity( 64)),
                SpinMutex::New( Stash::WithCapacity( 128)),
                SpinMutex::New( Stash::WithCapacity( 64)),
                SpinMutex::New( Stash::WithCapacity( 32)),
            ]);
            i += 1;
        }
        Self {
            _Queues:      stash.IntoBuff(),
            _WorkerCount: count,
            _Epoch:       PreemptionEpoch::New( 0),
        }
    }

    #[inline]
    pub const fn	WorkerCount( &self) -> u32
    {
        self._WorkerCount
    }

    #[inline]
    pub fn	Epoch( &self) -> &PreemptionEpoch
    {
        &self._Epoch
    }

    /// Submits a job to a target worker queue based on priority.
    pub fn	Submit( &self, job: PriorityJob, preferredWorker: Option< u32>)
    {
        let  	workerIdx = preferredWorker.unwrap_or( 0) % self._WorkerCount;
        let  	prioIdx = job.Priority().Index();
        self._Queues[workerIdx][prioIdx].Lock().Push( job);
    }

    /// Steals or retrieves the next highest-priority job for the calling worker.
    pub fn	StealNext( &self, workerIndex: u32) -> Option< PriorityJob>
    {
        let  	wIdx = workerIndex % self._WorkerCount;

        // 1. Try worker's own queues in descending priority (High -> Background)
        let  	mut p = 0usize;
        while p < K_PRIORITY_COUNT
        {
            let  	mut q = self._Queues[wIdx][p].Lock();
            if let Some( job) = q.Pop()
            {
                return Some( job);
            }
            p += 1;
        }

        // 2. Priority-aware work stealing from other workers (High -> Background)
        let  	mut prioLevel = 0usize;
        while prioLevel < K_PRIORITY_COUNT
        {
            let  	mut victimOffset = 1u32;
            while victimOffset < self._WorkerCount
            {
                let  	victimIdx = ( wIdx + victimOffset) % self._WorkerCount;
                let  	mut q = self._Queues[victimIdx][prioLevel].Lock();
                if let Some( stolenJob) = q.Pop()
                {
                    return Some( stolenJob);
                }
                victimOffset += 1;
            }
            prioLevel += 1;
        }

        None
    }

    /// Total count of pending jobs across all priority levels.
    pub fn	PendingCount( &self) -> u32
    {
        let  	mut total = 0u32;
        USeg::FromLen( self._WorkerCount).Traverse( |w| {
            let  	mut p = 0usize;
            while p < K_PRIORITY_COUNT
            {
                total += self._Queues[w][p].Lock().Size();
                p += 1;
            }
        });
        total
    }

    /// Count of pending jobs for a specific priority tier.
    pub fn	PendingCountForPriority( &self, prio: JobPriority) -> u32
    {
        let  	pIdx = prio.Index();
        let  	mut count = 0u32;
        USeg::FromLen( self._WorkerCount).Traverse( |w| {
            count += self._Queues[w][pIdx].Lock().Size();
        });
        count
    }
}
