// atelier.rs ------------------------------------------------------------------------------------------------------
use	crate::heist::maestro::{ Maestro, MaestroContext };
use	crate::silo::USeg;
use	crate::silo::arr::{ Arr, MutArr };
use	crate::silo::buff::Buff;
use	crate::silo::stash::Stash;
use	crate::stalks::work::{ SpinMutex, Spinlock, WorkPtr };
use	std::sync::atomic::{ AtomicU16, AtomicU32, Ordering };
use	std::sync::{ Arc, RwLock };
pub const K_JOB_CAPACITY: usize = 65536;
static LIFECYCLE_LOCK: Spinlock = Spinlock::New();
static GLOBAL_ATELIER: RwLock< Option< Arc< Atelier>>> = RwLock::new( None);

//-------------------------------------------------------------------------------------------------
// AtelierState — internal state shared across worker threads and Maestros.
pub struct AtelierState
{
    pub _SzThreads: u32,
    pub _SzSchedJob: AtomicU32,
    pub _SzSchedRunnables: AtomicU32,
    pub _SzLaunchWorkersStarted: AtomicU32,
    pub _SzPreds: Buff< AtomicU16>,
    pub _SuccIds: Buff< AtomicU16>,
    pub _JobPlacements: Buff< AtomicU16>,
    pub _JobBuff: Buff< SpinMutex< Option< WorkPtr>>>,
    pub _FreeJobStash: SpinMutex< Stash< u16>>,
    pub _Terminal: AtomicU16,
    pub _Maestros: Buff< Maestro>,
}
impl AtelierState
{
    pub fn	New( threads: u32) -> Arc< Self>
    {
        let  	maestro_count = if threads == 0 { 1 } else { threads };
        let  	maestros = Buff::FromDispenser( maestro_count, Maestro::New);
        let  	sz_preds = Buff::FromDispenser( K_JOB_CAPACITY as u32, |_| AtomicU16::new( 0));
        let  	succ_ids = Buff::FromDispenser( K_JOB_CAPACITY as u32, |_| AtomicU16::new( 0));
        let  	job_placements = Buff::FromDispenser( K_JOB_CAPACITY as u32, |_| AtomicU16::new( 0));
        let  	job_buff = Buff::FromDispenser( K_JOB_CAPACITY as u32, |_| SpinMutex::New( None));
        let  	mut free_stash = Stash::WithCapacity( K_JOB_CAPACITY as u32);
        for i in 1..K_JOB_CAPACITY as u32 {
            free_stash.PushBack( i as u16);
        }
        let  	state = Arc::new( Self {
            _SzThreads: threads,
            _SzSchedJob: AtomicU32::new( 0),
            _SzSchedRunnables: AtomicU32::new( 0),
            _SzLaunchWorkersStarted: AtomicU32::new( 0),
            _SzPreds: sz_preds,
            _SuccIds: succ_ids,
            _JobPlacements: job_placements,
            _JobBuff: job_buff,
            _FreeJobStash: SpinMutex::New( free_stash),
            _Terminal: AtomicU16::new( 0),
            _Maestros: maestros,
        });
        state._Maestros.Traverse( |m| {
            m.SetState( Arc::downgrade( &state));
        });
        let  	terminal = state.ConstructJob( 0, 0, WorkPtr::FromClosure( |_| {}));
        state._Terminal.store( terminal, Ordering::SeqCst);
        state._Maestros[0].SetCurSuccId( terminal);
        state
    }
    #[inline]
    pub fn	Terminal( &self) -> u16
    {
        self._Terminal.load( Ordering::Relaxed)
    }
    pub fn	AllocJob( &self, maestro_idx: u32) -> u16
    {
        let  	maestro = &self._Maestros[maestro_idx];
        if let  	Some( id) = maestro._JobCache.Lock().Pop() {
            return id;
        }
        let  	mut free = self._FreeJobStash.Lock();
        if free.Size() == 0 {
            return 0;
        }
        let  	mut cache = maestro._JobCache.Lock();
        let  	count = free.Size().min( 64);
        USeg::FromLen( count).Traverse( |_| {
            if let  	Some( id) = free.Pop() {
                cache.PushBack( id);
            }
        });
        cache.Pop().unwrap_or( 0)
    }
    #[inline]
    pub fn	TryAllocJob( &self, maestro_idx: u32) -> Option< u16>
    {
        let  	id = self.AllocJob( maestro_idx);
        if id != 0 { Some( id) } else { None }
    }
    pub fn	ResetJobSlot( &self, job_id: u16)
    {
        self._SuccIds[job_id as u32].store( 0, Ordering::SeqCst);
        self._SzPreds[job_id as u32].store( 0, Ordering::SeqCst);
        self._JobPlacements[job_id as u32].store( 0, Ordering::SeqCst);
        *self._JobBuff[job_id as u32].Lock() = None;
    }
    #[inline]
    pub fn	SetJobPlacement( &self, job_id: u16, placement: crate::heist::placement::ChorePlacement)
    {
        self._JobPlacements[job_id as u32].store( placement.ToPackedU16(), Ordering::SeqCst);
    }
    #[inline]
    pub fn	GetJobPlacement( &self, job_id: u16) -> crate::heist::placement::ChorePlacement
    {
        crate::heist::placement::ChorePlacement::FromPackedU16(
            self._JobPlacements[job_id as u32].load( Ordering::SeqCst),
        )
    }
    pub fn	EnqueueByPlacement( &self, job_id: u16, placement: crate::heist::placement::ChorePlacement)
    {
        self._SzSchedJob.fetch_add( 1, Ordering::SeqCst);
        let  	target_worker = placement.TargetWorker().unwrap_or( 0);
        let  	target_idx = if target_worker < self._SzThreads.max( 1) { target_worker } else { 0 };
        if placement.IsRequired() {
            self._Maestros[target_idx].EnqueueRequiredJob( job_id);
        } else {
            self._Maestros[target_idx].EnqueRunJob( job_id);
            self._SzSchedRunnables.fetch_add( 1, Ordering::SeqCst);
        }
    }
    pub fn	FreeJob( &self, maestro_idx: u32, job_id: u16)
    {
        self.ResetJobSlot( job_id);
        let  	maestro = &self._Maestros[maestro_idx];
        maestro.FlushTempQueue( self);
        let  	mut cache = maestro._JobCache.Lock();
        if cache.Size() < 256 {
            cache.PushBack( job_id);
        } else {
            let  	mut free = self._FreeJobStash.Lock();
            free.PushBack( job_id);
        }
    }
    pub fn	SetSucc( &self, job_id: u16, succ_id: u16)
    {
        self._SuccIds[job_id as u32].store( succ_id, Ordering::SeqCst);
        self._SzPreds[succ_id as u32].fetch_add( 1, Ordering::SeqCst);
    }
    pub fn	ConstructJob( &self, maestro_idx: u32, succ_id: u16, job: WorkPtr) -> u16
    {
        let  	job_id = self.AllocJob( maestro_idx);
        if job_id == 0 {
            return 0;
        }
        self.ResetJobSlot( job_id);
        *self._JobBuff[job_id as u32].Lock() = Some( job);
        if succ_id != 0 {
            self.SetSucc( job_id, succ_id);
        }
        job_id
    }
    pub fn	TryConstructJob( &self, maestro_idx: u32, succ_id: u16, job: WorkPtr) -> Option< u16>
    {
        let  	job_id = self.TryAllocJob( maestro_idx)?;
        self.ResetJobSlot( job_id);
        *self._JobBuff[job_id as u32].Lock() = Some( job);
        if succ_id != 0 {
            self.SetSucc( job_id, succ_id);
        }
        Some( job_id)
    }
    pub fn	ConstructEnqueArr( &self, maestro_idx: u32, succ_id: u16, buff: Buff< u16>) -> u16
    {
        self.ConstructJob(
            maestro_idx,
            succ_id,
            WorkPtr::FromClosure( move |worker| {
                for i in 0..buff.Cap() {
                    let  	job_id = buff[i];
                    if job_id != 0 {
                        worker.EnqueueJobId( job_id);
                    }
                }
            }),
        )
    }
    pub fn	GrabJob( &self, idx: u32, steal_seed: &mut u32) -> u16
    {
        let  	thief = &self._Maestros[idx];
        thief._SzStealAttempts.fetch_add( 1, Ordering::Relaxed);
        let  	sz = self._Maestros.Len();
        const KNUTH_MULT_HASH: u32 = 2654435761;
        *steal_seed = steal_seed.wrapping_mul( KNUTH_MULT_HASH).wrapping_add( 1);
        for m_idx in 0..sz {
            let  	maestro_idx = ( *steal_seed + m_idx) % sz;
            if maestro_idx == idx {
                continue;
            }
            let  	job_id = self._Maestros[maestro_idx].PopStealJob();
            if job_id != 0 {
                thief._SzStealSuccesses.fetch_add( 1, Ordering::Relaxed);
                return job_id;
            }
        }
        0
    }
    pub fn	ExecuteLoop( state: &Arc< AtelierState>, maestro_idx: u32)
    {
        let  	maestro = &state._Maestros[maestro_idx];
        if maestro_idx != 0 {
            state._SzLaunchWorkersStarted.fetch_add( 1, Ordering::Release);
        }
        maestro.FlushTempQueue( state);
        let  	mut job_id = 0u16;
        let  	mut steal_seed = maestro_idx;
        while state._SzSchedJob.load( Ordering::Acquire) != 0 {
            while job_id != 0 {
                let  	is_required = state.GetJobPlacement( job_id).IsRequired();
                let  	succ_id = state._SuccIds[job_id as u32].load( Ordering::Acquire);
                maestro.SetCurSuccId( succ_id);
                let  	job_opt = state._JobBuff[job_id as u32].Lock().take();
                if let  	Some( mut job) = job_opt {
                    let  	mut ctx = MaestroContext { maestro, state };
                    job.DoWork( &mut ctx);
                }
                maestro._SzProcessed.fetch_add( 1, Ordering::Relaxed);
                state.FreeJob( maestro_idx, job_id);
                let  	succ_id = maestro.CurSuccId();
                if succ_id != 0 {
                    let  	prev_pred = state._SzPreds[succ_id as u32].fetch_sub( 1, Ordering::SeqCst);
                    if prev_pred == 1 {
                        let  	placement = state.GetJobPlacement( succ_id);
                        if placement.CanExecuteOn( maestro_idx) {
                            job_id = succ_id;
                            state._SzSchedJob.fetch_add( 1, Ordering::SeqCst);
                            if !placement.IsRequired() {
                                state._SzSchedRunnables.fetch_add( 1, Ordering::SeqCst);
                            }
                        } else {
                            state.EnqueueByPlacement( succ_id, placement);
                            job_id = 0;
                        }
                    } else {
                        job_id = 0;
                    }
                } else {
                    job_id = 0;
                }
                if !is_required {
                    state._SzSchedRunnables.fetch_sub( 1, Ordering::SeqCst);
                }
                state._SzSchedJob.fetch_sub( 1, Ordering::SeqCst);
            }
            job_id = maestro.PopRunJob();
            if job_id == 0 && state._SzThreads >= 2 {
                job_id = state.GrabJob( maestro_idx, &mut steal_seed);
            }
            if job_id == 0 && state._SzSchedRunnables.load( Ordering::Acquire) == 0 {
                job_id = maestro.PopRequiredJob();
            }
            if job_id == 0 {
                maestro._SzYields.fetch_add( 1, Ordering::Relaxed);
                std::hint::spin_loop();
                std::thread::yield_now();
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Atelier — singleton work-stealing job pool, DAG dependency resolver, and multi-threaded orchestrator.
// Modeled directly from Trellis heist/atelier.h.
pub struct Atelier
{
    pub state: Arc< AtelierState>,
    _LaunchLock: Spinlock,
}
impl Atelier
{
    pub fn	New( threads: u32) -> Self
    {
        Self {
            state: AtelierState::New( threads),
            _LaunchLock: Spinlock::New(),
        }
    }
    #[inline]
    pub fn	IsImmediate( &self) -> bool
    {
        self.state._SzThreads == 0
    }
    #[inline]
    pub fn	SzThreads( &self) -> u32
    {
        self.state._SzThreads
    }
    #[inline]
    pub fn	MainMaestro( &self) -> &Maestro
    {
        &self.state._Maestros[0]
    }
    #[inline]
    pub fn	Maestros( &self) -> Arr< '_, Maestro> {
        self.state._Maestros.Arr()
    }
    #[inline]
    pub fn	Terminal( &self) -> u16
    {
        self.state.Terminal()
    }
    #[inline]
    pub fn	TryAllocJob( &self, maestro_idx: u32) -> Option< u16>
    {
        self.state.TryAllocJob( maestro_idx)
    }
    pub fn	TotalStealAttempts( &self) -> u32
    {
        let  	mut total = 0;
        let  	maestros = self.Maestros();
        for i in 0..maestros.Len() {
            total += maestros[i].StealAttempts();
        }
        total
    }
    pub fn	TotalStealSuccesses( &self) -> u32
    {
        let  	mut total = 0;
        let  	maestros = self.Maestros();
        for i in 0..maestros.Len() {
            total += maestros[i].StealSuccesses();
        }
        total
    }
    /// Execute bounded range partitions within a caller-owned thread scope.
    /// The action may borrow caller storage because every spawned task joins
    /// before this method returns. This does not use the legacy WorkPtr queue.
    pub fn	ForEachScopedRange< F>( &self, total: u32, action: F)
    where
        F: Fn( u32, u32, u32) + Sync,
    {
        if total == 0 {
            return;
        }
        let  	worker_count = self.state._SzThreads.max( 1).min( total);
        let  	chunk_size = total.div_ceil( worker_count);
        std::thread::scope( |scope| {
            let  	action = &action;
            for worker in 0..worker_count.saturating_sub( 1) {
                let  	start = worker * chunk_size;
                let  	end = ( start + chunk_size).min( total);
                scope.spawn( move || action( worker, start, end));
            }
            let  	worker = worker_count - 1;
            let  	start = worker * chunk_size;
            let  	end = ( start + chunk_size).min( total);
            action( worker, start, end);
        });
    }
    /// Transfer disjoint mutable partitions to a bounded caller-owned scope.
    /// The views are moved, never aliased, and all workers join before return.
    pub fn	ForEachScopedMut< 'a, T, F>( &self, values: MutArr< 'a, T>, action: F)
    where
        T: Send,
        F: Fn( u32, MutArr< 'a, T>) + Sync,
    {
        if values.IsEmpty() {
            return;
        }
        let  	worker_count = self.state._SzThreads.max( 1).min( values.Len());
        let  	mut partitions = Stash::WithCapacity( worker_count);
        partitions.Push( values);
        while partitions.Size() < worker_count {
            let  	next = partitions.Pop().expect( "scoped mutable partition is missing");
            let  	count = next.Len().div_ceil( 2);
            let  	( left, right) = next.SplitAt( count);
            partitions.Push( left);
            partitions.Push( right);
        }
        let  	partitions = SpinMutex::New( partitions);
        self.ForEachScopedRange( worker_count, |worker, _start, _end| {
            let  	partition = partitions.Lock().Pop()
                .expect( "scoped mutable partition is missing for worker");
            action( worker, partition);
        });
    }
    pub fn	DoLaunch( &self)
    {
        let  	_guard = self._LaunchLock.Lock();
        if self.state._SzThreads == 0 {
            return;
        }
        let  	state = &self.state;
        for i in 0..state._Maestros.Len() {
            state._Maestros[i].FlushTempQueue( state);
        }
        if state._SzThreads == 1 {
            AtelierState::ExecuteLoop( state, 0);
            return;
        }
        let  	worker_count = state._Maestros.Len().saturating_sub( 1);
        state._SzLaunchWorkersStarted.store( 0, Ordering::Release);
        let  	mut handles = Buff::FromDispenser( worker_count, |i| {
            let  	s_clone = state.clone();
            let  	idx = i + 1;
            Some( std::thread::spawn( move || {
                AtelierState::ExecuteLoop( &s_clone, idx);
            }))
        });
        while state._SzLaunchWorkersStarted.load( Ordering::Acquire) < worker_count {
            std::hint::spin_loop();
            std::thread::yield_now();
        }
        AtelierState::ExecuteLoop( state, 0);
        handles.TraverseMut( |h| {
            if let  	Some( handle) = h.take() {
                let  	_ = handle.join();
            }
        });
    }
    pub fn	DefaultThreadCount() -> u32
    {
        std::thread::available_parallelism()
            .map( |n| n.get() as u32)
            .unwrap_or( 1)
    }
    pub fn	LifecycleLock() -> &'static Spinlock {
        &LIFECYCLE_LOCK
    }
    pub fn	Reset( threads: u32) -> Arc< Atelier>
    {
        let  	_guard = Self::LifecycleLock().Lock();
        let  	mut write_guard = GLOBAL_ATELIER.write().unwrap();
        let  	inst = Arc::new( Atelier::New( threads));
        *write_guard = Some( inst.clone());
        inst
    }
    pub fn	Boot( threads: u32) -> Arc< Atelier>
    {
        Self::Reset( threads)
    }
    pub fn	Instance() -> Arc< Atelier>
    {
        {
            let  	read_guard = GLOBAL_ATELIER.read().unwrap();
            if let  	Some( ref inst) = *read_guard {
                return inst.clone();
            }
        }
        let  	mut write_guard = GLOBAL_ATELIER.write().unwrap();
        if let  	Some( ref inst) = *write_guard {
            return inst.clone();
        }
        let  	inst = Arc::new( Atelier::New( Self::DefaultThreadCount()));
        *write_guard = Some( inst.clone());
        inst
    }
}
