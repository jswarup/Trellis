// fifo.rs ---------------------------------------------------------------------------------------------------------
use std::mem::MaybeUninit;

//-------------------------------------------------------------------------------------------------

// Fifo — static, fixed-size circular buffer queue.
// Designed for cycle-accurate simulation and low-latency queuing.
// Zero dynamic allocations, matching Trellis silo::Fifo.
pub struct Fifo<T, const N: usize> {
    _Data: [MaybeUninit<T>; N],
    _Head: u32,
    _Tail: u32,
    _Size: u32,
}
impl<T, const N: usize> Fifo<T, N> {
    pub const fn New() -> Self {
        assert!(N > 0, "Fifo capacity must be > 0");
        Self {
            _Data: [const { MaybeUninit::uninit() }; N],
            _Head: 0,
            _Tail: 0,
            _Size: 0,
        }
    }
    #[inline]
    pub const fn IsEmpty(&self) -> bool {
        self._Size == 0
    }
    #[inline]
    pub const fn IsFull(&self) -> bool {
        self._Size == N as u32
    }
    #[inline]
    pub const fn Size(&self) -> u32 {
        self._Size
    }
    #[inline]
    pub const fn Capacity(&self) -> u32 {
        N as u32
    }
    pub fn PushBack(&mut self, val: T) -> bool {
        if self.IsFull() {
            return false;
        }
        self._Data[self._Tail as usize].write(val);
        self._Tail += 1;
        if self._Tail == N as u32 {
            self._Tail = 0;
        }
        self._Size += 1;
        true
    }
    pub fn PopFront(&mut self) -> Option<T> {
        if self.IsEmpty() {
            None
        } else {
            let val = unsafe { self._Data[self._Head as usize].assume_init_read() };
            self._Head += 1;
            if self._Head == N as u32 {
                self._Head = 0;
            }
            self._Size -= 1;
            Some(val)
        }
    }
    pub fn Front(&self) -> Option<&T> {
        if self.IsEmpty() {
            None
        } else {
            unsafe { Some(self._Data[self._Head as usize].assume_init_ref()) }
        }
    }
    pub fn FrontMut(&mut self) -> Option<&mut T> {
        if self.IsEmpty() {
            None
        } else {
            unsafe { Some(self._Data[self._Head as usize].assume_init_mut()) }
        }
    }
    pub fn Clear(&mut self) {
        while self.PopFront().is_some() {}
    }
}
impl<T, const N: usize> Default for Fifo<T, N> {
    fn default() -> Self {
        Self::New()
    }
}
impl<T, const N: usize> Drop for Fifo<T, N> {
    fn drop(&mut self) {
        self.Clear();
    }
}
