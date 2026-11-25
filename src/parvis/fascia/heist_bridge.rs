// heist_bridge.rs --------------------------------------------------------------------------------
//! Persistent Heist priority worker runtime bridged to Iced's `Subscription` event stream.

use crate::fascia::output::{DiagnosticSeverity, JobId};
use crate::fascia::python_console::ConsoleExecutionResult;
use crate::fascia::sketch_job::SketchJobOutcome;
use crate::fascia::tabs::TabId;
use crate::heist::scheduler::{JobPriority, PriorityJob, PriorityScheduler};
use crate::heist::token::CancellationToken;
use crate::stalks::work::SpinMutex;
use iced::futures::channel::mpsc;
use iced::futures::stream::BoxStream;
use iced::futures::StreamExt;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

//-------------------------------------------------------------------------------------------------

/// A single match returned by a background workspace search job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSearchHit
{
    pub path:    PathBuf,
    pub line:    usize,
    pub preview: String,
}

/// Events streamed from persistent Heist workers back to the Iced UI thread.
#[derive(Debug, Clone)]
pub enum HeistUiEvent
{
    Started
    {
        job_id: JobId,
        title:  String,
    },
    Progress
    {
        job_id:   JobId,
        progress: f32,
        stage:    String,
    },
    Log
    {
        job_id:   JobId,
        severity: DiagnosticSeverity,
        channel:  String,
        message:  String,
    },
    Finished
    {
        job_id: JobId,
    },
    SketchCompleted
    {
        tab_id:       TabId,
        generation:   u64,
        snapshot_rev: u64,
        cancellation: Arc<AtomicBool>,
        outcome:      SketchJobOutcome,
    },
    PythonCompleted
    {
        tab_id: Option<TabId>,
        result: ConsoleExecutionResult,
    },
    SearchCompleted
    {
        query:     String,
        hits:      Vec<WorkspaceSearchHit>,
        cancelled: bool,
    },
}

//-------------------------------------------------------------------------------------------------

/// Lightweight handle passed to Heist jobs for incremental progress and log streaming.
#[derive(Clone)]
pub struct HeistProgressReporter
{
    _JobId: JobId,
    _Tx:    mpsc::UnboundedSender<HeistUiEvent>,
}

impl HeistProgressReporter
{
    pub fn ReportProgress(&self, progress: f32, stage: impl Into<String>)
    {
        let _ = self._Tx.unbounded_send(HeistUiEvent::Progress {
            job_id:   self._JobId,
            progress: progress.clamp(0.0, 1.0),
            stage:    stage.into(),
        });
    }

    pub fn Log(&self, severity: DiagnosticSeverity, channel: impl Into<String>, message: impl Into<String>)
    {
        let _ = self._Tx.unbounded_send(HeistUiEvent::Log {
            job_id:   self._JobId,
            severity,
            channel:  channel.into(),
            message:  message.into(),
        });
    }

    pub fn Emit(&self, event: HeistUiEvent)
    {
        let _ = self._Tx.unbounded_send(event);
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Clone)]
struct RxHandle(Arc<Mutex<Option<mpsc::UnboundedReceiver<HeistUiEvent>>>>);

impl Hash for RxHandle
{
    fn hash<H: Hasher>(&self, state: &mut H)
    {
        Arc::as_ptr(&self.0).hash(state);
    }
}

fn stream_from_rx(handle: &RxHandle) -> BoxStream<'static, HeistUiEvent>
{
    let rx_opt = handle.0.lock().ok().and_then(|mut g| g.take());
    if let Some(rx) = rx_opt {
        return rx.boxed();
    }
    return iced::futures::stream::pending().boxed();
}

struct HeistPoolInner
{
    _Scheduler: PriorityScheduler,
    _WakeLock:  Mutex<u64>,
    _WakeCv:    Condvar,
    _Shutdown:  AtomicBool,
    _Tx:        mpsc::UnboundedSender<HeistUiEvent>,
    _Tokens:    SpinMutex<HashMap<JobId, CancellationToken>>,
    _NextJobId: AtomicU64,
}

/// Persistent Heist worker pool and priority scheduler for non-blocking Fascia UI operations.
pub struct HeistUiRuntime
{
    _Inner:    Arc<HeistPoolInner>,
    _RxHandle: RxHandle,
    _Workers:  Vec<std::thread::JoinHandle<()>>,
}

impl Default for HeistUiRuntime
{
    fn default() -> Self { return Self::New(); }
}

