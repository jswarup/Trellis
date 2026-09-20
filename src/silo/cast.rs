// cast.rs ---------------------------------------------------------------------------------------------------------------------

#![allow(clippy::not_unsafe_ptr_arg_deref)] // These traits are the raw-pointer safety boundary.
use crate::silo::arr::{Arr, MutArr};

//-----------------------------------------------------------------------------------------------------------------------------

pub trait ICastExt: Sized {
    /// Casts a value to another type, asserting size equivalence at runtime in debug mode.
    /// Acts as a postfix wrapper around std::mem::transmute.
    fn Cast<U>(self) -> U;
}

//-----------------------------------------------------------------------------------------------------------------------------

impl<T: Sized> ICastExt for T {
    #[inline(always)]
    fn Cast<U>(self) -> U {
        debug_assert_eq!(
            std::mem::size_of::<T>(),
            std::mem::size_of::<U>(),
            "Cast size mismatch"
        );
        let res = unsafe { std::mem::transmute_copy(&self) };
        std::mem::forget(self);
        res
    }
}

//-------------------------------------------------------------------------------------------------

pub trait IConstPtrMutRefExt<T: ?Sized> {
    fn MutRef<'a>(self) -> &'a mut T;
}

//-----------------------------------------------------------------------------------------------------------------------------

impl<T: ?Sized> IConstPtrMutRefExt<T> for *const T {
    #[inline(always)]
    #[allow(invalid_reference_casting)]
    fn MutRef<'a>(self) -> &'a mut T {
        unsafe { &mut *(self as *mut T) }
    }
}

//-------------------------------------------------------------------------------------------------
/// Converts a mutable raw pointer into a mutable reference.
pub trait IPtrRefExt<T: ?Sized> {
    fn MutRef<'a>(self) -> &'a mut T;
}

//-----------------------------------------------------------------------------------------------------------------------------

impl<T: ?Sized> IPtrRefExt<T> for *mut T {
    #[inline(always)]
    fn MutRef<'a>(self) -> &'a mut T {
        unsafe { &mut *self }
    }
}

//-----------------------------------------------------------------------------------------------------------------------------
/// Converts a raw const pointer into a shared reference.
pub trait IConstPtrRefExt<T: ?Sized> {
    fn Ref<'a>(self) -> &'a T;
}

//-----------------------------------------------------------------------------------------------------------------------------

impl<T: ?Sized> IConstPtrRefExt<T> for *const T {
    #[inline(always)]
    fn Ref<'a>(self) -> &'a T {
        unsafe { &*self }
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

/// Accesses elements through a mutable raw pointer.
pub trait IPtrAtExt<T> {
    fn RefAt<'a>(self, index: usize) -> &'a T;
    fn MutRefAt<'a>(self, index: usize) -> &'a mut T;
}

//-----------------------------------------------------------------------------------------------------------------------------

impl<T> IPtrAtExt<T> for *mut T {
    #[inline(always)]
    fn RefAt<'a>(self, index: usize) -> &'a T {
        unsafe { &*self.add(index) }
    }
    #[inline(always)]
    fn MutRefAt<'a>(self, index: usize) -> &'a mut T {
        unsafe { &mut *self.add(index) }
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

/// Accesses elements through a const raw pointer.
pub trait IConstPtrAtExt<T> {
    fn RefAt<'a>(self, index: usize) -> &'a T;
}

//-----------------------------------------------------------------------------------------------------------------------------

impl<T> IConstPtrAtExt<T> for *const T {
    #[inline(always)]
    fn RefAt<'a>(self, index: usize) -> &'a T {
        unsafe { &*self.add(index) }
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

/// Converts raw pointers into slice views without raw unsafe blocks.
pub trait IAllocRawExt {
    fn AllocRaw(self) -> *mut Self;
}

//-----------------------------------------------------------------------------------------------------------------------------

impl<T> IAllocRawExt for T {
    #[inline(always)]
    fn AllocRaw(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

pub trait IArrExt {
    fn CastArr(&self) -> Arr<'_, u8>;
    fn CastArrFrom<U: Copy>(&self) -> Arr<'_, U>;
}

//-----------------------------------------------------------------------------------------------------------------------------

pub trait IMutArrExt {
    fn CastMutArr<U: Copy>(&mut self) -> MutArr<'_, U>;
}

//-------------------------------------------------------------------------------------------------
/// A generic fat pointer wrapper that erases lifetimes and mutability rules.
/// Use with extreme caution for work-stealing/parallel contexts where disjoint access is guaranteed.
#[derive(Copy, Clone)]
pub struct MutAliasPtr<T: ?Sized> {
    pub _Ptr: *const T,
}
impl<T: ?Sized> MutAliasPtr<T> {
    #[inline(always)]
    pub fn New(ptr: &T) -> Self {
        Self {
            _Ptr: unsafe { std::mem::transmute_copy(&ptr) },
        }
    }
    #[inline(always)]
    pub fn NewMut(ptr: &mut T) -> Self {
        Self {
            _Ptr: unsafe { std::mem::transmute_copy(&ptr) },
        }
    }
    #[inline(always)]
    #[allow(invalid_reference_casting)]
    pub fn MutRef<'a>(&self) -> &'a mut T {
        unsafe { &mut *(self._Ptr as *mut T) }
    }
    #[inline(always)]
    pub fn Ref<'a>(&self) -> &'a T {
        unsafe { &*self._Ptr }
    }
}
unsafe impl<T: ?Sized> Send for MutAliasPtr<T> {}
unsafe impl<T: ?Sized> Sync for MutAliasPtr<T> {}

//-----------------------------------------------------------------------------------------------------------------------------
