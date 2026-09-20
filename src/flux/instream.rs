//-- instream.rs -----------------------------------------------------------------------------------------------------------------------
use	crate::silo::{ Arr, Buff };
use	std::io::Read;
use	std::{ cmp, fs, io, path::Path };

//---------------------------------------------------------------------------------------------------------------------------------

pub trait IStream {
    fn	Size( &self) -> u32;
    fn	At( &mut self, offset: u32) -> u8;
    fn	BytesAt( &mut self, offset: u32, count: u32) -> Arr< '_, u8>;
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct FixedStream< 'a> {
    _Arr: Arr< 'a, u8>,
}
impl< 'a> From<Arr<'a, u8>> for FixedStream< 'a> {
    fn	from( arr: Arr< 'a, u8>) -> Self {
        Self { _Arr: arr }
    }
}
impl< 'a> From<&'a str> for FixedStream< 'a> {
    fn	from( strVal: &'a str) -> Self {
        Self::from( crate::silo::arr::Arr::New( 
            strVal.as_ptr(),
            strVal.len() as u32,
        ))
    }
}
impl< 'a> IStream for FixedStream<'a>
{
    fn	Size( &self) -> u32
    {
        self._Arr.Size()
    }
    fn	At( &mut self, offset: u32) -> u8
    {
        if offset < self.Size() {
            *self._Arr.Get( offset).unwrap()
        } else {
            0
        }
    }
    fn	BytesAt( &mut self, offset: u32, count: u32) -> Arr< '_, u8> {
        let  	sz = self.Size();
        let  	start = offset;
        if offset < sz {
            let  	end = cmp::min( start.saturating_add( count), sz);
            self._Arr.Slice( offset, end - start)
        } else {
            let  	empty: &[u8] = &[];
            crate::silo::arr::Arr::New( empty.as_ptr(), empty.len() as u32)
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct BuffStream< R: Read>
{
    _Inner: R,
    _Buff: crate::silo::stash::Stash< u8>,
}
impl< R: Read> From< R> for BuffStream< R> {
    fn	from( inner: R) -> Self
    {
        Self {
            _Inner: inner,
            _Buff: crate::silo::stash::Stash::WithCapacity( 4096),
        }
    }
}
impl TryFrom< &Path> for BuffStream< fs::File> {
    type Error = io::Error;
    fn	try_from( path: &Path) -> io::Result< Self>
    {
        let  	file = fs::File::open( path)?;
        Ok( Self::from( file))
    }
}
impl TryFrom< &str> for BuffStream< fs::File> {
    type Error = io::Error;
    fn	try_from( path: &str) -> io::Result< Self>
    {
        let  	file = fs::File::open( path)?;
        Ok( Self::from( file))
    }
}
impl BuffStream< fs::File>
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
impl BuffStream< io::Stdin>
{
    pub fn	FromStdin() -> io::Result< Self>
    {
        Ok( Self::from( io::stdin()))
    }
}
impl< R: Read> BuffStream< R>
{
    pub fn	EnsureCached( &mut self, required: u32) -> io::Result< ()>
    {
        let  	mut currSize = self._Buff.Size();
        while currSize < required {
            let  	chunkSize = cmp::max( 4096, required - currSize);
            let  	mut chunk = Buff::FromDispenser( chunkSize, |_| 0u8);
            let  	readBytes = self._Inner.read( chunk.MutArr().into())? as u32;
            if readBytes == 0 {
                break;
            }
            let  	newSize = currSize + readBytes;
            self._Buff.Resize( newSize, |_| 0u8);
            let slice: &mut [u8] = self._Buff.MutArr().into();
            slice[currSize as usize..newSize as usize]
                .copy_from_slice( chunk.Arr().Slice( 0, readBytes).into());
            currSize = newSize;
        }
        Ok( ())
    }
    pub fn	ReadAll( &mut self) -> io::Result< ()>
    {
        let  	mut chunk = [0u8; 65536];
        loop {
            let  	readBytes = self._Inner.read( &mut chunk)? as u32;
            if readBytes == 0 {
                break;
            }
            let  	currSize = self._Buff.Size();
            let  	newSize = currSize + readBytes;
            self._Buff.Resize( newSize, |_| 0u8);
            let slice: &mut [u8] = self._Buff.MutArr().into();
            slice[currSize as usize..newSize as usize].copy_from_slice( &chunk[..readBytes as usize]);
        }
        Ok( ())
    }
}
impl< R: Read> IStream for BuffStream< R> {
    fn	Size( &self) -> u32
    {
        self._Buff.Size()
    }
    fn	At( &mut self, offset: u32) -> u8
    {
        let  	_ = self.EnsureCached( offset.saturating_add( 1));
        if offset < self.Size() {
            *self._Buff.Arr().Get( offset).unwrap()
        } else {
            0
        }
    }
    fn	BytesAt( &mut self, offset: u32, count: u32) -> Arr< '_, u8> {
        let  	_ = self.EnsureCached( offset.saturating_add( count));
        let  	sz = self.Size();
        let  	start = offset;
        if offset < sz {
            let  	end = cmp::min( start.saturating_add( count), sz);
            self._Buff.Arr().Slice( offset, end - start)
        } else {
            let  	empty: &[u8] = &[];
            crate::silo::arr::Arr::New( empty.as_ptr(), empty.len() as u32)
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
