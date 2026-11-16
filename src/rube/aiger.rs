// aiger.rs ---------------------------------------------------------------------------------------------------
use	crate::rube::layout::Layout;
use	crate::rube::module::{ KernelKind, KernelOp };
use	crate::rube::port::{ ModuleId, PortId };
use	crate::silo::{ Buff, IArr, Stash, USeg };

//-------------------------------------------------------------------------------------------------

/// Represents a 2-input AND gate in an AND-Inverter Graph: `lhs = rhs0 & rhs1`.
#[derive( Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct AigGate
{
    _Lhs:  u32,
    _Rhs0: u32,
    _Rhs1: u32,
}

impl AigGate
{
    pub const fn	New( lhs: u32, rhs0: u32, rhs1: u32) -> Self
    {
        Self {
            _Lhs:  lhs,
            _Rhs0: rhs0,
            _Rhs1: rhs1,
        }
    }

    #[inline] pub const fn	Lhs( &self) -> u32 { self._Lhs }
    #[inline] pub const fn	Rhs0( &self) -> u32 { self._Rhs0 }
    #[inline] pub const fn	Rhs1( &self) -> u32 { self._Rhs1 }
}

//-------------------------------------------------------------------------------------------------

/// Represents a sequential latch in AIGER with next-state literal and reset value.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct AigLatch
{
    _Lit:   u32,
    _Next:  u32,
    _Reset: u32,
    _Name:  String,
}

impl AigLatch
{
    pub fn	New( lit: u32, next: u32, reset: u32, name: String) -> Self
    {
        Self {
            _Lit:   lit,
            _Next:  next,
            _Reset: reset,
            _Name:  name,
        }
    }

    #[inline] pub const fn	Lit( &self) -> u32 { self._Lit }
    #[inline] pub const fn	Next( &self) -> u32 { self._Next }
    #[inline] pub const fn	Reset( &self) -> u32 { self._Reset }
    #[inline] pub fn	Name( &self) -> &str { &self._Name }
}

//-------------------------------------------------------------------------------------------------

/// Symbol table entry mapping an index to a descriptive signal name.
#[derive( Clone, Debug, PartialEq, Eq)]
pub struct AigSymbol
{
    _Kind:  char,
    _Index: u32,
    _Name:  String,
}

impl AigSymbol
{
    pub fn	New( kind: char, index: u32, name: String) -> Self
    {
        Self {
            _Kind:  kind,
            _Index: index,
            _Name:  name,
        }
    }

    #[inline] pub const fn	Kind( &self) -> char { self._Kind }
    #[inline] pub const fn	Index( &self) -> u32 { self._Index }
    #[inline] pub fn	Name( &self) -> &str { &self._Name }
}

//-------------------------------------------------------------------------------------------------

/// Complete AND-Inverter Graph (AIG) formal model.
#[derive( Clone, Debug, PartialEq, Eq, Default)]
pub struct Aig
{
    _MaxVar:   u32,
    _Inputs:   Buff< u32>,
    _Latches:  Buff< AigLatch>,
    _Outputs:  Buff< u32>,
    _Ands:     Buff< AigGate>,
    _Symbols:  Buff< AigSymbol>,
    _Comments: Buff< String>,
}

impl Aig
{
    pub fn	New(
        maxVar: u32,
        inputs: Buff< u32>,
        latches: Buff< AigLatch>,
        outputs: Buff< u32>,
        ands: Buff< AigGate>,
        symbols: Buff< AigSymbol>,
        comments: Buff< String>,
    ) -> Self
    {
        Self {
            _MaxVar:   maxVar,
            _Inputs:   inputs,
            _Latches:  latches,
            _Outputs:  outputs,
            _Ands:     ands,
            _Symbols:  symbols,
            _Comments: comments,
        }
    }

    #[inline] pub const fn	MaxVar( &self) -> u32 { self._MaxVar }
    #[inline] pub const fn	InputCount( &self) -> u32 { self._Inputs.Size() }
    #[inline] pub const fn	LatchCount( &self) -> u32 { self._Latches.Size() }
    #[inline] pub const fn	OutputCount( &self) -> u32 { self._Outputs.Size() }
    #[inline] pub const fn	AndCount( &self) -> u32 { self._Ands.Size() }

    #[inline] pub fn	Inputs( &self) -> &Buff< u32> { &self._Inputs }
    #[inline] pub fn	Latches( &self) -> &Buff< AigLatch> { &self._Latches }
    #[inline] pub fn	Outputs( &self) -> &Buff< u32> { &self._Outputs }
    #[inline] pub fn	Ands( &self) -> &Buff< AigGate> { &self._Ands }
    #[inline] pub fn	Symbols( &self) -> &Buff< AigSymbol> { &self._Symbols }
    #[inline] pub fn	Comments( &self) -> &Buff< String> { &self._Comments }
}

//-------------------------------------------------------------------------------------------------

/// Programmatic builder for creating normalized AND-Inverter Graphs with constant folding.
pub struct AigBuilder
{
    _MaxVar:   u32,
    _Inputs:   Stash< u32>,
    _Latches:  Stash< AigLatch>,
    _Outputs:  Stash< u32>,
    _Ands:     Stash< AigGate>,
    _Symbols:  Stash< AigSymbol>,
    _Comments: Stash< String>,
}

impl Default for AigBuilder
{
    fn	default() -> Self { Self::New() }
}

impl AigBuilder
{
    pub fn	New() -> Self
    {
        Self {
            _MaxVar:   0,
            _Inputs:   Stash::New(),
            _Latches:  Stash::New(),
            _Outputs:  Stash::New(),
            _Ands:     Stash::New(),
            _Symbols:  Stash::New(),
            _Comments: Stash::New(),
        }
    }

    pub fn	AddInput( &mut self, name: &str) -> u32
    {
        self._MaxVar += 1;
        let  	lit = self._MaxVar * 2;
        let  	idx = self._Inputs.Size();
        self._Inputs.Push( lit);
        if !name.is_empty()
        {
            self._Symbols.Push( AigSymbol::New( 'i', idx, name.to_string()));
        }
        lit
    }

