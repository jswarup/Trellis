// src/karst/memchan.rs

use crate::swarm::cpu::ComputeDevice;
use crate::swarm::traits::{BufferUsage, ComputeBuffer};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

//-------------------------------------------------------------------------------------------------
// Channel performance and traffic metrics.

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct MemChanStats {
    pub _ReadsServiced: u32,
    pub _WritesServiced: u32,
    pub _BytesWritten: u64,
    pub _BytesRead: u64,
}

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
}

impl MemChan {
    pub fn new(device: &ComputeDevice, chan_idx: u32, capacity: u64) -> Self {
        let label = format!("memchan_{}", chan_idx);
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
        }
    }

    #[inline]
    pub fn chan_idx(&self) -> u32 {
        self._chan_idx
    }

    #[inline]
    pub fn capacity(&self) -> u64 {
        self._capacity
    }

    #[inline]
    pub fn buffer(&self) -> &ComputeBuffer {
        &self._buffer
    }

    #[inline]
    pub fn stats(&self) -> MemChanStats {
        MemChanStats {
            _ReadsServiced: self._reads_serviced.load(Ordering::Relaxed),
            _WritesServiced: self._writes_serviced.load(Ordering::Relaxed),
            _BytesWritten: self._bytes_written.load(Ordering::Relaxed),
            _BytesRead: self._bytes_read.load(Ordering::Relaxed),
        }
    }

    pub fn write_word(&self, byte_addr: u32, val: u32) {
        let offset = (byte_addr % (self._capacity as u32)) as usize;
        let bytes = val.to_ne_bytes();
        if self._buffer.WriteAt(offset, &bytes).is_ok() {
            self._writes_serviced.fetch_add(1, Ordering::Relaxed);
            self._bytes_written.fetch_add(4, Ordering::Relaxed);
        }
    }

    pub fn read_word(&self, byte_addr: u32) -> u32 {
        let offset = (byte_addr % (self._capacity as u32)) as usize;
        let mut bytes = [0u8; 4];
        if self._buffer.ReadAt(offset, &mut bytes).is_ok() {
            self._reads_serviced.fetch_add(1, Ordering::Relaxed);
            self._bytes_read.fetch_add(4, Ordering::Relaxed);
            u32::from_ne_bytes(bytes)
        } else {
            0
        }
    }

    pub fn fill(&self, pattern: u8) {
        let _ = self._buffer.Fill(pattern);
    }

    pub fn verify(&self, pattern: u8) -> bool {
        self._buffer.Verify(pattern)
    }
}
