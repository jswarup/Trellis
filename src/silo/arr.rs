// arr.rs ----------------------------------------------------------------------------------------------------------

use crate::silo::seg::USeg;
use crate::silo::traits::{IArr, IArrMut};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr;

//-------------------------------------------------------------------------------------------------
// Arr — non-owning, borrowed contiguous array view.
// Exactly 16 bytes (_Ptr and _Size), zero-virtual, trivially copyable.
// Modeled directly from Trellis silo/arr.h.

#[derive(Clone, Copy)]
pub struct Arr<'a, T> {
    pub _Ptr: *const T,
    pub _Size: u32,
    _marker: PhantomData<&'a T>,
}

unsafe impl<'a, T: Sync> Send for Arr<'a, T> {}
unsafe impl<'a, T: Sync> Sync for Arr<'a, T> {}

impl<'a, T> Arr<'a, T> {
    //---------------------------------------------------------------------------------------------
    // Constructors & Factories

    #[inline]
    pub const fn Empty() -> Self {
        Self {
            _Ptr: ptr::NonNull::dangling().as_ptr(),
            _Size: 0,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub const fn New(ptr: *const T, size: u32) -> Self {
        Self {
            _Ptr: ptr,
            _Size: size,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn FromSlice(slice: &'a [T]) -> Self {
        Self {
            _Ptr: slice.as_ptr(),
            _Size: slice.len() as u32,
            _marker: PhantomData,
        }
    }

    //---------------------------------------------------------------------------------------------
    // Accessors & Queries

    #[inline]
    pub const fn Size(&self) -> u32 {
        self._Size
    }

    #[inline]
    pub const fn Len(&self) -> u32 {
        self._Size
    }

    #[inline]
    pub const fn IsEmpty(&self) -> bool {
        self._Size == 0
    }

    #[inline]
    pub const fn Data(&self) -> *const T {
        self._Ptr
    }

    #[inline]
    pub fn AsSlice(&self) -> &'a [T] {
        if self._Size == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self._Ptr, self._Size as usize) }
        }
    }

    #[inline]
    pub fn First(&self) -> Option<&'a T> {
        if self._Size == 0 {
            None
        } else {
            unsafe { Some(&*self._Ptr) }
        }
    }

    #[inline]
    pub fn Last(&self) -> Option<&'a T> {
        if self._Size == 0 {
            None
        } else {
            unsafe { Some(&*self._Ptr.add((self._Size - 1) as usize)) }
        }
    }

    #[inline]
    pub fn Get(&self, index: u32) -> Option<&'a T> {
        if index < self._Size {
            unsafe { Some(&*self._Ptr.add(index as usize)) }
        } else {
            None
        }
    }

    #[inline]
    pub const fn USeg(&self) -> USeg {
        USeg::FromLen(self._Size)
    }

    //---------------------------------------------------------------------------------------------
    // Slicing

    #[inline]
    pub fn LSnip(&self, count: u32) -> Self {
        let snip = if count < self._Size {
            count
        } else {
            self._Size
        };
        let remaining = self._Size - snip;
        let new_ptr = if remaining == 0 {
            ptr::NonNull::dangling().as_ptr()
        } else {
            unsafe { self._Ptr.add(snip as usize) }
        };
        Self::New(new_ptr, remaining)
    }

    #[inline]
    pub fn RSnip(&self, count: u32) -> Self {
        let snip = if count < self._Size {
            count
        } else {
            self._Size
        };
        let remaining = self._Size - snip;
        let new_ptr = if remaining == 0 {
            ptr::NonNull::dangling().as_ptr()
        } else {
            self._Ptr
        };
        Self::New(new_ptr, remaining)
    }

    #[inline]
    pub fn Slice(&self, start: u32, count: u32) -> Self {
        if start >= self._Size {
            return Self::Empty();
        }
        let available = self._Size - start;
        let take = if count < available { count } else { available };
        unsafe { Self::New(self._Ptr.add(start as usize), take) }
    }
}

impl<'a, T> Default for Arr<'a, T> {
    fn default() -> Self {
        Self::Empty()
    }
}

impl<'a, T> Deref for Arr<'a, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.AsSlice()
    }
}

impl<'a, T> Index<u32> for Arr<'a, T> {
    type Output = T;

    #[inline]
    fn index(&self, index: u32) -> &Self::Output {
        assert!(index < self._Size, "Index out of bounds");
        unsafe { &*self._Ptr.add(index as usize) }
    }
}

impl<'a, T> IArr<T> for Arr<'a, T> {
    #[inline]
    fn AsSlice(&self) -> &[T] {
        self.AsSlice()
    }

    #[inline]
    fn Len(&self) -> u32 {
        self._Size
    }
}

