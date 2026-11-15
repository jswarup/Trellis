// aiger.rs ---------------------------------------------------------------------------------------------------
use crate::rube::layout::Layout;
use crate::rube::module::{KernelKind, KernelOp};
use crate::rube::port::{ModuleId, PortId};
use crate::silo::{Buff, IArr, Stash};

//-------------------------------------------------------------------------------------------------

/// Represents a 2-input AND gate in an AND-Inverter Graph: `lhs = rhs0 & rhs1`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct AigGate
{
    _Lhs:  u32,
    _Rhs0: u32,
    _Rhs1: u32,
}

impl AigGate
{
    pub const fn New(lhs: u32, rhs0: u32, rhs1: u32) -> Self
    {
        Self { _Lhs:  lhs,
               _Rhs0: rhs0,
               _Rhs1: rhs1, }
    }

    #[inline]
    pub const fn Lhs(&self) -> u32 { self._Lhs }

    #[inline]
    pub const fn Rhs0(&self) -> u32 { self._Rhs0 }

    #[inline]
    pub const fn Rhs1(&self) -> u32 { self._Rhs1 }
}

//-------------------------------------------------------------------------------------------------

/// Represents a sequential latch in AIGER with next-state literal and reset value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AigLatch
{
    _Lit:   u32,
    _Next:  u32,
    _Reset: u32,
    _Name:  String,
}

impl AigLatch
{
    pub fn New(lit: u32, next: u32, reset: u32, name: String) -> Self
    {
        Self { _Lit:   lit,
               _Next:  next,
               _Reset: reset,
               _Name:  name, }
    }

    #[inline]
    pub const fn Lit(&self) -> u32 { self._Lit }

    #[inline]
    pub const fn Next(&self) -> u32 { self._Next }

    #[inline]
    pub const fn Reset(&self) -> u32 { self._Reset }

    #[inline]
    pub fn Name(&self) -> &str { &self._Name }
}

//-------------------------------------------------------------------------------------------------

/// Symbol table entry mapping an index to a descriptive signal name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AigSymbol
{
    _Kind:  char,
    _Index: u32,
    _Name:  String,
}

impl AigSymbol
{
    pub fn New(kind: char, index: u32, name: String) -> Self
    {
        Self { _Kind:  kind,
               _Index: index,
               _Name:  name, }
    }

    #[inline]
    pub const fn Kind(&self) -> char { self._Kind }

    #[inline]
    pub const fn Index(&self) -> u32 { self._Index }

    #[inline]
    pub fn Name(&self) -> &str { &self._Name }
}

//-------------------------------------------------------------------------------------------------

/// Complete AND-Inverter Graph (AIG) formal model.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Aig
{
    _MaxVar:   u32,
    _Inputs:   Buff<u32>,
    _Latches:  Buff<AigLatch>,
    _Outputs:  Buff<u32>,
    _Ands:     Buff<AigGate>,
    _Symbols:  Buff<AigSymbol>,
    _Comments: Buff<String>,
}

impl Aig
{
    pub fn New(maxVar: u32,
               inputs: Buff<u32>,
               latches: Buff<AigLatch>,
               outputs: Buff<u32>,
               ands: Buff<AigGate>,
               symbols: Buff<AigSymbol>,
               comments: Buff<String>)
               -> Self
    {
        Self { _MaxVar:   maxVar,
               _Inputs:   inputs,
               _Latches:  latches,
               _Outputs:  outputs,
               _Ands:     ands,
               _Symbols:  symbols,
               _Comments: comments, }
    }

    #[inline]
    pub const fn MaxVar(&self) -> u32 { self._MaxVar }

    #[inline]
    pub const fn InputCount(&self) -> u32 { self._Inputs.Size() }

    #[inline]
    pub const fn LatchCount(&self) -> u32 { self._Latches.Size() }

    #[inline]
    pub const fn OutputCount(&self) -> u32 { self._Outputs.Size() }

