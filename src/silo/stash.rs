use crate::silo::arr::{Arr, MutArr};
use crate::silo::buff::Buff;
use crate::silo::cast::{IConstPtrAtExt, IMutPtrSliceExt, IPtrAtExt, IPtrSliceExt};
use crate::silo::seg::USeg;
use crate::silo::stk::Stk;
use std::alloc::{Layout, alloc};
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr;
use std::sync::atomic::{AtomicU32, Ordering};

//-------------------------------------------------------------------------------------------------

// Stash — dynamic array container for building up elements backed by an owning Buff.
// Exactly 24 bytes (_Buff is 16 bytes, _Sz is 4 bytes + 4 alignment), zero-virtual.
// Modeled directly from Trellis silo/stash.h and Kosh silo/stash.rs.
// Designed for local instantiation to build and fill elements, extracting into a Buff.
pub struct Stash<T> {
    _Buff: Buff<T>,
    _Sz: AtomicU32,
}
unsafe impl<T: Send> Send for Stash<T> {}
unsafe impl<T: Sync> Sync for Stash<T> {}
impl<T> Stash<T> {
    //---------------------------------------------------------------------------------------------

    // Constructors & Factories
    #[inline]
    pub const fn New() -> Self {
        Self {
            _Buff: Buff::New(),
            _Sz: AtomicU32::new(0),
        }
    }
    pub fn WithCapacity(capacity: u32) -> Self {
        Self {
            _Buff: Buff::WithCapacity(capacity),
            _Sz: AtomicU32::new(0),
        }
    }
    pub fn FromDispenser<F: FnMut(u32) -> T>(
        capacity: u32,
        initial_size: u32,
        mut dispenser: F,
    ) -> Self {
        let cap = capacity.max(initial_size);
        let mut buff: Buff<T> = Buff::WithCapacity(cap);
        USeg::FromLen(initial_size).Traverse(|i| unsafe {
            ptr::write(buff.AsMutPtr().add(i as usize), dispenser(i));
        });
        Self {
            _Buff: buff,
            _Sz: AtomicU32::new(initial_size),
        }
    }

    //---------------------------------------------------------------------------------------------

    // Capacity & Growth
    fn Allocate(capacity: u32) -> *mut T {
        let layout = Layout::array::<T>(capacity as usize).expect("Capacity overflow");
        assert!(
            layout.size() > 0,
            "Zero-sized types unsupported in raw Buff"
        );
        let ptr = unsafe { alloc(layout) as *mut T };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        ptr
    }
    fn ReplaceBuffer(&mut self, new_cap: u32) {
        let cur_sz = self.Size();
        assert!(
            new_cap >= cur_sz,
            "New capacity cannot discard initialized elements"
        );
        let new_ptr = Self::Allocate(new_cap);
        let mut old_buff = self._Buff.Take();
        if cur_sz > 0 {
            unsafe {
                ptr::copy_nonoverlapping(old_buff.AsPtr(), new_ptr, cur_sz as usize);
            }
        }
        old_buff.Destroy(0);
        self._Buff = unsafe { Buff::FromRawParts(new_ptr, new_cap) };
    }
    fn grow(&mut self) {
        let cur_cap = self._Buff.Cap();
        let new_cap = if cur_cap == 0 {
            4
        } else {
            cur_cap.checked_mul(2).expect("Capacity overflow")
        };
        self.ReplaceBuffer(new_cap);
    }
    pub fn Reserve(&mut self, new_cap: u32) {
        if new_cap > self._Buff.Cap() {
            self.ReplaceBuffer(new_cap);
        }
    }

    //---------------------------------------------------------------------------------------------

