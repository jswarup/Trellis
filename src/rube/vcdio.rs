//-- vcdio.rs -------------------------------------------------------------------------------------------------------
use	crate::ShardTree;
use	crate::flux::instream::FixedStream;
use	crate::shard::{ Charset, IGrammar, Int, Parser };
use	crate::silo::{ Arr, Buff, IArr, Stash };

//-------------------------------------------------------------------------------------------------
/// Represents a variable declaration in a VCD file ($var).
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VcdVar
{
    pub _Type: String,
    pub _Bits: u32,
    pub _Id: String,
    pub _Name: String,
}
impl VcdVar
{
    pub fn	New( 
        vType: impl Into< String>, bits: u32, id: impl Into< String>, name: impl Into< String>,
    ) -> Self
    {
        Self {
            _Type: vType.into(),
            _Bits: bits,
            _Id: id.into(),
            _Name: name.into(),
        }
    }
}

//-------------------------------------------------------------------------------------------------
/// Represents a scope containing variables and nested sub-scopes ($scope ... $upscope).
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VcdScope
{
    pub _Type: String,
    pub _Name: String,
    pub _Vars: Buff< VcdVar>,
    pub _Scopes: Buff< VcdScope>,
}
impl VcdScope
{
    pub fn	New( 
        sType: impl Into< String>, name: impl Into< String>, vars: Buff< VcdVar>,
        scopes: Buff< VcdScope>,
    ) -> Self
    {
        Self {
            _Type: sType.into(),
            _Name: name.into(),
            _Vars: vars,
            _Scopes: scopes,
        }
    }
}

//-------------------------------------------------------------------------------------------------
/// Represents a value change assignment for a signal identifier.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VcdValue
{
    pub _Id: String,
    pub _ValStr: String,
}
impl VcdValue
{
    pub fn	New( id: impl Into< String>, valStr: impl Into< String>) -> Self
    {
        Self {
            _Id: id.into(),
            _ValStr: valStr.into(),
        }
    }
}

//-------------------------------------------------------------------------------------------------
/// Represents all signal value changes occurring at a given simulation time step (#<time>).
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VcdTimeStep
{
    pub _Time: u64,
    pub _Values: Buff< VcdValue>,
}
impl VcdTimeStep
{
    pub fn	New( time: u64, values: Buff< VcdValue>) -> Self
    {
        Self {
            _Time: time,
            _Values: values,
        }
    }
}

//-------------------------------------------------------------------------------------------------
/// Complete parsed in-memory model of a VCD file.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct VcdModel
{
    pub _Version: String,
    pub _Date: String,
    pub _Timescale: String,
    pub _Scopes: Buff< VcdScope>,
    pub _TimeSteps: Buff< VcdTimeStep>,
}
impl Default for VcdModel {
    fn	default() -> Self
    {
        Self::New()
    }
}
impl VcdModel
{
    pub fn	New() -> Self
    {
        Self {
            _Version: String::new(),
            _Date: String::new(),
            _Timescale: String::new(),
            _Scopes: Buff::New(),
            _TimeSteps: Buff::New(),
        }
    }
    /// Serializes this VcdModel into standard IEEE-1364 VCD text format.
    pub fn	Serialize( &self, out: &mut String)
    {
        if !self._Version.is_empty() {
            out.push_str( "$version\n   ");
            out.push_str( self._Version.trim());
            out.push_str( "\n$end\n");
        }
        if !self._Timescale.is_empty() {
            out.push_str( "$timescale ");
            out.push_str( self._Timescale.trim());
            out.push_str( " $end\n");
        }
        if !self._Date.is_empty() {
            out.push_str( "$date ");
            out.push_str( self._Date.trim());
            out.push_str( " $end\n");
        }
        Self::SerializeScopes( self._Scopes.Arr(), out);
        out.push_str( "$enddefinitions $end\n");
        let  	stepCount = self._TimeSteps.Size();
        if stepCount > 0 {
            let  	firstTs = &self._TimeSteps[0];
            if firstTs._Time == 0 && !firstTs._Values.IsEmpty() {
                out.push_str( "$dumpvars\n");
                firstTs._Values.Arr().Traverse( |val| {
                    Self::FormatValue( val, out);
                });
                out.push_str( "$end\n");
            }
            self._TimeSteps.Arr().Traverse( |ts| {
                if ts._Time > 0 || ( ts._Time == 0 && firstTs._Values.IsEmpty()) {
                    out.push_str( &format!( "#{}\n", ts._Time));
                    ts._Values.Arr().Traverse( |val| {
                        Self::FormatValue( val, out);
                    });
                }
            });
        }
    }
    fn	SerializeScopes( scopes: Arr< '_, VcdScope>, out: &mut String) {
        scopes.Traverse( |scope| {
            out.push_str( &format!( "$scope {} {} $end\n", scope._Type, scope._Name));
            scope._Vars.Arr().Traverse( |var| {
                out.push_str( &format!( 
                    "$var {} {} {} {} $end\n",
                    var._Type, var._Bits, var._Id, var._Name
                ));
            });
            Self::SerializeScopes( scope._Scopes.Arr(), out);
            out.push_str( "$upscope $end\n");
        });
    }
    fn	FormatValue( val: &VcdValue, out: &mut String)
    {
        let  	s = &val._ValStr;
        if s.len() <= 1 {
            out.push_str( s);
            out.push_str( &val._Id);
            out.push( '\n');
        } else {
            if s.starts_with( 'b') || s.starts_with( 'B') || s.starts_with( 'r') || s.starts_with( 'R') {
                out.push_str( s);
            } else {
                out.push( 'b');
                out.push_str( s);
            }
            out.push( ' ');
            out.push_str( &val._Id);
            out.push( '\n');
        }
    }
}