    #[inline]
    pub const fn AndCount(&self) -> u32 { self._Ands.Size() }

    #[inline]
    pub fn Inputs(&self) -> &Buff<u32> { &self._Inputs }

    #[inline]
    pub fn Latches(&self) -> &Buff<AigLatch> { &self._Latches }

    #[inline]
    pub fn Outputs(&self) -> &Buff<u32> { &self._Outputs }

    #[inline]
    pub fn Ands(&self) -> &Buff<AigGate> { &self._Ands }

    #[inline]
    pub fn Symbols(&self) -> &Buff<AigSymbol> { &self._Symbols }

    #[inline]
    pub fn Comments(&self) -> &Buff<String> { &self._Comments }
}

//-------------------------------------------------------------------------------------------------

/// Programmatic builder for creating normalized AND-Inverter Graphs with constant folding.
pub struct AigBuilder
{
    _MaxVar:   u32,
    _Inputs:   Stash<u32>,
    _Latches:  Stash<AigLatch>,
    _Outputs:  Stash<u32>,
    _Ands:     Stash<AigGate>,
    _Symbols:  Stash<AigSymbol>,
    _Comments: Stash<String>,
}

impl Default for AigBuilder
{
    fn default() -> Self { Self::New() }
}

impl AigBuilder
{
    pub fn New() -> Self
    {
        Self { _MaxVar:   0,
               _Inputs:   Stash::New(),
               _Latches:  Stash::New(),
               _Outputs:  Stash::New(),
               _Ands:     Stash::New(),
               _Symbols:  Stash::New(),
               _Comments: Stash::New(), }
    }

    pub fn AddInput(&mut self, name: &str) -> u32
    {
        self._MaxVar += 1;
        let lit = self._MaxVar * 2;
        let idx = self._Inputs.Size();
        self._Inputs.Push(lit);
        if !name.is_empty() {
            self._Symbols.Push(AigSymbol::New('i', idx, name.to_string()));
        }
        lit
    }

    pub fn AddLatch(&mut self, name: &str, nextLit: u32, reset: u32) -> u32
    {
        self._MaxVar += 1;
        let lit = self._MaxVar * 2;
        let idx = self._Latches.Size();
        self._Latches
            .Push(AigLatch::New(lit, nextLit, reset, name.to_string()));
        if !name.is_empty() {
            self._Symbols.Push(AigSymbol::New('l', idx, name.to_string()));
        }
        lit
    }

    pub fn AddOutput(&mut self, lit: u32, name: &str)
    {
        let idx = self._Outputs.Size();
        self._Outputs.Push(lit);
        if !name.is_empty() {
            self._Symbols.Push(AigSymbol::New('o', idx, name.to_string()));
        }
    }

    pub fn AddAnd(&mut self, lit1: u32, lit2: u32) -> u32
    {
        if lit1 == 0 || lit2 == 0 {
            return 0;
        }
        if lit1 == 1 {
            return lit2;
        }
        if lit2 == 1 {
            return lit1;
        }
        if lit1 == lit2 {
            return lit1;
        }
        if lit1 == (lit2 ^ 1) {
            return 0;
        }
        let (rhs0, rhs1) = if lit1 >= lit2 { (lit1, lit2) } else { (lit2, lit1) };
        let mut existing = 0;
        self._Ands.Arr().Traverse(|gate| {
            if gate.Rhs0() == rhs0 && gate.Rhs1() == rhs1 {
                existing = gate.Lhs();
            }
        });
        if existing != 0 {
            return existing;
        }
        self._MaxVar += 1;
        let lhs = self._MaxVar * 2;
        self._Ands.Push(AigGate::New(lhs, rhs0, rhs1));
        lhs
    }

    #[inline]
    pub fn AddNot(&mut self, lit: u32) -> u32 { lit ^ 1 }

    #[inline]
    pub fn AddNand(&mut self, lit1: u32, lit2: u32) -> u32
    {
        let a = self.AddAnd(lit1, lit2);
        self.AddNot(a)
    }

