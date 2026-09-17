// stk.rs ----------------------------------------------------------------------------------------------------------
use crate::silo::arr::{Arr, MutArr};
use crate::silo::seg::USeg;
use crate::silo::traits::IArr;
use std::sync::atomic::{AtomicU32, Ordering};

//-------------------------------------------------------------------------------------------------

// Stk — non-owning atomic stack view over an Arr buffer and a shared atomic size counter.
// Exactly 24 bytes (_Size pointer and MutArr view), zero-virtual, cache-friendly.
// Modeled directly from Trellis silo/stk.h and Kosh silo/stk.rs.
pub struct Stk< 'a, T> {
    pub _Size: Option< &'a AtomicU32>,
    pub _Arr: MutArr< 'a, T>,
}
impl< 'a, T> Stk<'a, T>
{

    //---------------------------------------------------------------------------------------------

    // Constructors & Factories
    #[inline]
    pub const fn  New() -> Self
    {
        Self {
            _Size: None,
            _Arr: MutArr::Empty(),
        }
    }
    #[inline]
    pub const fn  Create( size_ptr: &'a AtomicU32, arr: MutArr<'a, T>) -> Self
    {
        Self {
            _Size: Some( size_ptr),
            _Arr: arr,
        }
    }

    //---------------------------------------------------------------------------------------------

    // Size & Capacity Queries
    #[inline]
    pub fn  Size( &self) -> u32
    {
        self._Size.map( |s| s.load( Ordering::Acquire)).unwrap_or( 0)
    }
    #[inline]
    pub fn  SetSize( &self, size: u32)
    {
        if let  Some( s) = self._Size
        {
            s.store( size, Ordering::Release);
        }
    }
    #[inline]
    pub fn  Capacity( &self) -> u32
    {
        self._Arr.Size()
    }
    #[inline]
    pub fn  SzVoid( &self) -> u32
    {
        self.Capacity().saturating_sub( self.Size())
    }
    #[inline]
    pub fn  USeg( &self) -> USeg
    {
        USeg::FromLen( self.Size())
    }
    #[inline]
    pub fn  ArrView( &self) -> Arr< '_, T> {
        self._Arr.AsArr().RSnip( self.SzVoid())
    }

    //---------------------------------------------------------------------------------------------

    // Lock-Free Stack Operations
    pub fn  Pop( &self, val: &mut T) -> bool
    {
        let  size_atomic = match self._Size
        {
            Some( s) => s,
            None => return false,
        };
        loop
        {
            let  sz = size_atomic.load( Ordering::Acquire);
            if sz == 0
            {
                return false;
            }
            if size_atomic
                .compare_exchange_weak( sz, sz - 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                unsafe {
                    std::ptr::swap( self._Arr._Ptr.add( ( sz - 1) as usize), val);
                }
                return true;
            }
        }
    }
    pub fn  Push( &self, mut val: T) -> bool
    {
        let  size_atomic = match self._Size
        {
            Some( s) => s,
            None => return false,
        };
        let  cap = self.Capacity();
        loop
        {
            let  sz = size_atomic.load( Ordering::Acquire);
            if sz >= cap
            {
                return false;
            }
            if size_atomic
                .compare_exchange_weak( sz, sz + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                unsafe {
                    std::ptr::swap( self._Arr._Ptr.add( sz as usize), &mut val);
                }
                return true;
            }
        }
    }
}
impl< 'a, T> Default for Stk<'a, T>
{
    fn  default() -> Self
    {
        Self::New()
    }
}
impl< 'a, T> IArr<T> for Stk<'a, T>
{
    #[inline]
    fn  AsSlice( &self) -> &[T]
    {
        let  sz = self.Size();
        if sz == 0
        {
            &[]
        } else
        {
            unsafe { std::slice::from_raw_parts( self._Arr._Ptr, sz as usize) }
        }
    }
    #[inline]
    fn  Len( &self) -> u32
    {
        self.Size()
    }
}