//-------------------------------------------------------------------------------------------------
// MutArr — mutable borrowed contiguous array view.
// Exactly 16 bytes (_Ptr and _Size), zero-virtual.

pub struct MutArr<'a, T> {
    pub _Ptr: *mut T,
    pub _Size: u32,
    _marker: PhantomData<&'a mut T>,
}

unsafe impl<'a, T: Send> Send for MutArr<'a, T> {}
unsafe impl<'a, T: Sync> Sync for MutArr<'a, T> {}

impl<'a, T> MutArr<'a, T> {
    #[inline]
    pub const fn Empty() -> Self {
        Self {
            _Ptr: ptr::NonNull::dangling().as_ptr(),
            _Size: 0,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub const fn New(ptr: *mut T, size: u32) -> Self {
        Self {
            _Ptr: ptr,
            _Size: size,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn FromMutSlice(slice: &'a mut [T]) -> Self {
        Self {
            _Size: slice.len() as u32,
            _Ptr: slice.as_mut_ptr(),
            _marker: PhantomData,
        }
    }

    #[inline]
    pub const fn Size(&self) -> u32 {
        self._Size
    }

    #[inline]
    pub const fn Len(&self) -> u32 {
        self._Size
    }

    #[inline]
    pub const fn IsEmpty(&self) -> bool {
        self._Size == 0
    }

    #[inline]
    pub const fn Data(&self) -> *mut T {
        self._Ptr
    }

    #[inline]
    pub fn AsSlice(&self) -> &[T] {
        if self._Size == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self._Ptr, self._Size as usize) }
        }
    }

    #[inline]
    pub fn AsMutSlice(&mut self) -> &mut [T] {
        if self._Size == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self._Ptr, self._Size as usize) }
        }
    }

    #[inline]
    pub fn AsArr(&self) -> Arr<'_, T> {
        Arr::New(self._Ptr, self._Size)
    }

    #[inline]
    pub const fn USeg(&self) -> USeg {
        USeg::FromLen(self._Size)
    }

    #[inline]
    pub fn Swap(&mut self, i: u32, j: u32) {
        assert!(i < self._Size && j < self._Size, "Index out of bounds");
        unsafe {
            ptr::swap(self._Ptr.add(i as usize), self._Ptr.add(j as usize));
        }
    }

    #[inline]
    pub fn SetAt(&mut self, k: u32, val: T) {
        assert!(k < self._Size, "Index out of bounds");
        unsafe {
            *self._Ptr.add(k as usize) = val;
        }
    }

    #[inline]
    pub fn SwapAt(&mut self, k: u32, val: &mut T) {
        assert!(k < self._Size, "Index out of bounds");
        unsafe {
            ptr::swap(self._Ptr.add(k as usize), val);
        }
    }

    #[inline]
    pub fn LSnip(&mut self, count: u32) -> Self {
        let snip = if count < self._Size {
            count
        } else {
            self._Size
        };
        let remaining = self._Size - snip;
        let new_ptr = if remaining == 0 {
            ptr::NonNull::dangling().as_ptr()
        } else {
            unsafe { self._Ptr.add(snip as usize) }
        };
        Self::New(new_ptr, remaining)
    }

    #[inline]
    pub fn RSnip(&mut self, count: u32) -> Self {
        let snip = if count < self._Size {
            count
        } else {
            self._Size
        };
        let remaining = self._Size - snip;
        let new_ptr = if remaining == 0 {
            ptr::NonNull::dangling().as_ptr()
        } else {
            self._Ptr
        };
        Self::New(new_ptr, remaining)
    }
}

impl<'a, T> Default for MutArr<'a, T> {
    fn default() -> Self {
        Self::Empty()
    }
}

impl<'a, T> Deref for MutArr<'a, T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.AsSlice()
    }
}

impl<'a, T> DerefMut for MutArr<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.AsMutSlice()
    }
}

impl<'a, T> Index<u32> for MutArr<'a, T> {
    type Output = T;

    #[inline]
    fn index(&self, index: u32) -> &Self::Output {
        assert!(index < self._Size, "Index out of bounds");
        unsafe { &*self._Ptr.add(index as usize) }
    }
}

impl<'a, T> IndexMut<u32> for MutArr<'a, T> {
    #[inline]
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        assert!(index < self._Size, "Index out of bounds");
        unsafe { &mut *self._Ptr.add(index as usize) }
    }
}

impl<'a, T> IArr<T> for MutArr<'a, T> {
    #[inline]
    fn AsSlice(&self) -> &[T] {
        self.AsSlice()
    }

    #[inline]
    fn Len(&self) -> u32 {
        self._Size
    }
}

impl<'a, T> IArrMut<T> for MutArr<'a, T> {
    #[inline]
    fn AsMutSlice(&mut self) -> &mut [T] {
        self.AsMutSlice()
    }
}