    pub fn AddOr(&mut self, lit1: u32, lit2: u32) -> u32
    {
        let not1 = self.AddNot(lit1);
        let not2 = self.AddNot(lit2);
        let nand = self.AddAnd(not1, not2);
        self.AddNot(nand)
    }

    #[inline]
    pub fn AddNor(&mut self, lit1: u32, lit2: u32) -> u32
    {
        let or_lit = self.AddOr(lit1, lit2);
        self.AddNot(or_lit)
    }

    pub fn AddXor(&mut self, lit1: u32, lit2: u32) -> u32
    {
        let or_term = self.AddOr(lit1, lit2);
        let nand_term = self.AddNand(lit1, lit2);
        self.AddAnd(or_term, nand_term)
    }

    #[inline]
    pub fn AddXnor(&mut self, lit1: u32, lit2: u32) -> u32
    {
        let xor_lit = self.AddXor(lit1, lit2);
        self.AddNot(xor_lit)
    }

    pub fn AddComment(&mut self, comment: &str)
    {
        self._Comments.Push(comment.to_string());
    }

    pub fn Build(self) -> Aig
    {
        Aig::New(self._MaxVar,
                 self._Inputs.IntoBuff(),
                 self._Latches.IntoBuff(),
                 self._Outputs.IntoBuff(),
                 self._Ands.IntoBuff(),
                 self._Symbols.IntoBuff(),
                 self._Comments.IntoBuff())
    }
}

//-------------------------------------------------------------------------------------------------

fn encode_leb128(mut val: u32, out: &mut Stash<u8>)
{
    while (val & !0x7F) != 0 {
        out.Push(((val & 0x7F) | 0x80) as u8);
        val >>= 7;
    }
    out.Push((val & 0x7F) as u8);
}

//-------------------------------------------------------------------------------------------------

fn decode_leb128(bytes: &[u8], pos: &mut usize) -> Result<u32, String>
{
    let mut result: u32 = 0;
    let mut shift: u32 = 0;
    while *pos < bytes.len() {
        let byte = bytes[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u32) << shift;
        if (byte & 0x80) == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift > 35 {
            return Err("LEB128 integer overflow".to_string());
        }
    }
    Err("Unexpected EOF in LEB128 sequence".to_string())
}

//-------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------

fn parse_header(line: &str) -> Result<(u32, u32, u32, u32, u32), String>
{
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 || parts[0] != "aag" {
        return Err("Invalid AIGER header (expected 'aag M I L O A')".to_string());
    }
    let m = parts[1].parse().map_err(|e| format!("{}", e))?;
    let i = parts[2].parse().map_err(|e| format!("{}", e))?;
    let l = parts[3].parse().map_err(|e| format!("{}", e))?;
    let o = parts[4].parse().map_err(|e| format!("{}", e))?;
    let a = parts[5].parse().map_err(|e| format!("{}", e))?;
    Ok((m, i, l, o, a))
}

//-------------------------------------------------------------------------------------------------

fn parse_latches<'a, I>(lines: &mut I, count: u32) -> Result<Stash<AigLatch>, String>
where
    I: Iterator<Item = &'a str>,
{
    let mut latches = Stash::New();
    for _ in 0..count {
        let line = lines.next().ok_or("Missing latch specification")?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err("Malformed latch line".to_string());
        }
        let lit: u32 = parts[0].parse().map_err(|e| format!("{}", e))?;
        let next: u32 = parts[1].parse().map_err(|e| format!("{}", e))?;
        let reset: u32 = if parts.len() > 2 {
            parts[2].parse().map_err(|e| format!("{}", e))?
        } else {
            0
        };
        latches.Push(AigLatch::New(lit, next, reset, String::new()));
    }
    Ok(latches)
}

//-------------------------------------------------------------------------------------------------

