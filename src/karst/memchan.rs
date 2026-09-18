// src/karst/memchan.rs
use crate::swarm::cpu::ComputeDevice;
use crate::swarm::traits::{BufferUsage, ComputeBuffer};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

//-------------------------------------------------------------------------------------------------

// Channel performance and traffic metrics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct MemChanStats {
pub struct DChanStats
{
    pub _ReadsServiced: u32,
    pub _WritesServiced: u32,
    pub _BytesWritten: u64,
    pub _BytesRead: u64,
}

pub type MemChanStats = DChanStats;

//-------------------------------------------------------------------------------------------------

// MemChan — simulates a DDR5-8800 physical memory channel backed by a Swarm ComputeBuffer.
pub struct MemChan {
    _chan_idx: u32,
    _capacity: u64,
    _buffer: ComputeBuffer,
    _reads_serviced: AtomicU32,
    _writes_serviced: AtomicU32,
    _bytes_written: AtomicU64,
    _bytes_read: AtomicU64,
// KarstDChan — simulates a DDR5-8800 physical memory channel backed by a Swarm ComputeBuffer.
pub struct KarstDChan
{
    _ChanIdx: u32,
    _Capacity: u64,
    _Buffer: ComputeBuffer,
    _ReadsServiced: AtomicU32,
    _WritesServiced: AtomicU32,
    _BytesWritten: AtomicU64,
    _BytesRead: AtomicU64,
}
impl MemChan {
    pub fn new(device: &ComputeDevice, chan_idx: u32, capacity: u64) -> Self {
        let label = format!("memchan_{}", chan_idx);

pub type MemChan = KarstDChan;

impl KarstDChan
{
    pub fn New(device: &ComputeDevice, chanIdx: u32, capacity: u64) -> Self
    {
        let label = format!("dchan_{}", chanIdx);
        let usage = BufferUsage::Storage() | BufferUsage::ReadWrite();
        let buffer = device.CreateBuffer(&label, capacity as usize, usage);
        Self {
            _chan_idx: chan_idx,
            _capacity: capacity,
            _buffer: buffer,
            _reads_serviced: AtomicU32::new(0),
            _writes_serviced: AtomicU32::new(0),
            _bytes_written: AtomicU64::new(0),
            _bytes_read: AtomicU64::new(0),
            _ChanIdx: chanIdx,
            _Capacity: capacity,
            _Buffer: buffer,
            _ReadsServiced: AtomicU32::new(0),
            _WritesServiced: AtomicU32::new(0),
            _BytesWritten: AtomicU64::new(0),
            _BytesRead: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn chan_idx(&self) -> u32 {
        self._chan_idx
    pub fn new(device: &ComputeDevice, chanIdx: u32, capacity: u64) -> Self
    {
        Self::New(device, chanIdx, capacity)
    }

    #[inline]
    pub fn capacity(&self) -> u64 {
        self._capacity
    pub fn ChanIdx(&self) -> u32
    {
        self._ChanIdx
    }

    #[inline]
    pub fn buffer(&self) -> &ComputeBuffer {
        &self._buffer
    pub fn chan_idx(&self) -> u32
    {
        self.ChanIdx()
    }

    #[inline]
    pub fn stats(&self) -> MemChanStats {
        MemChanStats {
            _ReadsServiced: self._reads_serviced.load(Ordering::Relaxed),
            _WritesServiced: self._writes_serviced.load(Ordering::Relaxed),
            _BytesWritten: self._bytes_written.load(Ordering::Relaxed),
            _BytesRead: self._bytes_read.load(Ordering::Relaxed),
    pub fn Capacity(&self) -> u64
    {
        self._Capacity
    }

    #[inline]
    pub fn capacity(&self) -> u64
    {
        self.Capacity()
    }

    #[inline]
    pub fn Buffer(&self) -> &ComputeBuffer
    {
        &self._Buffer
    }

    #[inline]
    pub fn buffer(&self) -> &ComputeBuffer
    {
        self.Buffer()
    }

    #[inline]
    pub fn Stats(&self) -> DChanStats
    {
        DChanStats {
            _ReadsServiced: self._ReadsServiced.load(Ordering::Relaxed),
            _WritesServiced: self._WritesServiced.load(Ordering::Relaxed),
            _BytesWritten: self._BytesWritten.load(Ordering::Relaxed),
            _BytesRead: self._BytesRead.load(Ordering::Relaxed),
        }
    }
    pub fn write_word(&self, byte_addr: u32, val: u32) -> Result<(), &'static str> {
        if !byte_addr.is_multiple_of(4) {

    #[inline]
    pub fn stats(&self) -> DChanStats
    {
        self.Stats()
    }

    pub fn WriteWord(&self, byteAddr: u32, val: u32) -> Result<(), &'static str>
    {
        if !byteAddr.is_multiple_of(4) {
            return Err("Unaligned word access");
        }
        let offset = byte_addr as usize;
        if offset + 4 > self._capacity as usize {
        let offset = byteAddr as usize;
        if offset + 4 > self._Capacity as usize {
            return Err("Out of bounds memory access");
        }
        let bytes = val.to_ne_bytes();
        if self._buffer.WriteAt(offset, &bytes).is_ok() {
            self._writes_serviced.fetch_add(1, Ordering::Relaxed);
            self._bytes_written.fetch_add(4, Ordering::Relaxed);
        if self._Buffer.WriteAt(offset, &bytes).is_ok() {
            self._WritesServiced.fetch_add(1, Ordering::Relaxed);
            self._BytesWritten.fetch_add(4, Ordering::Relaxed);
            Ok(())
        } else {
            Err("Failed to write to compute buffer")
        }
    }
    pub fn read_word(&self, byte_addr: u32) -> Result<u32, &'static str> {
        if !byte_addr.is_multiple_of(4) {

    #[inline]
    pub fn write_word(&self, byteAddr: u32, val: u32) -> Result<(), &'static str>
    {
        self.WriteWord(byteAddr, val)
    }

    pub fn ReadWord(&self, byteAddr: u32) -> Result<u32, &'static str>
    {
        if !byteAddr.is_multiple_of(4) {
            return Err("Unaligned word access");
        }
        let offset = byte_addr as usize;
        if offset + 4 > self._capacity as usize {
        let offset = byteAddr as usize;
        if offset + 4 > self._Capacity as usize {
            return Err("Out of bounds memory access");
        }
        let mut bytes = [0u8; 4];
        if self._buffer.ReadAt(offset, &mut bytes).is_ok() {
            self._reads_serviced.fetch_add(1, Ordering::Relaxed);
            self._bytes_read.fetch_add(4, Ordering::Relaxed);
        if self._Buffer.ReadAt(offset, &mut bytes).is_ok() {
            self._ReadsServiced.fetch_add(1, Ordering::Relaxed);
            self._BytesRead.fetch_add(4, Ordering::Relaxed);
            Ok(u32::from_ne_bytes(bytes))
        } else {
            Err("Failed to read from compute buffer")
        }
    }
    pub fn fill(&self, pattern: u8) {
        let _ = self._buffer.Fill(pattern);

    #[inline]
    pub fn read_word(&self, byteAddr: u32) -> Result<u32, &'static str>
    {
        self.ReadWord(byteAddr)
    }
    pub fn verify(&self, pattern: u8) -> bool {
        self._buffer.Verify(pattern)

    pub fn Fill(&self, pattern: u8)
    {
        let _ = self._Buffer.Fill(pattern);
    }

    #[inline]
    pub fn fill(&self, pattern: u8)
    {
        self.Fill(pattern);
    }

    pub fn Verify(&self, pattern: u8) -> bool
    {
        self._Buffer.Verify(pattern)
    }

    #[inline]
    pub fn verify(&self, pattern: u8) -> bool
    {
        self.Verify(pattern)
    }
}
