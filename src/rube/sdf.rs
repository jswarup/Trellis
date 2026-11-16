// sdf.rs ------------------------------------------------------------------------------------------------------
use	crate::ShardTree;
use	crate::shard::*;
use	crate::silo::{ Buff, IArr, Stash };

//-------------------------------------------------------------------------------------------------

/// Timing corner for selecting delay values from a min:typ:max triple.
#[derive( Copy, Clone, Debug, PartialEq, Eq)]
pub enum DelayCorner
{
    Min,
    Typ,
    Max,
}

//-------------------------------------------------------------------------------------------------

/// Triple-valued delay representation (min:typ:max).
#[derive( Copy, Clone, Debug, PartialEq, Default)]
pub struct DelayTriple
{
    _Min: f64,
    _Typ: f64,
    _Max: f64,
}

impl DelayTriple
{
    pub const fn	New( min: f64, typ: f64, max: f64) -> Self
    {
        Self {
            _Min: min,
            _Typ: typ,
            _Max: max,
        }
    }

    pub const fn	Single( val: f64) -> Self
    {
        Self {
            _Min: val,
            _Typ: val,
            _Max: val,
        }
    }

    #[inline] pub const fn	Min( &self) -> f64 { self._Min }
    #[inline] pub const fn	Typ( &self) -> f64 { self._Typ }
    #[inline] pub const fn	Max( &self) -> f64 { self._Max }

