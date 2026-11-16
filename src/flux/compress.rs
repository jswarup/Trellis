//-- compress.rs -----------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::silo::buff::Buff;
use	crate::silo::stash::Stash;
use	std::io::{ Read, Write };

pub const K_DEFAULT_CHUNK_SIZE: usize = 65536;

//--------------------------------------------------------------------------------------------------
/// Compression scheme applied to Flux stream chunks and payloads.
#[derive( Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum FlxCompressKind
{
    #[default]
    None      = 0,
    Rle       = 1,
    ChunkedLz = 2,
}

impl FlxCompressKind
{
    pub const fn	ToU8( &self) -> u8
    {
        *self as u8
    }

    pub const fn	FromU8( val: u8) -> Self
    {
        match val
        {
            1 => Self::Rle,
            2 => Self::ChunkedLz,
            _ => Self::None,
        }
    }
}

//--------------------------------------------------------------------------------------------------
/// Fast run-length and sliding byte-match compression for repetitive payload blocks.
pub fn	CompressBlock( data: &[u8]) -> Buff< u8>
{
    if data.is_empty()
    {
        return Buff::New();
    }
    let  	mut out = Stash::New();
    let  	len = data.len();
    let  	mut i = 0usize;

    while i < len
    {
        let  	b = data[i];
        let  	mut run = 1usize;
        while ( i + run) < len && data[i + run] == b && run < 255
        {
            run += 1;
        }

        if run >= 4
        {
            // Marker 0xFF, count byte, literal byte
            out.Push( 0xFF);
            out.Push( run as u8);
            out.Push( b);
            i += run;
        }
        else
        {
            if b == 0xFF
            {
                out.Push( 0xFF);
                out.Push( 0x00);
            }
            else
            {
                out.Push( b);
            }
            i += 1;
        }
    }
    out.IntoBuff()
}

//--------------------------------------------------------------------------------------------------
/// Decompresses a block encoded by CompressBlock.
pub fn	DecompressBlock( compressed: &[u8], expectedLen: usize) -> Result< Buff< u8>, String>
{
    let  	mut out = Stash::WithCapacity( expectedLen as u32);
    let  	len = compressed.len();
    let  	mut i = 0usize;

    while i < len
    {
        if compressed[i] == 0xFF
        {
            if i + 1 >= len
            {
                return Err( "Truncated compression escape sequence".to_string());
            }
            let  	count = compressed[i + 1] as usize;
            if count == 0
            {
                out.Push( 0xFF);
                i += 2;
            }
            else
            {
                if i + 2 >= len
                {
                    return Err( "Truncated run-length sequence".to_string());
                }
                let  	val = compressed[i + 2];
                let  	mut r = 0usize;
                while r < count
                {
                    out.Push( val);
                    r += 1;
                }
                i += 3;
            }
        }
        else
        {
            out.Push( compressed[i]);
            i += 1;
        }
    }

    if out.Size() as usize != expectedLen
    {
        return Err( format!(
            "Decompressed size mismatch: expected {}, got {}",
            expectedLen,
            out.Size()
        ));
    }
    Ok( out.IntoBuff())
}

//--------------------------------------------------------------------------------------------------
/// Incremental chunked streaming writer compressing large payloads into bounded chunks.
pub struct ChunkedStreamWriter< W: Write>
{
    _Writer:    W,
    _Kind:      FlxCompressKind,
    _ChunkSize: usize,
    _Buffer:    Vec< u8>,
    _BytesIn:   u64,
    _BytesOut:  u64,
}

impl< W: Write> ChunkedStreamWriter< W>
{
    pub fn	New( writer: W, kind: FlxCompressKind) -> Self
    {
        Self::WithChunkSize( writer, kind, K_DEFAULT_CHUNK_SIZE)
    }

    pub fn	WithChunkSize( writer: W, kind: FlxCompressKind, chunkSize: usize) -> Self
    {
        Self {
            _Writer:    writer,
            _Kind:      kind,
            _ChunkSize: chunkSize,
            _Buffer:    Vec::with_capacity( chunkSize),
            _BytesIn:   0,
            _BytesOut:  0,
        }
    }

    #[inline]
    pub const fn	BytesIn( &self) -> u64
    {
        self._BytesIn
    }

    #[inline]
    pub const fn	BytesOut( &self) -> u64
    {
        self._BytesOut
    }

