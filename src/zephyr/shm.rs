// src/zephyr/shm.rs
//
// Inter-VM PCIe Shared Memory Protocol (Oriole zephyr_pcie_dual_vm adaptation).
// Implements BAR2 direct-mapped shared memory ring buffer layout with IVCB header and
// Fletcher-32 validated packet stream.
use	crate::silo::{ Arr, Buff };
use	std::sync::atomic::{ compiler_fence, Ordering };

//-------------------------------------------------------------------------------------------------
// Protocol Constants
pub const SHM_IVCB_MAGIC: u32 = 0x5A45_5048;                           // 'ZEPH'
pub const SHM_PKT_MAGIC: u32 = 0x504B_5431;                            // 'PKT1'
/// Default BAR2 is 4 MiB; reserve 256 B at offset 0 for IVCB control block.
pub const SHM_RING_CAPACITY: u32 = 4 * 1024 * 1024 - 256;
pub const SHM_IVCB_SIZE: usize = 256;
pub const SHM_PKT_SIZE: usize = 280;
pub const SHM_PAYLOAD_CAPACITY: usize = 256;

//-------------------------------------------------------------------------------------------------
// Fletcher-32 Checksum Algorithm (Bitwise identical to Oriole pcie_shm.c)
pub fn	Fletcher32( data: Arr< '_, u8>) -> u32
{
    let  	mut sum1: u32 = 0xffff;
    let  	mut sum2: u32 = 0xffff;
    let  	mut words = data.Len() / 2;
    let  	mut offset = 0u32;
    while words > 0 {
        let  	mut tlen = if words > 359 { 359 } else { words };
        words -= tlen;
        while tlen > 0 {
            let  	word = u16::from_le_bytes( [data[offset], data[offset + 1]]) as u32;
            offset += 2;
            sum1 += word;
            sum2 += sum1;
            tlen -= 1;
        }
        sum1 = ( sum1 & 0xffff) + ( sum1 >> 16);
        sum2 = ( sum2 & 0xffff) + ( sum2 >> 16);
    }
    if ( data.Len() & 1) != 0 {
        sum1 += data[offset] as u32;
        sum2 += sum1;
        sum1 = ( sum1 & 0xffff) + ( sum1 >> 16);
        sum2 = ( sum2 & 0xffff) + ( sum2 >> 16);
    }
    sum1 = ( sum1 & 0xffff) + ( sum1 >> 16);
    sum2 = ( sum2 & 0xffff) + ( sum2 >> 16);
    ( sum2 << 16) | sum1
}

//-------------------------------------------------------------------------------------------------
// Inter-VM Control Block (IVCB) - 256 bytes packed at offset 0 of BAR2
#[derive( Copy, Clone, Debug, PartialEq, Eq)]
pub struct ShmIvcb
{
    pub _Magic: u32,
    pub _Version: u32,
    pub _SenderReady: u32,
    pub _ReceiverReady: u32,
    pub _WriteOffset: u32,
    pub _ReadOffset: u32,
    pub _RingSize: u32,
    pub _Reserved: [u32; 57],
}
impl Default for ShmIvcb {
    fn	default() -> Self
    {
        Self::New( SHM_RING_CAPACITY)
    }
}
impl ShmIvcb
{
    pub fn	New( ringSize: u32) -> Self
    {
        Self {
            _Magic: SHM_IVCB_MAGIC,
            _Version: 1,
            _SenderReady: 0,
            _ReceiverReady: 0,
            _WriteOffset: 0,
            _ReadOffset: 0,
            _RingSize: ringSize,
            _Reserved: [0u32; 57],
        }
    }
    #[inline]
    pub fn	IsValid( &self) -> bool
    {
        self._Magic == SHM_IVCB_MAGIC && self._Version == 1
    }
    #[inline]
    pub fn	IsSenderReady( &self) -> bool
    {
        self._SenderReady != 0
    }
    #[inline]
    pub fn	IsReceiverReady( &self) -> bool
    {
        self._ReceiverReady != 0
    }
    #[inline]
    pub fn	SetSenderReady( &mut self, ready: bool)
    {
        self._SenderReady = if ready { 1 } else { 0 };
    }
    #[inline]
    pub fn	SetReceiverReady( &mut self, ready: bool)
    {
        self._ReceiverReady = if ready { 1 } else { 0 };
    }
    #[inline]
    pub fn	WriteOffset( &self) -> u32
    {
        self._WriteOffset
    }
    #[inline]
    pub fn	SetWriteOffset( &mut self, offset: u32)
    {
        self._WriteOffset = offset;
    }
    #[inline]
    pub fn	ReadOffset( &self) -> u32
    {
        self._ReadOffset
    }
    #[inline]
    pub fn	SetReadOffset( &mut self, offset: u32)
    {
        self._ReadOffset = offset;
    }
    #[inline]
    pub fn	RingSize( &self) -> u32
    {
        self._RingSize
    }
    pub fn	ToBytes( &self) -> [u8; SHM_IVCB_SIZE]
    {
        let  	mut out = [0u8; SHM_IVCB_SIZE];
        out[0..4].copy_from_slice( &self._Magic.to_le_bytes());
        out[4..8].copy_from_slice( &self._Version.to_le_bytes());
        out[8..12].copy_from_slice( &self._SenderReady.to_le_bytes());
        out[12..16].copy_from_slice( &self._ReceiverReady.to_le_bytes());
        out[16..20].copy_from_slice( &self._WriteOffset.to_le_bytes());
        out[20..24].copy_from_slice( &self._ReadOffset.to_le_bytes());
        out[24..28].copy_from_slice( &self._RingSize.to_le_bytes());
        let  	mut off = 28u32;
        let  	mut i = 0u32;
        while i < 57 {
            out[off as usize..( off + 4) as usize].copy_from_slice( &self._Reserved[i as usize].to_le_bytes());
            off += 4;
            i += 1;
        }
        out
    }
    pub fn	FromBytes( bytes: Arr< '_, u8>) -> Result< Self, &'static str>
    {
        if bytes.Len() < SHM_IVCB_SIZE as u32 {
            return Err( "Byte buffer too small for ShmIvcb");
        }
        let  	word = |i| u32::from_le_bytes( [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        let  	magic = word( 0);
        let  	version = word( 4);
        let  	senderReady = word( 8);
        let  	receiverReady = word( 12);
        let  	writeOffset = word( 16);
        let  	readOffset = word( 20);
        let  	ringSize = word( 24);
        let  	mut reserved = [0u32; 57];
        let  	mut off = 28u32;
        let  	mut i = 0u32;
        while i < 57 {
            reserved[i as usize] = word( off);
            off += 4;
            i += 1;
        }
        Ok( Self {
            _Magic: magic,
            _Version: version,
            _SenderReady: senderReady,
            _ReceiverReady: receiverReady,
            _WriteOffset: writeOffset,
            _ReadOffset: readOffset,
            _RingSize: ringSize,
            _Reserved: reserved,
        })
    }
}

//-------------------------------------------------------------------------------------------------
// Shared Memory Packet - 280 bytes packed inside ring buffer
#[derive( Copy, Clone, Debug, PartialEq, Eq)]
pub struct ShmPacket
{
    pub _Magic: u32,
    pub _SeqNum: u32,
    pub _PayloadLen: u32,
    pub _Checksum: u32,
    pub _TimestampMs: u64,
    pub _Payload: [u8; SHM_PAYLOAD_CAPACITY],
}
impl Default for ShmPacket {
    fn	default() -> Self
    {
        Self {
            _Magic: SHM_PKT_MAGIC,
            _SeqNum: 0,
            _PayloadLen: 0,
            _Checksum: 0,
            _TimestampMs: 0,
            _Payload: [0u8; SHM_PAYLOAD_CAPACITY],
        }
    }
}
impl ShmPacket
{
    pub fn	New( seqNum: u32, timestampMs: u64, payload: Arr< '_, u8>) -> Self
    {
        let  	len = payload.Len().min( SHM_PAYLOAD_CAPACITY as u32);
        let  	mut buf = [0u8; SHM_PAYLOAD_CAPACITY];
        payload.USeg().RSnip( payload.Len() - len).Traverse( |i| buf[i as usize] = payload[i]);
        let  	csum = Fletcher32( Arr::New( buf.as_ptr(), len));
        Self {
            _Magic: SHM_PKT_MAGIC,
            _SeqNum: seqNum,
            _PayloadLen: len,
            _Checksum: csum,
            _TimestampMs: timestampMs,
            _Payload: buf,
        }
    }
    #[inline]
    pub fn	VerifyChecksum( &self) -> bool
    {
        let  	len = self._PayloadLen.min( SHM_PAYLOAD_CAPACITY as u32);
        let  	computed = Fletcher32( Arr::New( self._Payload.as_ptr(), len));
        computed == self._Checksum
    }
    #[inline]
    pub fn	PayloadBytes( &self) -> Arr< '_, u8>
    {
        let  	len = self._PayloadLen.min( SHM_PAYLOAD_CAPACITY as u32);
        Arr::New( self._Payload.as_ptr(), len)
    }
    #[inline]
    pub fn	PayloadStr( &self) -> &str
    {
        let  	bytes = self.PayloadBytes();
        let  	raw = bytes.into();
        std::str::from_utf8( raw).unwrap_or( "")
    }
    pub fn	ToBytes( &self) -> [u8; SHM_PKT_SIZE]
    {
        let  	mut out = [0u8; SHM_PKT_SIZE];
        out[0..4].copy_from_slice( &self._Magic.to_le_bytes());
        out[4..8].copy_from_slice( &self._SeqNum.to_le_bytes());
        out[8..12].copy_from_slice( &self._PayloadLen.to_le_bytes());
        out[12..16].copy_from_slice( &self._Checksum.to_le_bytes());
        out[16..24].copy_from_slice( &self._TimestampMs.to_le_bytes());
        out[24..280].copy_from_slice( &self._Payload);
        out
    }
    pub fn	FromBytes( bytes: Arr< '_, u8>) -> Result< Self, &'static str>
    {
        if bytes.Len() < SHM_PKT_SIZE as u32 {
            return Err( "Byte buffer too small for ShmPacket");
        }
        let  	word = |i| u32::from_le_bytes( [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        let  	magic = word( 0);
        let  	seqNum = word( 4);
        let  	payloadLen = word( 8);
        let  	checksum = word( 12);
        let  	timestampMs = u64::from_le_bytes( [
            bytes[16], bytes[17], bytes[18], bytes[19],
            bytes[20], bytes[21], bytes[22], bytes[23],
        ]);
        let  	mut payload = [0u8; SHM_PAYLOAD_CAPACITY];
        crate::silo::arr::MutArr::from( &mut payload)
            .CopyFrom( bytes.Slice( 24, SHM_PAYLOAD_CAPACITY as u32));
        Ok( Self {
            _Magic: magic,
            _SeqNum: seqNum,
            _PayloadLen: payloadLen,
            _Checksum: checksum,
            _TimestampMs: timestampMs,
            _Payload: payload,
        })
    }
}

//-------------------------------------------------------------------------------------------------
// ShmRingBuffer - Emulates BAR2 PCIe Shared Memory Region for Dual VM
pub struct ShmRingBuffer
{
    _Ivcb: ShmIvcb,
    _Storage: Buff< u8>,
    _Capacity: u32,
}
impl ShmRingBuffer
{
    pub fn	New( capacity: u32) -> Self
    {
        let  	cap = if capacity < SHM_PKT_SIZE as u32 {
            SHM_RING_CAPACITY
        } else {
            capacity
        };
        Self {
            _Ivcb: ShmIvcb::New( cap),
            _Storage: Buff::< u8>::FromDispenser( cap, |_| 0u8),
            _Capacity: cap,
        }
    }
    #[inline]
    pub fn	Ivcb( &self) -> &ShmIvcb
    {
        &self._Ivcb
    }
    #[inline]
    pub fn	IvcbMut( &mut self) -> &mut ShmIvcb
    {
        &mut self._Ivcb
    }
    pub fn	InitSender( &mut self)
    {
        self._Ivcb._Magic = SHM_IVCB_MAGIC;
        self._Ivcb._Version = 1;
        self._Ivcb._WriteOffset = 0;
        self._Ivcb._ReadOffset = 0;
        self._Ivcb._RingSize = self._Capacity;
        self._Ivcb._SenderReady = 1;
        compiler_fence( Ordering::Release);
    }
    pub fn	InitReceiver( &mut self)
    {
        self._Ivcb._ReceiverReady = 1;
        compiler_fence( Ordering::Release);
    }
    pub fn	PushPacket( &mut self, pkt: &ShmPacket) -> Result< (), &'static str>
    {
        let  	pktBytes = pkt.ToBytes();
        let  	pktLen = SHM_PKT_SIZE as u32;
        let  	mut wr = self._Ivcb._WriteOffset;
        if wr + pktLen > self._Capacity {
            wr = 0;
        }
        let  	slice: &mut [u8] = self._Storage.MutArr().into();
        slice[wr as usize..wr as usize + SHM_PKT_SIZE].copy_from_slice( &pktBytes);
        compiler_fence( Ordering::Release);
        self._Ivcb._WriteOffset = wr + pktLen;
        Ok( ())
    }
    pub fn	PopPacket( &mut self) -> Result< Option< ShmPacket>, &'static str>
    {
        let  	rd = self._Ivcb._ReadOffset;
        let  	wr = self._Ivcb._WriteOffset;
        if rd == wr {
            return Ok( None);
        }
        let  	pktLen = SHM_PKT_SIZE as u32;
        let  	effectiveRd = if rd + pktLen > self._Capacity {
            0
        } else {
            rd
        };
        let  	pkt = ShmPacket::FromBytes( self._Storage.Arr().Slice( effectiveRd, pktLen))?;
        compiler_fence( Ordering::Acquire);
        self._Ivcb._ReadOffset = effectiveRd + pktLen;
        if pkt._Magic != SHM_PKT_MAGIC {
            return Err( "Invalid packet magic in SHM ring");
        }
        Ok( Some( pkt))
    }
    #[inline]
    pub fn	HasPackets( &self) -> bool
    {
        self._Ivcb._ReadOffset != self._Ivcb._WriteOffset
    }
}