    // Dynamic Insertion & Extraction
    pub fn Push(&mut self, val: T) {
        self.PushBack(val);
    }
    pub fn PushBack(&mut self, val: T) {
        let cur_sz = self.Size();
        if cur_sz >= self._Buff.Cap() {
            self.grow();
        }
        unsafe {
            ptr::write(self._Buff.AsMutPtr().add(cur_sz as usize), val);
        }
        self._Sz.store(cur_sz + 1, Ordering::Release);
    }
    pub fn Pop(&mut self) -> Option<T> {
        let cur_sz = self.Size();
        if cur_sz == 0 {
            None
        } else {
            let new_sz = cur_sz - 1;
            self._Sz.store(new_sz, Ordering::Release);
            unsafe { Some(ptr::read(self._Buff.AsPtr().add(new_sz as usize))) }
        }
    }
    pub fn PopBack(&mut self) -> bool {
        self.Pop().is_some()
    }
    pub fn Clear(&mut self) {
        while self.Pop().is_some() {}
    }
    pub fn ExtractBuff(mut self) -> Buff<T> {
        let cur_sz = self.Size();
        let cur_cap = self._Buff.Cap();
        if cur_sz == cur_cap {
            self._Sz.store(0, Ordering::Release);
            return self._Buff.Take();
        }
        if cur_sz == 0 {
            self._Sz.store(0, Ordering::Release);
            return Buff::New();
        }
        self.ReplaceBuffer(cur_sz);
        self._Sz.store(0, Ordering::Release);
        self._Buff.Take()
    }

    //---------------------------------------------------------------------------------------------

    // Accessors & Queries
    #[inline]
    pub fn Size(&self) -> u32 {
        self._Sz.load(Ordering::Acquire)
    }
    #[inline]
    pub fn Len(&self) -> u32 {
        self.Size()
    }
    #[inline]
    pub fn Capacity(&self) -> u32 {
        self._Buff.Cap()
    }
    #[inline]
    pub fn IsEmpty(&self) -> bool {
        self.Size() == 0
    }
    #[inline]
    pub fn Data(&self) -> *const T {
        self._Buff.AsPtr()
    }
    #[inline]
    pub fn DataMut(&mut self) -> *mut T {
        self._Buff.AsMutPtr()
    }
    #[inline]
    pub fn AsSlice(&self) -> &[T] {
        let sz = self.Size();
        self._Buff.AsPtr().AsSlice(sz as usize)
    }
    #[inline]
    pub fn AsMutSlice(&mut self) -> &mut [T] {
        let sz = self.Size();
        self._Buff.AsMutPtr().AsMutSlice(sz as usize)
    }
    #[inline]
    pub fn AsArr(&self) -> Arr<'_, T> {
        Arr::New(self._Buff.AsPtr(), self.Size())
    }
    #[inline]
    pub fn AsMutArr(&mut self) -> MutArr<'_, T> {
        let sz = self.Size();
        MutArr::New(self._Buff.AsMutPtr(), sz)
    }
    #[inline]
    pub fn Arr(&self) -> Arr<'_, T> {
        self.AsArr()
    }
    #[inline]
    pub fn MutArr(&mut self) -> MutArr<'_, T> {
        self.AsMutArr()
    }
    #[inline]
    pub fn StkView<'a>(&'a self) -> Stk<'a, T> {
        Stk::Create(
            &self._Sz,
            MutArr::New(self._Buff.AsPtr() as *mut T, self._Buff.Cap()),
        ) // Stk needs a MutArr, wait, `Stk::Create` borrows `self._Sz`. But `StkView` returns a Stk containing `MutArr`. Is `self._Buff.AsMutPtr()` allowed here?
        // Wait, StkView borrows `self` immutably (`&self`), so it can't create `MutArr`!
        // Let's look at the original: `MutArr::New(self._Buff._Ptr, self._Buff._Cap)`. Wait, `_Ptr` was `*mut T`, so it just copied it. Yes, `AsPtr() as *mut T` will do the same.
    }
    #[inline]
    pub fn USeg(&self) -> USeg {
        USeg::FromLen(self.Size())
    }
}
impl<T> Default for Stash<T> {
    fn default() -> Self {
        Self::New()
    }
}
impl<T> Deref for Stash<T> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.AsSlice()
    }
}
impl<T> DerefMut for Stash<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.AsMutSlice()
    }
}
impl<T> Index<u32> for Stash<T> {
    type Output = T;
    #[inline]
    fn index(&self, index: u32) -> &Self::Output {
        assert!(index < self.Size(), "Index out of bounds");
        self._Buff.AsPtr().RefAt(index as usize)
    }
}
impl<T> IndexMut<u32> for Stash<T> {
    #[inline]
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        assert!(index < self.Size(), "Index out of bounds");
        self._Buff.AsMutPtr().MutRefAt(index as usize)
    }
}
impl<T> Drop for Stash<T> {
    fn drop(&mut self) {
        let cur_sz = self.Size();
        self._Buff.Destroy(cur_sz);
    }
}