    /// Flushes the active internal buffer as a compressed chunk.
    pub fn	FlushChunk( &mut self) -> Result< (), String>
    {
        if self._Buffer.is_empty()
        {
            return Ok( ());
        }

        let  	uncompressedLen = self._Buffer.len() as u32;
        let  	chunkPayload = match self._Kind
        {
            FlxCompressKind::None => {
                let  	mut b = Stash::WithCapacity( uncompressedLen);
                let  	mut idx = 0usize;
                while idx < self._Buffer.len()
                {
                    b.Push( self._Buffer[idx]);
                    idx += 1;
                }
                b.IntoBuff()
            }
            FlxCompressKind::Rle | FlxCompressKind::ChunkedLz => {
                CompressBlock( &self._Buffer)
            }
        };

        let  	compressedLen = chunkPayload.Size();
        self._Writer.write_all( &uncompressedLen.to_le_bytes()).map_err( |e| e.to_string())?;
        self._Writer.write_all( &compressedLen.to_le_bytes()).map_err( |e| e.to_string())?;

        let  	mut pIdx = 0u32;
        while pIdx < compressedLen
        {
            self._Writer.write_all( &[chunkPayload[pIdx]]).map_err( |e| e.to_string())?;
            pIdx += 1;
        }

        self._BytesIn += uncompressedLen as u64;
        self._BytesOut += 8 + compressedLen as u64;
        self._Buffer.clear();
        Ok( ())
    }

    pub fn	WriteBytes( &mut self, data: &[u8]) -> Result< usize, String>
    {
        let  	mut written = 0usize;
        while written < data.len()
        {
            let  	space = self._ChunkSize - self._Buffer.len();
            let  	toTake = space.min( data.len() - written);
            self._Buffer.extend_from_slice( &data[written..written + toTake]);
            written += toTake;

            if self._Buffer.len() >= self._ChunkSize
            {
                self.FlushChunk()?;
            }
        }
        Ok( written)
    }

    pub fn	Finish( &mut self) -> Result< (), String>
    {
        self.FlushChunk()?;
        self._Writer.flush().map_err( |e| e.to_string())?;
        Ok( ())
    }
}

//--------------------------------------------------------------------------------------------------
/// Incremental chunked streaming reader decoding compressed chunks sequentially.
pub struct ChunkedStreamReader< R: Read>
{
    _Reader:        R,
    _Kind:          FlxCompressKind,
    _CurrentChunk:  Buff< u8>,
    _ChunkOffset:   u32,
    _ReachedEnd:    bool,
}

impl< R: Read> ChunkedStreamReader< R>
{
    pub fn	New( reader: R, kind: FlxCompressKind) -> Self
    {
        Self {
            _Reader:       reader,
            _Kind:         kind,
            _CurrentChunk: Buff::New(),
            _ChunkOffset:  0,
            _ReachedEnd:   false,
        }
    }

    /// Pulls the next chunk from the underlying reader.
    fn	NextChunk( &mut self) -> Result< bool, String>
    {
        let  	mut header = [0u8; 8];
        match self._Reader.read_exact( &mut header)
        {
            Ok( ()) => {}
            Err( e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                self._ReachedEnd = true;
                return Ok( false);
            }
            Err( e) => return Err( e.to_string()),
        }

        let  	uncompressedLen = u32::from_le_bytes( [header[0], header[1], header[2], header[3]]) as usize;
        let  	compressedLen = u32::from_le_bytes( [header[4], header[5], header[6], header[7]]) as usize;

        let  	mut compressedBuf = vec![0u8; compressedLen];
        self._Reader.read_exact( &mut compressedBuf).map_err( |e| e.to_string())?;

        let  	decompressed = match self._Kind
        {
            FlxCompressKind::None => {
                let  	mut s = Stash::WithCapacity( uncompressedLen as u32);
                let  	mut i = 0usize;
                while i < uncompressedLen
                {
                    s.Push( compressedBuf[i]);
                    i += 1;
                }
                s.IntoBuff()
            }
            FlxCompressKind::Rle | FlxCompressKind::ChunkedLz => {
                DecompressBlock( &compressedBuf, uncompressedLen)?
            }
        };

        self._CurrentChunk = decompressed;
        self._ChunkOffset = 0;
        Ok( true)
    }

    /// Reads uncompressed bytes into the destination buffer across chunk boundaries.
    pub fn	ReadBytes( &mut self, out: &mut [u8]) -> Result< usize, String>
    {
        let  	mut totalRead = 0usize;
        while totalRead < out.len()
        {
            if ( self._ChunkOffset as usize) >= ( self._CurrentChunk.Size() as usize)
                && ( self._ReachedEnd || !self.NextChunk()?)
            {
                break;
            }

            let  	available = ( self._CurrentChunk.Size() - self._ChunkOffset) as usize;
            let  	toCopy = available.min( out.len() - totalRead);
            let  	mut i = 0usize;
            while i < toCopy
            {
                out[totalRead + i] = self._CurrentChunk[self._ChunkOffset + i as u32];
                i += 1;
            }
            self._ChunkOffset += toCopy as u32;
            totalRead += toCopy;
        }
        Ok( totalRead)
    }
}