    pub fn	AddLatch( &mut self, name: &str, nextLit: u32, reset: u32) -> u32
    {
        self._MaxVar += 1;
        let  	lit = self._MaxVar * 2;
        let  	idx = self._Latches.Size();
        self._Latches.Push( AigLatch::New( lit, nextLit, reset, name.to_string()));
        if !name.is_empty()
        {
            self._Symbols.Push( AigSymbol::New( 'l', idx, name.to_string()));
        }
        lit
    }

    pub fn	AddOutput( &mut self, lit: u32, name: &str)
    {
        let  	idx = self._Outputs.Size();
        self._Outputs.Push( lit);
        if !name.is_empty()
        {
            self._Symbols.Push( AigSymbol::New( 'o', idx, name.to_string()));
        }
    }

    pub fn	AddAnd( &mut self, lit1: u32, lit2: u32) -> u32
    {
        if lit1 == 0 || lit2 == 0
        {
            return 0;
        }
        if lit1 == 1
        {
            return lit2;
        }
        if lit2 == 1
        {
            return lit1;
        }
        if lit1 == lit2
        {
            return lit1;
        }
        if lit1 == ( lit2 ^ 1)
        {
            return 0;
        }
        let  	( rhs0, rhs1) = if lit1 >= lit2 { ( lit1, lit2) } else { ( lit2, lit1) };
        let  	mut existing = 0;
        self._Ands.Arr().Traverse( |gate| {
            if gate.Rhs0() == rhs0 && gate.Rhs1() == rhs1
            {
                existing = gate.Lhs();
            }
        });
        if existing != 0
        {
            return existing;
        }
        self._MaxVar += 1;
        let  	lhs = self._MaxVar * 2;
        self._Ands.Push( AigGate::New( lhs, rhs0, rhs1));
        lhs
    }

    #[inline] pub fn	AddNot( &mut self, lit: u32) -> u32 { lit ^ 1 }

    #[inline]
    pub fn	AddNand( &mut self, lit1: u32, lit2: u32) -> u32
    {
        let  	a = self.AddAnd( lit1, lit2);
        self.AddNot( a)
    }

    pub fn	AddOr( &mut self, lit1: u32, lit2: u32) -> u32
    {
        let  	not1 = self.AddNot( lit1);
        let  	not2 = self.AddNot( lit2);
        let  	nand = self.AddAnd( not1, not2);
        self.AddNot( nand)
    }

    #[inline]
    pub fn	AddNor( &mut self, lit1: u32, lit2: u32) -> u32
    {
        let  	orLit = self.AddOr( lit1, lit2);
        self.AddNot( orLit)
    }

    pub fn	AddXor( &mut self, lit1: u32, lit2: u32) -> u32
    {
        let  	orTerm = self.AddOr( lit1, lit2);
        let  	nandTerm = self.AddNand( lit1, lit2);
        self.AddAnd( orTerm, nandTerm)
    }

    #[inline]
    pub fn	AddXnor( &mut self, lit1: u32, lit2: u32) -> u32
    {
        let  	xorLit = self.AddXor( lit1, lit2);
        self.AddNot( xorLit)
    }

    pub fn	AddComment( &mut self, comment: &str)
    {
        self._Comments.Push( comment.to_string());
    }

    pub fn	Build( self) -> Aig
    {
        Aig::New(
            self._MaxVar,
            self._Inputs.IntoBuff(),
            self._Latches.IntoBuff(),
            self._Outputs.IntoBuff(),
            self._Ands.IntoBuff(),
            self._Symbols.IntoBuff(),
            self._Comments.IntoBuff(),
        )
    }
}

//-------------------------------------------------------------------------------------------------

fn	EncodeLeb128( mut val: u32, out: &mut Stash< u8>)
{
    while ( val & !0x7F) != 0
    {
        out.Push( ( ( val & 0x7F) | 0x80) as u8);
        val >>= 7;
    }
    out.Push( ( val & 0x7F) as u8);
}

//-------------------------------------------------------------------------------------------------

fn	DecodeLeb128( bytes: &[u8], pos: &mut usize) -> Result< u32, String>
{
    let  	mut result: u32 = 0;
    let  	mut shift: u32 = 0;
    while *pos < bytes.len()
    {
        let  	byte = bytes[*pos];
        *pos += 1;
        result |= ( ( byte & 0x7F) as u32) << shift;
        if ( byte & 0x80) == 0
        {
            return Ok( result);
        }
        shift += 7;
        if shift > 35
        {
            return Err( "LEB128 integer overflow".to_string());
        }
    }
    Err( "Unexpected EOF in LEB128 sequence".to_string())
}

//-------------------------------------------------------------------------------------------------

fn	ParseHeader( line: &str) -> Result< ( u32, u32, u32, u32, u32), String>
{
    let  	parts: Vec< &str> = line.split_whitespace().collect();
    if parts.len() < 6 || parts[0] != "aag"
    {
        return Err( "Invalid AIGER header (expected 'aag M I L O A')".to_string());
    }
    let  	m = parts[1].parse().map_err( |e| format!( "{}", e))?;
    let  	i = parts[2].parse().map_err( |e| format!( "{}", e))?;
    let  	l = parts[3].parse().map_err( |e| format!( "{}", e))?;
    let  	o = parts[4].parse().map_err( |e| format!( "{}", e))?;
    let  	a = parts[5].parse().map_err( |e| format!( "{}", e))?;
    Ok( ( m, i, l, o, a))
}

//-------------------------------------------------------------------------------------------------

fn	ParseLatches< 'a, I>( lines: &mut I, count: u32) -> Result< Stash< AigLatch>, String>
where
    I: Iterator< Item = &'a str>,
{
    let  	mut latches = Stash::New();
    let  	mut err = None;
    USeg::FromLen( count).Span( |_| {
        let  	lineOpt = lines.next();
        if lineOpt.is_none()
        {
            err = Some( "Missing latch specification".to_string());
            return false;
        }
        let  	line = lineOpt.unwrap();
        let  	parts: Vec< &str> = line.split_whitespace().collect();
        if parts.len() < 2
        {
            err = Some( "Malformed latch line".to_string());
            return false;
        }
        let  	litRes = parts[0].parse::< u32>();
        let  	nextRes = parts[1].parse::< u32>();
        if litRes.is_err() || nextRes.is_err()
        {
            err = Some( "Malformed latch numbers".to_string());
            return false;
        }
        let  	reset = if parts.len() > 2 { parts[2].parse::< u32>().unwrap_or( 0) } else { 0 };
        latches.Push( AigLatch::New( litRes.unwrap(), nextRes.unwrap(), reset, String::new()));
        true
    });
    if let Some( e) = err
    {
        return Err( e);
    }
    Ok( latches)
}

