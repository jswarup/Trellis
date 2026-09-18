use crate::silo::cast::{IConstPtrAtExt, IConstPtrRefExt, IPtrAtExt};
use crate::silo::seg::USeg;
use crate::silo::traits::{IArr, IArrMut};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::ptr;

//-------------------------------------------------------------------------------------------------

// Arr — non-owning, borrowed contiguous array view.
// Exactly 16 bytes (_Ptr and _Size), zero-virtual, trivially copyable.
// Modeled directly from Trellis silo/arr.h.
#[derive(Clone, Copy)]
pub struct Arr<'a, T> {
    _Ptr: *const T,
    _Size: u32,
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
}

impl<'a, T> From<&'a [T]> for Arr<'a, T> {
    #[inline]
    fn from(slice: &'a [T]) -> Self {
        Self::FromSlice(slice)
    }
}

impl<'a, T, const N: usize> From<&'a [T; N]> for Arr<'a, T> {
    #[inline]
    fn from(arr: &'a [T; N]) -> Self {
        Self::FromSlice(arr.as_slice())
    }
}

impl<'a, T> Arr<'a, T> {

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
    pub fn First(&self) -> Option<&'a T> {
        if self._Size == 0 {
            None
        } else {
            Some(self._Ptr.Ref())
        }
    }
    #[inline]
    pub fn Last(&self) -> Option<&'a T> {
        if self._Size == 0 {
            None
        } else {
            Some(self._Ptr.RefAt((self._Size - 1) as usize))
        }
    }
    #[inline]
    pub fn Get(&self, index: u32) -> Option<&'a T> {
        if index < self._Size {
            Some(self._Ptr.RefAt(index as usize))
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
impl<'a, T> Index<u32> for Arr<'a, T> {
    type Output = T;
    #[inline]
    fn index(&self, index: u32) -> &Self::Output {
        assert!(index < self._Size, "Index out of bounds");
        self._Ptr.RefAt(index as usize)
    }
}
impl<'a, T> IArr<T> for Arr<'a, T> {
    #[inline]
    fn AsArr(&self) -> Arr<'_, T> {
        Arr::New(self._Ptr, self._Size)
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
    _Ptr: *mut T,
    _Size: u32,
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
        if i != j {
            std::mem::swap(
                self._Ptr.MutRefAt(i as usize),
                self._Ptr.MutRefAt(j as usize),
            );
        }
    }
    #[inline]
    pub fn SetAt(&mut self, k: u32, val: T) {
        assert!(k < self._Size, "Index out of bounds");
        *self._Ptr.MutRefAt(k as usize) = val;
    }
    #[inline]
    pub fn SwapAt(&mut self, k: u32, val: &mut T) {
        assert!(k < self._Size, "Index out of bounds");
        std::mem::swap(self._Ptr.MutRefAt(k as usize), val);
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
impl<'a, T> Index<u32> for MutArr<'a, T> {
    type Output = T;
    #[inline]
    fn index(&self, index: u32) -> &Self::Output {
        assert!(index < self._Size, "Index out of bounds");
        self._Ptr.RefAt(index as usize)
    }
}
impl<'a, T> IndexMut<u32> for MutArr<'a, T> {
    #[inline]
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        assert!(index < self._Size, "Index out of bounds");
        self._Ptr.MutRefAt(index as usize)
    }
}
impl<'a, T> IArr<T> for MutArr<'a, T> {
    #[inline]
    fn AsArr(&self) -> Arr<'_, T> {
        self.AsArr()
    }
    #[inline]
    fn Len(&self) -> u32 {
        self._Size
    }
}
impl<'a, T> IArrMut<T> for MutArr<'a, T> {
    #[inline]
    fn AsMutArr(&mut self) -> MutArr<'_, T> {
        MutArr::New(self._Ptr, self._Size)
    }
}