    #[inline]
    pub fn	Select( &self, corner: DelayCorner) -> f64
    {
        match corner
        {
            DelayCorner::Min => self._Min,
            DelayCorner::Typ => self._Typ,
            DelayCorner::Max => self._Max,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// IOPATH delay specification: path from an input port to an output port.
#[derive( Clone, Debug, PartialEq)]
pub struct SdfIoPath
{
    _InputPort:  String,
    _OutputPort: String,
    _RiseDelay:  DelayTriple,
    _FallDelay:  DelayTriple,
}

impl SdfIoPath
{
    pub fn	New(
        inputPort: impl Into< String>,
        outputPort: impl Into< String>,
        rise: DelayTriple,
        fall: DelayTriple,
    ) -> Self
    {
        Self {
            _InputPort:  inputPort.into(),
            _OutputPort: outputPort.into(),
            _RiseDelay:  rise,
            _FallDelay:  fall,
        }
    }

    #[inline] pub fn	InputPort( &self) -> &str { &self._InputPort }
    #[inline] pub fn	OutputPort( &self) -> &str { &self._OutputPort }
    #[inline] pub const fn	RiseDelay( &self) -> &DelayTriple { &self._RiseDelay }
    #[inline] pub const fn	FallDelay( &self) -> &DelayTriple { &self._FallDelay }

    #[inline]
    pub fn	AverageDelay( &self, corner: DelayCorner) -> f64
    {
        ( self._RiseDelay.Select( corner) + self._FallDelay.Select( corner) ) * 0.5
    }
}

//-------------------------------------------------------------------------------------------------

/// INTERCONNECT delay specification between two ports.
#[derive( Clone, Debug, PartialEq)]
pub struct SdfInterconnect
{
    _StartPort: String,
    _EndPort:   String,
    _Delay:     DelayTriple,
}

impl SdfInterconnect
{
    pub fn	New(
        startPort: impl Into< String>,
        endPort: impl Into< String>,
        delay: DelayTriple,
    ) -> Self
    {
        Self {
            _StartPort: startPort.into(),
            _EndPort:   endPort.into(),
            _Delay:     delay,
        }
    }

    #[inline] pub fn	StartPort( &self) -> &str { &self._StartPort }
    #[inline] pub fn	EndPort( &self) -> &str { &self._EndPort }
    #[inline] pub const fn	Delay( &self) -> &DelayTriple { &self._Delay }
}

//-------------------------------------------------------------------------------------------------

/// Timing check kind (e.g. SETUP or HOLD).
#[derive( Copy, Clone, Debug, PartialEq, Eq)]
pub enum SdfCheckKind
{
    Setup,
    Hold,
    Recovery,
    Removal,
    Width,
}

//-------------------------------------------------------------------------------------------------

/// Timing check constraint inside an SDF cell.
#[derive( Clone, Debug, PartialEq)]
pub struct SdfTimingCheck
{
    _Kind:      SdfCheckKind,
    _Port:      String,
    _ClockPort: String,
    _Limit:     DelayTriple,
}

impl SdfTimingCheck
{
    pub fn	New(
        kind: SdfCheckKind,
        port: impl Into< String>,
        clockPort: impl Into< String>,
        limit: DelayTriple,
    ) -> Self
    {
        Self {
            _Kind:      kind,
            _Port:      port.into(),
            _ClockPort: clockPort.into(),
            _Limit:     limit,
        }
    }

    #[inline] pub const fn	Kind( &self) -> SdfCheckKind { self._Kind }
    #[inline] pub fn	Port( &self) -> &str { &self._Port }
    #[inline] pub fn	ClockPort( &self) -> &str { &self._ClockPort }
    #[inline] pub const fn	Limit( &self) -> &DelayTriple { &self._Limit }
}

//-------------------------------------------------------------------------------------------------

/// An individual CELL entry in an SDF file.
#[derive( Clone, Debug, PartialEq)]
pub struct SdfCell
{
    _CellType:      String,
    _Instance:      String,
    _IoPaths:       Buff< SdfIoPath>,
    _Interconnects: Buff< SdfInterconnect>,
    _TimingChecks:  Buff< SdfTimingCheck>,
}

impl SdfCell
{
    pub fn	New(
        cellType: impl Into< String>,
        instance: impl Into< String>,
        ioPaths: Buff< SdfIoPath>,
        interconnects: Buff< SdfInterconnect>,
        timingChecks: Buff< SdfTimingCheck>,
    ) -> Self
    {
        Self {
            _CellType:      cellType.into(),
            _Instance:      instance.into(),
            _IoPaths:       ioPaths,
            _Interconnects: interconnects,
            _TimingChecks:  timingChecks,
        }
    }

    #[inline] pub fn	CellType( &self) -> &str { &self._CellType }
    #[inline] pub fn	Instance( &self) -> &str { &self._Instance }
    #[inline] pub fn	IoPaths( &self) -> &Buff< SdfIoPath> { &self._IoPaths }
    #[inline] pub fn	Interconnects( &self) -> &Buff< SdfInterconnect> { &self._Interconnects }
    #[inline] pub fn	TimingChecks( &self) -> &Buff< SdfTimingCheck> { &self._TimingChecks }
}

//-------------------------------------------------------------------------------------------------

/// Root representation of an IEEE 1497 Standard Delay Format (SDF) file.
#[derive( Clone, Debug, PartialEq, Default)]
pub struct SdfFile
{
    _Version:   String,
    _Design:    String,
    _Timescale: String,
    _Cells:     Buff< SdfCell>,
}

impl SdfFile
{
    pub fn	New(
        version: impl Into< String>,
        design: impl Into< String>,
        timescale: impl Into< String>,
        cells: Buff< SdfCell>,
    ) -> Self
    {
        Self {
            _Version:   version.into(),
            _Design:    design.into(),
            _Timescale: timescale.into(),
            _Cells:     cells,
        }
    }

    #[inline] pub fn	Version( &self) -> &str { &self._Version }
    #[inline] pub fn	Design( &self) -> &str { &self._Design }
    #[inline] pub fn	Timescale( &self) -> &str { &self._Timescale }
    #[inline] pub fn	Cells( &self) -> &Buff< SdfCell> { &self._Cells }

    /// Finds the first IOPATH delay matching cell type and port names.
    pub fn	FindIoPathDelay(
        &self,
        cellType: &str,
        inPort: &str,
        outPort: &str,
        corner: DelayCorner,
    ) -> Option< f64>
    {
        let  	mut found = None;
        self._Cells.Arr().Traverse( |cell| {
            if cell.CellType() == cellType
            {
                cell.IoPaths().Arr().Traverse( |p| {
                    if p.InputPort() == inPort && p.OutputPort() == outPort
                    {
                        found = Some( p.AverageDelay( corner));
                    }
                });
            }
        });
        found
    }

    /// Finds the first INTERCONNECT delay between two ports.
    pub fn	FindInterconnectDelay(
        &self,
        startPort: &str,
        endPort: &str,
        corner: DelayCorner,
    ) -> Option< f64>
    {
        let  	mut found = None;
        self._Cells.Arr().Traverse( |cell| {
            cell.Interconnects().Arr().Traverse( |ic| {
                if ic.StartPort() == startPort && ic.EndPort() == endPort
                {
                    found = Some( ic.Delay().Select( corner));
                }
            });
        });
        found
    }
}

//-------------------------------------------------------------------------------------------------

/// Parser context for accumulating tokens during SDF parsing.
pub struct SdfParseCtx
{
    pub _Version:        String,
    pub _Design:         String,
    pub _Timescale:      String,
    pub _Cells:          Stash< SdfCell>,
    pub _CurrentCellType: String,
    pub _CurrentInst:    String,
    pub _CurrentIoPaths: Stash< SdfIoPath>,
    pub _CurrentIcs:     Stash< SdfInterconnect>,
    pub _CurrentChecks:  Stash< SdfTimingCheck>,
    pub _TokenStr1:      String,
    pub _TokenStr2:      String,
    pub _TokenStr3:      String,
    pub _Triple1:        DelayTriple,
    pub _Triple2:        DelayTriple,
    pub _CurrentKind:    SdfCheckKind,
}

impl Default for SdfParseCtx
{
    fn	default() -> Self { Self::New() }
}

impl SdfParseCtx
{
    pub fn	New() -> Self
    {
        Self {
            _Version:        String::new(),
            _Design:         String::new(),
            _Timescale:      "1ns".to_string(),
            _Cells:          Stash::New(),
            _CurrentCellType: String::new(),
            _CurrentInst:    String::new(),
            _CurrentIoPaths: Stash::New(),
            _CurrentIcs:     Stash::New(),
            _CurrentChecks:  Stash::New(),
            _TokenStr1:      String::new(),
            _TokenStr2:      String::new(),
            _TokenStr3:      String::new(),
            _Triple1:        DelayTriple::default(),
            _Triple2:        DelayTriple::default(),
            _CurrentKind:    SdfCheckKind::Setup,
        }
    }

    pub fn	ClearWorkingCell( &mut self)
    {
        self._CurrentCellType.clear();
        self._CurrentInst.clear();
        self._CurrentIoPaths = Stash::New();
        self._CurrentIcs = Stash::New();
        self._CurrentChecks = Stash::New();
    }

    pub fn	CommitCell( &mut self) -> bool
    {
        if !self._CurrentCellType.is_empty() || !self._CurrentInst.is_empty()
        {
            let  	cell = SdfCell::New( 
                std::mem::take( &mut self._CurrentCellType),
                std::mem::take( &mut self._CurrentInst),
                std::mem::take( &mut self._CurrentIoPaths).IntoBuff(),
                std::mem::take( &mut self._CurrentIcs).IntoBuff(),
                std::mem::take( &mut self._CurrentChecks).IntoBuff(),
            );
            self._Cells.Push( cell);
        }
        self.ClearWorkingCell();
        true
    }

    pub fn	CommitIoPath( &mut self) -> bool
    {
        let  	rise = self._Triple1;
        let  	fall = if self._Triple2 == DelayTriple::default() {
            self._Triple1
        } else {
            self._Triple2
        };
        let  	path = SdfIoPath::New( 
            std::mem::take( &mut self._TokenStr1),
            std::mem::take( &mut self._TokenStr2),
            rise,
            fall,
        );
        self._CurrentIoPaths.Push( path);
        self._Triple1 = DelayTriple::default();
        self._Triple2 = DelayTriple::default();
        true
    }

    pub fn	CommitInterconnect( &mut self) -> bool
    {
        let  	ic = SdfInterconnect::New( 
            std::mem::take( &mut self._TokenStr1),
            std::mem::take( &mut self._TokenStr2),
            self._Triple1,
        );
        self._CurrentIcs.Push( ic);
        self._Triple1 = DelayTriple::default();
        true
    }

    pub fn	CommitTimingCheck( &mut self) -> bool
    {
        let  	chk = SdfTimingCheck::New( 
            self._CurrentKind,
            std::mem::take( &mut self._TokenStr1),
            std::mem::take( &mut self._TokenStr2),
            self._Triple1,
        );
        self._CurrentChecks.Push( chk);
        self._Triple1 = DelayTriple::default();
        true
    }

    pub fn	IntoFile( self) -> SdfFile
    {
        SdfFile::New( self._Version, self._Design, self._Timescale, self._Cells.IntoBuff())
    }
}

//-------------------------------------------------------------------------------------------------

/// Parses a delay triple `(min:typ:max)` or single value `(val)`.
fn	ParseDelayTriple( slice: &[u8]) -> DelayTriple
{
    let  	s = std::str::from_utf8( slice).unwrap_or( "").trim();
    let  	clean = s.trim_matches( |c| c == '(' || c == ')' ).trim();
    let  	parts: Vec< &str> = clean.split( ':').collect();
    if parts.len() == 3
    {
        let  	min = parts[0].trim().parse::< f64>().unwrap_or( 0.0);
        let  	typ = parts[1].trim().parse::< f64>().unwrap_or( min);
        let  	max = parts[2].trim().parse::< f64>().unwrap_or( typ);
        DelayTriple::New( min, typ, max)
    }
    else if parts.len() == 1
    {
        let  	val = parts[0].trim().parse::< f64>().unwrap_or( 0.0);
        DelayTriple::Single( val)
    }
    else
    {
        DelayTriple::default()
    }
}

//-------------------------------------------------------------------------------------------------

/// Stateless Shard grammar for parsing Standard Delay Format (SDF) files.
pub struct SdfGrammar;

impl IGrammar< SdfParseCtx> for SdfGrammar
{
    fn	Match( &self, parser: &mut crate::shard::Parser, ctx: &mut SdfParseCtx) -> bool
    {
        let  	notEol = Charset::EndLine().Negative();
        let  	notQuote = Charset::from( &b"\"\r\n"[..]).Negative();
        let  	notCloseParen = Charset::from( &b")"[..]).Negative();
        let  	notWhitespaceOrParen = Charset::from( &b" \t\r\n()"[..]).Negative();

        let  	notStar = Charset::from( &b"*"[..]).Negative();
        let  	comment = ShardTree!( 
            ( "//" < *notEol )
            | ( "/*" < *notStar < "*/" )
        );

        let  	optSpace = ShardTree!( *( [ " \t\r\n" ] | comment ) );
        let  	reqSpace = ShardTree!( +( [ " \t\r\n" ] | comment ) );

        let  	quotedString = ShardTree!( 
            "\"" < *notQuote < "\""
        );

        let  	ident = ShardTree!( +notWhitespaceOrParen );

        let  	tripleValue = ShardTree!( 
            "(" < *notCloseParen < ")"
        );

        let  	iopathStmt = ShardTree!( 
            ( "(" < optSpace < "IOPATH" < reqSpace
              < ident[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._TokenStr1 = std::str::from_utf8( s).unwrap_or( "").trim().to_string();
                    true
                } ]
              < reqSpace
              < ident[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._TokenStr2 = std::str::from_utf8( s).unwrap_or( "").trim().to_string();
                    true
                } ]
              < +( reqSpace < tripleValue[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    if ctx._Triple1 == DelayTriple::default()
                    {
                        ctx._Triple1 = ParseDelayTriple( s);
                    }
                    else
                    {
                        ctx._Triple2 = ParseDelayTriple( s);
                    }
                    true
                } ] )
              < optSpace < ")" )[ |_arr, ctx: &mut SdfParseCtx| ctx.CommitIoPath() ]
        );

        let  	interconnectStmt = ShardTree!( 
            ( "(" < optSpace < "INTERCONNECT" < reqSpace
              < ident[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._TokenStr1 = std::str::from_utf8( s).unwrap_or( "").trim().to_string();
                    true
                } ]
              < reqSpace
              < ident[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._TokenStr2 = std::str::from_utf8( s).unwrap_or( "").trim().to_string();
                    true
                } ]
              < reqSpace
              < tripleValue[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._Triple1 = ParseDelayTriple( s);
                    true
                } ]
              < ?( reqSpace < tripleValue )
              < optSpace < ")" )[ |_arr, ctx: &mut SdfParseCtx| ctx.CommitInterconnect() ]
        );

        let  	setupStmt = ShardTree!( 
            ( "(" < optSpace < "SETUP" < reqSpace
              < ident[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._TokenStr1 = std::str::from_utf8( s).unwrap_or( "").trim().to_string();
                    ctx._CurrentKind = SdfCheckKind::Setup;
                    true
                } ]
              < reqSpace
              < ?( "(" < optSpace < "posedge" < reqSpace )
              < ident[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._TokenStr2 = std::str::from_utf8( s).unwrap_or( "").trim().to_string();
                    true
                } ]
              < ?( optSpace < ")" )
              < reqSpace
              < tripleValue[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._Triple1 = ParseDelayTriple( s);
                    true
                } ]
              < optSpace < ")" )[ |_arr, ctx: &mut SdfParseCtx| ctx.CommitTimingCheck() ]
        );

        let  	holdStmt = ShardTree!( 
            ( "(" < optSpace < "HOLD" < reqSpace
              < ident[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._TokenStr1 = std::str::from_utf8( s).unwrap_or( "").trim().to_string();
                    ctx._CurrentKind = SdfCheckKind::Hold;
                    true
                } ]
              < reqSpace
              < ?( "(" < optSpace < "posedge" < reqSpace )
              < ident[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._TokenStr2 = std::str::from_utf8( s).unwrap_or( "").trim().to_string();
                    true
                } ]
              < ?( optSpace < ")" )
              < reqSpace
              < tripleValue[ |arr, ctx: &mut SdfParseCtx| {
                    let  	s: &[u8] = arr.into();
                    ctx._Triple1 = ParseDelayTriple( s);
                    true
                } ]
              < optSpace < ")" )[ |_arr, ctx: &mut SdfParseCtx| ctx.CommitTimingCheck() ]
        );

        let  	delayBlock = ShardTree!( 
            "(" < optSpace < "DELAY" < optSpace
            < "(" < optSpace < ( "ABSOLUTE" | "INCREMENT" ) < optSpace
            < *( optSpace < ( iopathStmt | interconnectStmt ) )
            < optSpace < ")"
            < optSpace < ")"
        );

        let  	timingCheckBlock = ShardTree!( 
            "(" < optSpace < "TIMINGCHECK" < optSpace
            < *( optSpace < ( setupStmt | holdStmt ) )
            < optSpace < ")"
        );

        let  	cellTypeStmt = ShardTree!( 
            "(" < optSpace < "CELLTYPE" < reqSpace
            < ( quotedString | ident )[ |arr, ctx: &mut SdfParseCtx| {
                  let  	s: &[u8] = arr.into();
                  ctx._CurrentCellType = std::str::from_utf8( s).unwrap_or( "").trim_matches( '"').to_string();
                  true
              } ]
            < optSpace < ")"
        );

        let  	instanceStmt = ShardTree!( 
            "(" < optSpace < "INSTANCE"
            < ?( reqSpace < ( quotedString | ident )[ |arr, ctx: &mut SdfParseCtx| {
                  let  	s: &[u8] = arr.into();
                  ctx._CurrentInst = std::str::from_utf8( s).unwrap_or( "").trim_matches( '"').to_string();
                  true
              } ] )
            < optSpace < ")"
        );

        let  	cellStmt = ShardTree!( 
            ( "(" < optSpace < "CELL" < optSpace
              < *( optSpace < ( cellTypeStmt | instanceStmt | delayBlock | timingCheckBlock ) )
              < optSpace < ")" )[ |_arr, ctx: &mut SdfParseCtx| ctx.CommitCell() ]
        );

        let  	headerStmt = ShardTree!( 
            ( "(" < optSpace < "SDFVERSION" < reqSpace < ( quotedString | ident )[ |arr, ctx: &mut SdfParseCtx| {
                let  	s: &[u8] = arr.into();
                ctx._Version = std::str::from_utf8( s).unwrap_or( "").trim_matches( '"').to_string();
                true
            } ] < optSpace < ")" )
            | ( "(" < optSpace < "DESIGN" < reqSpace < ( quotedString | ident )[ |arr, ctx: &mut SdfParseCtx| {
                let  	s: &[u8] = arr.into();
                ctx._Design = std::str::from_utf8( s).unwrap_or( "").trim_matches( '"').to_string();
                true
            } ] < optSpace < ")" )
            | ( "(" < optSpace < "TIMESCALE" < reqSpace < ( quotedString | ident )[ |arr, ctx: &mut SdfParseCtx| {
                let  	s: &[u8] = arr.into();
                ctx._Timescale = std::str::from_utf8( s).unwrap_or( "").trim_matches( '"').to_string();
                true
            } ] < optSpace < ")" )
            | ( "(" < optSpace < ( "DATE" | "PROGRAM" | "VERSION" | "DIVIDER" | "VOLTAGE" | "PROCESS" | "TEMPERATURE" )
                < *( reqSpace < ( quotedString | ident | tripleValue ) ) < optSpace < ")" )
        );

        let  	sdfFileGrammar = ShardTree!( 
            optSpace
            < "(" < optSpace < "DELAYFILE" < optSpace
            < *( optSpace < ( headerStmt | cellStmt ) )
            < optSpace < ")"
            < optSpace
        );

        sdfFileGrammar.Match( parser, ctx)
    }
}

//-------------------------------------------------------------------------------------------------

/// Parses an IEEE 1497 Standard Delay Format (SDF) file from a string slice.
pub fn	ParseSdf( input: &str) -> Result< SdfFile, String>
{
    let  	mut ctx = SdfParseCtx::New();
    SdfGrammar
        .ParseStr( input, &mut ctx)
        .map( |_| ctx.IntoFile())
        .map_err( |e| e.to_string())
}