//-------------------------------------------------------------------------------------------------

fn	ParseAnds< 'a, I>( lines: &mut I, count: u32) -> Result< Stash< AigGate>, String>
where
    I: Iterator< Item = &'a str>,
{
    let  	mut ands = Stash::New();
    let  	mut err = None;
    USeg::FromLen( count).Span( |_| {
        let  	lineOpt = lines.next();
        if lineOpt.is_none()
        {
            err = Some( "Missing AND gate specification".to_string());
            return false;
        }
        let  	line = lineOpt.unwrap();
        let  	parts: Vec< &str> = line.split_whitespace().collect();
        if parts.len() < 3
        {
            err = Some( "Malformed AND gate line".to_string());
            return false;
        }
        let  	lhsRes = parts[0].parse::< u32>();
        let  	rhs0Res = parts[1].parse::< u32>();
        let  	rhs1Res = parts[2].parse::< u32>();
        if lhsRes.is_err() || rhs0Res.is_err() || rhs1Res.is_err()
        {
            err = Some( "Malformed AND gate literals".to_string());
            return false;
        }
        ands.Push( AigGate::New( lhsRes.unwrap(), rhs0Res.unwrap(), rhs1Res.unwrap()));
        true
    });
    if let Some( e) = err
    {
        return Err( e);
    }
    Ok( ands)
}

//-------------------------------------------------------------------------------------------------

fn	ParseSymbolsAndComments( lineVec: &[&str]) -> ( Stash< AigSymbol>, Stash< String>)
{
    let  	mut symbols = Stash::New();
    let  	mut comments = Stash::New();
    let  	mut inComments = false;

    USeg::FromLen( lineVec.len() as u32).Traverse( |i| {
        let  	line = lineVec[i as usize];
        let  	trimmed = line.trim();
        if inComments
        {
            comments.Push( trimmed.to_string());
            return;
        }
        if trimmed == "c"
        {
            inComments = true;
            return;
        }
        if let Some( c) = trimmed.chars().next()
            && (c == 'i' || c == 'l' || c == 'o')
        {
            let  	rest = &trimmed[1..];
            if let Some( ( idxStr, name)) = rest.split_once( ' ')
                && let Ok( idx) = idxStr.parse::< u32>()
            {
                symbols.Push( AigSymbol::New( c, idx, name.to_string()));
            }
        }
    });
    ( symbols, comments)
}

//-------------------------------------------------------------------------------------------------

fn	ParseLineUntilNewline( bytes: &[u8], pos: &mut usize) -> Result< String, String>
{
    let  	start = *pos;
    while *pos < bytes.len() && bytes[*pos] != b'\n'
    {
        *pos += 1;
    }
    if *pos >= bytes.len() && start == *pos
    {
        return Err( "Unexpected EOF reading line".to_string());
    }
    let  	s = std::str::from_utf8( &bytes[start..*pos])
        .map_err( |e| format!( "{}", e))?
        .trim_end_matches( '\r')
        .to_string();
    if *pos < bytes.len() && bytes[*pos] == b'\n'
    {
        *pos += 1;
    }
    Ok( s)
}

//-------------------------------------------------------------------------------------------------

impl Aig
{
    /// Serializes the AIG to ASCII `.aag` format.
    pub fn	ToAscii( &self) -> String
    {
        let  	mut out = String::new();
        out.push_str( &format!(
            "aag {} {} {} {} {}\n",
            self._MaxVar,
            self.InputCount(),
            self.LatchCount(),
            self.OutputCount(),
            self.AndCount()
        ));
        self._Inputs.Arr().Traverse( |&lit| {
            out.push_str( &format!( "{}\n", lit));
        });
        self._Latches.Arr().Traverse( |latch| {
            if latch.Reset() != 0
            {
                out.push_str( &format!( "{} {} {}\n", latch.Lit(), latch.Next(), latch.Reset()));
            }
            else
            {
                out.push_str( &format!( "{} {}\n", latch.Lit(), latch.Next()));
            }
        });
        self._Outputs.Arr().Traverse( |&lit| {
            out.push_str( &format!( "{}\n", lit));
        });
        self._Ands.Arr().Traverse( |gate| {
            out.push_str( &format!( "{} {} {}\n", gate.Lhs(), gate.Rhs0(), gate.Rhs1()));
        });
        self._Symbols.Arr().Traverse( |sym| {
            out.push_str( &format!( "{}{} {}\n", sym.Kind(), sym.Index(), sym.Name()));
        });
        if self._Comments.Size() > 0
        {
            out.push_str( "c\n");
            self._Comments.Arr().Traverse( |c| {
                out.push_str( &format!( "{}\n", c));
            });
        }
        out
    }

