use crate::silo::arr::{Arr, MutArr};
use crate::silo::cast::IPtrAtExt;
use crate::silo::useg::USeg;
use std::alloc::{Layout, alloc, dealloc};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::ptr;

//-------------------------------------------------------------------------------------------------
// Buff — owning, heap-allocated, fixed-capacity contiguous buffer.
// Exactly 16 bytes (_Ptr and _Cap), zero-virtual, trivially moveable.
// Modeled directly from Trellis silo/buff.h and Kosh silo/buff.rs.
// Buff represents a fixed-capacity allocation with no Push/growth overhead.
// Dynamic filling and building should prefer Stash which extracts into a Buff.
pub struct Buff<T> {
    _Ptr: *mut T,
    _Cap: u32,
    _marker: PhantomData<T>,
}
unsafe impl<T: Send> Send for Buff<T> {}
unsafe impl<T: Sync> Sync for Buff<T> {}
impl<T: Clone> Clone for Buff<T> {
    fn clone(&self) -> Self {
        Buff::FromArr(self.Arr())
    }
}
impl<T: std::fmt::Debug> std::fmt::Debug for Buff<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let slice = if self._Cap == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self._Ptr, self._Cap as usize) }
        };
        f.debug_list().entries(slice).finish()
    }
}
impl<T: PartialEq> PartialEq for Buff<T> {
    fn eq(&self, other: &Self) -> bool {
        if self._Cap != other._Cap {
            return false;
        }
        if self._Cap == 0 {
            return true;
        }
        let sArr = self.Arr();
        let oArr = other.Arr();
        let mut same = true;
        sArr.USeg().Traverse(|i| {
            if sArr[i] != oArr[i] {
                same = false;
            }
        });
        same
    }
}
impl<T: Eq> Eq for Buff<T> {}
impl<T> Buff<T> {
    //---------------------------------------------------------------------------------------------
    // Constructors & Destructor
    #[inline]
    pub const fn New() -> Self {
        Self {
            _Ptr: ptr::NonNull::dangling().as_ptr(),
            _Cap: 0,
            _marker: PhantomData,
        }
    }
    /// Creates a `Buff` directly from a raw pointer and capacity.
    ///
    /// # Safety
    /// `ptr` must point to an allocated block of `cap` elements allocated with standard layout,
    /// and all `cap` elements must be initialized or safe to drop.
    #[inline]
    pub const unsafe fn FromRawParts(ptr: *mut T, cap: u32) -> Self {
        Self {
            _Ptr: ptr,
            _Cap: cap,
            _marker: PhantomData,
        }
    }
    pub fn WithCapacity(capacity: u32) -> Self {
        if capacity == 0 {
            return Self::New();
        }
        let layout = Layout::array::<T>(capacity as usize).expect("Capacity overflow");
        assert!(
            layout.size() > 0,
            "Zero-sized types unsupported in raw Buff"
        );
        let ptr = unsafe { alloc(layout) as *mut T };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Self {
            _Ptr: ptr,
            _Cap: capacity,
            _marker: PhantomData,
        }
    }
    pub fn FromDispenser<F: FnMut(u32) -> T>(capacity: u32, mut dispenser: F) -> Self {
        if capacity == 0 {
            return Self::New();
        }
        let layout = Layout::array::<T>(capacity as usize).expect("Capacity overflow");
        assert!(
            layout.size() > 0,
            "Zero-sized types unsupported in raw Buff"
        );
        let ptr = unsafe { alloc(layout) as *mut T };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        struct Guard<T> {
            ptr: *mut T,
            cap: u32,
            init: u32,
        }
        impl<T> Drop for Guard<T> {
            fn drop(&mut self) {
                for i in 0..self.init {
                    unsafe {
                        ptr::drop_in_place(self.ptr.add(i as usize));
                    }
                }
                let layout = Layout::array::<T>(self.cap as usize).unwrap();
                unsafe {
                    dealloc(self.ptr as *mut u8, layout);
                }
            }
        }
        let mut guard = Guard {
            ptr,
            cap: capacity,
            init: 0,
        };
        for i in 0..capacity {
            unsafe {
                ptr::write(guard.ptr.add(i as usize), dispenser(i));
            }
            guard.init += 1;
        }
        let buff = unsafe { Self::FromRawParts(guard.ptr, guard.cap) };
        std::mem::forget(guard);
        buff
    }
    pub fn FromArr(arr: Arr<'_, T>) -> Self
    where
        T: Clone,
    {
        let len = arr.Len();
        if len == 0 {
            return Self::New();
        }
        let layout = Layout::array::<T>(len as usize).expect("Capacity overflow");
        assert!(
            layout.size() > 0,
            "Zero-sized types unsupported in raw Buff"
        );
        let ptr = unsafe { alloc(layout) as *mut T };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        struct Guard<T> {
            ptr: *mut T,
            cap: u32,
            init: u32,
        }
        impl<T> Drop for Guard<T> {
            fn drop(&mut self) {
                for i in 0..self.init {
                    unsafe {
                        ptr::drop_in_place(self.ptr.add(i as usize));
                    }
                }
                let layout = Layout::array::<T>(self.cap as usize).unwrap();
                unsafe {
                    dealloc(self.ptr as *mut u8, layout);
                }
            }
        }
        let mut guard = Guard {
            ptr,
            cap: len,
            init: 0,
        };
        for i in 0..len {
            unsafe {
                ptr::write(guard.ptr.add(i as usize), arr[i].clone());
            }
            guard.init += 1;
        }
        let buff = unsafe { Self::FromRawParts(guard.ptr, guard.cap) };
        std::mem::forget(guard);
        buff
    }

    //---------------------------------------------------------------------------------------------
    // Accessors & Queries
    #[inline]
    pub const fn Cap(&self) -> u32 {
        self._Cap
    }
    #[inline]
    pub const fn Size(&self) -> u32 {
        self._Cap
    }
    #[inline]
    pub const fn Len(&self) -> u32 {
        self._Cap
    }
    #[inline]
    pub const fn IsEmpty(&self) -> bool {
        self._Cap == 0
    }
    #[inline]
    pub fn Arr(&self) -> Arr<'_, T> {
        Arr::New(self._Ptr, self._Cap)
    }
    #[inline]
    pub fn MutArr(&mut self) -> MutArr<'_, T> {
        MutArr::New(self._Ptr, self._Cap)
    }
    #[inline]
    pub fn CastArr(&self) -> Arr<'_, u8>
    where
        T: Copy,
    {
        Arr::New(
            self._Ptr as *const u8,
            self._Cap * (std::mem::size_of::<T>() as u32),
        )
    }
    #[inline]
    pub fn CastArrFrom<U: Copy>(&self) -> Arr<'_, U>
    where
        T: Copy,
    {
        let sz_t = std::mem::size_of::<T>() as u32;
        let sz_u = std::mem::size_of::<U>() as u32;
        assert!(sz_u > 0, "Cannot cast to ZST");
        assert_eq!(
            (self._Cap * sz_t) % sz_u,
            0,
            "Buff size in bytes not aligned to target type"
        );
        Arr::New(self._Ptr as *const U, (self._Cap * sz_t) / sz_u)
    }
    #[inline]
    pub const fn USeg(&self) -> USeg {
        USeg::FromLen(self._Cap)
    }
    #[inline]
    pub fn Traverse<F: FnMut(&T)>(&self, mut f: F) {
        self.USeg().Traverse(|i| f(&self[i]));
    }
    #[inline]
    pub fn TraverseMut<F: FnMut(&mut T)>(&mut self, mut f: F) {
        let useg = self.USeg();
        useg.Traverse(|i| f(&mut self[i]));
    }
    #[inline]
    pub fn Span<F: FnMut(&T) -> bool>(&self, mut f: F) -> bool {
        self.USeg().Span(|i| f(&self[i]))
    }
    #[inline]
    pub fn Take(&mut self) -> Self {
        let ptr = self._Ptr;
        let cap = self._Cap;
        self._Ptr = ptr::NonNull::dangling().as_ptr();
        self._Cap = 0;
        unsafe { Self::FromRawParts(ptr, cap) }
    }
    pub fn Destroy(&mut self, initialized_size: u32) {
        assert!(
            initialized_size <= self._Cap,
            "Cannot destroy more elements than capacity"
        );
        if initialized_size > 0 && !self._Ptr.is_null() {
            USeg::FromLen(initialized_size).Traverse(|i| unsafe {
                ptr::drop_in_place(self._Ptr.add(i as usize));
            });
        }
        if self._Cap > 0 && !self._Ptr.is_null() {
            let layout = Layout::array::<T>(self._Cap as usize).unwrap();
            unsafe {
                dealloc(self._Ptr as *mut u8, layout);
            }
            self._Ptr = ptr::NonNull::dangling().as_ptr();
            self._Cap = 0;
        }
    }
}
impl<T> Default for Buff<T> {
    fn default() -> Self {
        Self::New()
    }
}
impl<T> Index<u32> for Buff<T> {
    type Output = T;
    #[inline]
    fn index(&self, index: u32) -> &Self::Output {
        assert!(index < self._Cap, "Index out of bounds");
        self._Ptr.RefAt(index as usize)
    }
}
impl<T> IndexMut<u32> for Buff<T> {
    #[inline]
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        assert!(index < self._Cap, "Index out of bounds");
        self._Ptr.MutRefAt(index as usize)
    }
}
impl<T> Drop for Buff<T> {
    fn drop(&mut self) {
        self.Destroy(self._Cap);
    }
}

//-------------------------------------------------------------------------------------------------

#[macro_export]
macro_rules! Buff {
    () => {
        $crate::silo::buff::Buff::WithCapacity( 0)
    };
    ( $( $x:expr ),* ) => {
        {
            let  	temp = core::mem::ManuallyDrop::new( [ $( $x ),* ]);
            $crate::silo::buff::Buff::FromDispenser( temp.len() as u32, |i| unsafe { core::ptr::read( &temp[i as usize]) })
        }
    };
    ( $( $x:expr ),+ , ) => {
        $crate::Buff![ $( $x ),* ]
    };
    ( $elem:expr ; $n:expr ) => {
        {
            let  	count: u32 = ($n).try_into().expect( "Count must fit in u32");
            $crate::silo::buff::Buff::FromDispenser( count, |_| $elem.clone())
        }
    };
}
