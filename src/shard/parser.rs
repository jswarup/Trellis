//-- parser.rs -------------------------------------------------------------------------------------------------------------------

use crate::flux::instream::IStream;
use crate::silo::Stash;

//---------------------------------------------------------------------------------------------------------------------------------

pub trait IGrammar {
    fn Match(&self, parser: &mut Parser) -> bool;
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct Parser<'p> {
    pub _InStream: &'p mut dyn IStream,
    pub _Markers: Stash<u32>,
}

//---------------------------------------------------------------------------------------------------------------------------------

unsafe impl<'p> Send for Parser<'p> {}
unsafe impl<'p> Sync for Parser<'p> {}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'p> Parser<'p> {
    pub fn New(stream: &'p mut dyn IStream) -> Self {
        Self {
            _InStream: stream,
            _Markers: Stash::New(),
        }
    }
    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn ParseGrammar(&mut self, grammar: &(impl IGrammar + ?Sized), mark: u32) -> Option<u32> {
        self._Markers.Push(mark);
        let matched = grammar.Match(self);
        let completedMark = self.CurrMark();

        let mut marker = (0 as u32);
        marker = self._Markers.Pop().expect("missing parse marker");
        if matched {
            if !self._Markers.IsEmpty() {
                self.SetCurrMark(completedMark);
            }
            Some(completedMark)
        } else {
            None
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    /// Updates the active parse marker.
    pub fn SetCurrMark(&mut self, mark: u32) {
        let markers = self._Markers.AsArr();
        let last = markers.Size() - (1 as u32);
        unsafe { *self._Markers.DataMut().add(last as usize) = mark };
    }

    /// Returns the active parse marker.
    pub fn CurrMark(&self) -> u32 {
        *self._Markers.AsArr().Last().unwrap()
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn InStream(&mut self) -> &mut dyn IStream {
        self._InStream
    }

    pub fn GetAt(&mut self, marker: u32) -> u8 {
        self._InStream.At(marker)
    }

    pub fn Incr(&mut self, mut marker: u32) -> Option<u32> {
        marker += (1 as u32);
        if marker <= self._InStream.Size() {
            Some(marker)
        } else {
            None
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
