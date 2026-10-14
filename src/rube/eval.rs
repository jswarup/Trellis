// eval.rs -----------------------------------------------------------------------------------------
use crate::rube::trigger::{FUTR_MASK, TriggerId, TriggerWad};
use crate::silo::Arr;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

//-------------------------------------------------------------------------------------------------
pub(super) trait ITriggerEval
{
    fn Current(&self, id: TriggerId) -> u64;
    fn Flags(&self, id: TriggerId) -> u8;
    fn WriteFuture(&mut self, id: TriggerId, value: u64, flags: u8);
}

impl ITriggerEval for TriggerWad<u64>
{
    #[inline]
    fn Current(&self, id: TriggerId) -> u64
    {
        self.CurrentVal(id)
    }

    #[inline]
    fn Flags(&self, id: TriggerId) -> u8
    {
        self.Flags(id)
    }

    #[inline]
    fn WriteFuture(&mut self, id: TriggerId, value: u64, flags: u8)
    {
        self.WriteFuture(id, value, flags);
    }
}

//-------------------------------------------------------------------------------------------------
/// Temporary atomic access to existing storage, bounded by an exclusive bank borrow.
/// Current values and the current flag bits remain unchanged until workers join.
#[derive(Copy, Clone)]
pub(super) struct ParallelTriggers<'a>
{
    _Current: Arr<'a, u64>,
    _Future: Arr<'a, AtomicU64>,
    _Flags: Arr<'a, AtomicU8>,
}

impl<'a> ParallelTriggers<'a>
{
    pub const SUPPORTED: bool = std::mem::align_of::<AtomicU64>()
        <= std::mem::align_of::<u64>();

    pub fn New(triggers: &'a mut TriggerWad<u64>) -> Self
    {
        const { assert!(Self::SUPPORTED, "parallel trigger storage requires atomic alignment") };
        let (current, future, flags) = triggers.ParallelBorrow();
        // SAFETY: the integer buffers are initialized and atomically aligned. The
        // exclusive bank borrow prevents non-atomic access for this view's lifetime.
        // Every parallel access to future values/flag bytes uses the same atomic size.
        // Joining the worker scope happens before normal bank access resumes.
        Self { _Current: current,
               _Future: Arr::New(future.Data().cast(), future.Len()),
               _Flags: Arr::New(flags.Data().cast(), flags.Len()) }
    }
}

impl ITriggerEval for ParallelTriggers<'_>
{
    #[inline]
    fn Current(&self, id: TriggerId) -> u64
    {
        self._Current[id]
    }

    #[inline]
    fn Flags(&self, id: TriggerId) -> u8
    {
        self._Flags[id].load(Ordering::Relaxed)
    }

    #[inline]
    fn WriteFuture(&mut self, id: TriggerId, value: u64, flags: u8)
    {
        self._Future[id].store(value, Ordering::Relaxed);
        // Each output has one writer. Readers may concurrently inspect current
        // bits; those bits are preserved, so a read needs no ordering with writes.
        let previous = self._Flags[id].load(Ordering::Relaxed);
        self._Flags[id].store((previous & !FUTR_MASK) | flags, Ordering::Relaxed);
    }
}

//-------------------------------------------------------------------------------------------------