//-------------------------------------------------------------------------------------------------
/// Serializes a VcdModel into the target output string.
pub fn	SerializeVcd( model: &VcdModel, out: &mut String)
{
    model.Serialize( out);
}

//-------------------------------------------------------------------------------------------------

struct ScopeBuilder
{
    _Type: String,
    _Name: String,
    _Vars: Stash< VcdVar>,
    _ChildScopes: Stash< VcdScope>,
}
pub struct VcdParserCtx
{
    _Version: String,
    _Date: String,
    _Timescale: String,
    _ScopeStack: Stash< ScopeBuilder>,
    _RootScopes: Stash< VcdScope>,
    _TimeSteps: Stash< VcdTimeStep>,
    _CurrentTime: u64,
    _CurrentValues: Stash< VcdValue>,
    _TempStash: Stash< String>,
}
impl VcdParserCtx
{
    fn	New() -> Self
    {
        Self {
            _Version: String::new(),
            _Date: String::new(),
            _Timescale: String::new(),
            _ScopeStack: Stash::New(),
            _RootScopes: Stash::New(),
            _TimeSteps: Stash::New(),
            _CurrentTime: 0,
            _CurrentValues: Stash::New(),
            _TempStash: Stash::New(),
        }
    }
    fn	DrainScopeStack( &mut self)
    {
        while self._ScopeStack.Size() > 0 {
            let  	popped = self._ScopeStack.Pop().unwrap();
            let  	scope = VcdScope {
                _Type: popped._Type,
                _Name: popped._Name,
                _Vars: popped._Vars.IntoBuff(),
                _Scopes: popped._ChildScopes.IntoBuff(),
            };
            if self._ScopeStack.Size() > 0 {
                let  	parentIdx = self._ScopeStack.Size() - 1;
                self._ScopeStack[parentIdx]._ChildScopes.Push( scope);
            } else {
                self._RootScopes.Push( scope);
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------

impl VcdParserCtx
{
    #[inline( always)]
    pub fn	PushTemp( &mut self, arr: Arr< '_, u8>) -> bool {
        self._TempStash.Push( arr.Str().to_string());
        true
    }
    #[inline( always)]
    pub fn	SetVersion( &mut self) -> bool
    {
        if self._TempStash.Size() > 0 {
            let  	mut joined = String::new();
            self._TempStash.Arr().Traverse( |s| {
                if !joined.is_empty() {
                    joined.push( ' ');
                }
                joined.push_str( s);
            });
            self._Version = joined.trim().to_string();
            self._TempStash.Clear();
        }
        true
    }
    #[inline( always)]
    pub fn	SetTimescale( &mut self) -> bool
    {
        if self._TempStash.Size() > 0 {
            let  	mut joined = String::new();
            self._TempStash.Arr().Traverse( |s| {
                if !joined.is_empty() {
                    joined.push( ' ');
                }
                joined.push_str( s);
            });
            self._Timescale = joined.trim().to_string();
            self._TempStash.Clear();
        }
        true
    }
    #[inline( always)]
    pub fn	SetDate( &mut self) -> bool
    {
        if self._TempStash.Size() > 0 {
            let  	mut joined = String::new();
            self._TempStash.Arr().Traverse( |s| {
                if !joined.is_empty() {
                    joined.push( ' ');
                }
                joined.push_str( s);
            });
            self._Date = joined.trim().to_string();
            self._TempStash.Clear();
        }
        true
    }
    #[inline( always)]
    pub fn	PushScope( &mut self) -> bool
    {
        if self._TempStash.Size() >= 2 {
            let  	builder = ScopeBuilder {
                _Type: self._TempStash[0].clone(),
                _Name: self._TempStash[1].clone(),
                _Vars: Stash::New(),
                _ChildScopes: Stash::New(),
            };
            self._ScopeStack.Push( builder);
        }
        self._TempStash.Clear();
        true
    }
    #[inline( always)]
    pub fn	PushVar( &mut self) -> bool
    {
        if self._TempStash.Size() >= 4 {
            let  	v = VcdVar {
                _Type: self._TempStash[0].clone(),
                _Bits: self._TempStash[1].parse().unwrap_or( 1),
                _Id: self._TempStash[2].clone(),
                _Name: self._TempStash[3].clone(),
            };
            if self._ScopeStack.Size() > 0 {
                let  	topIdx = self._ScopeStack.Size() - 1;
                self._ScopeStack[topIdx]._Vars.Push( v);
            }
        }
        self._TempStash.Clear();
        true
    }
    #[inline( always)]
    pub fn	PopScope( &mut self) -> bool
    {
        if self._ScopeStack.Size() > 0 {
            let  	popped = self._ScopeStack.Pop().unwrap();
            let  	scope = VcdScope {
                _Type: popped._Type,
                _Name: popped._Name,
                _Vars: popped._Vars.IntoBuff(),
                _Scopes: popped._ChildScopes.IntoBuff(),
            };
            if self._ScopeStack.Size() > 0 {
                let  	parentIdx = self._ScopeStack.Size() - 1;
                self._ScopeStack[parentIdx]._ChildScopes.Push( scope);
            } else {
                self._RootScopes.Push( scope);
            }
        }
        true
    }
    #[inline( always)]
    pub fn	EndDefinitions( &mut self) -> bool
    {
        self.DrainScopeStack();
        true
    }
    #[inline( always)]
    pub fn	AddTime( &mut self, arr: Arr< '_, u8>) -> bool {
        // Push previous time step if it has values or at time 0
        if self._CurrentValues.Size() > 0 || self._CurrentTime == 0 {
            let  	ts = VcdTimeStep {
                _Time: self._CurrentTime,
                _Values: self._CurrentValues.TakeBuff(),
            };
            self._TimeSteps.Push( ts);
        }
        self._CurrentTime = arr.Str().parse().unwrap_or( 0);
        true
    }
    #[inline( always)]
    pub fn	AddScalarVal( &mut self, arr: Arr< '_, u8>) -> bool {
        let  	s = arr.Str();
        if s.len() >= 2 {
            let  	valStr = s[0..1].to_string();
            let  	idStr = s[1..].to_string();
            self._CurrentValues.Push( VcdValue {
                _Id: idStr,
                _ValStr: valStr,
            });
        }
        true
    }
    #[inline( always)]
    pub fn	PushVectorVal( &mut self, arr: Arr< '_, u8>) -> bool {
        self._TempStash.Push( arr.Str().to_string());
        true
    }
    #[inline( always)]
    pub fn	PushVectorId( &mut self, arr: Arr< '_, u8>) -> bool {
        if self._TempStash.Size() > 0 {
            let  	valStr = self._TempStash[0].clone();
            let  	idStr = arr.Str().to_string();
            self._CurrentValues.Push( VcdValue {
                _Id: idStr,
                _ValStr: valStr,
            });
            self._TempStash.Clear();
        }
        true
    }
}

//-------------------------------------------------------------------------------------------------
/// Immutable Shard grammar combinator for parsing IEEE-1364 VCD streams.
pub struct VcdGrammar;
pub type VcdShard = VcdGrammar;

impl IGrammar< VcdParserCtx> for VcdGrammar
{
    fn	Match( &self, parser: &mut Parser, ctx: &mut VcdParserCtx) -> bool
    {
        let  	nonWs = *Charset::NonSpace();
        let  	notDollar = Charset::from( &b"$"[..]).Negative();
        let  	notDollarWs = notDollar & nonWs;
        let  	vcdGrammar = ShardTree!( 
            *( 
                *[ " \t\r\n" ]
                < ( 
                    ( "$version" < *( notDollar )[ |arr, ctx: &mut VcdParserCtx| ctx.PushTemp( arr) ] < "$end"[ |_arr, ctx: &mut VcdParserCtx| ctx.SetVersion() ] )
                    | ( "$timescale" < *( notDollar )[ |arr, ctx: &mut VcdParserCtx| ctx.PushTemp( arr) ] < "$end"[ |_arr, ctx: &mut VcdParserCtx| ctx.SetTimescale() ] )
                    | ( "$date" < *( notDollar )[ |arr, ctx: &mut VcdParserCtx| ctx.PushTemp( arr) ] < "$end"[ |_arr, ctx: &mut VcdParserCtx| ctx.SetDate() ] )
                    | ( "$scope" < +( +[ " \t\r\n" ] < ( +notDollarWs)[ |arr, ctx: &mut VcdParserCtx| ctx.PushTemp( arr) ] ) < *[ " \t\r\n" ] < "$end"[ |_arr, ctx: &mut VcdParserCtx| ctx.PushScope() ] )
                    | ( "$var" < +( +[ " \t\r\n" ] < ( +notDollarWs)[ |arr, ctx: &mut VcdParserCtx| ctx.PushTemp( arr) ] ) < *[ " \t\r\n" ] < "$end"[ |_arr, ctx: &mut VcdParserCtx| ctx.PushVar() ] )
                    | ( "$upscope" < *[ " \t\r\n" ] < "$end"[ |_arr, ctx: &mut VcdParserCtx| ctx.PopScope() ] )
                    | ( "$enddefinitions" < *[ " \t\r\n" ] < "$end"[ |_arr, ctx: &mut VcdParserCtx| ctx.EndDefinitions() ] )
                    | ( "$dumpvars" )
                    | ( "$comment" < *( notDollar ) < "$end" )
                    | ( "$end" )
                    | ( "#" < Int[ |arr, ctx: &mut VcdParserCtx| ctx.AddTime( arr) ] )
                    | ( ( "b" | "B" ) < ( +[ "01xXzZ" ])[ |arr, ctx: &mut VcdParserCtx| ctx.PushVectorVal( arr) ] < *[ " \t\r\n" ] < ( +nonWs)[ |arr, ctx: &mut VcdParserCtx| ctx.PushVectorId( arr) ] )
                    | ( ( [ "01xXzZ" ] < +nonWs )[ |arr, ctx: &mut VcdParserCtx| ctx.AddScalarVal( arr) ] )
                )
                < *[ " \t\r\n" ]
            )
        );
        parser
            .ParseGrammar( &vcdGrammar, ctx, parser.CurrMark())
            .is_some()
    }
}

//-------------------------------------------------------------------------------------------------
/// Parses a VCD formatted string into an in-memory VcdModel using Shard grammar combinators.
pub fn	ParseVcd( input: &str) -> Result< VcdModel, String>
{
    let input_len = u32::try_from(input.len())
        .map_err(|_| "VCD stream is too large to parse".to_string())?;
    let  	mut stream = FixedStream::from( input);
    let  	mut parser = Parser::New( &mut stream);
    let  	mut ctx = VcdParserCtx::New();
    let  	grammar = VcdGrammar;
    let  	res = parser.ParseGrammar( &grammar, &mut ctx, 0);
    if res == Some(input_len) {
        ctx.DrainScopeStack();
        // Flush last time step
        if ctx._CurrentValues.Size() > 0 {
            let  	ts = VcdTimeStep {
                _Time: ctx._CurrentTime,
                _Values: ctx._CurrentValues.TakeBuff(),
            };
            ctx._TimeSteps.Push( ts);
        }
        let  	mut model = VcdModel::New();
        model._Version = ctx._Version;
        model._Date = ctx._Date;
        model._Timescale = ctx._Timescale;
        model._Scopes = ctx._RootScopes.IntoBuff();
        model._TimeSteps = ctx._TimeSteps.IntoBuff();
        Ok( model)
    } else {
        Err( "Failed to parse VCD stream".to_string())
    }
}
