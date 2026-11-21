use	crate::silo::arr::{ Arr, MutArr };
use	crate::silo::useg::USeg;
use	std::sync::atomic::{ AtomicU32, Ordering };

const LOCK_BIT: u32 = 0x8000_0000;
const SIZE_MASK: u32 = 0x7FFF_FFFF;

//-------------------------------------------------------------------------------------------------
// Stk — non-owning atomic stack view over an Arr buffer and a shared atomic size counter.
// Exactly 24 bytes (_Size pointer and MutArr view), zero-virtual, cache-friendly.
// Modeled directly from Trellis silo/stk.h and Kosh silo/stk.rs.
// Uses MSB of _Size as a spinlock bit (LOCK_BIT = 0x8000_0000) during push/pop publication,
// guaranteeing that consumers never observe an unwritten slot, producers never race on a slot,
// and failed pushes leave caller's value ownership intact.
pub struct Stk< 'a, T> {
    _Size: Option< &'a AtomicU32>,
    _Arr: MutArr< 'a, T>,
}
unsafe impl< 'a, T: Send> Send for Stk< 'a, T> {}
unsafe impl< 'a, T: Send> Sync for Stk< 'a, T> {}
impl< 'a, T> Stk<'a, T>
{

    //---------------------------------------------------------------------------------------------
    // Constructors & Factories
    #[inline]
    pub const fn	New() -> Self
    {
        Self {
            _Size: None,
            _Arr: MutArr::Empty(),
        }
    }
    #[inline]
    pub const fn	Create( size_ptr: &'a AtomicU32, arr: MutArr<'a, T>) -> Self
    {
        Self {
            _Size: Some( size_ptr),
            _Arr: arr,
        }
    }

    //---------------------------------------------------------------------------------------------
    // Size & Capacity Queries
    #[inline]
    pub fn	Size( &self) -> u32
    {
        self._Size.map( |s| s.load( Ordering::Acquire) & SIZE_MASK).unwrap_or( 0)
    }
    #[inline]
    pub fn	Capacity( &self) -> u32
    {
        self._Arr.Size()
    }
    #[inline]
    pub fn	SzVoid( &self) -> u32
    {
        self.Capacity().saturating_sub( self.Size())
    }
    #[inline]
    pub fn	USeg( &self) -> USeg
    {
        USeg::FromLen( self.Size())
    }
    //---------------------------------------------------------------------------------------------
    // Atomic Stack Operations
    pub fn	Pop( &self, val: &mut T) -> bool
    {
        let  	size_atomic = match self._Size {
            Some( s) => s,
            None => return false,
        };
        loop {
            let  	cur = size_atomic.load( Ordering::Acquire);
            if ( cur & LOCK_BIT) != 0 {
                std::hint::spin_loop();
                continue;
            }
            let  	sz = cur & SIZE_MASK;
            if sz == 0 {
                return false;
            }
            if size_atomic
                .compare_exchange_weak(
                    sz,
                    sz | LOCK_BIT,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                let  	popped = unsafe { std::ptr::read( self._Arr.Data().add( ( sz - 1) as usize)) };
                *val = popped;
                size_atomic.store( sz - 1, Ordering::Release);
                return true;
            }
            std::hint::spin_loop();
        }
    }
    pub fn	Push( &self, val: T) -> bool
    {
        let  	size_atomic = match self._Size {
            Some( s) => s,
            None => return false,
        };
        let  	cap = self.Capacity().min( SIZE_MASK);
        loop {
            let  	cur = size_atomic.load( Ordering::Acquire);
            if ( cur & LOCK_BIT) != 0 {
                std::hint::spin_loop();
                continue;
            }
            let  	sz = cur & SIZE_MASK;
            if sz >= cap {
                return false;
            }
            if size_atomic
                .compare_exchange_weak(
                    sz,
                    sz | LOCK_BIT,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .is_ok()
            {
                unsafe {
                    std::ptr::write( self._Arr.Data().add( sz as usize), val);
                }
                size_atomic.store( sz + 1, Ordering::Release);
                return true;
            }
            std::hint::spin_loop();
        }
    }
    pub fn	PushX( &self, val: &mut T) -> bool
    where
        T: Default,
    {
        let  	size_atomic = match self._Size {
            Some( s) => s,
            None => return false,
        };
        let  	cap = self.Capacity().min( SIZE_MASK);
        let  	cur = size_atomic.load( Ordering::Acquire);
        if ( cur & LOCK_BIT) != 0 {
            return false;
        }
        let  	sz = cur & SIZE_MASK;
        if sz >= cap {
            return false;
        }
        if size_atomic
            .compare_exchange(
                sz,
                sz | LOCK_BIT,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_err()
        {
            return false;
        }
        let  	item = std::mem::take( val);
        unsafe {
            std::ptr::write( self._Arr.Data().add( sz as usize), item);
        }
        size_atomic.store( sz + 1, Ordering::Release);
        true
    }
    #[inline]
    pub fn	Arr( &self) -> Arr< '_, T> {
        let  	sz = self.Size();
        self._Arr.Arr().Slice( 0, sz)
    }
    #[inline]
    pub fn	MutArr( &mut self) -> MutArr< 'a, T> {
        let  	sz = self.Size();
        let  	start = self.Capacity() - sz;
        unsafe { MutArr::New( self._Arr.Data().add( start as usize), sz) }
    }
}
impl< 'a, T> Default for Stk<'a, T>
{
    fn	default() -> Self
    {
        Self::New()
    }
}
