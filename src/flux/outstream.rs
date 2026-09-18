//-- outstream.rs ----------------------------------------------------------------------------------------------------------------------
use crate::silo::{Arr, Buff};
use std::io::{Result, Write};
use std::{
    cmp, fs, io,
    path::Path,
    slice::{from_raw_parts, from_raw_parts_mut},
};

//---------------------------------------------------------------------------------------------------------------------------------

pub enum OutSource<'a, W: Write> {
    Fixed(Arr<'a, u8>),
    Streaming(W, Buff<u8>),
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct OutStream<'a, W: Write = io::Sink> {
    _Source: OutSource<'a, W>,
    _Marker: u32,
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> From<Arr<'a, u8>> for OutStream<'a, io::Sink> {
    fn from(arr: Arr<'a, u8>) -> Self {
        Self {
            _Source: OutSource::Fixed(arr),
            _Marker: 0,
        }
    }
}

impl<'a, W: Write> From<W> for OutStream<'a, W> {
    fn from(inner: W) -> Self {
        let buff = Buff::New();
        Self {
            _Source: OutSource::Streaming(inner, buff),
            _Marker: 0,
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> TryFrom<&Path> for OutStream<'a, fs::File> {
    type Error = io::Error;
    fn try_from(path: &Path) -> io::Result<Self> {
        let file = fs::File::create(path)?;
        Ok(Self::from(file))
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> TryFrom<&str> for OutStream<'a, fs::File> {
    type Error = io::Error;
    fn try_from(path: &str) -> io::Result<Self> {
        let file = fs::File::create(path)?;
        Ok(Self::from(file))
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> OutStream<'a, fs::File> {
    pub fn FromFile<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::try_from(path.as_ref())
    }

    pub fn FromPath(path: &Path) -> io::Result<Self> {
        Self::try_from(path)
    }

    pub fn FromFileHandle(file: fs::File) -> io::Result<Self> {
        Ok(Self::from(file))
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
impl<'a, W: Write> OutStream<'a, W> {
    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn Position(&self) -> u32 {
        self._Marker
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn SetPosition(&mut self, pos: u32) {
        self._Marker = pos;
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a, W: Write> Write for OutStream<'a, W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let amt = buf.len();
        if amt == 0 {
            return Ok(0);
        }

        match &mut self._Source {
            OutSource::Fixed(arr) => {
                let pos = self._Marker as usize;
                let currSize = arr.Size() as usize;

                if pos >= currSize {
                    return Ok(0);
                }

                let available = currSize - pos;
                let len = cmp::min(available, amt);

                unsafe {
                    let ptr = arr.Data() as *mut u8;
                    let slice = from_raw_parts_mut(ptr, currSize);
                    slice[pos..pos + len].copy_from_slice(&buf[..len]);
                }

                self._Marker += len as u32;
                Ok(len)
            }
            OutSource::Streaming(inner, buff) => {
                let mut pos = self._Marker as usize;
                let cacheSize = buff.Size() as usize;
                let mut written = 0;

                while written < amt {
                    if pos == cacheSize {
                        // Flush cache
                        unsafe {
                            let ptr = buff.AsPtr().cast::<u8>();
                            let slice = from_raw_parts(ptr, cacheSize);
                            inner.write_all(slice)?;
                        }
                        pos = 0;
                        self._Marker = 0;
                    }

                    let available = cacheSize - pos;
                    let len = cmp::min(available, amt - written);

                    unsafe {
                        let ptr = buff.AsMutPtr().cast::<u8>();
                        let slice = from_raw_parts_mut(ptr, cacheSize);
                        slice[pos..pos + len].copy_from_slice(&buf[written..written + len]);
                    }

                    pos += len;
                    written += len;
                    self._Marker = pos as u32;
                }

                Ok(written)
            }
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    fn flush(&mut self) -> Result<()> {
        match &mut self._Source {
            OutSource::Fixed(_) => Ok(()),
            OutSource::Streaming(inner, buff) => {
                let pos = self._Marker as usize;
                if pos > 0 {
                    unsafe {
                        let ptr = buff.AsPtr().cast::<u8>();
                        let slice = from_raw_parts(ptr, pos);
                        inner.write_all(slice)?;
                    }
                    self._Marker = 0;
                }
                inner.flush()
            }
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a, W: Write> Drop for OutStream<'a, W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
