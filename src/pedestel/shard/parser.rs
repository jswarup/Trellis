//-- parser.rs -------------------------------------------------------------------------------------------------------------------
use	crate::flux::instream::{ FixedStream, IStream };
use	crate::silo::Stash;

//---------------------------------------------------------------------------------------------------------------------------------

#[derive( Clone, Debug, PartialEq, Eq)]
pub enum ParseError
{
    StreamTooLarge,
    SyntaxError { mark: u32 },
    TrailingGarbage { mark: u32, total: u32 },
}

impl std::fmt::Display for ParseError
{
    fn	fmt( &self, f: &mut std::fmt::Formatter< '_>) -> std::fmt::Result
    {
        match self
        {
            Self::StreamTooLarge => write!( f, "Input stream exceeds maximum parseable size"),
            Self::SyntaxError { mark } => write!( f, "Syntax error at mark {}", mark),
            Self::TrailingGarbage { mark, total } =>
            {
                write!( f, "Unparsed trailing characters at mark {} (total {})", mark, total)
            }
        }
    }
}

impl std::error::Error for ParseError {}

//---------------------------------------------------------------------------------------------------------------------------------

pub trait IGrammar< Ctx = ()> {
    fn	Match( &self, parser: &mut Parser, ctx: &mut Ctx) -> bool;

    fn	ParseStr( &self, input: &str, ctx: &mut Ctx) -> Result< u32, ParseError>
    where
        Self: Sized,
    {
        let  	input_len = u32::try_from( input.len() ).map_err( |_| ParseError::StreamTooLarge )?;
        let  	mut stream = FixedStream::from( input);
        let  	mut parser = Parser::New( &mut stream);
        let  	res = parser.ParseGrammar( self, ctx, 0);
        match res
        {
            Some( mark) if mark == input_len => Ok( mark),
            Some( mark) => Err( ParseError::TrailingGarbage { mark, total: input_len }),
            None => Err( ParseError::SyntaxError { mark: parser.CurrMark() }),
        }
    }

    fn	ParseStream( &self, stream: &mut dyn IStream, ctx: &mut Ctx) -> Result< u32, ParseError>
    where
        Self: Sized,
    {
        let  	total_len = stream.Size();
        let  	mut parser = Parser::New( stream);
        let  	res = parser.ParseGrammar( self, ctx, 0);
        match res
        {
            Some( mark) if mark == total_len => Ok( mark),
            Some( mark) => Err( ParseError::TrailingGarbage { mark, total: total_len }),
            None => Err( ParseError::SyntaxError { mark: parser.CurrMark() }),
        }
    }
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
    /// Executes a parse transaction: if `op` returns `Some(val)`, marks are committed;
    /// if `op` returns `None`, the parser mark is rolled back to the start.
    pub fn	Transaction< T, F>( &mut self, op: F) -> Option< T>
    where
        F: FnOnce( &mut Self) -> Option< T>,
    {
        let  	saveMark = self.CurrMark();
        let  	res = op( self);
        if res.is_none() {
            self.SetCurrMark( saveMark);
        }
        res
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// Updates the active parse marker.
    pub fn	SetCurrMark( &mut self, mark: u32)
    {
        if !self._Markers.IsEmpty() {
            let  	last = self._Markers.Size() - 1;
            *self._Markers.MutArr().GetMut( last).unwrap() = mark;
        }
    }
    /// Returns the active parse marker.
    pub fn	CurrMark( &self) -> u32
    {
        self._Markers.Arr().Last().copied().unwrap_or( 0)
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