    /// Serializes the AIG to binary `.aig` format.
    pub fn	ToBinary( &self) -> Buff< u8>
    {
        let  	mut bytes = Stash::New();
        let  	header = format!(
            "aig {} {} {} {} {}\n",
            self._MaxVar,
            self.InputCount(),
            self.LatchCount(),
            self.OutputCount(),
            self.AndCount()
        );
        let  	hdrBytes = header.as_bytes();
        USeg::FromLen( hdrBytes.len() as u32).Traverse( |i| bytes.Push( hdrBytes[i as usize]));

        self._Latches.Arr().Traverse( |latch| {
            let  	line = if latch.Reset() != 0
            {
                format!( "{} {}\n", latch.Next(), latch.Reset())
            }
            else
            {
                format!( "{}\n", latch.Next())
            };
            let  	b = line.as_bytes();
            USeg::FromLen( b.len() as u32).Traverse( |i| bytes.Push( b[i as usize]));
        });
        self._Outputs.Arr().Traverse( |&lit| {
            let  	line = format!( "{}\n", lit);
            let  	b = line.as_bytes();
            USeg::FromLen( b.len() as u32).Traverse( |i| bytes.Push( b[i as usize]));
        });
        self._Ands.Arr().Traverse( |gate| {
            let  	delta0 = gate.Lhs() - gate.Rhs0();
            let  	delta1 = gate.Rhs0() - gate.Rhs1();
            EncodeLeb128( delta0, &mut bytes);
            EncodeLeb128( delta1, &mut bytes);
        });
        self._Symbols.Arr().Traverse( |sym| {
            let  	line = format!( "{}{} {}\n", sym.Kind(), sym.Index(), sym.Name());
            let  	b = line.as_bytes();
            USeg::FromLen( b.len() as u32).Traverse( |i| bytes.Push( b[i as usize]));
        });
        if self._Comments.Size() > 0
        {
            bytes.Push( b'c');
            bytes.Push( b'\n');
            self._Comments.Arr().Traverse( |c| {
                let  	line = format!( "{}\n", c);
                let  	b = line.as_bytes();
                USeg::FromLen( b.len() as u32).Traverse( |i| bytes.Push( b[i as usize]));
            });
        }
        bytes.IntoBuff()
    }

    /// Parses an ASCII `.aag` formatted string into an AIG structure.
    pub fn	FromAscii( input: &str) -> Result< Self, String>
    {
        let  	allLines: Vec< &str> = input.lines().filter( |l| !l.trim().is_empty()).collect();
        if allLines.is_empty()
        {
            return Err( "Empty AIGER input".to_string());
        }
        let  	( maxVar, numInputs, numLatches, numOutputs, numAnds) = ParseHeader( allLines[0])?;

        let  	mut lineIdx = 1usize;
        let  	mut inputs = Stash::New();
        let  	mut err = None;

        USeg::FromLen( numInputs).Span( |_| {
            if lineIdx >= allLines.len()
            {
                err = Some( "Missing input literal".to_string());
                return false;
            }
            let  	line = allLines[lineIdx];
            lineIdx += 1;
            match line.trim().parse::< u32>()
            {
                Ok( lit) => {
                    inputs.Push( lit);
                    true
                }
                Err( e) => {
                    err = Some( format!( "{}", e));
                    false
                }
            }
        });
        if let Some( e) = err
        {
            return Err( e);
        }

        let  	mut remIter = allLines[lineIdx..].iter().copied();
        let  	latches = ParseLatches( &mut remIter, numLatches)?;
        lineIdx += numLatches as usize;

        let  	mut outputs = Stash::New();
        USeg::FromLen( numOutputs).Span( |_| {
            if lineIdx >= allLines.len()
            {
                err = Some( "Missing output literal".to_string());
                return false;
            }
            let  	line = allLines[lineIdx];
            lineIdx += 1;
            match line.trim().parse::< u32>()
            {
                Ok( lit) => {
                    outputs.Push( lit);
                    true
                }
                Err( e) => {
                    err = Some( format!( "{}", e));
                    false
                }
            }
        });
        if let Some( e) = err
        {
            return Err( e);
        }

        let  	mut remIter2 = allLines[lineIdx..].iter().copied();
        let  	ands = ParseAnds( &mut remIter2, numAnds)?;
        lineIdx += numAnds as usize;

        let  	( symbols, comments) = if lineIdx < allLines.len()
        {
            ParseSymbolsAndComments( &allLines[lineIdx..])
        }
        else
        {
            ( Stash::New(), Stash::New())
        };

        Ok( Aig::New(
            maxVar,
            inputs.IntoBuff(),
            latches.IntoBuff(),
            outputs.IntoBuff(),
            ands.IntoBuff(),
            symbols.IntoBuff(),
            comments.IntoBuff(),
        ))
    }

    /// Parses a binary `.aig` formatted byte slice into an AIG structure.
    pub fn	FromBinary( bytes: &[u8]) -> Result< Self, String>
    {
        let  	mut pos = 0usize;
        let  	headerLine = ParseLineUntilNewline( bytes, &mut pos)?;
        let  	parts: Vec< &str> = headerLine.split_whitespace().collect();
        if parts.len() < 6 || parts[0] != "aig"
        {
            return Err( "Invalid binary AIGER header (expected 'aig M I L O A')".to_string());
        }
        let  	maxVar: u32 = parts[1].parse().map_err( |e| format!( "{}", e))?;
        let  	numInputs: u32 = parts[2].parse().map_err( |e| format!( "{}", e))?;
        let  	numLatches: u32 = parts[3].parse().map_err( |e| format!( "{}", e))?;
        let  	numOutputs: u32 = parts[4].parse().map_err( |e| format!( "{}", e))?;
        let  	numAnds: u32 = parts[5].parse().map_err( |e| format!( "{}", e))?;

        let  	mut inputs = Stash::New();
        USeg::FromLen( numInputs).Traverse( |idx| {
            inputs.Push( ( idx + 1) * 2);
        });

        let  	mut latches = Stash::New();
        let  	mut err = None;
        USeg::FromLen( numLatches).Span( |idx| {
            let  	lit = 2 * ( numInputs + idx + 1);
            let  	lineRes = ParseLineUntilNewline( bytes, &mut pos);
            if lineRes.is_err()
            {
                err = Some( lineRes.err().unwrap());
                return false;
            }
            let  	line = lineRes.unwrap();
            let  	lParts: Vec< &str> = line.split_whitespace().collect();
            if lParts.is_empty()
            {
                err = Some( "Missing next literal in latch".to_string());
                return false;
            }
            let  	next = match lParts[0].parse::< u32>()
            {
                Ok( n) => n,
                Err( e) => {
                    err = Some( format!( "{}", e));
                    return false;
                }
            };
            let  	reset = if lParts.len() > 1 { lParts[1].parse::< u32>().unwrap_or( 0) } else { 0 };
            latches.Push( AigLatch::New( lit, next, reset, String::new()));
            true
        });
        if let Some( e) = err
        {
            return Err( e);
        }

        let  	mut outputs = Stash::New();
        USeg::FromLen( numOutputs).Span( |_| {
            let  	lineRes = ParseLineUntilNewline( bytes, &mut pos);
            if lineRes.is_err()
            {
                err = Some( lineRes.err().unwrap());
                return false;
            }
            let  	line = lineRes.unwrap();
            match line.trim().parse::< u32>()
            {
                Ok( lit) => {
                    outputs.Push( lit);
                    true
                }
                Err( e) => {
                    err = Some( format!( "{}", e));
                    false
                }
            }
        });
        if let Some( e) = err
        {
            return Err( e);
        }

        let  	mut ands = Stash::New();
        USeg::FromLen( numAnds).Span( |idx| {
            let  	lhs = 2 * ( numInputs + numLatches + 1 + idx);
            let  	delta0 = match DecodeLeb128( bytes, &mut pos)
            {
                Ok( d) => d,
                Err( e) => {
                    err = Some( e);
                    return false;
                }
            };
            let  	rhs0 = lhs - delta0;
            let  	delta1 = match DecodeLeb128( bytes, &mut pos)
            {
                Ok( d) => d,
                Err( e) => {
                    err = Some( e);
                    return false;
                }
            };
            let  	rhs1 = rhs0 - delta1;
            ands.Push( AigGate::New( lhs, rhs0, rhs1));
            true
        });
        if let Some( e) = err
        {
            return Err( e);
        }

        let  	( symbols, comments) = if pos < bytes.len()
        {
            let  	remaining = std::str::from_utf8( &bytes[pos..]).unwrap_or( "");
            let  	remLines: Vec< &str> = remaining.lines().collect();
            ParseSymbolsAndComments( &remLines)
        }
        else
        {
            ( Stash::New(), Stash::New())
        };

        Ok( Aig::New(
            maxVar,
            inputs.IntoBuff(),
            latches.IntoBuff(),
            outputs.IntoBuff(),
            ands.IntoBuff(),
            symbols.IntoBuff(),
            comments.IntoBuff(),
        ))
    }