fn parse_ands<'a, I>(lines: &mut I, count: u32) -> Result<Stash<AigGate>, String>
where
    I: Iterator<Item = &'a str>,
{
    let mut ands = Stash::New();
    for _ in 0..count {
        let line = lines.next().ok_or("Missing AND gate specification")?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err("Malformed AND gate line".to_string());
        }
        let lhs: u32 = parts[0].parse().map_err(|e| format!("{}", e))?;
        let rhs0: u32 = parts[1].parse().map_err(|e| format!("{}", e))?;
        let rhs1: u32 = parts[2].parse().map_err(|e| format!("{}", e))?;
        ands.Push(AigGate::New(lhs, rhs0, rhs1));
    }
    Ok(ands)
}

//-------------------------------------------------------------------------------------------------

fn parse_symbols_and_comments<'a, I>(lines: &mut I) -> (Stash<AigSymbol>, Stash<String>)
where
    I: Iterator<Item = &'a str>,
{
    let mut symbols = Stash::New();
    let mut comments = Stash::New();
    let mut in_comments = false;
    for line in lines {
        let trimmed = line.trim();
        if in_comments {
            comments.Push(trimmed.to_string());
            continue;
        }
        if trimmed == "c" {
            in_comments = true;
            continue;
        }
        if let Some(c) = trimmed.chars().next()
            && (c == 'i' || c == 'l' || c == 'o')
        {
            let rest = &trimmed[1..];
            if let Some((idx_str, name)) = rest.split_once(' ')
                && let Ok(idx) = idx_str.parse::<u32>()
            {
                symbols.Push(AigSymbol::New(c, idx, name.to_string()));
            }
        }
    }
    (symbols, comments)
}

//-------------------------------------------------------------------------------------------------

fn parse_line_until_newline(bytes: &[u8], pos: &mut usize) -> Result<String, String>
{
    let start = *pos;
    while *pos < bytes.len() && bytes[*pos] != b'\n' {
        *pos += 1;
    }
    if *pos >= bytes.len() && start == *pos {
        return Err("Unexpected EOF reading line".to_string());
    }
    let s = std::str::from_utf8(&bytes[start..*pos])
        .map_err(|e| format!("{}", e))?
        .trim_end_matches('\r')
        .to_string();
    if *pos < bytes.len() && bytes[*pos] == b'\n' {
        *pos += 1;
    }
    Ok(s)
}

//-------------------------------------------------------------------------------------------------

impl Aig
{
    /// Serializes the AIG to ASCII `.aag` format.
    pub fn ToAscii(&self) -> String
    {
        let mut out = String::new();
        out.push_str(&format!("aag {} {} {} {} {}\n",
                              self._MaxVar,
                              self.InputCount(),
                              self.LatchCount(),
                              self.OutputCount(),
                              self.AndCount()));
        self._Inputs.Arr().Traverse(|&lit| {
            out.push_str(&format!("{}\n", lit));
        });
        self._Latches.Arr().Traverse(|latch| {
            if latch.Reset() != 0 {
                out.push_str(&format!("{} {} {}\n", latch.Lit(), latch.Next(), latch.Reset()));
            } else {
                out.push_str(&format!("{} {}\n", latch.Lit(), latch.Next()));
            }
        });
        self._Outputs.Arr().Traverse(|&lit| {
            out.push_str(&format!("{}\n", lit));
        });
        self._Ands.Arr().Traverse(|gate| {
            out.push_str(&format!("{} {} {}\n", gate.Lhs(), gate.Rhs0(), gate.Rhs1()));
        });
        self._Symbols.Arr().Traverse(|sym| {
            out.push_str(&format!("{}{} {}\n", sym.Kind(), sym.Index(), sym.Name()));
        });
        if self._Comments.Size() > 0 {
            out.push_str("c\n");
            self._Comments.Arr().Traverse(|c| {
                out.push_str(&format!("{}\n", c));
            });
        }
        out
    }

