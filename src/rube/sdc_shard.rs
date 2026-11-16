//-- sdc_shard.rs ------------------------------------------------------------------------------------------------
use	crate::ShardTree;
use	crate::rube::sdc::
{
    ClockConstraint, DelayKind, DelayType, PathConstraint, PathKind, PortDelayConstraint,
    SdcConstraints,
};
use	crate::shard::
{
    Charset, IGrammar, Real, UInt,
};
use	crate::silo::{ Arr, Stash };

//-------------------------------------------------------------------------------------------------

pub struct SdcParseCtx
{
    pub _Clocks:      Stash< ClockConstraint>,
    pub _Delays:      Stash< PortDelayConstraint>,
    pub _Paths:       Stash< PathConstraint>,
    pub _Name:        String,
    pub _ClockName:   String,
    pub _PortName:    String,
    pub _Period:      f64,
    pub _Rise:        f64,
    pub _Fall:        f64,
    pub _HasWaveform: bool,
    pub _Delay:       f64,
    pub _DelayType:   DelayType,
    pub _DelayKind:   DelayKind,
    pub _Cycles:      u32,
    pub _From:        Option< String>,
    pub _To:          Option< String>,
    pub _Through:     Option< String>,
}

impl Default for SdcParseCtx
{
    fn	default() -> Self
    {
        Self::New()
    }
}

impl SdcParseCtx
{
    pub fn	New() -> Self
    {
        Self {
            _Clocks:      Stash::New(),
            _Delays:      Stash::New(),
            _Paths:       Stash::New(),
            _Name:        String::new(),
            _ClockName:   String::new(),
            _PortName:    String::new(),
            _Period:      0.0,
            _Rise:        0.0,
            _Fall:        0.0,
            _HasWaveform: false,
            _Delay:       0.0,
            _DelayType:   DelayType::Both,
            _DelayKind:   DelayKind::Input,
            _Cycles:      2,
            _From:        None,
            _To:          None,
            _Through:     None,
        }
    }

    pub fn	ClearWorking( &mut self)
    {
        self._Name.clear();
        self._ClockName.clear();
        self._PortName.clear();
        self._Period = 0.0;
        self._Rise = 0.0;
        self._Fall = 0.0;
        self._HasWaveform = false;
        self._Delay = 0.0;
        self._DelayType = DelayType::Both;
        self._DelayKind = DelayKind::Input;
        self._Cycles = 2;
        self._From = None;
        self._To = None;
        self._Through = None;
    }

