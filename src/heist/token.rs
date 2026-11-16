//-- token.rs --------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	std::sync::Arc;
use	std::sync::atomic::{ AtomicBool, AtomicU64, Ordering };

//--------------------------------------------------------------------------------------------------
/// Cooperative cancellation token supporting epoch-based preemption for long-running jobs.
#[derive( Clone)]
pub struct CancellationToken
{
    _Cancelled: Arc< AtomicBool>,
    _Epoch:     Arc< AtomicU64>,
}

impl Default for CancellationToken {
    fn	default() -> Self
    {
        Self::New()
    }
}

impl CancellationToken
{
    pub fn	New() -> Self
    {
        Self {
            _Cancelled: Arc::new( AtomicBool::new( false)),
            _Epoch:     Arc::new( AtomicU64::new( 0)),
        }
    }

    pub fn	WithEpoch( initialEpoch: u64) -> Self
    {
        Self {
            _Cancelled: Arc::new( AtomicBool::new( false)),
            _Epoch:     Arc::new( AtomicU64::new( initialEpoch)),
        }
    }

    #[inline]
    pub fn	Cancel( &self)
    {
        self._Cancelled.store( true, Ordering::Release);
        self._Epoch.fetch_add( 1, Ordering::Release);
    }

    #[inline]
    pub fn	IsCancelled( &self) -> bool
    {
        self._Cancelled.load( Ordering::Acquire)
    }

    #[inline]
    pub fn	Epoch( &self) -> u64
    {
        self._Epoch.load( Ordering::Acquire)
    }

    #[inline]
    pub fn	AdvanceEpoch( &self) -> u64
    {
        self._Epoch.fetch_add( 1, Ordering::AcqRel) + 1
    }

    #[inline]
    pub fn	ShouldPreempt( &self, expectedEpoch: u64) -> bool
    {
        self.IsCancelled() || self.Epoch() > expectedEpoch
    }
}

//--------------------------------------------------------------------------------------------------
/// Atomic epoch tracker for cooperative timeslice preemption.
pub struct PreemptionEpoch
{
    _Value: AtomicU64,
}

impl Default for PreemptionEpoch {
    fn	default() -> Self
    {
        Self::New( 0)
    }
}

impl PreemptionEpoch
{
    pub fn	New( initial: u64) -> Self
    {
        Self {
            _Value: AtomicU64::new( initial),
        }
    }

    #[inline]
    pub fn	Current( &self) -> u64
    {
        self._Value.load( Ordering::Acquire)
    }

    #[inline]
    pub fn	Advance( &self) -> u64
    {
        self._Value.fetch_add( 1, Ordering::AcqRel) + 1
    }

    #[inline]
    pub fn	ShouldYield( &self, jobEpoch: u64) -> bool
    {
        self.Current() > jobEpoch
    }
}
