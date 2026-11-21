// vcd_stream.rs ----------------------------------------------------------------------------------
use	crate::rube::engine::SimEngine;
use	crate::rube::layout::Layout;
use	crate::rube::port::PortId;
use	crate::silo::{ Buff, Stash, USeg };
use	std::io::Write;

//-------------------------------------------------------------------------------------------------

/// High-throughput streaming IEEE-1364 VCD writer operating in bounded O(1) memory.
pub struct VcdStreamingWriter< W: Write>
{
    _Writer:      W,
    _TrigToIdStr: Buff< String>,
    _TrigBits:    Buff< u32>,
    _Timescale:   String,
    _HeaderDone:  bool,
}

impl< W: Write> VcdStreamingWriter< W>
{
    pub fn	New( writer: W, layout: &Layout, engine: &SimEngine) -> Self
    {
        Self::WithTimescale( writer, layout, engine, "1ns")
    }

    pub fn	WithTimescale(
        writer: W,
        layout: &Layout,
        engine: &SimEngine,
        timescale: impl Into< String>,
    ) -> Self
    {
        let  	trigCount = engine.Triggers().Size();
        let  	mut trigToIdStr = Stash::WithCapacity( trigCount);
        let  	mut trigBits = Stash::WithCapacity( trigCount);
        USeg::FromLen( trigCount).Traverse( |i| {
            trigToIdStr.Push( Self::GenerateIdStr( i));
            trigBits.Push( 1);
        });
        let  	portCount = layout.Ports().Size();
        USeg::FromLen( portCount).Traverse( |pIdx| {
            let  	portId = PortId::In( pIdx);
            let  	trigId = engine.GetPortTrigger( portId);
            if trigId != u32::MAX && trigId < trigCount
                && let Some( port ) = layout.Ports().Get( pIdx)
            {
                trigBits[trigId] = port.Type().Bits();
            }
        });
        Self {
            _Writer:      writer,
            _TrigToIdStr: trigToIdStr.IntoBuff(),
            _TrigBits:    trigBits.IntoBuff(),
            _Timescale:   timescale.into(),
            _HeaderDone:  false,
        }
    }

    #[inline] pub const fn	IsHeaderDone( &self) -> bool { self._HeaderDone }
    #[inline] pub fn	Timescale( &self) -> &str { &self._Timescale }

    pub fn	WriteHeader( &mut self, layout: &Layout, engine: &SimEngine) -> std::io::Result< ()>
    {
        writeln!( self._Writer, "$version\n   Trellis Rube Engine\n$end")?;
        writeln!( self._Writer, "$timescale {} $end", self._Timescale)?;

        let  	writer = &mut self._Writer;
        let  	trigToIdStr = &self._TrigToIdStr;

        layout.TraverseModules( |module, _, enter| {
            if !enter
            {
                let _ = writeln!( writer, "$upscope $end");
                return true;
            }
            let _ = writeln!( writer, "$scope module {} $end", layout.LocalName( module.Id()));
            let  	writePort = |pId: PortId, w: &mut W| {
                let  	port = layout.Port( pId);
                let  	trigId = engine.GetPortTrigger( pId);
                if trigId < trigToIdStr.Size()
                {
                    let  	name = port
                        .Name()
                        .strip_prefix( module.Name())
                        .and_then( |suffix| suffix.strip_prefix( '.'))
                        .unwrap_or( port.Name());
                    let _ = writeln!( 
                        w,
                        "$var wire {} {} {} $end",
                        port.Type().Bits(),
                        trigToIdStr[trigId],
                        name
                    );
                }
            };
            module.InPorts().Traverse( |idx| writePort( PortId::In( idx), writer));
            module.OutPorts().Traverse( |idx| writePort( PortId::Out( idx), writer));
            true
        });

        writeln!( self._Writer, "$enddefinitions $end")?;
        writeln!( self._Writer, "$dumpvars")?;

        let  	trigCount = engine.Triggers().Size();
        let  	writer = &mut self._Writer;
        let  	trigBits = &self._TrigBits;

        USeg::FromLen( trigCount).Traverse( |i| {
            let  	val = engine.GetTrigger( i);
            let  	bits = if i < trigBits.Size() { trigBits[i] } else { 1 };
            let  	isX = engine.Triggers().IsX( i);
            let  	isZ = engine.Triggers().IsI( i);
            let  	vcdId = &trigToIdStr[i];
            let _ = Self::WriteFormattedVal( writer, val, bits, isX, isZ, vcdId);
        });

        writeln!( self._Writer, "$end")?;
        self._HeaderDone = true;
        self._Writer.flush()
    }

