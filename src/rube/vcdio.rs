//-- vcdio.rs -------------------------------------------------------------------------------------------------------

use crate::flux::instream::FixedStream;
use crate::shard::{Charset, IGrammar, Int, Parser};
use crate::silo::{Arr, Buff, IArr, Stash};
use crate::ShardTree;

//-------------------------------------------------------------------------------------------------

/// Represents a variable declaration in a VCD file ($var).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VcdVar {
    pub _Type: String,
    pub _Bits: u32,
    pub _Id:   String,
    pub _Name: String,
}

impl VcdVar {
    pub fn New(
        vType: impl Into<String>,
        bits: u32,
        id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            _Type: vType.into(),
            _Bits: bits,
            _Id:   id.into(),
            _Name: name.into(),
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Represents a scope containing variables and nested sub-scopes ($scope ... $upscope).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VcdScope {
    pub _Type:   String,
    pub _Name:   String,
    pub _Vars:   Buff<VcdVar>,
    pub _Scopes: Buff<VcdScope>,
}

impl VcdScope {
    pub fn New(
        sType: impl Into<String>,
        name: impl Into<String>,
        vars: Buff<VcdVar>,
        scopes: Buff<VcdScope>,
    ) -> Self {
        Self {
            _Type:   sType.into(),
            _Name:   name.into(),
            _Vars:   vars,
            _Scopes: scopes,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Represents a value change assignment for a signal identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VcdValue {
    pub _Id:     String,
    pub _ValStr: String,
}

impl VcdValue {
    pub fn New(id: impl Into<String>, valStr: impl Into<String>) -> Self {
        Self {
            _Id:     id.into(),
            _ValStr: valStr.into(),
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Represents all signal value changes occurring at a given simulation time step (#<time>).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VcdTimeStep {
    pub _Time:   u64,
    pub _Values: Buff<VcdValue>,
}

impl VcdTimeStep {
    pub fn New(time: u64, values: Buff<VcdValue>) -> Self {
        Self {
            _Time:   time,
            _Values: values,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Complete parsed in-memory model of a VCD file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VcdModel {
    pub _Version:   String,
    pub _Date:      String,
    pub _Timescale: String,
    pub _Scopes:    Buff<VcdScope>,
    pub _TimeSteps: Buff<VcdTimeStep>,
}

impl Default for VcdModel {
    fn default() -> Self {
        Self::New()
    }
}

impl VcdModel {
    pub fn New() -> Self {
        Self {
            _Version:   String::new(),
            _Date:      String::new(),
            _Timescale: String::new(),
            _Scopes:    Buff::New(),
            _TimeSteps: Buff::New(),
        }
    }

    /// Serializes this VcdModel into standard IEEE-1364 VCD text format.
    pub fn Serialize(&self, out: &mut String) {
        if !self._Version.is_empty() {
            out.push_str("$version\n   ");
            out.push_str(self._Version.trim());
            out.push_str("\n$end\n");
        }
        if !self._Timescale.is_empty() {
            out.push_str("$timescale ");
            out.push_str(self._Timescale.trim());
            out.push_str(" $end\n");
        }
        if !self._Date.is_empty() {
            out.push_str("$date ");
            out.push_str(self._Date.trim());
            out.push_str(" $end\n");
        }

        Self::SerializeScopes(self._Scopes.AsArr(), out);

        out.push_str("$enddefinitions $end\n");

        let stepCount = self._TimeSteps.Size();
        if stepCount > 0 {
            let firstTs = &self._TimeSteps[0];
            if firstTs._Time == 0 && !firstTs._Values.IsEmpty() {
                out.push_str("$dumpvars\n");
                firstTs._Values.AsArr().Traverse(|val| {
                    Self::FormatValue(val, out);
                });
                out.push_str("$end\n");
            }

            self._TimeSteps.AsArr().Traverse(|ts| {
                if ts._Time > 0 || (ts._Time == 0 && firstTs._Values.IsEmpty()) {
                    out.push_str(&format!("#{}\n", ts._Time));
                    ts._Values.AsArr().Traverse(|val| {
                        Self::FormatValue(val, out);
                    });
                }
            });
        }
    }

    fn SerializeScopes(scopes: Arr<'_, VcdScope>, out: &mut String) {
        scopes.Traverse(|scope| {
            out.push_str(&format!("$scope {} {} $end\n", scope._Type, scope._Name));
            scope._Vars.AsArr().Traverse(|var| {
                out.push_str(&format!(
                    "$var {} {} {} {} $end\n",
                    var._Type, var._Bits, var._Id, var._Name
                ));
            });
            Self::SerializeScopes(scope._Scopes.AsArr(), out);
            out.push_str("$upscope $end\n");
        });
    }

    fn FormatValue(val: &VcdValue, out: &mut String) {
        let s = &val._ValStr;
        if s.len() <= 1 {
            out.push_str(s);
            out.push_str(&val._Id);
            out.push('\n');
        } else {
            if s.starts_with('b') || s.starts_with('B') || s.starts_with('r') || s.starts_with('R') {
                out.push_str(s);
            } else {
                out.push('b');
                out.push_str(s);
            }
            out.push(' ');
            out.push_str(&val._Id);
            out.push('\n');
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Serializes a VcdModel into the target output string.
pub fn SerializeVcd(model: &VcdModel, out: &mut String) {
    model.Serialize(out);
}

//-------------------------------------------------------------------------------------------------

struct ScopeBuilder {
    _Type:        String,
    _Name:        String,
    _Vars:        Stash<VcdVar>,
    _ChildScopes: Stash<VcdScope>,
}

pub(crate) struct VcdParserCtx {
    _Version:       String,
    _Date:          String,
    _Timescale:     String,
    _ScopeStack:    Stash<ScopeBuilder>,
    _RootScopes:    Stash<VcdScope>,
    _TimeSteps:     Stash<VcdTimeStep>,
    _CurrentTime:   u64,
    _CurrentValues: Stash<VcdValue>,
    _TempStash:     Stash<String>,
}

impl VcdParserCtx {
    fn New() -> Self {
        Self {
            _Version:       String::new(),
            _Date:          String::new(),
            _Timescale:     String::new(),
            _ScopeStack:    Stash::New(),
            _RootScopes:    Stash::New(),
            _TimeSteps:     Stash::New(),
            _CurrentTime:   0,
            _CurrentValues: Stash::New(),
            _TempStash:     Stash::New(),
        }
    }

    fn DrainScopeStack(&mut self) {
        while self._ScopeStack.Size() > 0 {
            let popped = self._ScopeStack.Pop().unwrap();
            let scope = VcdScope {
                _Type:   popped._Type,
                _Name:   popped._Name,
                _Vars:   popped._Vars.IntoBuff(),
                _Scopes: popped._ChildScopes.IntoBuff(),
            };
            if self._ScopeStack.Size() > 0 {
                let parentIdx = self._ScopeStack.Size() - 1;
                self._ScopeStack[parentIdx]._ChildScopes.Push(scope);
            } else {
                self._RootScopes.Push(scope);
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct VcdParserCtxMM(*mut VcdParserCtx);

impl VcdParserCtxMM {
    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    fn Get(&self) -> &mut VcdParserCtx {
        unsafe { &mut *self.0 }
    }

    #[inline(always)]
    fn PushTemp(&self, arr: Arr<'_, u8>) -> bool {
        self.Get()._TempStash.Push(arr.AsStr().to_string());
        true
    }

    #[inline(always)]
    fn SetVersion(&self) -> bool {
        let ctx = self.Get();
        if ctx._TempStash.Size() > 0 {
            let mut joined = String::new();
            ctx._TempStash.AsArr().Traverse(|s| {
                if !joined.is_empty() {
                    joined.push(' ');
                }
                joined.push_str(s);
            });
            ctx._Version = joined.trim().to_string();
            ctx._TempStash.Clear();
        }
        true
    }

    #[inline(always)]
    fn SetTimescale(&self) -> bool {
        let ctx = self.Get();
        if ctx._TempStash.Size() > 0 {
            let mut joined = String::new();
            ctx._TempStash.AsArr().Traverse(|s| {
                if !joined.is_empty() {
                    joined.push(' ');
                }
                joined.push_str(s);
            });
            ctx._Timescale = joined.trim().to_string();
            ctx._TempStash.Clear();
        }
        true
    }

    #[inline(always)]
    fn SetDate(&self) -> bool {
        let ctx = self.Get();
        if ctx._TempStash.Size() > 0 {
            let mut joined = String::new();
            ctx._TempStash.AsArr().Traverse(|s| {
                if !joined.is_empty() {
                    joined.push(' ');
                }
                joined.push_str(s);
            });
            ctx._Date = joined.trim().to_string();
            ctx._TempStash.Clear();
        }
        true
    }

    #[inline(always)]
    fn PushScope(&self) -> bool {
        let ctx = self.Get();
        if ctx._TempStash.Size() >= 2 {
            let builder = ScopeBuilder {
                _Type:        ctx._TempStash[0].clone(),
                _Name:        ctx._TempStash[1].clone(),
                _Vars:        Stash::New(),
                _ChildScopes: Stash::New(),
            };
            ctx._ScopeStack.Push(builder);
        }
        ctx._TempStash.Clear();
        true
    }

    #[inline(always)]
    fn PushVar(&self) -> bool {
        let ctx = self.Get();
        if ctx._TempStash.Size() >= 4 {
            let v = VcdVar {
                _Type: ctx._TempStash[0].clone(),
                _Bits: ctx._TempStash[1].parse().unwrap_or(1),
                _Id:   ctx._TempStash[2].clone(),
                _Name: ctx._TempStash[3].clone(),
            };
            if ctx._ScopeStack.Size() > 0 {
                let topIdx = ctx._ScopeStack.Size() - 1;
                ctx._ScopeStack[topIdx]._Vars.Push(v);
            }
        }
        ctx._TempStash.Clear();
        true
    }

    #[inline(always)]
    fn PopScope(&self) -> bool {
        let ctx = self.Get();
        if ctx._ScopeStack.Size() > 0 {
            let popped = ctx._ScopeStack.Pop().unwrap();
            let scope = VcdScope {
                _Type:   popped._Type,
                _Name:   popped._Name,
                _Vars:   popped._Vars.IntoBuff(),
                _Scopes: popped._ChildScopes.IntoBuff(),
            };
            if ctx._ScopeStack.Size() > 0 {
                let parentIdx = ctx._ScopeStack.Size() - 1;
                ctx._ScopeStack[parentIdx]._ChildScopes.Push(scope);
            } else {
                ctx._RootScopes.Push(scope);
            }
        }
        true
    }

    #[inline(always)]
    fn EndDefinitions(&self) -> bool {
        self.Get().DrainScopeStack();
        true
    }

    #[inline(always)]
    fn AddTime(&self, arr: Arr<'_, u8>) -> bool {
        let ctx = self.Get();
        // Push previous time step if it has values or at time 0
        if ctx._CurrentValues.Size() > 0 || ctx._CurrentTime == 0 {
            let ts = VcdTimeStep {
                _Time:   ctx._CurrentTime,
                _Values: ctx._CurrentValues.ToBuff(),
            };
            ctx._TimeSteps.Push(ts);
            ctx._CurrentValues.Clear();
        }
        ctx._CurrentTime = arr.AsStr().parse().unwrap_or(0);
        true
    }

    #[inline(always)]
    fn AddScalarVal(&self, arr: Arr<'_, u8>) -> bool {
        let ctx = self.Get();
        let s = arr.AsStr();
        if s.len() >= 2 {
            let valStr = s[0..1].to_string();
            let idStr = s[1..].to_string();
            ctx._CurrentValues.Push(VcdValue {
                _Id:     idStr,
                _ValStr: valStr,
            });
        }
        true
    }

    #[inline(always)]
    fn PushVectorVal(&self, arr: Arr<'_, u8>) -> bool {
        self.Get()._TempStash.Push(arr.AsStr().to_string());
        true
    }

    #[inline(always)]
    fn PushVectorId(&self, arr: Arr<'_, u8>) -> bool {
        let ctx = self.Get();
        if ctx._TempStash.Size() > 0 {
            let valStr = ctx._TempStash[0].clone();
            let idStr = arr.AsStr().to_string();
            ctx._CurrentValues.Push(VcdValue {
                _Id:     idStr,
                _ValStr: valStr,
            });
            ctx._TempStash.Clear();
        }
        true
    }
}

//-------------------------------------------------------------------------------------------------

/// Shard grammar combinator for parsing IEEE-1364 VCD streams.
pub struct VcdShard<'a> {
    pub _Model: &'a mut VcdModel,
}

impl<'a> IGrammar for VcdShard<'a> {
    fn Match(&self, parser: &mut Parser) -> bool {
        let modelPtr: *mut VcdModel = self._Model as *const VcdModel as *mut VcdModel;
        let model = unsafe { &mut *modelPtr };

        let mut ctx = VcdParserCtx::New();
        let ctxMM = VcdParserCtxMM(&mut ctx as *mut _);

        let nonWs = *Charset::NonSpace();
        let notDollar = Charset::from(&b"$"[..]).Negative();
        let notDollarWs = notDollar & nonWs;

        let vcdGrammar = ShardTree!(
            *(
                *[ " \t\r\n" ]
                < (
                    ( "$version" < *( notDollar )[ |arr| ctxMM.PushTemp( arr) ] < "$end"[ |_arr| ctxMM.SetVersion() ] )
                    | ( "$timescale" < *( notDollar )[ |arr| ctxMM.PushTemp( arr) ] < "$end"[ |_arr| ctxMM.SetTimescale() ] )
                    | ( "$date" < *( notDollar )[ |arr| ctxMM.PushTemp( arr) ] < "$end"[ |_arr| ctxMM.SetDate() ] )
                    | ( "$scope" < +( +[ " \t\r\n" ] < (+notDollarWs)[ |arr| ctxMM.PushTemp( arr) ] ) < *[ " \t\r\n" ] < "$end"[ |_arr| ctxMM.PushScope() ] )
                    | ( "$var" < +( +[ " \t\r\n" ] < (+notDollarWs)[ |arr| ctxMM.PushTemp( arr) ] ) < *[ " \t\r\n" ] < "$end"[ |_arr| ctxMM.PushVar() ] )
                    | ( "$upscope" < *[ " \t\r\n" ] < "$end"[ |_arr| ctxMM.PopScope() ] )
                    | ( "$enddefinitions" < *[ " \t\r\n" ] < "$end"[ |_arr| ctxMM.EndDefinitions() ] )
                    | ( "$dumpvars" )
                    | ( "$comment" < *( notDollar ) < "$end" )
                    | ( "$end" )
                    | ( "#" < Int[ |arr| ctxMM.AddTime( arr) ] )
                    | ( ( "b" | "B" ) < (+[ "01xXzZ" ])[ |arr| ctxMM.PushVectorVal( arr) ] < *[ " \t\r\n" ] < (+nonWs)[ |arr| ctxMM.PushVectorId( arr) ] )
                    | ( ( [ "01xXzZ" ] < +nonWs )[ |arr| ctxMM.AddScalarVal( arr) ] )
                )
                < *[ " \t\r\n" ]
            )
        );

        if parser.ParseGrammar(&vcdGrammar, parser.CurrMark()).is_none() {
            return false;
        }

        ctx.DrainScopeStack();

        // Flush last time step
        if ctx._CurrentValues.Size() > 0 {
            let ts = VcdTimeStep {
                _Time:   ctx._CurrentTime,
                _Values: ctx._CurrentValues.ToBuff(),
            };
            ctx._TimeSteps.Push(ts);
        }

        model._Version = ctx._Version;
        model._Date = ctx._Date;
        model._Timescale = ctx._Timescale;
        model._Scopes = ctx._RootScopes.IntoBuff();
        model._TimeSteps = ctx._TimeSteps.IntoBuff();

        true
    }
}

//-------------------------------------------------------------------------------------------------

/// Parses a VCD formatted string into an in-memory VcdModel using Shard grammar combinators.
pub fn ParseVcd(input: &str) -> Result<VcdModel, String> {
    let mut stream = FixedStream::from(input);
    let mut model = VcdModel::New();
    let mut parser = Parser::New(&mut stream);
    let shard = VcdShard { _Model: &mut model };
    let res = parser.ParseGrammar(&shard, 0);
    if res.is_some() {
        Ok(model)
    } else {
        Err("Failed to parse VCD stream".to_string())
    }
}
