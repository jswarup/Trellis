// atelier.rs ------------------------------------------------------------------------------------------------------

use crate::heist::maestro::{Maestro, MaestroContext};
use crate::silo::buff::Buff;
use crate::silo::stash::Stash;
use crate::stalks::work::{SpinMutex, Spinlock, WorkPtr};
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

pub const K_JOB_CAPACITY: usize = 65536;

static LIFECYCLE_LOCK: Spinlock = Spinlock::New();
static GLOBAL_ATELIER: RwLock<Option<Arc<Atelier>>> = RwLock::new(None);

//-------------------------------------------------------------------------------------------------
// AtelierState — internal state shared across worker threads and Maestros.

pub struct AtelierState {
    pub _SzThreads: u32,
    pub _SzSchedJob: AtomicU32,
    pub _SzPreds: Vec<AtomicU16>,
    pub _SuccIds: Vec<AtomicU16>,
    pub _JobBuff: Vec<SpinMutex<Option<WorkPtr>>>,
    pub _FreeJobStash: SpinMutex<Stash<u16>>,
    pub _Terminal: AtomicU16,
    pub _Maestros: Vec<Maestro>,
}

impl AtelierState {
    pub fn New(threads: u32) -> Arc<Self> {
        let maestro_count = if threads == 0 { 1 } else { threads as usize };
        let mut maestros = Vec::with_capacity(maestro_count);
        for i in 0..maestro_count {
            maestros.push(Maestro::New(i as u32));
        }

        let mut sz_preds = Vec::with_capacity(K_JOB_CAPACITY);
        let mut succ_ids = Vec::with_capacity(K_JOB_CAPACITY);
        let mut job_buff = Vec::with_capacity(K_JOB_CAPACITY);
        for _ in 0..K_JOB_CAPACITY {
            sz_preds.push(AtomicU16::new(0));
            succ_ids.push(AtomicU16::new(0));
            job_buff.push(SpinMutex::New(None));
        }

        let mut free_stash = Stash::WithCapacity(K_JOB_CAPACITY as u32);
        for i in 1..K_JOB_CAPACITY as u32 {
            free_stash.PushBack(i as u16);
        }

        let state = Arc::new(Self {
            _SzThreads: threads,
            _SzSchedJob: AtomicU32::new(0),
            _SzPreds: sz_preds,
            _SuccIds: succ_ids,
            _JobBuff: job_buff,
            _FreeJobStash: SpinMutex::New(free_stash),
            _Terminal: AtomicU16::new(0),
            _Maestros: maestros,
        });

        for m in &state._Maestros {
            m.SetState(Arc::downgrade(&state));
        }

        let terminal = state.ConstructJob(0, 0, WorkPtr::FromClosure(|_| {}));
        state._Terminal.store(terminal, Ordering::SeqCst);
        state._Maestros[0].SetCurSuccId(terminal);

        state
    }

    #[inline]
    pub fn Terminal(&self) -> u16 {
        self._Terminal.load(Ordering::Relaxed)
    }

    pub fn AllocJob(&self, maestro_idx: u32) -> u16 {
        let maestro = &self._Maestros[maestro_idx as usize];
        loop {
            if let Some(id) = maestro._JobCache.Lock().Pop() {
                return id;
            }

            let mut free = self._FreeJobStash.Lock();
            if free.Size() == 0 {
                drop(free);
                std::hint::spin_loop();
                std::thread::yield_now();
                continue;
            }

            let mut cache = maestro._JobCache.Lock();
            let count = free.Size().min(64);
            for _ in 0..count {
                if let Some(id) = free.Pop() {
                    cache.PushBack(id);
                }
            }
        }
    }

    pub fn FreeJob(&self, maestro_idx: u32, job_id: u16) {
        let maestro = &self._Maestros[maestro_idx as usize];
        maestro.FlushTempQueue(self);
        let mut cache = maestro._JobCache.Lock();
        if cache.Size() < 256 {
            cache.PushBack(job_id);
        } else {
            let mut free = self._FreeJobStash.Lock();
            free.PushBack(job_id);
        }
    }

    pub fn SetSucc(&self, job_id: u16, succ_id: u16) {
        self._SuccIds[job_id as usize].store(succ_id, Ordering::SeqCst);
        self._SzPreds[succ_id as usize].fetch_add(1, Ordering::SeqCst);
    }

    pub fn ConstructJob(&self, maestro_idx: u32, succ_id: u16, job: WorkPtr) -> u16 {
        let job_id = self.AllocJob(maestro_idx);
        if job_id == 0 {
            return 0;
        }
        *self._JobBuff[job_id as usize].Lock() = Some(job);
        if succ_id != 0 {
            self.SetSucc(job_id, succ_id);
        }
        job_id
    }

