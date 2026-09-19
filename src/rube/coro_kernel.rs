//-- coro_kernel.rs ------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------------

use	crate::rube::trigger::TriggerId;
use	crate::silo::{ Arr, Buff, USeg };
use	crate::stalks::Coro;
use	std::cell::UnsafeCell;
use	std::ops::{ Index, IndexMut };
use	std::sync::Arc;

//------------------------------------------------------------------------------------------------------------------

pub const CORO_MAX_PORTS: usize = 64;

//------------------------------------------------------------------------------------------------------------------
/// Fixed-capacity port values array (up to 64 values) for zero-allocation coroutine exchange.
/// Modeled directly from Trellis `coro_kernel.h`.
#[derive( Copy, Clone, Debug, PartialEq, Eq)]
pub struct CoroPorts
{
    pub _Vals: [u64; CORO_MAX_PORTS],
    pub _Len: u32,
}
impl Default for CoroPorts {
    #[inline]
    fn	default() -> Self
    {
        Self::New()
    }
}
impl CoroPorts
{
    #[inline]
    pub const fn	New() -> Self
    {
        Self {
            _Vals: [0u64; CORO_MAX_PORTS],
            _Len: 0,
        }
    }
    #[inline]
    pub const fn	Empty() -> Self
    {
        Self::New()
    }
    #[inline]
    pub fn	Single( val: impl Into< u64>) -> Self
    {
        let  	mut ports = Self::New();
        ports._Vals[0] = val.into();
        ports._Len = 1;
        ports
    }
    #[inline]
    pub fn	Pair( v1: impl Into< u64>, v2: impl Into< u64>) -> Self
    {
        let  	mut ports = Self::New();
        ports._Vals[0] = v1.into();
        ports._Vals[1] = v2.into();
        ports._Len = 2;
        ports
    }
    pub fn	FromSlice( slice: &[u64]) -> Self
    {
        let  	mut ports = Self::New();
        let  	count = slice.len().min( CORO_MAX_PORTS);
        let  	mut i = 0;
        while i < count {
            ports._Vals[i] = slice[i];
            i += 1;
        }
        ports._Len = count as u32;
        ports
    }
    pub fn	FromArr( arr: Arr< '_, u64>) -> Self {
        let  	mut ports = Self::New();
        let  	count = arr.Size().min( CORO_MAX_PORTS as u32);
        USeg::FromLen( count).Traverse( |i| {
            ports._Vals[i as usize] = arr[i];
        });
        ports._Len = count;
        ports
    }
    #[inline]
    pub const fn	Len( &self) -> u32
    {
        self._Len
    }
    #[inline]
    pub const fn	IsEmpty( &self) -> bool
    {
        self._Len == 0
    }
    #[inline]
    pub fn	GetBool( &self, idx: u32) -> bool
    {
        assert!( idx < self._Len, "Index out of bounds");
        ( self._Vals[idx as usize] & 1) != 0
    }
    #[inline]
    pub fn	GetU64( &self, idx: u32) -> u64
    {
        assert!( idx < self._Len, "Index out of bounds");
        self._Vals[idx as usize]
    }
    #[inline]
    pub fn	GetU32( &self, idx: u32) -> u32
    {
        assert!( idx < self._Len, "Index out of bounds");
        self._Vals[idx as usize] as u32
    }
    #[inline]
    pub fn	Set( &mut self, idx: u32, val: impl Into< u64>)
    {
        assert!( idx < self._Len, "Index out of bounds");
        self._Vals[idx as usize] = val.into();
    }
    #[inline]
    pub fn	Push( &mut self, val: impl Into< u64>)
    {
        assert!( 
            ( self._Len as usize) < CORO_MAX_PORTS,
            "CoroPorts capacity exceeded"
        );
        self._Vals[self._Len as usize] = val.into();
        self._Len += 1;
    }
}
impl Index< u32> for CoroPorts {
    type Output = u64;
    #[inline]
    fn	index( &self, idx: u32) -> &Self::Output
    {
        assert!( idx < self._Len, "Index out of bounds");
        &self._Vals[idx as usize]
    }
}
impl IndexMut< u32> for CoroPorts {
    #[inline]
    fn	index_mut( &mut self, idx: u32) -> &mut Self::Output
    {
        assert!( idx < self._Len, "Index out of bounds");
        &mut self._Vals[idx as usize]
    }
}
impl Index< usize> for CoroPorts {
    type Output = u64;
    #[inline]
    fn	index( &self, idx: usize) -> &Self::Output
    {
        assert!( ( idx as u32) < self._Len, "Index out of bounds");
        &self._Vals[idx]
    }
}
impl IndexMut< usize> for CoroPorts {
    #[inline]
    fn	index_mut( &mut self, idx: usize) -> &mut Self::Output
    {
        assert!( ( idx as u32) < self._Len, "Index out of bounds");
        &mut self._Vals[idx]
    }
}

//------------------------------------------------------------------------------------------------------------------

pub type CoroInstance = Coro< CoroPorts, CoroPorts, ()>;
pub type CoroKernelFactory = Arc< dyn Fn() -> CoroInstance + Send + Sync>;

//------------------------------------------------------------------------------------------------------------------
/// Thread-safe cell container holding a CoroInstance.
pub struct CoroCell
{
    _Inner: UnsafeCell< CoroInstance>,
}
unsafe impl Sync for CoroCell {}
unsafe impl Send for CoroCell {}
impl CoroCell
{
    #[inline]
    pub fn	New( coro: CoroInstance) -> Self
    {
        Self {
            _Inner: UnsafeCell::new( coro),
        }
    }
    #[inline]
    #[allow( clippy::mut_from_ref)]
    pub fn	GetMut( &self) -> &mut CoroInstance
    {
        unsafe { &mut *self._Inner.get() }
    }
}

//------------------------------------------------------------------------------------------------------------------
/// Compiled warp executing a batch of homogeneous coroutine module instances.
pub struct CoroWarp
{
    pub _ModStart: u32,
    pub _Count: u32,
    pub _Instances: Buff< CoroCell>,
    pub _InTriggers: Buff< Buff< TriggerId>>,
    pub _OutTriggers: Buff< Buff< TriggerId>>,
}
impl CoroWarp
{
    #[inline]
    pub fn	New( 
        modStart: u32, count: u32, instances: Buff< CoroCell>, inTriggers: Buff< Buff< TriggerId>>,
        outTriggers: Buff< Buff< TriggerId>>,
    ) -> Self
    {
        Self {
            _ModStart: modStart,
            _Count: count,
            _Instances: instances,
            _InTriggers: inTriggers,
            _OutTriggers: outTriggers,
        }
    }
}
