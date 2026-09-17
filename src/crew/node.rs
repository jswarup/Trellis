// src/crew/node.rs
use crate::stalks::work::SpinMutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

//-------------------------------------------------------------------------------------------------

// Node performance and co-simulation metrics.
#[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct NodeStats
{
    pub _BytesSent: u32,
    pub _BytesReceived: u32,
    pub _ReadsServiced: u32,
    pub _WritesServiced: u32,
}

//-------------------------------------------------------------------------------------------------

// Concrete representation of a co-simulated VM node endpoint.
pub struct CrewNode
{
    _id: u32,
    _is_online: AtomicBool,
    _rx_queue: SpinMutex< VecDeque< u8>>,
    _reads_serviced: AtomicU32,
    _writes_serviced: AtomicU32,
    _bytes_sent: AtomicU32,
    _bytes_received: AtomicU32,
}
impl CrewNode
{
    pub fn  new( id: u32) -> Self
    {
        Self {
            _id: id,
            _is_online: AtomicBool::new( false),
            _rx_queue: SpinMutex::New( VecDeque::new()),
            _reads_serviced: AtomicU32::new( 0),
            _writes_serviced: AtomicU32::new( 0),
            _bytes_sent: AtomicU32::new( 0),
            _bytes_received: AtomicU32::new( 0),
        }
    }
    #[inline]
    pub fn  id( &self) -> u32
    {
        self._id
    }
    #[inline]
    pub fn  is_online( &self) -> bool
    {
        self._is_online.load( Ordering::Acquire)
    }
    #[inline]
    pub fn  set_online( &self, online: bool)
    {
        self._is_online.store( online, Ordering::Release);
    }
    pub fn  push_rx( &self, byte: u8)
    {
        let  mut rx = self._rx_queue.Lock();
        rx.push_back( byte);
    }
    pub fn  pop_rx( &self, out_byte: &mut u8) -> bool
    {
        let  mut rx = self._rx_queue.Lock();
        if let  Some( b) = rx.pop_front()
        {
            *out_byte = b;
            self._bytes_received.fetch_add( 1, Ordering::Relaxed);
            true
        } else
        {
            false
        }
    }
    pub fn  rx_count( &self) -> u32
    {
        let  rx = self._rx_queue.Lock();
        rx.len() as u32
    }
    pub fn  clear_rx( &self)
    {
        let  mut rx = self._rx_queue.Lock();
        rx.clear();
    }
    pub fn  get_stats( &self) -> NodeStats
    {
        NodeStats {
            _BytesSent: self._bytes_sent.load( Ordering::Relaxed),
            _BytesReceived: self._bytes_received.load( Ordering::Relaxed),
            _ReadsServiced: self._reads_serviced.load( Ordering::Relaxed),
            _WritesServiced: self._writes_serviced.load( Ordering::Relaxed),
        }
    }
    pub fn  reset_stats( &self)
    {
        self._bytes_sent.store( 0, Ordering::Relaxed);
        self._bytes_received.store( 0, Ordering::Relaxed);
        self._reads_serviced.store( 0, Ordering::Relaxed);
        self._writes_serviced.store( 0, Ordering::Relaxed);
    }
    pub fn  record_read( &self)
    {
        self._reads_serviced.fetch_add( 1, Ordering::Relaxed);
    }
    pub fn  record_write( &self)
    {
        self._writes_serviced.fetch_add( 1, Ordering::Relaxed);
    }
    pub fn  record_byte_sent( &self)
    {
        self._bytes_sent.fetch_add( 1, Ordering::Relaxed);
    }
}
