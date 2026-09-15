// buff.rs ---------------------------------------------------------------------------------------------------------

use crate::silo::arr::{Arr, MutArr};
use crate::silo::seg::USeg;
use crate::silo::traits::{IArr, IArrMut};
use std::alloc::{Layout, alloc, dealloc};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr;

//-------------------------------------------------------------------------------------------------
// Buff — owning, heap-allocated, fixed-capacity contiguous buffer.
// Exactly 16 bytes (_Ptr and _Cap), zero-virtual, trivially moveable.
// Modeled directly from Trellis silo/buff.h and Kosh silo/buff.rs.
// Buff represents a fixed-capacity allocation with no Push/growth overhead.
// Dynamic filling and building should prefer Stash which extracts into a Buff.

pub struct Buff<T> {
    pub _Ptr: *mut T,
    pub _Cap: u32,
    _marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for Buff<T> {}
unsafe impl<T: Sync> Sync for Buff<T> {}

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
        let buff = Self::WithCapacity(capacity);
        USeg::FromLen(capacity).Traverse(|i| unsafe {
            ptr::write(buff._Ptr.add(i as usize), dispenser(i));
        });
        buff
    }

    pub fn FromSlice(slice: &[T]) -> Self
    where
        T: Clone,
    {
        let len = slice.len() as u32;
        let buff = Self::WithCapacity(len);
        USeg::FromLen(len).Traverse(|i| unsafe {
            ptr::write(buff._Ptr.add(i as usize), slice[i as usize].clone());
        });
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
    pub const fn Data(&self) -> *const T {
        self._Ptr
    }

    #[inline]
    pub fn DataMut(&mut self) -> *mut T {
        self._Ptr
    }

    #[inline]
    pub const fn AsPtr(&self) -> *const T {
        self._Ptr
    }

    #[inline]
    pub fn AsMutPtr(&mut self) -> *mut T {
        self._Ptr
    }

    #[inline]
    pub fn AsSlice(&self) -> &[T] {
        if self._Cap == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self._Ptr, self._Cap as usize) }
        }
    }

    #[inline]
    pub fn AsMutSlice(&mut self) -> &mut [T] {
        if self._Cap == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self._Ptr, self._Cap as usize) }
        }
    }

    #[inline]
    pub fn AsArr(&self) -> Arr<'_, T> {
        Arr::New(self._Ptr, self._Cap)
    }

    #[inline]
    pub fn AsMutArr(&mut self) -> MutArr<'_, T> {
        MutArr::New(self._Ptr, self._Cap)
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
    pub fn Iter(&self) -> std::slice::Iter<'_, T> {
        self.AsSlice().iter()
    }

    #[inline]
    pub fn IterMut(&mut self) -> std::slice::IterMut<'_, T> {
        self.AsMutSlice().iter_mut()
    }
}

impl<T> Default for Buff<T> {
    fn default() -> Self {
        Self::New()
    }
}

impl<T> Deref for Buff<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.AsSlice()
    }
}

impl<T> DerefMut for Buff<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.AsMutSlice()
    }
}

impl<T> Index<u32> for Buff<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: u32) -> &Self::Output {
        assert!(index < self._Cap, "Index out of bounds");
        unsafe { &*self._Ptr.add(index as usize) }
    }
}

impl<T> IndexMut<u32> for Buff<T> {
    #[inline]
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        assert!(index < self._Cap, "Index out of bounds");
        unsafe { &mut *self._Ptr.add(index as usize) }
    }
}

impl<T> IArr<T> for Buff<T> {
    #[inline]
    fn AsSlice(&self) -> &[T] {
        self.AsSlice()
    }

    #[inline]
    fn Len(&self) -> u32 {
        self._Cap
    }
}

impl<T> IArrMut<T> for Buff<T> {
    #[inline]
    fn AsMutSlice(&mut self) -> &mut [T] {
        self.AsMutSlice()
    }
}

impl<'a, T> IntoIterator for &'a Buff<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.Iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Buff<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.IterMut()
    }
}

impl<T> Drop for Buff<T> {
    fn drop(&mut self) {
        if self._Cap > 0 && !self._Ptr.is_null() {
            // Drop initialized elements
            USeg::FromLen(self._Cap).Traverse(|i| unsafe {
                ptr::drop_in_place(self._Ptr.add(i as usize));
            });
            let layout = Layout::array::<T>(self._Cap as usize).unwrap();
            unsafe {
                dealloc(self._Ptr as *mut u8, layout);
            }
        }
    }
}