    pub fn	ExtractTarget( bytes: Arr< '_, u8>) -> String
    {
        let  	raw: &[u8] = bytes.into();
        let  	s = std::str::from_utf8( raw).unwrap_or( "").trim();
        let  	trimmed = s.trim_matches( |c| c == '[' || c == ']').trim();
        if let  	Some( target) = trimmed.strip_prefix( "get_ports ") {
            target.trim_matches( |c| c == '{' || c == '}' || c == '"').trim().to_string()
        } else if let  	Some( target) = trimmed.strip_prefix( "get_pins ") {
            target.trim_matches( |c| c == '{' || c == '}' || c == '"').trim().to_string()
        } else if let  	Some( target) = trimmed.strip_prefix( "get_clocks ") {
            target.trim_matches( |c| c == '{' || c == '}' || c == '"').trim().to_string()
        } else {
            trimmed.trim_matches( |c| c == '{' || c == '}' || c == '"').trim().to_string()
        }
    }

    pub fn	SetName( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        self._Name = Self::ExtractTarget( bytes);
        true
    }

    pub fn	SetClockName( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        self._ClockName = Self::ExtractTarget( bytes);
        true
    }

    pub fn	SetPortName( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        self._PortName = Self::ExtractTarget( bytes);
        true
    }

    pub fn	SetPeriod( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        let  	raw: &[u8] = bytes.into();
        if let  	Ok( s) = std::str::from_utf8( raw) {
            self._Period = s.parse::< f64>().unwrap_or( 0.0);
            true
        } else {
            false
        }
    }

    pub fn	SetRise( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        let  	raw: &[u8] = bytes.into();
        if let  	Ok( s) = std::str::from_utf8( raw) {
            self._Rise = s.parse::< f64>().unwrap_or( 0.0);
            true
        } else {
            false
        }
    }

    pub fn	SetFall( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        let  	raw: &[u8] = bytes.into();
        if let  	Ok( s) = std::str::from_utf8( raw) {
            self._Fall = s.parse::< f64>().unwrap_or( 0.0);
            self._HasWaveform = true;
            true
        } else {
            false
        }
    }

    pub fn	SetDelay( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        let  	raw: &[u8] = bytes.into();
        if let  	Ok( s) = std::str::from_utf8( raw) {
            self._Delay = s.parse::< f64>().unwrap_or( 0.0);
            true
        } else {
            false
        }
    }

    pub fn	SetDelayType( &mut self, dt: DelayType) -> bool
    {
        self._DelayType = dt;
        true
    }

    pub fn	SetCycles( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        let  	raw: &[u8] = bytes.into();
        if let  	Ok( s) = std::str::from_utf8( raw) {
            self._Cycles = s.parse::< u32>().unwrap_or( 2);
            true
        } else {
            false
        }
    }

    pub fn	SetFrom( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        self._From = Some( Self::ExtractTarget( bytes));
        true
    }

    pub fn	SetTo( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        self._To = Some( Self::ExtractTarget( bytes));
        true
    }

    pub fn	SetThrough( &mut self, bytes: Arr< '_, u8>) -> bool
    {
        self._Through = Some( Self::ExtractTarget( bytes));
        true
    }

    pub fn	CommitClock( &mut self) -> bool
    {
        if !self._HasWaveform && self._Period > 0.0 {
            self._Fall = self._Period / 2.0;
        }
        let  	name = if !self._Name.is_empty() {
            std::mem::take( &mut self._Name)
        } else if !self._PortName.is_empty() {
            self._PortName.clone()
        } else {
            "clk".to_string()
        };
        let  	clk = ClockConstraint::New( 
            name,
            self._Period,
            self._Rise,
            self._Fall,
            std::mem::take( &mut self._PortName),
        );
        self._Clocks.Push( clk);
        self.ClearWorking();
        true
    }

    pub fn	SetDelayKind( &mut self, kind: DelayKind) -> bool
    {
        self._DelayKind = kind;
        true
    }

    pub fn	CommitDelay( &mut self) -> bool
    {
        let  	delay = PortDelayConstraint::New( 
            self._DelayKind,
            std::mem::take( &mut self._ClockName),
            self._Delay,
            std::mem::take( &mut self._PortName),
            self._DelayType,
        );
        self._Delays.Push( delay);
        self.ClearWorking();
        true
    }

    pub fn	CommitFalsePath( &mut self) -> bool
    {
        let  	path = PathConstraint::New( 
            PathKind::FalsePath,
            0,
            self._From.take(),
            self._To.take(),
            self._Through.take(),
        );
        self._Paths.Push( path);
        self.ClearWorking();
        true
    }

    pub fn	CommitMulticyclePath( &mut self) -> bool
    {
        let  	path = PathConstraint::New( 
            PathKind::MulticyclePath,
            self._Cycles,
            self._From.take(),
            self._To.take(),
            None,
        );
        self._Paths.Push( path);
        self.ClearWorking();
        true
    }
}

//-------------------------------------------------------------------------------------------------

pub struct SdcGrammar;
pub type SdcShard = SdcGrammar;

impl IGrammar< SdcParseCtx> for SdcGrammar
{
    fn	Match( &self, parser: &mut crate::shard::Parser, ctx: &mut SdcParseCtx) -> bool
    {
        let  	notBracket = Charset::from( &b"[]\r\n"[..]).Negative();
        let  	notBrace = Charset::from( &b"{}\r\n"[..]).Negative();
        let  	notQuote = Charset::from( &b"\"\r\n"[..]).Negative();
        let  	notDelim = Charset::from( &b" \t\r\n;{}[]#\""[..]).Negative();
        let  	notEol = Charset::EndLine().Negative();

        let  	target = ShardTree!( 
            ( "[" < *notBracket < "]" )
            | ( "{" < *notBrace < "}" )
            | ( "\"" < *notQuote < "\"" )
            | ( +notDelim )
        );

        let  	clockOpt = ShardTree!( 
            ( "-name" < +[ " \t" ] < target[ |arr, ctx: &mut SdcParseCtx| ctx.SetName( arr) ] )
            | ( "-period" < +[ " \t" ] < Real[ |arr, ctx: &mut SdcParseCtx| ctx.SetPeriod( arr) ] )
            | ( "-waveform" < +[ " \t" ] < ?( "{" < *[ " \t" ] ) < Real[ |arr, ctx: &mut SdcParseCtx| ctx.SetRise( arr) ] < +[ " \t" ] < Real[ |arr, ctx: &mut SdcParseCtx| ctx.SetFall( arr) ] < ?( *[ " \t" ] < "}" ) )
            | ( target[ |arr, ctx: &mut SdcParseCtx| ctx.SetPortName( arr) ] )
        );

        let  	delayOpt = ShardTree!( 
            ( "-clock" < +[ " \t" ] < target[ |arr, ctx: &mut SdcParseCtx| ctx.SetClockName( arr) ] )
            | ( "-max"[ |_arr, ctx: &mut SdcParseCtx| ctx.SetDelayType( DelayType::Max) ] )
            | ( "-min"[ |_arr, ctx: &mut SdcParseCtx| ctx.SetDelayType( DelayType::Min) ] )
            | ( Real[ |arr, ctx: &mut SdcParseCtx| ctx.SetDelay( arr) ] )
            | ( target[ |arr, ctx: &mut SdcParseCtx| ctx.SetPortName( arr) ] )
        );

        let  	falsePathOpt = ShardTree!( 
            ( "-from" < +[ " \t" ] < target[ |arr, ctx: &mut SdcParseCtx| ctx.SetFrom( arr) ] )
            | ( "-to" < +[ " \t" ] < target[ |arr, ctx: &mut SdcParseCtx| ctx.SetTo( arr) ] )
            | ( "-through" < +[ " \t" ] < target[ |arr, ctx: &mut SdcParseCtx| ctx.SetThrough( arr) ] )
        );

        let  	multicycleOpt = ShardTree!( 
            ( "-from" < +[ " \t" ] < target[ |arr, ctx: &mut SdcParseCtx| ctx.SetFrom( arr) ] )
            | ( "-to" < +[ " \t" ] < target[ |arr, ctx: &mut SdcParseCtx| ctx.SetTo( arr) ] )
            | ( UInt[ |arr, ctx: &mut SdcParseCtx| ctx.SetCycles( arr) ] )
        );

        let  	sdcLine = ShardTree!( 
            *[ " \t" ]
            < ( 
                ( "create_clock" < *( +[ " \t" ] < clockOpt ) )[ |_arr, ctx: &mut SdcParseCtx| ctx.CommitClock() ]
                | ( ( ( "set_input_delay"[ |_arr, ctx: &mut SdcParseCtx| ctx.SetDelayKind( DelayKind::Input) ] )
                      | ( "set_output_delay"[ |_arr, ctx: &mut SdcParseCtx| ctx.SetDelayKind( DelayKind::Output) ] ) )
                    < *( +[ " \t" ] < delayOpt ) )[ |_arr, ctx: &mut SdcParseCtx| ctx.CommitDelay() ]
                | ( "set_false_path" < *( +[ " \t" ] < falsePathOpt ) )[ |_arr, ctx: &mut SdcParseCtx| ctx.CommitFalsePath() ]
                | ( "set_multicycle_path" < *( +[ " \t" ] < multicycleOpt ) )[ |_arr, ctx: &mut SdcParseCtx| ctx.CommitMulticyclePath() ]
                | ( '#' < *notEol )
                | ( *notEol )
            )
            < *[ " \t" ]
            < ?( ( ?'\r' < '\n' ) | '\r' )
        );

        let  	sdcScript = ShardTree!( *( sdcLine ) );
        parser.ParseGrammar( &sdcScript, ctx, parser.CurrMark() ).is_some()
    }
}

//-------------------------------------------------------------------------------------------------
/// Parses an SDC constraints script into structured `SdcConstraints` using Shard grammar combinators.
pub fn	ParseSdc( sdc_text: &str) -> Result< SdcConstraints, String>
{
    let  	mut ctx = SdcParseCtx::New();
    SdcGrammar
        .ParseStr( sdc_text, &mut ctx)
        .map_err( |e| e.to_string())?;
    Ok( SdcConstraints::New( ctx._Clocks.IntoBuff(), ctx._Delays.IntoBuff(), ctx._Paths.IntoBuff()))
}