    /// Lowers a digital circuit from a `Layout` module hierarchy into an AIG.
    pub fn	FromLayout( layout: &Layout, topMod: ModuleId) -> Result< Self, String>
    {
        let  	mut builder = AigBuilder::New();
        let  	topModule = layout.Module( topMod);
        let  	inCount = topModule.InPorts().Size();
        let  	mut portLits: Vec< ( PortId, u32)> = Vec::new();

        USeg::FromLen( inCount).Traverse( |i| {
            let  	portId = layout.InPort( topMod, i);
            let  	portName = layout.LocalPortName( portId);
            let  	lit = builder.AddInput( portName);
            portLits.push( ( portId, lit));
        });

        LowerModuleContents( layout, topMod, topMod, &mut builder, &mut portLits)?;

        let  	outCount = topModule.OutPorts().Size();
        USeg::FromLen( outCount).Span( |i| {
            let  	outPort = layout.OutPort( topMod, i);
            let  	outName = layout.LocalPortName( outPort);
            let  	drv = layout.DriverOf( outPort);
            let  	litRes = if drv.IsValid()
            {
                FindOrResolvePortLit( layout, topMod, drv, &mut builder, &mut portLits)
            }
            else
            {
                Ok( 0)
            };
            match litRes
            {
                Ok( lit) => {
                    builder.AddOutput( lit, outName);
                    true
                }
                Err( e) => {
                    eprintln!( "Error resolving port: {}", e);
                    false
                }
            }
        });

        builder.AddComment( &format!( "Lowered from Trellis Layout module '{}'", layout.LocalName( topMod)));
        Ok( builder.Build())
    }
}

//-------------------------------------------------------------------------------------------------

fn	FindOrResolvePortLit(
    layout: &Layout,
    topMod: ModuleId,
    port: PortId,
    builder: &mut AigBuilder,
    portLits: &mut Vec< ( PortId, u32)>,
) -> Result< u32, String>
{
    let  	mut found = None;
    USeg::FromLen( portLits.len() as u32).Traverse( |i| {
        let  	( p, lit) = portLits[i as usize];
        if p == port
        {
            found = Some( lit);
        }
    });
    if let Some( lit) = found
    {
        return Ok( lit);
    }

    let  	owner = layout.Port( port).Owner();
    if owner == topMod
    {
        let  	drv = layout.DriverOf( port);
        if drv.IsValid() && drv != port
        {
            let  	lit = FindOrResolvePortLit( layout, topMod, drv, builder, portLits)?;
            portLits.push( ( port, lit));
            return Ok( lit);
        }
        return Ok( 0);
    }
    let  	ownerMod = layout.Module( owner);
    if let KernelKind::Fast( op) = ownerMod.Kernel()
    {
        let  	lit = LowerFastKernel( layout, topMod, owner, *op, builder, portLits)?;
        portLits.push( ( port, lit));
        return Ok( lit);
    }
    let  	drv = layout.DriverOf( port);
    if drv.IsValid() && drv != port
    {
        let  	lit = FindOrResolvePortLit( layout, topMod, drv, builder, portLits)?;
        portLits.push( ( port, lit));
        return Ok( lit);
    }
    Ok( 0)
}

//-------------------------------------------------------------------------------------------------

fn	LowerFastKernel(
    layout: &Layout,
    topMod: ModuleId,
    gateMod: ModuleId,
    op: KernelOp,
    builder: &mut AigBuilder,
    portLits: &mut Vec< ( PortId, u32)>,
) -> Result< u32, String>
{
    let  	in0Port = layout.InPort( gateMod, 0);
    let  	drv0 = layout.DriverOf( in0Port);
    let  	lit0 = if drv0.IsValid()
    {
        FindOrResolvePortLit( layout, topMod, drv0, builder, portLits)?
    }
    else
    {
        0
    };
    let  	lit1 = if op == KernelOp::Not
    {
        0
    }
    else
    {
        let  	in1Port = layout.InPort( gateMod, 1);
        let  	drv1 = layout.DriverOf( in1Port);
        if drv1.IsValid()
        {
            FindOrResolvePortLit( layout, topMod, drv1, builder, portLits)?
        }
        else
        {
            0
        }
    };
    let  	resultLit = match op
    {
        KernelOp::And => builder.AddAnd( lit0, lit1),
        KernelOp::Nand => builder.AddNand( lit0, lit1),
        KernelOp::Or => builder.AddOr( lit0, lit1),
        KernelOp::Nor => builder.AddNor( lit0, lit1),
        KernelOp::Xor => builder.AddXor( lit0, lit1),
        KernelOp::Xnor => builder.AddXnor( lit0, lit1),
        KernelOp::Not => builder.AddNot( lit0),
        _ => return Err( format!( "Unsupported gate operation {:?} in AIGER lowering", op)),
    };
    Ok( resultLit)
}

