// stash.rs --------------------------------------------------------------------------------------------------------
use crate::silo::arr::{Arr, MutArr};
use crate::silo::buff::Buff;
use crate::silo::seg::USeg;
use crate::silo::stk::Stk;
use crate::silo::traits::{IArr, IArrMut};
use std::alloc::{Layout, alloc, dealloc};
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

//-------------------------------------------------------------------------------------------------

// Stash — dynamic array container for building up elements backed by an owning Buff.
// Exactly 24 bytes (_Buff is 16 bytes, _Sz is 4 bytes + 4 alignment), zero-virtual.
// Modeled directly from Trellis silo/stash.h and Kosh silo/stash.rs.
// Designed for local instantiation to build and fill elements, extracting into a Buff.
pub struct Stash< T>
{
    pub _Buff: Buff< T>,
    pub _Sz: AtomicU32,
}
unsafe impl< T: Send> Send for Stash< T>
{ }
unsafe impl< T: Sync> Sync for Stash< T>
{ }
impl< T> Stash< T>
{

    //---------------------------------------------------------------------------------------------

    // Constructors & Factories
    #[inline]
    pub const fn  New() -> Self
    {
        Self {
            _Buff: Buff::New(),
            _Sz: AtomicU32::new( 0),
        }
    }
    pub fn  WithCapacity( capacity: u32) -> Self
    {
        Self {
            _Buff: Buff::WithCapacity( capacity),
            _Sz: AtomicU32::new( 0),
        }
    }
    pub fn  FromDispenser< F: FnMut( u32) -> T>(
        capacity: u32,
        initial_size: u32,
        mut dispenser: F,
    ) -> Self
    {
        let  cap = capacity.max( initial_size);
        let  buff: Buff< T> = Buff::WithCapacity( cap);
        USeg::FromLen( initial_size).Traverse( |i| unsafe {
            ptr::write( buff._Ptr.add( i as usize), dispenser( i));
        });
        Self {
            _Buff: buff,
            _Sz: AtomicU32::new( initial_size),
        }
    }

    //---------------------------------------------------------------------------------------------

    // Capacity & Growth
    fn  grow( &mut self)
    {
        let  cur_cap = self._Buff._Cap;
        let  new_cap = if cur_cap == 0 { 4 } else { cur_cap * 2 };
        let  new_layout = Layout::array::<T>( new_cap as usize).expect( "Capacity overflow");
        let  new_ptr = unsafe { alloc( new_layout) as *mut T };
        if new_ptr.is_null()
        {
            std::alloc::handle_alloc_error( new_layout);
        }
        let  cur_sz = self.Size();
        if cur_sz > 0 && !self._Buff._Ptr.is_null()
        {
            unsafe {
                ptr::copy_nonoverlapping( self._Buff._Ptr, new_ptr, cur_sz as usize);
                let  old_layout = Layout::array::<T>( cur_cap as usize).unwrap();
                dealloc( self._Buff._Ptr as *mut u8, old_layout);
            }
        }
        self._Buff._Ptr = new_ptr;
        self._Buff._Cap = new_cap;
    }
    pub fn  Reserve( &mut self, new_cap: u32)
    {
        if new_cap > self._Buff._Cap
        {
            let  new_layout = Layout::array::<T>( new_cap as usize).expect( "Capacity overflow");
            let  new_ptr = unsafe { alloc( new_layout) as *mut T };
            if new_ptr.is_null()
            {
                std::alloc::handle_alloc_error( new_layout);
            }
            let  cur_sz = self.Size();
            if cur_sz > 0 && !self._Buff._Ptr.is_null()
            {
                unsafe {
                    ptr::copy_nonoverlapping( self._Buff._Ptr, new_ptr, cur_sz as usize);
                    let  old_layout = Layout::array::<T>( self._Buff._Cap as usize).unwrap();
                    dealloc( self._Buff._Ptr as *mut u8, old_layout);
                }
            }
            self._Buff._Ptr = new_ptr;
            self._Buff._Cap = new_cap;
        }
    }

    //---------------------------------------------------------------------------------------------

    // Dynamic Insertion & Extraction
    pub fn  Push( &mut self, val: T)
    {
        self.PushBack( val);
    }
    pub fn  PushBack( &mut self, val: T)
    {
        let  cur_sz = self.Size();
        if cur_sz >= self._Buff._Cap
        {
            self.grow();
        }
        unsafe {
            ptr::write( self._Buff._Ptr.add( cur_sz as usize), val);
        }
        self._Sz.store( cur_sz + 1, Ordering::Release);
    }
    pub fn  Pop( &mut self) -> Option< T>
    {
        let  cur_sz = self.Size();
        if cur_sz == 0
        {
            None
        } else
        {
            let  new_sz = cur_sz - 1;
            self._Sz.store( new_sz, Ordering::Release);
            unsafe { Some( ptr::read( self._Buff._Ptr.add( new_sz as usize))) }
        }
    }
    pub fn  PopBack( &mut self) -> bool
    {
        self.Pop().is_some()
    }
    pub fn  Clear( &mut self)
    {
        while self.Pop().is_some()
        { }
    }
    pub fn  ExtractBuff( mut self, shrink_to_fit: bool) -> Buff< T>
    {
        let  cur_sz = self.Size();
        if shrink_to_fit && cur_sz < self._Buff._Cap && cur_sz > 0
        {
            let  new_layout = Layout::array::<T>( cur_sz as usize).expect( "Capacity overflow");
            let  new_ptr = unsafe { alloc( new_layout) as *mut T };
            if new_ptr.is_null()
            {
                std::alloc::handle_alloc_error( new_layout);
            }
            unsafe {
                ptr::copy_nonoverlapping( self._Buff._Ptr, new_ptr, cur_sz as usize);
                let  old_layout = Layout::array::<T>( self._Buff._Cap as usize).unwrap();
                dealloc( self._Buff._Ptr as *mut u8, old_layout);
            }
            self._Buff._Ptr = new_ptr;
            self._Buff._Cap = cur_sz;
        } else if cur_sz == 0
        {
            // Free empty buffer
            if self._Buff._Cap > 0 && !self._Buff._Ptr.is_null()
            {
                unsafe {
                    let  old_layout = Layout::array::<T>( self._Buff._Cap as usize).unwrap();
                    dealloc( self._Buff._Ptr as *mut u8, old_layout);
                }
            }
            self._Buff._Ptr = ptr::NonNull::dangling().as_ptr();
            self._Buff._Cap = 0;
        } else
        {
            self._Buff._Cap = cur_sz;
        }
        let  ptr = self._Buff._Ptr;
        let  cap = self._Buff._Cap;
        // Neutralize self so drop does not free or double drop
        self._Buff._Ptr = ptr::NonNull::dangling().as_ptr();
        self._Buff._Cap = 0;
        self._Sz.store( 0, Ordering::Release);
        unsafe { Buff::FromRawParts( ptr, cap) }
    }