    /// Serializes the AIG to binary `.aig` format.
    pub fn ToBinary(&self) -> Buff<u8>
    {
        let mut bytes = Stash::New();
        let header = format!("aig {} {} {} {} {}\n",
                             self._MaxVar,
                             self.InputCount(),
                             self.LatchCount(),
                             self.OutputCount(),
                             self.AndCount());
        header.as_bytes().iter().for_each(|&b| bytes.Push(b));
        self._Latches.Arr().Traverse(|latch| {
            let line = if latch.Reset() != 0 {
                format!("{} {}\n", latch.Next(), latch.Reset())
            } else {
                format!("{}\n", latch.Next())
            };
            line.as_bytes().iter().for_each(|&b| bytes.Push(b));
        });
        self._Outputs.Arr().Traverse(|&lit| {
            let line = format!("{}\n", lit);
            line.as_bytes().iter().for_each(|&b| bytes.Push(b));
        });
        self._Ands.Arr().Traverse(|gate| {
            let delta0 = gate.Lhs() - gate.Rhs0();
            let delta1 = gate.Rhs0() - gate.Rhs1();
            encode_leb128(delta0, &mut bytes);
            encode_leb128(delta1, &mut bytes);
        });
        self._Symbols.Arr().Traverse(|sym| {
            let line = format!("{}{} {}\n", sym.Kind(), sym.Index(), sym.Name());
            line.as_bytes().iter().for_each(|&b| bytes.Push(b));
        });
        if self._Comments.Size() > 0 {
            "c\n".as_bytes().iter().for_each(|&b| bytes.Push(b));
            self._Comments.Arr().Traverse(|c| {
                let line = format!("{}\n", c);
                line.as_bytes().iter().for_each(|&b| bytes.Push(b));
            });
        }
        bytes.IntoBuff()
    }

    /// Parses an ASCII `.aag` formatted string into an AIG structure.
    pub fn FromAscii(input: &str) -> Result<Self, String>
    {
        let mut lines = input.lines().filter(|l| !l.trim().is_empty());
        let header_line = lines.next().ok_or("Empty AIGER input")?;
        let (max_var, num_inputs, num_latches, num_outputs, num_ands) = parse_header(header_line)?;

        let mut inputs = Stash::New();
        for _ in 0..num_inputs {
            let line = lines.next().ok_or("Missing input literal")?;
            let lit: u32 = line.trim().parse().map_err(|e| format!("{}", e))?;
            inputs.Push(lit);
        }
        let latches = parse_latches(&mut lines, num_latches)?;
        let mut outputs = Stash::New();
        for _ in 0..num_outputs {
            let line = lines.next().ok_or("Missing output literal")?;
            let lit: u32 = line.trim().parse().map_err(|e| format!("{}", e))?;
            outputs.Push(lit);
        }
        let ands = parse_ands(&mut lines, num_ands)?;
        let (symbols, comments) = parse_symbols_and_comments(&mut lines);

        Ok(Aig::New(max_var,
                    inputs.IntoBuff(),
                    latches.IntoBuff(),
                    outputs.IntoBuff(),
                    ands.IntoBuff(),
                    symbols.IntoBuff(),
                    comments.IntoBuff()))
    }