//-------------------------------------------------------------------------------------------------

fn	LowerModuleContents(
    layout: &Layout,
    topMod: ModuleId,
    parentMod: ModuleId,
    builder: &mut AigBuilder,
    portLits: &mut Vec< ( PortId, u32)>,
) -> Result< (), String>
{
    let  	children = layout.Children( parentMod);
    let  	count = children.Size();
    USeg::FromLen( count).Span( |i| {
        let  	childId = children[i];
        let  	child = layout.Module( childId);
        if let KernelKind::Fast( op) = child.Kernel()
        {
            let  	outPort = layout.OutPort( childId, 0);
            let  	mut exists = false;
            USeg::FromLen( portLits.len() as u32).Traverse( |k| {
                if portLits[k as usize].0 == outPort
                {
                    exists = true;
                }
            });
            if !exists
            {
                match LowerFastKernel( layout, topMod, childId, *op, builder, portLits)
                {
                    Ok( lit) => {
                        portLits.push( ( outPort, lit));
                    }
                    Err( e) => {
                        eprintln!( "Error lowering fast kernel: {}", e);
                        return false;
                    }
                }
            }
        }
        else
        {
            if let Err( e) = LowerModuleContents( layout, topMod, childId, builder, portLits)
            {
                eprintln!( "Error lowering submodule: {}", e);
                return false;
            }
            let  	childOutCount = child.OutPorts().Size();
            USeg::FromLen( childOutCount).Traverse( |o| {
                let  	outPort = layout.OutPort( childId, o);
                let  	drv = layout.DriverOf( outPort);
                if drv.IsValid()
                    && let Ok( lit) = FindOrResolvePortLit( layout, topMod, drv, builder, portLits)
                {
                    portLits.push( ( outPort, lit));
                }
            });
        }
        true
    });
    Ok( ())
}

//-------------------------------------------------------------------------------------------------

/// Parses an AIGER model from an ASCII or binary string slice.
pub fn	ParseAiger( input: &str) -> Result< Aig, String>
{
    if input.starts_with( "aig ")
    {
        Aig::FromBinary( input.as_bytes())
    }
    else
    {
        Aig::FromAscii( input)
    }
}

/// Parses an AIGER model from a raw byte slice (supporting both binary and ASCII).
pub fn	ParseAigerBinary( bytes: &[u8]) -> Result< Aig, String>
{
    if bytes.starts_with( b"aig " )
    {
        Aig::FromBinary( bytes)
    }
    else if bytes.starts_with( b"aag " )
    {
        let  	s = std::str::from_utf8( bytes).map_err( |e| format!( "{}", e))?;
        Aig::FromAscii( s)
    }
    else
    {
        Err( "Unrecognized AIGER format header (expected 'aig' or 'aag')".to_string())
    }
}

//-------------------------------------------------------------------------------------------------

/// Unites two combinational AIG models into a single miter AIG.
///
/// The miter evaluates both circuits with the same primary inputs and computes:
/// `miter_out = OR_k ( out1_k XOR out2_k )`
/// If both circuits are functionally equivalent, `miter_out == 0` for all inputs.
pub fn	CreateMiter( aig1: &Aig, aig2: &Aig) -> Result< Aig, String>
{
    if aig1.InputCount() != aig2.InputCount()
    {
        return Err( format!(
            "Miter input count mismatch: {} vs {}",
            aig1.InputCount(),
            aig2.InputCount()
        ));
    }
    if aig1.OutputCount() != aig2.OutputCount()
    {
        return Err( format!(
            "Miter output count mismatch: {} vs {}",
            aig1.OutputCount(),
            aig2.OutputCount()
        ));
    }

    let  	numInputs = aig1.InputCount();
    let  	numOutputs = aig1.OutputCount();
    let  	mut builder = AigBuilder::New();

    // 1. Create shared inputs
    USeg::FromLen( numInputs).Traverse( |i| {
        let  	mut symName = String::new();
        aig1.Symbols().Traverse( |s| {
            if s.Kind() == 'i' && s.Index() == i
            {
                symName = s.Name().to_string();
            }
        });
        builder.AddInput( &symName);
    });

    // 2. Map gates of aig1
    // Lit mapping: 0 -> 0, 1 -> 1, 2*i -> 2*i for inputs.
    let  	maxVar1 = aig1.MaxVar() as usize;
    let  	mut map1 = vec![0u32; maxVar1 + 1];
    USeg::FromLen( numInputs).Traverse( |i| {
        let  	v = ( i + 1) as usize;
        map1[v] = ( v as u32) * 2;
    });

    let  	remap1 = |lit: u32, map: &[u32]| -> u32 {
        let  	v = ( lit / 2) as usize;
        let  	inv = ( lit & 1) != 0;
        let  	mappedVarLit = if v < map.len() { map[v] } else { 0 };
        if inv { mappedVarLit ^ 1 } else { mappedVarLit }
    };

    aig1.Ands().Traverse( |gate| {
        let  	v = ( gate.Lhs() / 2) as usize;
        let  	r0 = remap1( gate.Rhs0(), &map1);
        let  	r1 = remap1( gate.Rhs1(), &map1);
        let  	newLhs = builder.AddAnd( r0, r1);
        if v < map1.len()
        {
            map1[v] = newLhs;
        }
    });

    // 3. Map gates of aig2
    let  	maxVar2 = aig2.MaxVar() as usize;
    let  	mut map2 = vec![0u32; maxVar2 + 1];
    USeg::FromLen( numInputs).Traverse( |i| {
        let  	v = ( i + 1) as usize;
        map2[v] = ( v as u32) * 2;
    });

    let  	remap2 = |lit: u32, map: &[u32]| -> u32 {
        let  	v = ( lit / 2) as usize;
        let  	inv = ( lit & 1) != 0;
        let  	mappedVarLit = if v < map.len() { map[v] } else { 0 };
        if inv { mappedVarLit ^ 1 } else { mappedVarLit }
    };

    aig2.Ands().Traverse( |gate| {
        let  	v = ( gate.Lhs() / 2) as usize;
        let  	r0 = remap2( gate.Rhs0(), &map2);
        let  	r1 = remap2( gate.Rhs1(), &map2);
        let  	newLhs = builder.AddAnd( r0, r1);
        if v < map2.len()
        {
            map2[v] = newLhs;
        }
    });

    // 4. XOR corresponding outputs and OR-reduce them
    let  	mut miterOut = 0u32;
    USeg::FromLen( numOutputs).Traverse( |o| {
        let  	out1Lit = aig1.Outputs()[o];
        let  	out2Lit = aig2.Outputs()[o];
        let  	m1 = remap1( out1Lit, &map1);
        let  	m2 = remap2( out2Lit, &map2);
        let  	diff = builder.AddXor( m1, m2);
        miterOut = builder.AddOr( miterOut, diff);
    });

    builder.AddOutput( miterOut, "miter_out");
    builder.AddComment( "Equivalence checking miter" );
    Ok( builder.Build())
}

