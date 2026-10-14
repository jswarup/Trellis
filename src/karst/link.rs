// src/karst/link.rs
use crate::karst::address::MemoryFault;
//-------------------------------------------------------------------------------------------------
// KarstFlit — packed 64-bit transaction word carried over KarstLinks.
// Bit [63]:    Request: IsWrite. Response: IsFault (1 = fault, 0 = read data).
// Bits [62:56]: Source node ID (7 bits: 0..127)
// Bits [55:32]: Target byte address (24 bits: up to 16 MB addressable space)
// Bits [31:0]: Data word, or MemoryFault code when a response has bit 63 set.
// Successful writes have no response. Direction determines the bit-63 meaning;
// response flits must never be interpreted as requests.
#[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct KarstFlit
{
    pub _Addr: u32,
    pub _Data: u32,
    pub _SrcId: u8,
    pub _IsWrite: bool,
}
impl KarstFlit
{
    pub const fn PackFaultResponse( addr: u32, srcId: u8, fault: MemoryFault) -> u64
    {
        Self::Pack( addr, fault as u32, srcId, true)
    }

    /// Only use on the response path, not on a write request.
    pub const fn ResponseFault( &self) -> Option< MemoryFault>
    {
        if self._IsWrite
        {
            Some( MemoryFault::FromCode( self._Data))
        }
        else
        {
            None
        }
    }

    pub const fn	new( addr: u32, data: u32, src_id: u8, is_write: bool) -> Self
    {
        Self {
            _Addr: addr,
            _Data: data,
            _SrcId: src_id,
            _IsWrite: is_write,
        }
    }
    #[inline]
    pub const fn	Pack( addr: u32, data: u32, src_id: u8, is_write: bool) -> u64
    {
        let  	mut raw: u64 = if is_write { 1u64 << 63 } else { 0 };
        raw |= ( ( src_id & 0x7F) as u64) << 56;
        raw |= ( ( addr & 0x00FF_FFFF) as u64) << 32;
        raw |= data as u64;
        raw
    }
    #[inline]
    pub const fn	Unpack( raw: u64) -> Self
    {
        Self {
            _IsWrite: ( raw & ( 1u64 << 63)) != 0,
            _SrcId: ( ( raw >> 56) & 0x7F) as u8,
            _Addr: ( ( raw >> 32) & 0x00FF_FFFF) as u32,
            _Data: ( raw & 0xFFFF_FFFF) as u32,
        }
    }
    #[inline]
    pub fn	to_raw( &self) -> u64
    {
        Self::Pack( self._Addr, self._Data, self._SrcId, self._IsWrite)
    }
    #[inline]
    pub fn	ToLeBytes( &self) -> [u8; 8]
    {
        self.to_raw().to_le_bytes()
    }
    #[inline]
    pub fn	to_le_bytes( &self) -> [u8; 8]
    {
        self.ToLeBytes()
    }
    #[inline]
    pub fn	FromLeBytes( bytes: [u8; 8]) -> Self
    {
        Self::Unpack( u64::from_le_bytes( bytes))
    }
    #[inline]
    pub fn	from_le_bytes( bytes: [u8; 8]) -> Self
    {
        Self::FromLeBytes( bytes)
    }
}

//-------------------------------------------------------------------------------------------------
// KarstLink — bidirectional streaming link state with valid / data / ready handshaking.
#[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct KarstLinkChannel
{
    _valid: bool,
    _data:  u64,
    _ready: bool,
}
impl KarstLinkChannel
{
    pub const fn	new() -> Self
    {
        Self {
            _valid: false,
            _data: 0,
            _ready: true,
        }
    }
    pub const fn	new_with( valid: bool, data: u64, ready: bool) -> Self
    {
        Self {
            _valid: valid,
            _data: data,
            _ready: ready,
        }
    }
    #[inline]
    pub const fn	valid( &self) -> bool
    {
        self._valid
    }
    #[inline]
    pub const fn	data( &self) -> u64
    {
        self._data
    }
    #[inline]
    pub const fn	ready( &self) -> bool
    {
        self._ready
    }
    #[inline]
    pub fn	is_transfer( &self) -> bool
    {
        self._valid && self._ready
    }
    #[inline]
    pub fn	clear( &mut self)
    {
        self._valid = false;
        self._data = 0;
    }
    #[inline]
    pub fn	set_valid( &mut self, valid: bool)
    {
        self._valid = valid;
    }
    #[inline]
    pub fn	set_data( &mut self, data: u64)
    {
        self._data = data;
    }
    #[inline]
    pub fn	set_ready( &mut self, ready: bool)
    {
        self._ready = ready;
    }
}
#[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct KarstLink
{
    _tx: KarstLinkChannel,
    _rx: KarstLinkChannel,
}
impl KarstLink
{
    pub const fn	new() -> Self
    {
        Self {
            _tx: KarstLinkChannel::new(),
            _rx: KarstLinkChannel::new(),
        }
    }
    pub const fn	new_with( tx: KarstLinkChannel, rx: KarstLinkChannel) -> Self
    {
        Self { _tx: tx, _rx: rx }
    }
    #[inline]
    pub const fn	tx( &self) -> &KarstLinkChannel
    {
        &self._tx
    }
    #[inline]
    pub fn	tx_mut( &mut self) -> &mut KarstLinkChannel
    {
        &mut self._tx
    }
    #[inline]
    pub const fn	rx( &self) -> &KarstLinkChannel
    {
        &self._rx
    }
    #[inline]
    pub fn	rx_mut( &mut self) -> &mut KarstLinkChannel
    {
        &mut self._rx
    }
    /// Point-to-point connection transfer between two bidirectional links a and b.
    /// a.tx drives b.rx; b.tx drives a.rx.
    #[inline]
    pub fn	transfer( a: &mut KarstLink, b: &mut KarstLink)
    {
        b._rx._valid = a._tx._valid;
        b._rx._data = a._tx._data;
        a._tx._ready = b._rx._ready;
        a._rx._valid = b._tx._valid;
        a._rx._data = b._tx._data;
        b._tx._ready = a._rx._ready;
    }
}