    pub fn	DumpCycle( &mut self, engine: &SimEngine) -> std::io::Result< ()>
    {
        self.DumpTimestamp( engine.CycleCount() as u64)?;
        let  	trigCount = engine.Triggers().Size();
        let  	writer = &mut self._Writer;
        let  	trigToIdStr = &self._TrigToIdStr;
        let  	trigBits = &self._TrigBits;

        USeg::FromLen( trigCount).Traverse( |i| {
            if engine.Triggers().IsEdge( i)
            {
                let  	val = engine.GetTrigger( i);
                let  	bits = if i < trigBits.Size() { trigBits[i] } else { 1 };
                let  	isX = engine.Triggers().IsX( i);
                let  	isZ = engine.Triggers().IsI( i);
                let  	vcdId = &trigToIdStr[i];
                let _ = Self::WriteFormattedVal( writer, val, bits, isX, isZ, vcdId);
            }
        });
        Ok( ())
    }

    pub fn	DumpTimestamp( &mut self, timestamp: u64) -> std::io::Result< ()>
    {
        writeln!( self._Writer, "#{}", timestamp)
    }

    pub fn	DumpValueChange(
        &mut self,
        trigId: u32,
        val: u64,
        isX: bool,
        isZ: bool,
    ) -> std::io::Result< ()>
    {
        if trigId < self._TrigToIdStr.Size()
        {
            let  	bits = self._TrigBits[trigId];
            let  	id = &self._TrigToIdStr[trigId];
            Self::WriteFormattedVal( &mut self._Writer, val, bits, isX, isZ, id)?;
        }
        Ok( ())
    }

    pub fn	Flush( &mut self) -> std::io::Result< ()>
    {
        self._Writer.flush()
    }

    pub fn	IntoInner( self) -> W
    {
        self._Writer
    }

    // Helpers
    fn	GenerateIdStr( mut val: u32) -> String
    {
        let  	mut res = String::new();
        loop
        {
            let  	mut ch = ( ( val % 92) as u8) + 33;
            if ch >= b'#'
            {
                ch += 1;
            }
            if ch >= b'$'
            {
                ch += 1;
            }
            res.push( ch as char);
            val /= 92;
            if val == 0
            {
                break;
            }
        }
        res
    }

    fn	WriteFormattedVal(
        w: &mut W,
        val: u64,
        bits: u32,
        isX: bool,
        isZ: bool,
        id: &str,
    ) -> std::io::Result< ()>
    {
        if bits <= 1
        {
            let  	c = if isZ {
                'z'
            } else if isX {
                'x'
            } else if ( val & 1) != 0 {
                '1'
            } else {
                '0'
            };
            writeln!( w, "{}{}", c, id)
        }
        else
        {
            write!( w, "b")?;
            if isZ
            {
                write!( w, "z")?;
            }
            else if isX
            {
                write!( w, "x")?;
            }
            else
            {
                let  	mut started = false;
                USeg::FromLen( bits).TraverseRev( |i| {
                    let  	mask = 1u64 << i;
                    if ( val & mask) != 0
                    {
                        let _ = write!( w, "1");
                        started = true;
                    }
                    else if started || i == 0
                    {
                        let _ = write!( w, "0");
                    }
                });
            }
            writeln!( w, " {}", id)
        }
    }
}