//-------------------------------------------------------------------------------------------------

/// Result of Combinational Equivalence Checking (CEC).
#[derive( Clone, Debug, PartialEq, Eq)]
pub enum CecResult
{
    /// Both circuits compute identical Boolean functions across all inputs.
    Equivalent,
    /// Circuits differ: contains the input vector (one bool per primary input) triggering discrepancy.
    Counterexample( Buff< bool>),
}

//-------------------------------------------------------------------------------------------------

/// Evaluates an AIG model on a specific input assignment.
/// Returns the Boolean values of all primary outputs.
pub fn	EvaluateAig( aig: &Aig, inputValues: &[bool]) -> Buff< bool>
{
    let  	maxVar = aig.MaxVar() as usize;
    let  	mut values = vec![false; maxVar + 1];
    values[0] = false;

    let  	inCount = aig.InputCount() as usize;
    let  	given = inputValues.len().min( inCount);
    USeg::FromLen( given as u32).Traverse( |i| {
        values[i as usize + 1] = inputValues[i as usize];
    });

    let  	evalLit = |lit: u32, vals: &[bool]| -> bool {
        let  	v = ( lit / 2) as usize;
        let  	inv = ( lit & 1) != 0;
        let  	val = if v < vals.len() { vals[v] } else { false };
        if inv { !val } else { val }
    };

    aig.Ands().Traverse( |gate| {
        let  	v = ( gate.Lhs() / 2) as usize;
        let  	b0 = evalLit( gate.Rhs0(), &values);
        let  	b1 = evalLit( gate.Rhs1(), &values);
        if v < values.len()
        {
            values[v] = b0 && b1;
        }
    });

    let  	mut outBuff = Stash::WithCapacity( aig.OutputCount());
    aig.Outputs().Traverse( |&outLit| {
        outBuff.Push( evalLit( outLit, &values));
    });
    outBuff.IntoBuff()
}

//-------------------------------------------------------------------------------------------------

/// Simulates the AIG using 64-bit parallel words for high-throughput random testing.
fn	EvaluateAig64( aig: &Aig, inputWords: &[u64]) -> u64
{
    let  	maxVar = aig.MaxVar() as usize;
    let  	mut values = vec![0u64; maxVar + 1];
    values[0] = 0;

    let  	inCount = aig.InputCount() as usize;
    let  	given = inputWords.len().min( inCount);
    USeg::FromLen( given as u32).Traverse( |i| {
        values[i as usize + 1] = inputWords[i as usize];
    });

    let  	evalLit = |lit: u32, vals: &[u64]| -> u64 {
        let  	v = ( lit / 2) as usize;
        let  	inv = ( lit & 1) != 0;
        let  	w = if v < vals.len() { vals[v] } else { 0 };
        if inv { !w } else { w }
    };

    aig.Ands().Traverse( |gate| {
        let  	v = ( gate.Lhs() / 2) as usize;
        let  	w0 = evalLit( gate.Rhs0(), &values);
        let  	w1 = evalLit( gate.Rhs1(), &values);
        if v < values.len()
        {
            values[v] = w0 & w1;
        }
    });

    if aig.OutputCount() == 0
    {
        0
    }
    else
    {
        evalLit( aig.Outputs()[0], &values)
    }
}

//-------------------------------------------------------------------------------------------------

/// Fast PRNG for generating random simulation patterns.
fn	LcgNext( state: &mut u64) -> u64
{
    *state = state.wrapping_mul( 6364136223846793005).wrapping_add( 1442695040888963407);
    *state
}

//-------------------------------------------------------------------------------------------------

/// A compact, recursive DPLL SAT solver for formal AIGER equivalence resolution.
struct Dpll
{
    _NumVars:    usize,
    _Clauses:    Vec< Vec< i32>>,
    _Assignment: Vec< i8>, // 0: unassigned, 1: true, -1: false
}

impl Dpll
{
    fn	New( numVars: usize, clauses: Vec< Vec< i32>>) -> Self
    {
        Self {
            _NumVars:    numVars,
            _Clauses:    clauses,
            _Assignment: vec![0; numVars + 1],
        }
    }

    fn	LitVal( &self, lit: i32) -> i8
    {
        let  	v = lit.unsigned_abs() as usize;
        let  	a = self._Assignment[v];
        if lit > 0 { a } else { -a }
    }

