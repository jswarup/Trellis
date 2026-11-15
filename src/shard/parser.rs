//-- parser.rs -------------------------------------------------------------------------------------------------------------------
use	crate::flux::instream::IStream;
use	crate::silo::Stash;

//---------------------------------------------------------------------------------------------------------------------------------

pub trait IGrammar< Ctx = ()> {
    fn	Match( &self, parser: &mut Parser, ctx: &mut Ctx) -> bool;
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct Parser< 'p> {
    _InStream: &'p mut dyn IStream,
    _Markers: Stash< u32>,
}

//---------------------------------------------------------------------------------------------------------------------------------

const K_MAX_PARSE_DEPTH: u32 = 1024;
impl< 'p> Parser<'p>
{
    pub fn	New( stream: &'p mut dyn IStream) -> Self {
        Self {
            _InStream: stream,
            _Markers: Stash::New(),
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn	ParseGrammar< Ctx>( &mut self, grammar: &( impl IGrammar< Ctx> + ?Sized), ctx: &mut Ctx, mark: u32) -> Option< u32>
    {
        if self._Markers.Size() >= K_MAX_PARSE_DEPTH {
            return None;
        }
        self._Markers.Push( mark);
        let  	matched = grammar.Match( self, ctx);
        let  	completedMark = self.CurrMark();
        let  	_ = self._Markers.Pop().expect( "missing parse marker");
        if matched {
            if !self._Markers.IsEmpty() {
                self.SetCurrMark( completedMark);
            }
            Some( completedMark)
        } else {
            None
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn	Parse( &mut self, grammar: &( impl IGrammar<()> + ?Sized), mark: u32) -> Option< u32>
    {
        self.ParseGrammar( grammar, &mut (), mark)
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// Updates the active parse marker.
    pub fn	SetCurrMark( &mut self, mark: u32)
    {
        let  	markers = self._Markers.Arr();
        let  	last = markers.Size() - 1;
        *self._Markers.MutArr().GetMut( last).unwrap() = mark;
    }
    /// Returns the active parse marker.
    pub fn	CurrMark( &self) -> u32
    {
        *self._Markers.Arr().Last().unwrap()
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn	InStream( &mut self) -> &mut dyn IStream
    {
        self._InStream
    }
    pub fn	GetAt( &mut self, marker: u32) -> u8
    {
        self._InStream.At( marker)
    }
    pub fn	Incr( &mut self, mut marker: u32) -> Option< u32>
    {
        marker += 1;
        if marker <= self._InStream.Size() {
            Some( marker)
        } else {
            None
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
