//-- vcd.rs -------------------------------------------------------------------------------------------------------
use	crate::rube::engine::SimEngine;
use	crate::rube::layout::Layout;
use	crate::rube::port::PortId;
use	crate::silo::{ Buff, IArr, Stash, USeg };

//-------------------------------------------------------------------------------------------------
/// Serializes digital circuit simulation state and transition cycles into IEEE-1364 VCD format.
pub struct VcdWriter
{
    _TrigToIdStr: Buff< String>,
    _TrigBits:    Buff< u32>,
}

//-------------------------------------------------------------------------------------------------

impl VcdWriter
{
    pub fn	New( layout: &Layout, engine: &SimEngine) -> Self
    {
        let  	trigCount = engine._Triggers.Size();
        let  	mut trigToIdStr = Stash::WithCapacity( trigCount);
        let  	mut trigBits = Stash::WithCapacity( trigCount);
        USeg::FromLen( trigCount).Traverse( |i| {
            trigToIdStr.Push( Self::GenerateIdStr( i));
            trigBits.Push( 1);
        });
        // Resolve bit width for each trigger by finding the first port mapped to it
        let  	portCount = layout._Ports.Size();
        USeg::FromLen( portCount).Traverse( |pIdx| {
            let  	portId = PortId::In( pIdx);
            let  	trigId = engine.GetPortTrigger( portId);
            if trigId != u32::MAX && trigId < trigCount {
                if let  	Some( port) = layout._Ports.AsArr().Get( pIdx) {
                    trigBits[trigId] = port.Type().Bits();
                }
            }
        });
        Self {
            _TrigToIdStr: trigToIdStr.IntoBuff(),
            _TrigBits:    trigBits.IntoBuff(),
        }
    }

    //---------------------------------------------------------------------------------------------
    /// Writes standard VCD header with module hierarchy, port descriptors, and initial dumpvars.
    pub fn	WriteHeader( &self, layout: &Layout, engine: &SimEngine, out: &mut String)
    {
        out.push_str( "$version\n   Segue Rube Engine\n$end\n");
        out.push_str( "$timescale 1ns $end\n");
        layout._Modules.AsArr().Traverse( |module| {
            out.push_str( &format!( "$scope module {} $end\n", module._Name));
            module._InPorts.Traverse( |idx| {
                let  	portId = PortId::In( idx);
                if let  	Some( port) = layout._Ports.AsArr().Get( idx) {
                    let  	trigId = engine.GetPortTrigger( portId);
                    if trigId != u32::MAX && trigId < self._TrigToIdStr.Size() {
                        let  	vcdId = &self._TrigToIdStr[trigId];
                        let  	bits = port.Type().Bits();
                        out.push_str( &format!( 
                            "$var wire {} {} {} $end\n",
                            bits,
                            vcdId,
                            port.Name()
                        ));
                    }
                }
            });
            module._OutPorts.Traverse( |idx| {
                let  	portId = PortId::Out( idx);
                if let  	Some( port) = layout._Ports.AsArr().Get( idx) {
                    let  	trigId = engine.GetPortTrigger( portId);
                    if trigId != u32::MAX && trigId < self._TrigToIdStr.Size() {
                        let  	vcdId = &self._TrigToIdStr[trigId];
                        let  	bits = port.Type().Bits();
                        out.push_str( &format!( 
                            "$var wire {} {} {} $end\n",
                            bits,
                            vcdId,
                            port.Name()
                        ));
                    }
                }
            });
            out.push_str( "$upscope $end\n");
        });
        out.push_str( "$enddefinitions $end\n");
        out.push_str( "$dumpvars\n");
        // Dump initial values
        USeg::FromLen( engine._Triggers.Size()).Traverse( |i| {
            let  	val = engine.GetTrigger( i);
            let  	bits = if i < self._TrigBits.Size() {
                self._TrigBits[i]
            } else {
                1
            };
            let  	isX = engine._Triggers.IsX( i);
            let  	isZ = engine._Triggers.IsI( i);
            let  	vcdId = &self._TrigToIdStr[i];
            Self::FormatVal( val, bits, isX, isZ, vcdId, out);
        });
        out.push_str( "$end\n");
    }

    //---------------------------------------------------------------------------------------------
    /// Emits a simulation cycle timestamp and all signals that experienced an edge event.
    pub fn	DumpCycle( &self, engine: &SimEngine, out: &mut String)
    {
        out.push_str( &format!( "#{}\n", engine._CycleCount));
        USeg::FromLen( engine._Triggers.Size()).Traverse( |i| {
            if engine._Triggers.IsEdge( i) {
                let  	val = engine.GetTrigger( i);
                let  	bits = if i < self._TrigBits.Size() {
                    self._TrigBits[i]
                } else {
                    1
                };
                let  	isX = engine._Triggers.IsX( i);
                let  	isZ = engine._Triggers.IsI( i);
                let  	vcdId = &self._TrigToIdStr[i];
                Self::FormatVal( val, bits, isX, isZ, vcdId, out);
            }
        });
    }

    //---------------------------------------------------------------------------------------------

    fn	GenerateIdStr( mut val: u32) -> String
    {
        let  	mut res = String::new();
        loop {
            let  	mut ch = ( ( val % 92) as u8) + 33;
            if ch >= b'#' {
                ch += 1;
            }
            if ch >= b'$' {
                ch += 1;
            }
            res.push( ch as char);
            val /= 92;
            if val == 0 {
                break;
            }
        }
        res
    }

    //---------------------------------------------------------------------------------------------

    fn	FormatVal( val: u64, bits: u32, isX: bool, isZ: bool, id: &str, out: &mut String)
    {
        if bits <= 1 {
            if isZ {
                out.push( 'z');
            } else if isX {
                out.push( 'x');
            } else {
                out.push( if ( val & 1) != 0 { '1' } else { '0' });
            }
            out.push_str( id);
            out.push( '\n');
        } else {
            out.push( 'b');
            if isZ {
                out.push( 'z');
            } else if isX {
                out.push( 'x');
            } else {
                let  	mut started = false;
                USeg::FromLen( bits).TraverseRev( |i| {
                    let  	mask = 1u64 << i;
                    if ( val & mask) != 0 {
                        out.push( '1');
                        started = true;
                    } else if started || i == 0 {
                        out.push( '0');
                    }
                });
            }
            out.push( ' ');
            out.push_str( id);
            out.push( '\n');
        }
    }
}