    fn	UnitPropagate( &mut self) -> bool
    {
        let  	mut changed = true;
        while changed
        {
            changed = false;
            let  	numClauses = self._Clauses.len();
            for cIdx in 0..numClauses
            {
                let  	clause = &self._Clauses[cIdx];
                let  	mut unassignedLit = 0i32;
                let  	mut clauseSatisfied = false;
                let  	mut allOthersFalse = true;

                for &lit in clause
                {
                    let  	val = self.LitVal( lit);
                    if val == 1
                    {
                        clauseSatisfied = true;
                        break;
                    }
                    else if val == 0
                    {
                        if unassignedLit == 0
                        {
                            unassignedLit = lit;
                        }
                        else
                        {
                            allOthersFalse = false;
                        }
                    }
                }

                if clauseSatisfied
                {
                    continue;
                }
                if unassignedLit == 0
                {
                    // Conflict: clause is entirely false
                    return false;
                }
                if allOthersFalse
                {
                    // Unit clause found!
                    let  	v = unassignedLit.unsigned_abs() as usize;
                    let  	newVal = if unassignedLit > 0 { 1 } else { -1 };
                    self._Assignment[v] = newVal;
                    changed = true;
                }
            }
        }
        true
    }

    fn	Solve( &mut self) -> bool
    {
        let  	saved = self._Assignment.clone();
        if !self.UnitPropagate()
        {
            self._Assignment = saved;
            return false;
        }

        // Find first unassigned variable
        let  	mut nextVar = 0usize;
        for v in 1..=self._NumVars
        {
            if self._Assignment[v] == 0
            {
                nextVar = v;
                break;
            }
        }

        if nextVar == 0
        {
            // All variables assigned and no conflicts!
            return true;
        }

        // Try true
        self._Assignment[nextVar] = 1;
        if self.Solve()
        {
            return true;
        }

        // Try false
        self._Assignment = saved;
        self._Assignment[nextVar] = -1;
        self.Solve()
    }
}

//-------------------------------------------------------------------------------------------------

/// Checks whether two combinational AIG models are functionally equivalent.
///
/// Uses high-throughput 64-bit random simulation filtering for instant counterexample rejection,
/// followed by DPLL SAT resolution for complete formal verification.
pub fn	CheckEquivalence( aig1: &Aig, aig2: &Aig) -> Result< CecResult, String>
{
    let  	miter = CreateMiter( aig1, aig2)?;
    let  	numInputs = miter.InputCount() as usize;
    let  	miterOutLit = miter.Outputs()[0];

    // Constant 0 output -> provably equivalent
    if miterOutLit == 0
    {
        return Ok( CecResult::Equivalent);
    }
    // Constant 1 output -> provably counterexample on all inputs
    if miterOutLit == 1
    {
        let  	zeros = Stash::WithCapacity( numInputs as u32);
        return Ok( CecResult::Counterexample( zeros.IntoBuff()));
    }

    // Step 1: 64-bit Random simulation filtering (16 waves of 64 vectors = 1024 tests)
    let  	mut rng = 0x123456789ABCDEF0u64;
    let  	waves = 16u32;
    for _ in 0..waves
    {
        let  	mut inputWords = vec![0u64; numInputs];
        USeg::FromLen( numInputs as u32).Traverse( |i| {
            inputWords[i as usize] = LcgNext( &mut rng);
        });

        let  	outWord = EvaluateAig64( &miter, &inputWords);
        if outWord != 0
        {
            // Find first failing bit
            let  	bitIdx = outWord.trailing_zeros();
            let  	mut ceInput = Stash::WithCapacity( numInputs as u32);
            USeg::FromLen( numInputs as u32).Traverse( |i| {
                let  	bitVal = ( ( inputWords[i as usize] >> bitIdx) & 1) != 0;
                ceInput.Push( bitVal);
            });
            return Ok( CecResult::Counterexample( ceInput.IntoBuff()));
        }
    }

    // Step 2: Exhaustive simulation for small circuits (<= 16 inputs)
    if numInputs <= 16
    {
        let  	totalPatterns = 1u64 << numInputs;
        for p in 0..totalPatterns
        {
            let  	mut inputVals = vec![false; numInputs];
            USeg::FromLen( numInputs as u32).Traverse( |i| {
                inputVals[i as usize] = ( ( p >> i) & 1) != 0;
            });
            let  	res = EvaluateAig( &miter, &inputVals);
            if res[0]
            {
                let  	mut ceInput = Stash::WithCapacity( numInputs as u32);
                USeg::FromLen( numInputs as u32).Traverse( |i| {
                    ceInput.Push( inputVals[i as usize]);
                });
                return Ok( CecResult::Counterexample( ceInput.IntoBuff()));
            }
        }
        return Ok( CecResult::Equivalent);
    }

    // Step 3: Formal Tseitin CNF encoding + DPLL SAT solver for general circuits
    let  	maxVar = miter.MaxVar() as usize;
    let  	mut clauses = Vec::new();

    let  	toCnfLit = |lit: u32| -> i32 {
        let  	v = ( lit / 2) as i32;
        if ( lit & 1) != 0 { -v } else { v }
    };

    // Encode each AND gate z = x & y:
    // (not x or not y or z) & (x or not z) & (y or not z)
    miter.Ands().Traverse( |gate| {
        let  	z = toCnfLit( gate.Lhs());
        let  	x = toCnfLit( gate.Rhs0());
        let  	y = toCnfLit( gate.Rhs1());

        clauses.push( vec![-x, -y, z]);
        clauses.push( vec![x, -z]);
        clauses.push( vec![y, -z]);
    });

    // Assert miter output == 1
    clauses.push( vec![toCnfLit( miterOutLit)]);

    let  	mut solver = Dpll::New( maxVar, clauses);
    if solver.Solve()
    {
        // SAT: counterexample found!
        let  	mut ceInput = Stash::WithCapacity( numInputs as u32);
        USeg::FromLen( numInputs as u32).Traverse( |i| {
            let  	v = ( i + 1) as usize;
            let  	val = solver._Assignment[v] > 0;
            ceInput.Push( val);
        });
        Ok( CecResult::Counterexample( ceInput.IntoBuff()))
    }
    else
    {
        // UNSAT: circuits are provably equivalent!
        Ok( CecResult::Equivalent)
    }
}