    /// Parses a binary `.aig` formatted byte slice into an AIG structure.
    pub fn FromBinary(bytes: &[u8]) -> Result<Self, String>
    {
        let mut pos = 0usize;
        let header_line = parse_line_until_newline(bytes, &mut pos)?;
        let parts: Vec<&str> = header_line.split_whitespace().collect();
        if parts.len() < 6 || parts[0] != "aig" {
            return Err("Invalid binary AIGER header (expected 'aig M I L O A')".to_string());
        }
        let max_var: u32 = parts[1].parse().map_err(|e| format!("{}", e))?;
        let num_inputs: u32 = parts[2].parse().map_err(|e| format!("{}", e))?;
        let num_latches: u32 = parts[3].parse().map_err(|e| format!("{}", e))?;
        let num_outputs: u32 = parts[4].parse().map_err(|e| format!("{}", e))?;
        let num_ands: u32 = parts[5].parse().map_err(|e| format!("{}", e))?;

        let mut inputs = Stash::New();
        for idx in 1..=num_inputs {
            inputs.Push(idx * 2);
        }
        let mut latches = Stash::New();
        for idx in 1..=num_latches {
            let lit = 2 * (num_inputs + idx);
            let line = parse_line_until_newline(bytes, &mut pos)?;
            let l_parts: Vec<&str> = line.split_whitespace().collect();
            let next: u32 = l_parts.first().ok_or("Missing next")?.parse().map_err(|e| format!("{}", e))?;
            let reset: u32 = if l_parts.len() > 1 {
                l_parts[1].parse().map_err(|e| format!("{}", e))?
            } else {
                0
            };
            latches.Push(AigLatch::New(lit, next, reset, String::new()));
        }
        let mut outputs = Stash::New();
        for _ in 0..num_outputs {
            let line = parse_line_until_newline(bytes, &mut pos)?;
            let lit: u32 = line.trim().parse().map_err(|e| format!("{}", e))?;
            outputs.Push(lit);
        }
        let mut ands = Stash::New();
        for idx in 0..num_ands {
            let lhs = 2 * (num_inputs + num_latches + 1 + idx);
            let delta0 = decode_leb128(bytes, &mut pos)?;
            let rhs0 = lhs - delta0;
            let delta1 = decode_leb128(bytes, &mut pos)?;
            let rhs1 = rhs0 - delta1;
            ands.Push(AigGate::New(lhs, rhs0, rhs1));
        }
        let (symbols, comments) = if pos < bytes.len() {
            let remaining = std::str::from_utf8(&bytes[pos..]).unwrap_or("");
            let mut rem_lines = remaining.lines();
            parse_symbols_and_comments(&mut rem_lines)
        } else {
            (Stash::New(), Stash::New())
        };
        Ok(Aig::New(max_var,
                    inputs.IntoBuff(),
                    latches.IntoBuff(),
                    outputs.IntoBuff(),
                    ands.IntoBuff(),
                    symbols.IntoBuff(),
                    comments.IntoBuff()))
    }

    /// Lowers a digital circuit from a `Layout` module hierarchy into an AIG.
    pub fn FromLayout(layout: &Layout, top_mod: ModuleId) -> Result<Self, String>
    {
        let mut builder = AigBuilder::New();
        let top_module = layout.Module(top_mod);
        let in_count = top_module.InPorts().Size();
        let mut port_lits: Vec<(PortId, u32)> = Vec::new();

        for i in 0..in_count {
            let port_id = layout.InPort(top_mod, i);
            let port_name = layout.LocalPortName(port_id);
            let lit = builder.AddInput(port_name);
            port_lits.push((port_id, lit));
        }

        lower_module_contents(layout, top_mod, top_mod, &mut builder, &mut port_lits)?;

        let out_count = top_module.OutPorts().Size();
        for i in 0..out_count {
            let out_port = layout.OutPort(top_mod, i);
            let out_name = layout.LocalPortName(out_port);
            let drv = layout.DriverOf(out_port);
            let lit = if drv.IsValid() {
                find_or_resolve_port_lit(layout, top_mod, drv, &mut builder, &mut port_lits)?
            } else {
                0
            };
            builder.AddOutput(lit, out_name);
        }
        builder.AddComment(&format!("Lowered from Trellis Layout module '{}'", layout.LocalName(top_mod)));
        Ok(builder.Build())
    }
}

//-------------------------------------------------------------------------------------------------