impl HeistUiRuntime
{
    pub fn New() -> Self
    {
        let worker_count = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).clamp(2, 6))
            .unwrap_or(2) as u32;
        let (tx, rx) = mpsc::unbounded();
        let inner = Arc::new(HeistPoolInner {
            _Scheduler: PriorityScheduler::New(worker_count),
            _WakeLock:  Mutex::new(0),
            _WakeCv:    Condvar::new(),
            _Shutdown:  AtomicBool::new(false),
            _Tx:        tx,
            _Tokens:    SpinMutex::New(HashMap::new()),
            _NextJobId: AtomicU64::new(1),
        });

        let mut workers = Vec::with_capacity(worker_count as usize);
        for idx in 0..worker_count {
            let pool = inner.clone();
            if let Ok(handle) = std::thread::Builder::new()
                .name(format!("fascia-heist-{idx}"))
                .spawn(move || worker_loop(idx, pool))
            {
                workers.push(handle);
            }
        }

        return Self {
            _Inner:    inner,
            _RxHandle: RxHandle(Arc::new(Mutex::new(Some(rx)))),
            _Workers:  workers,
        };
    }

    pub fn WorkerCount(&self) -> u32 { return self._Inner._Scheduler.WorkerCount(); }

    pub fn PendingCount(&self) -> u32 { return self._Inner._Scheduler.PendingCount(); }

    /// Submits a job to the persistent Heist `PriorityScheduler` with cooperative cancellation.
    pub fn Submit<F>(
        &self,
        job_id: JobId,
        priority: JobPriority,
        title: impl Into<String>,
        task: F,
    ) -> CancellationToken
    where
        F: FnOnce(&CancellationToken, &HeistProgressReporter) + Send + 'static,
    {
        let token = CancellationToken::New();
        let token_for_job = token.clone();
        let token_return = token.clone();
        self._Inner._Tokens.Lock().insert(job_id, token);

        let title_str = title.into();
        let tx = self._Inner._Tx.clone();
        let inner_ref = self._Inner.clone();
        let seq = self._Inner._NextJobId.fetch_add(1, Ordering::Relaxed);

        let _ = tx.unbounded_send(HeistUiEvent::Started {
            job_id,
            title: title_str,
        });

        let job = PriorityJob::WithToken(seq, priority, token_for_job.clone(), move || {
            let reporter = HeistProgressReporter {
                _JobId: job_id,
                _Tx:    tx.clone(),
            };
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                task(&token_for_job, &reporter);
            }));
            inner_ref._Tokens.Lock().remove(&job_id);
            let _ = tx.unbounded_send(HeistUiEvent::Finished { job_id });
        });

        self._Inner._Scheduler.Submit(job, None);
        if let Ok(mut guard) = self._Inner._WakeLock.lock() {
            *guard = guard.wrapping_add(1);
            self._Inner._WakeCv.notify_one();
        }
        return token_return;
    }

    /// Cancels a running or queued job by `JobId`.
    pub fn Cancel(&self, job_id: JobId) -> bool
    {
        if let Some(token) = self._Inner._Tokens.Lock().get(&job_id) {
            token.Cancel();
            return true;
        }
        return false;
    }

    /// Emits an event directly onto the UI subscription stream.
    pub fn Emit(&self, event: HeistUiEvent)
    {
        let _ = self._Inner._Tx.unbounded_send(event);
    }

    /// Iced subscription draining `HeistUiEvent`s without polling.
    pub fn Subscription(&self) -> iced::Subscription<HeistUiEvent>
    {
        return iced::Subscription::run_with(self._RxHandle.clone(), stream_from_rx);
    }
}

impl Drop for HeistUiRuntime
{
    fn drop(&mut self)
    {
        self._Inner._Shutdown.store(true, Ordering::Release);
        if let Ok(mut guard) = self._Inner._WakeLock.lock() {
            *guard = guard.wrapping_add(1);
            self._Inner._WakeCv.notify_all();
        }
        for handle in self._Workers.drain(..) {
            let _ = handle.join();
        }
    }
}

fn worker_loop(worker_idx: u32, pool: Arc<HeistPoolInner>)
{
    loop {
        if pool._Shutdown.load(Ordering::Acquire) {
            return;
        }
        if let Some(job) = pool._Scheduler.StealNext(worker_idx) {
            let _ = job.Execute();
            continue;
        }
        let Ok(guard) = pool._WakeLock.lock() else {
            return;
        };
        if pool._Shutdown.load(Ordering::Acquire) {
            return;
        }
        if pool._Scheduler.PendingCount() == 0 {
            drop(pool._WakeCv.wait(guard));
        }
    }
}
