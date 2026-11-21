//-- outstream.rs ----------------------------------------------------------------------------------------------------------------------
use	crate::silo::{ Arr, Buff };
use	std::io::{ Result, Write };
use	std::{ cmp, fs, io, path::Path };

//---------------------------------------------------------------------------------------------------------------------------------

pub enum OutSource< 'a, W: Write> {
    Fixed( Arr< 'a, u8>),
    Streaming( W, Buff< u8>),
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct OutStream< 'a, W: Write = io::Sink> {
    _Source: OutSource< 'a, W>,
    _Marker: u32,
}
const K_OUTSTREAM_CACHE_BYTES: u32 = 4096;

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a> From<Arr<'a, u8>> for OutStream< 'a, io::Sink> {
    fn	from( arr: Arr< 'a, u8>) -> Self {
        Self {
            _Source: OutSource::Fixed( arr),
            _Marker: 0,
        }
    }
}
impl< 'a, W: Write> OutStream<'a, W>
{
    pub fn	WithCacheSize( inner: W, cache_size: u32) -> Self
    {
        let  	size = if cache_size == 0 {
            K_OUTSTREAM_CACHE_BYTES
        } else {
            cache_size
        };
        let  	buff = Buff::FromDispenser( size, |_| 0u8);
        Self {
            _Source: OutSource::Streaming( inner, buff),
            _Marker: 0,
        }
    }
}
impl< 'a, W: Write> From<W> for OutStream<'a, W>
{
    fn	from( inner: W) -> Self
    {
        Self::WithCacheSize( inner, K_OUTSTREAM_CACHE_BYTES)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a> TryFrom<&Path> for OutStream<'a, fs::File>
{
    type Error = io::Error;
    fn	try_from( path: &Path) -> io::Result< Self>
    {
        let  	file = fs::File::create( path)?;
        Ok( Self::from( file))
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a> TryFrom<&str> for OutStream<'a, fs::File>
{
    type Error = io::Error;
    fn	try_from( path: &str) -> io::Result< Self>
    {
        let  	file = fs::File::create( path)?;
        Ok( Self::from( file))
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a> OutStream<'a, fs::File>
{
    pub fn	FromFile< P: AsRef< Path>>( path: P) -> io::Result< Self>
    {
        Self::try_from( path.as_ref())
    }
    pub fn	FromPath( path: &Path) -> io::Result< Self>
    {
        Self::try_from( path)
    }
    pub fn	FromFileHandle( file: fs::File) -> io::Result< Self>
    {
        Ok( Self::from( file))
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a, W: Write> OutStream<'a, W>
{

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn	Position( &self) -> u32
    {
        self._Marker
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn	SetPosition( &mut self, pos: u32)
    {
        self._Marker = pos;
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a, W: Write> Write for OutStream<'a, W>
{
    fn	write( &mut self, buf: &[u8]) -> Result< usize>
    {
        let  	amt = buf.len().min( u32::MAX as usize) as u32;
        if amt == 0 {
            return Ok( 0);
        }
        match &mut self._Source {
            OutSource::Fixed( arr) => {
                let  	pos = self._Marker;
                let  	currSize = arr.Size();
                if pos >= currSize {
                    return Ok( 0);
                }
                let  	available = currSize - pos;
                let  	len = cmp::min( available, amt);
                arr.AsMutSlice()[pos as usize..( pos + len) as usize]
                    .copy_from_slice( &buf[..len as usize]);
                self._Marker += len;
                Ok( len as usize)
            }
            OutSource::Streaming( inner, buff) => {
                let  	mut pos = self._Marker;
                let  	cacheSize = buff.Size();
                let  	mut written = 0;
                while written < amt {
                    if pos == cacheSize {
                        // Flush cache
                        inner.write_all( buff.Arr().into())?;
                        pos = 0;
                        self._Marker = 0;
                    }
                    let  	available = cacheSize - pos;
                    let  	len = cmp::min( available, amt - written);
                    let  	slice: &mut [u8] = buff.MutArr().into();
                    slice[pos as usize..( pos + len) as usize]
                        .copy_from_slice( &buf[written as usize..( written + len) as usize]);
                    pos += len;
                    written += len;
                    self._Marker = pos;
                }
                Ok( written as usize)
            }
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    fn	flush( &mut self) -> Result< ()>
    {
        match &mut self._Source {
            OutSource::Fixed( _) => Ok( ()),
            OutSource::Streaming( inner, buff) => {
                let  	pos = self._Marker;
                if pos > 0 {
                    inner.write_all( buff.Arr().Slice( 0, pos).into())?;
                    self._Marker = 0;
                }
                inner.flush()
            }
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a, W: Write> Drop for OutStream<'a, W>
{
    fn	drop( &mut self)
    {
        let  	_ = self.flush();
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