fn find_or_resolve_port_lit(layout: &Layout,
                            top_mod: ModuleId,
                            port: PortId,
                            builder: &mut AigBuilder,
                            port_lits: &mut Vec<(PortId, u32)>)
                            -> Result<u32, String>
{
    if let Some(&(_, lit)) = port_lits.iter().find(|(p, _)| *p == port) {
        return Ok(lit);
    }
    let owner = layout.Port(port).Owner();
    if owner == top_mod {
        let drv = layout.DriverOf(port);
        if drv.IsValid() && drv != port {
            let lit = find_or_resolve_port_lit(layout, top_mod, drv, builder, port_lits)?;
            port_lits.push((port, lit));
            return Ok(lit);
        }
        return Ok(0);
    }
    let owner_mod = layout.Module(owner);
    if let KernelKind::Fast(op) = owner_mod.Kernel() {
        let lit = lower_fast_kernel(layout, top_mod, owner, *op, builder, port_lits)?;
        port_lits.push((port, lit));
        return Ok(lit);
    }
    let drv = layout.DriverOf(port);
    if drv.IsValid() && drv != port {
        let lit = find_or_resolve_port_lit(layout, top_mod, drv, builder, port_lits)?;
        port_lits.push((port, lit));
        return Ok(lit);
    }
    Ok(0)
}

//-------------------------------------------------------------------------------------------------

fn lower_fast_kernel(layout: &Layout,
                     top_mod: ModuleId,
                     gate_mod: ModuleId,
                     op: KernelOp,
                     builder: &mut AigBuilder,
                     port_lits: &mut Vec<(PortId, u32)>)
                     -> Result<u32, String>
{
    let in0_port = layout.InPort(gate_mod, 0);
    let drv0 = layout.DriverOf(in0_port);
    let lit0 = if drv0.IsValid() {
        find_or_resolve_port_lit(layout, top_mod, drv0, builder, port_lits)?
    } else {
        0
    };
    let lit1 = if op == KernelOp::Not {
        0
    } else {
        let in1_port = layout.InPort(gate_mod, 1);
        let drv1 = layout.DriverOf(in1_port);
        if drv1.IsValid() {
            find_or_resolve_port_lit(layout, top_mod, drv1, builder, port_lits)?
        } else {
            0
        }
    };
    let result_lit = match op {
        KernelOp::And => builder.AddAnd(lit0, lit1),
        KernelOp::Nand => builder.AddNand(lit0, lit1),
        KernelOp::Or => builder.AddOr(lit0, lit1),
        KernelOp::Nor => builder.AddNor(lit0, lit1),
        KernelOp::Xor => builder.AddXor(lit0, lit1),
        KernelOp::Xnor => builder.AddXnor(lit0, lit1),
        KernelOp::Not => builder.AddNot(lit0),
        _ => return Err(format!("Unsupported gate operation {:?} in AIGER lowering", op)),
    };
    Ok(result_lit)
}

//-------------------------------------------------------------------------------------------------

fn lower_module_contents(layout: &Layout,
                         top_mod: ModuleId,
                         parent_mod: ModuleId,
                         builder: &mut AigBuilder,
                         port_lits: &mut Vec<(PortId, u32)>)
                         -> Result<(), String>
{
    let children = layout.Children(parent_mod);
    let count = children.Size();
    for i in 0..count {
        let child_id = children[i];
        let child = layout.Module(child_id);
        if let KernelKind::Fast(op) = child.Kernel() {
            let out_port = layout.OutPort(child_id, 0);
            if !port_lits.iter().any(|(p, _)| *p == out_port) {
                let lit = lower_fast_kernel(layout, top_mod, child_id, *op, builder, port_lits)?;
                port_lits.push((out_port, lit));
            }
        } else {
            lower_module_contents(layout, top_mod, child_id, builder, port_lits)?;
            let child_out_count = child.OutPorts().Size();
            for o in 0..child_out_count {
                let out_port = layout.OutPort(child_id, o);
                let drv = layout.DriverOf(out_port);
                if drv.IsValid() {
                    let lit = find_or_resolve_port_lit(layout, top_mod, drv, builder, port_lits)?;
                    port_lits.push((out_port, lit));
                }
            }
        }
    }
    Ok(())
}

//-------------------------------------------------------------------------------------------------