    pub fn ConstructEnqueArr(&self, maestro_idx: u32, succ_id: u16, buff: Buff<u16>) -> u16 {
        self.ConstructJob(
            maestro_idx,
            succ_id,
            WorkPtr::FromClosure(move |worker| {
                for i in 0..buff.Cap() {
                    let job_id = buff[i];
                    if job_id != 0 {
                        worker.EnqueueJobId(job_id);
                    }
                }
            }),
        )
    }

    pub fn GrabJob(&self, idx: u32, steal_seed: &mut u32) -> u16 {
        let sz = self._Maestros.len() as u32;
        const KNUTH_MULT_HASH: u32 = 2654435761;
        *steal_seed = steal_seed.wrapping_mul(KNUTH_MULT_HASH).wrapping_add(1);
        for m_idx in 0..sz {
            let maestro_idx = (*steal_seed + m_idx) % sz;
            if maestro_idx == idx {
                continue;
            }
            let job_id = self._Maestros[maestro_idx as usize].PopJob();
            if job_id != 0 {
                return job_id;
            }
        }
        0
    }

    pub fn ExecuteLoop(state: &Arc<AtelierState>, maestro_idx: u32) {
        let maestro = &state._Maestros[maestro_idx as usize];
        maestro.FlushTempQueue(state);
        let mut job_id = 0u16;
        let mut steal_seed = maestro_idx;

        while state._SzSchedJob.load(Ordering::Acquire) != 0 {
            while job_id != 0 {
                let succ_id = state._SuccIds[job_id as usize].load(Ordering::Acquire);
                maestro.SetCurSuccId(succ_id);

                let job_opt = state._JobBuff[job_id as usize].Lock().take();
                if let Some(mut job) = job_opt {
                    let mut ctx = MaestroContext { maestro, state };
                    job.DoWork(&mut ctx);
                }

                maestro._SzProcessed.fetch_add(1, Ordering::Relaxed);
                state.FreeJob(maestro_idx, job_id);

                let succ_id = maestro.CurSuccId();
                if succ_id != 0 {
                    let prev_pred = state._SzPreds[succ_id as usize].fetch_sub(1, Ordering::SeqCst);
                    if prev_pred == 1 {
                        job_id = succ_id;
                        state._SzSchedJob.fetch_add(1, Ordering::SeqCst);
                    } else {
                        job_id = 0;
                    }
                } else {
                    job_id = 0;
                }
                state._SzSchedJob.fetch_sub(1, Ordering::SeqCst);
            }

            job_id = maestro.PopJob();
            if job_id == 0 && state._SzThreads >= 2 {
                job_id = state.GrabJob(maestro_idx, &mut steal_seed);
            }
            if job_id == 0 {
                std::hint::spin_loop();
                std::thread::yield_now();
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Atelier — singleton work-stealing job pool, DAG dependency resolver, and multi-threaded orchestrator.
// Modeled directly from Trellis heist/atelier.h.

pub struct Atelier {
    pub state: Arc<AtelierState>,
}

impl Atelier {
    pub fn New(threads: u32) -> Self {
        Self {
            state: AtelierState::New(threads),
        }
    }

    #[inline]
    pub fn IsImmediate(&self) -> bool {
        self.state._SzThreads == 0
    }

    #[inline]
    pub fn SzThreads(&self) -> u32 {
        self.state._SzThreads
    }

    #[inline]
    pub fn MainMaestro(&self) -> &Maestro {
        &self.state._Maestros[0]
    }

    #[inline]
    pub fn Maestros(&self) -> &[Maestro] {
        &self.state._Maestros
    }

    #[inline]
    pub fn Terminal(&self) -> u16 {
        self.state.Terminal()
    }

    pub fn DoLaunch(&self) {
        let _guard = Self::LifecycleLock().Lock();
        if self.state._SzThreads == 0 {
            return;
        }

        let state = &self.state;
        state._Maestros[0].FlushTempQueue(state);

        if state._SzThreads == 1 {
            AtelierState::ExecuteLoop(state, 0);
            return;
        }

        let worker_count = state._Maestros.len().saturating_sub(1);
        let mut handles = Vec::with_capacity(worker_count);
        for i in 0..worker_count {
            let s_clone = state.clone();
            let idx = (i + 1) as u32;
            handles.push(std::thread::spawn(move || {
                AtelierState::ExecuteLoop(&s_clone, idx);
            }));
        }

        AtelierState::ExecuteLoop(state, 0);

        for h in handles {
            let _ = h.join();
        }
    }

    pub fn DefaultThreadCount() -> u32 {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1)
    }

    pub fn LifecycleLock() -> &'static Spinlock {
        &LIFECYCLE_LOCK
    }

    pub fn Reset(threads: u32) -> Arc<Atelier> {
        let _guard = Self::LifecycleLock().Lock();
        let mut write_guard = GLOBAL_ATELIER.write().unwrap();
        let inst = Arc::new(Atelier::New(threads));
        *write_guard = Some(inst.clone());
        inst
    }

    pub fn Boot(threads: u32) -> Arc<Atelier> {
        Self::Reset(threads)
    }

    pub fn Instance() -> Arc<Atelier> {
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
