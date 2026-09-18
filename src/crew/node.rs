//-- node.rs --------------------------------------------------------------------------------------

//--------------------------------------------------------------------------------------------------

use crate::silo::fifo::Fifo;
use crate::stalks::work::SpinMutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

//--------------------------------------------------------------------------------------------------

/// Node performance and co-simulation metrics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct NodeStats {
    pub _BytesSent: u32,
    pub _BytesReceived: u32,
    pub _BytesRejected: u32,
    pub _RxQueueFull: u32,
    pub _ReadsServiced: u32,
    pub _WritesServiced: u32,
}

//--------------------------------------------------------------------------------------------------

/// Concrete representation of a co-simulated VM node endpoint.
pub struct CrewNode {
    _Id: u32,
    _IsOnline: AtomicBool,
    _RxQueue: SpinMutex<Fifo<u8, 256>>,
    _ReadsServiced: AtomicU32,
    _WritesServiced: AtomicU32,
    _BytesSent: AtomicU32,
    _BytesReceived: AtomicU32,
    _BytesRejected: AtomicU32,
    _RxQueueFull: AtomicU32,
}

impl CrewNode {
    pub fn New(id: u32) -> Self {
        Self {
            _Id: id,
            _IsOnline: AtomicBool::new(false),
            _RxQueue: SpinMutex::New(Fifo::New()),
            _ReadsServiced: AtomicU32::new(0),
            _WritesServiced: AtomicU32::new(0),
            _BytesSent: AtomicU32::new(0),
            _BytesReceived: AtomicU32::new(0),
            _BytesRejected: AtomicU32::new(0),
            _RxQueueFull: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn new(id: u32) -> Self {
        Self::New(id)
    }

    #[inline]
    pub fn Id(&self) -> u32 {
        self._Id
    }

    #[inline]
    pub fn id(&self) -> u32 {
        self.Id()
    }

    #[inline]
    pub fn IsOnline(&self) -> bool {
        self._IsOnline.load(Ordering::Acquire)
    }

    #[inline]
    pub fn is_online(&self) -> bool {
        self.IsOnline()
    }

    #[inline]
    pub fn SetOnline(&self, online: bool) {
        self._IsOnline.store(online, Ordering::Release);
    }

    #[inline]
    pub fn set_online(&self, online: bool) {
        self.SetOnline(online);
    }

    pub fn PushRx(&self, byte: u8) -> bool {
        let mut rx = self._RxQueue.Lock();
        if rx.PushBack(byte) {
            self._BytesReceived.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            self._BytesRejected.fetch_add(1, Ordering::Relaxed);
            self._RxQueueFull.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    #[inline]
    pub fn push_rx(&self, byte: u8) -> bool {
        self.PushRx(byte)
    }

    pub fn CanPushRx(&self) -> bool {
        !self._RxQueue.Lock().IsFull()
    }

    pub fn PopRx(&self, outByte: &mut u8) -> bool {
        let mut rx = self._RxQueue.Lock();
        if let Some(b) = rx.PopFront() {
            *outByte = b;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn pop_rx(&self, outByte: &mut u8) -> bool {
        self.PopRx(outByte)
    }

    pub fn RxCount(&self) -> u32 {
        let rx = self._RxQueue.Lock();
        rx.Size()
    }

    #[inline]
    pub fn rx_count(&self) -> u32 {
        self.RxCount()
    }

    pub fn ClearRx(&self) {
        let mut rx = self._RxQueue.Lock();
        rx.Clear();
    }

    #[inline]
    pub fn clear_rx(&self) {
        self.ClearRx();
    }

    pub fn GetStats(&self) -> NodeStats {
        NodeStats {
            _BytesSent: self._BytesSent.load(Ordering::Relaxed),
            _BytesReceived: self._BytesReceived.load(Ordering::Relaxed),
            _BytesRejected: self._BytesRejected.load(Ordering::Relaxed),
            _RxQueueFull: self._RxQueueFull.load(Ordering::Relaxed),
            _ReadsServiced: self._ReadsServiced.load(Ordering::Relaxed),
            _WritesServiced: self._WritesServiced.load(Ordering::Relaxed),
        }
    }

    #[inline]
    pub fn get_stats(&self) -> NodeStats {
        self.GetStats()
    }

    pub fn ResetStats(&self) {
        self._BytesSent.store(0, Ordering::Relaxed);
        self._BytesReceived.store(0, Ordering::Relaxed);
        self._BytesRejected.store(0, Ordering::Relaxed);
        self._RxQueueFull.store(0, Ordering::Relaxed);
        self._ReadsServiced.store(0, Ordering::Relaxed);
        self._WritesServiced.store(0, Ordering::Relaxed);
    }

    #[inline]
    pub fn reset_stats(&self) {
        self.ResetStats();
    }

    #[inline]
    pub fn RecordRead(&self) {
        self._ReadsServiced.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_read(&self) {
        self.RecordRead();
    }

    #[inline]
    pub fn RecordWrite(&self) {
        self._WritesServiced.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_write(&self) {
        self.RecordWrite();
    }

    #[inline]
    pub fn RecordByteSent(&self) {
        self._BytesSent.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_byte_sent(&self) {
        self.RecordByteSent();
    }
}
