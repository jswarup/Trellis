// work.rs ---------------------------------------------------------------------------------------------------------
use std::sync::atomic::{AtomicBool, Ordering};

//-------------------------------------------------------------------------------------------------

// Spinlock — lightweight CAS-based spinlock with CPU pause.
// Modeled directly from Trellis stalks/atm.h.
pub struct Spinlock
{
    _locked: AtomicBool,
}
pub struct SpinlockGuard< 'a> {
    _lock: &'a Spinlock,
}
impl Spinlock
{
    pub const fn  New() -> Self
    {
        Self {
            _locked: AtomicBool::new( false),
        }
    }
    pub fn  Acquire( &self)
    {
        while self._locked.swap( true, Ordering::Acquire)
        {
            while self._locked.load( Ordering::Relaxed)
            {
                std::hint::spin_loop();
            }
        }
    }
    pub fn  Release( &self)
    {
        self._locked.store( false, Ordering::Release);
    }
    pub fn  Lock( &self) -> SpinlockGuard< '_> {
        self.Acquire();
        SpinlockGuard { _lock: self }
    }
}
impl Default for Spinlock
{
    fn  default() -> Self
    {
        Self::New()
    }
}
impl< 'a> Drop for SpinlockGuard<'a>
{
    fn  drop( &mut self)
    {
        self._lock.Release();
    }
}

//-------------------------------------------------------------------------------------------------

// SpinMutex — data-protecting mutual exclusion lock backed by Spinlock.
pub struct SpinMutex< T>
{
    _lock: Spinlock,
    _data: std::cell::UnsafeCell< T>,
}
unsafe impl< T: Send> Send for SpinMutex< T>
{ }
unsafe impl< T: Send> Sync for SpinMutex< T>
{ }
pub struct SpinMutexGuard< 'a, T> {
    _guard: SpinlockGuard< 'a>,
    _data: *mut T,
}
impl< T> SpinMutex< T>
{
    pub const fn  New( val: T) -> Self
    {
        Self {
            _lock: Spinlock::New(),
            _data: std::cell::UnsafeCell::new( val),
        }
    }
    pub fn  Lock( &self) -> SpinMutexGuard< '_, T> {
        let  guard = self._lock.Lock();
        let  data = self._data.get();
        SpinMutexGuard {
            _guard: guard,
            _data: data,
        }
    }
}
impl< 'a, T> std::ops::Deref for SpinMutexGuard<'a, T>
{
    type Target = T;
    fn  deref( &self) -> &Self::Target
    {
        unsafe { &*self._data }
    }
}
impl< 'a, T> std::ops::DerefMut for SpinMutexGuard<'a, T>
{
    fn  deref_mut( &mut self) -> &mut Self::Target
    {
        unsafe { &mut *self._data }
    }
}

//-------------------------------------------------------------------------------------------------

// IWorker — zero-virtual worker context capable of receiving and scheduling jobs.
pub trait IWorker {
    fn  PostJob( &mut self, job: WorkPtr);
    fn  WorkerIndex( &self) -> u32;
    fn  EnqueueJobId( &mut self, _job_id: u16)
    { }
}

//-------------------------------------------------------------------------------------------------

pub type WorkerFn = Box< dyn FnOnce( &mut dyn IWorker) + Send>;
pub struct WorkPtr
{
    _func: Option< WorkerFn>,
}
impl WorkPtr
{
    pub const fn  Null() -> Self
    {
        Self { _func: None }
    }
    pub fn  FromClosure< F>( f: F) -> Self
    where
        F: FnOnce( &mut dyn IWorker) + Send + 'static,
    {
        Self {
            _func: Some( Box::new( f)),
        }
    }
    pub fn  FromFn( f: fn( &mut dyn IWorker)) -> Self
    {
        Self::FromClosure( move |w| f( w))
    }
    pub fn  IsNull( &self) -> bool
    {
        self._func.is_none()
    }
    pub fn  DoWork( &mut self, worker: &mut dyn IWorker)
    {
        if let  Some( task) = self._func.take()
        {
            task( worker);
        }
    }
}
impl Default for WorkPtr
{
    fn  default() -> Self
    {
        Self::Null()
    }
}