    //---------------------------------------------------------------------------------------------

    // Accessors & Queries
    #[inline]
    pub fn  Size( &self) -> u32
    {
        self._Sz.load( Ordering::Acquire)
    }
    #[inline]
    pub fn  Len( &self) -> u32
    {
        self.Size()
    }
    #[inline]
    pub fn  Capacity( &self) -> u32
    {
        self._Buff._Cap
    }
    #[inline]
    pub fn  IsEmpty( &self) -> bool
    {
        self.Size() == 0
    }
    #[inline]
    pub fn  Data( &self) -> *const T
    {
        self._Buff._Ptr
    }
    #[inline]
    pub fn  DataMut( &mut self) -> *mut T
    {
        self._Buff._Ptr
    }
    #[inline]
    pub fn  AsSlice( &self) -> &[T]
    {
        let  sz = self.Size();
        if sz == 0
        {
            &[]
        } else
        {
            unsafe { std::slice::from_raw_parts( self._Buff._Ptr, sz as usize) }
        }
    }
    #[inline]
    pub fn  AsMutSlice( &mut self) -> &mut [T]
    {
        let  sz = self.Size();
        if sz == 0
        {
            &mut []
        } else
        {
            unsafe { std::slice::from_raw_parts_mut( self._Buff._Ptr, sz as usize) }
        }
    }
    #[inline]
    pub fn  AsArr( &self) -> Arr< '_, T> {
        Arr::New( self._Buff._Ptr, self.Size())
    }
    #[inline]
    pub fn  AsMutArr( &mut self) -> MutArr< '_, T> {
        let  sz = self.Size();
        MutArr::New( self._Buff._Ptr, sz)
    }
    #[inline]
    pub fn  StkView< 'a>(&'a self) -> Stk< 'a, T> {
        Stk::Create( &self._Sz, MutArr::New( self._Buff._Ptr, self._Buff._Cap))
    }
    #[inline]
    pub fn  USeg( &self) -> USeg
    {
        USeg::FromLen( self.Size())
    }
}
impl< T> Default for Stash< T>
{
    fn  default() -> Self
    {
        Self::New()
    }
}
impl< T> Deref for Stash< T>
{
    type Target = [T];
    #[inline]
    fn  deref( &self) -> &Self::Target
    {
        self.AsSlice()
    }
}
impl< T> DerefMut for Stash< T>
{
    #[inline]
    fn  deref_mut( &mut self) -> &mut Self::Target
    {
        self.AsMutSlice()
    }
}
impl< T> Index< u32> for Stash< T>
{
    type Output = T;
    #[inline]
    fn  index( &self, index: u32) -> &Self::Output
    {
        assert!( index < self.Size(), "Index out of bounds");
        unsafe { &*self._Buff._Ptr.add( index as usize) }
    }
}
impl< T> IndexMut< u32> for Stash< T>
{
    #[inline]
    fn  index_mut( &mut self, index: u32) -> &mut Self::Output
    {
        assert!( index < self.Size(), "Index out of bounds");
        unsafe { &mut *self._Buff._Ptr.add( index as usize) }
    }
}
impl< T> IArr< T> for Stash< T>
{
    #[inline]
    fn  AsSlice( &self) -> &[T]
    {
        self.AsSlice()
    }
    #[inline]
    fn  Len( &self) -> u32
    {
        self.Size()
    }
}
impl< T> IArrMut< T> for Stash< T>
{
    #[inline]
    fn  AsMutSlice( &mut self) -> &mut [T]
    {
        self.AsMutSlice()
    }
}
impl< T> Drop for Stash< T>
{
    fn  drop( &mut self)
    {
        let  cur_sz = self.Size();
        if cur_sz > 0 && !self._Buff._Ptr.is_null()
        {
            USeg::FromLen( cur_sz).Traverse( |i| unsafe {
                ptr::drop_in_place( self._Buff._Ptr.add( i as usize));
            });
        }
        if self._Buff._Cap > 0 && !self._Buff._Ptr.is_null()
        {
            let  layout = Layout::array::<T>( self._Buff._Cap as usize).unwrap();
            unsafe {
                dealloc( self._Buff._Ptr as *mut u8, layout);
            }
            self._Buff._Ptr = ptr::NonNull::dangling().as_ptr();
            self._Buff._Cap = 0;
        }
    }
}
