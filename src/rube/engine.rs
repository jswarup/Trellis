//-- engine.rs ------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------------

use	crate::heist::Atelier;
use	crate::rube::coro_kernel::{ CORO_MAX_PORTS, CoroCell, CoroWarp };
use	crate::rube::layout::Layout;
use	crate::rube::module::{ Eval4State, FastWarp };
use	crate::rube::port::PortId;
use	crate::rube::trigger::{ CURR_I, CURR_MASK, CURR_X, FUTR_I, FUTR_MASK, FUTR_X, TriggerId, TriggerWad };
use	crate::silo::{ Buff, USeg };
use	crate::stalks::coro::ICoro;

//------------------------------------------------------------------------------------------------------------------
/// Simulation execution mode.
#[derive( Copy, Clone, PartialEq, Eq, Debug)]
pub enum SimEngineMode {
    Serial,
    Parallel( u32),
}
impl Default for SimEngineMode {
    #[inline]
    fn	default() -> Self
    {
        Self::Serial
    }
}

//------------------------------------------------------------------------------------------------------------------
/// Digital circuit simulation engine supporting Serial and Parallel Drive modes.
/// Modeled directly from Trellis `engine.h`.
pub struct SimEngine
{
    pub _Triggers: TriggerWad< u64>,
    pub _FastWarps: Buff< FastWarp>,
    pub _CoroWarps: Buff< CoroWarp>,
    pub _PortToTrigger: Buff< TriggerId>,
    pub _CycleCount: usize,
    pub _Mode: SimEngineMode,
    pub _ClkPort: PortId,
}
impl Default for SimEngine {
    #[inline]
    fn	default() -> Self
    {
        Self {
            _Triggers: TriggerWad::default(),
            _FastWarps: Buff::New(),
            _CoroWarps: Buff::New(),
            _PortToTrigger: Buff::New(),
            _CycleCount: 0,
            _Mode: SimEngineMode::Serial,
            _ClkPort: PortId::Invalid(),
        }
    }
}
impl SimEngine
{
    pub fn	Create( layout: &Layout) -> Self
    {
        let  	portToTrigger = layout.PortToTrigger();
        let  	triggers = layout.BuildTriggers( &portToTrigger);
        let  	fastWarps = layout.CompileWarps( &portToTrigger);
        let  	coroWarps = layout.CompileCoroWarps( &portToTrigger);
        Self {
            _Triggers: triggers,
            _FastWarps: fastWarps,
            _CoroWarps: coroWarps,
            _PortToTrigger: portToTrigger,
            _CycleCount: 0,
            _Mode: SimEngineMode::Serial,
            _ClkPort: PortId::Invalid(),
        }
    }
    #[inline]
    pub fn	WithMode( &mut self, mode: SimEngineMode)
    {
        self._Mode = mode;
    }
    #[inline]
    pub fn	WithClock( &mut self, clkPort: PortId) -> &mut Self
    {
        self._ClkPort = clkPort;
        self
    }
    #[inline]
    pub const fn	GetClock( &self) -> PortId
    {
        self._ClkPort
    }
    #[inline]
    pub fn	GetPortTrigger( &self, portId: PortId) -> TriggerId
    {
        let  	idx = portId.Index();
        if idx >= self._PortToTrigger.Size() {
            return u32::MAX;
        }
        self._PortToTrigger[idx]
    }
    #[inline]
    pub fn	GetTrigger( &self, id: TriggerId) -> u64
    {
        self._Triggers.Current( id)
    }
    #[inline]
    pub fn	SetTrigger( &mut self, id: TriggerId, val: u64, isX: bool, isI: bool)
    {
        self._Triggers.SetFuture( id, val, isX, isI);
    }
    #[inline]
    pub fn	SetTriggerImmediate( &mut self, id: TriggerId, val: u64, isX: bool, isI: bool)
    {
        self._Triggers.SetImmediate( id, val, isX, isI);
    }
    #[inline]
    pub fn	Get( &self, portId: PortId) -> u64
    {
        let  	trigId = self.GetPortTrigger( portId);
        if trigId == u32::MAX {
            return 0;
        }
        self._Triggers.Current( trigId)
    }
    #[inline]
    pub fn	GetBool( &self, portId: PortId) -> bool
    {
        ( self.Get( portId) & 1) != 0
    }
    #[inline]
    pub fn	GetU32( &self, portId: PortId) -> u32
    {
        self.Get( portId) as u32
    }
    #[inline]
    pub fn	Set( &mut self, portId: PortId, val: u64, isX: bool, isI: bool) -> bool
    {
        let  	trigId = self.GetPortTrigger( portId);
        if trigId == u32::MAX {
            return false;
        }
        self.SetTriggerImmediate( trigId, val, isX, isI);
        true
    }
    #[inline]
    pub fn	SetBool( &mut self, portId: PortId, val: bool) -> bool
    {
        self.Set( portId, if val { 1 } else { 0 }, false, false)
    }
    #[inline]
    pub fn	SetU32( &mut self, portId: PortId, val: u32) -> bool
    {
        self.Set( portId, val as u64, false, false)
    }
    #[inline]
    pub fn	IsX( &self, portId: PortId) -> bool
    {
        let  	trigId = self.GetPortTrigger( portId);
        if trigId == u32::MAX {
            return true;
        }
        self._Triggers.IsX( trigId)
    }
    #[inline]
    pub fn	IsI( &self, portId: PortId) -> bool
    {
        let  	trigId = self.GetPortTrigger( portId);
        if trigId == u32::MAX {
            return false;
        }
        self._Triggers.IsI( trigId)
    }
    #[inline]
    pub fn	IsZ( &self, portId: PortId) -> bool
    {
        self.IsI( portId)
    }
    #[inline]
    pub fn	IsValid( &self, portId: PortId) -> bool
    {
        let  	trigId = self.GetPortTrigger( portId);
        if trigId == u32::MAX {
            return false;
        }
        self._Triggers.IsValid( trigId)
    }
    pub fn	EvalCoroInstance( 
        coroCell: &CoroCell, inTriggers: &Buff< TriggerId>, outTriggers: &Buff< TriggerId>,
        triggers: &mut TriggerWad< u64>,
    )
    {
        let  	inLen = inTriggers.Size();
        let  	outLen = outTriggers.Size();
        let  	mut inPorts = crate::rube::coro_kernel::CoroPorts::New();
        let  	inCount = ( inLen as usize).min( CORO_MAX_PORTS);
        let  	mut k = 0;
        while k < inCount {
            inPorts._Vals[k] = triggers.Current( inTriggers[k as u32]);
            k += 1;
        }
        inPorts._Len = inCount as u32;
        let  	coro = coroCell.GetMut();
        if coro.IsDone() {
            return;
        }
        let  	res = coro.Resume( inPorts);
        if let  	crate::stalks::CoroRes::Yield( ports) = res
            && outLen > 0 {
                let  	outCount = ( outLen as usize).min( ports.Len() as usize);
                let  	mut outK = 0;
                while outK < outCount {
                    triggers.SetFuture( outTriggers[outK as u32], ports._Vals[outK], false, false);
                    outK += 1;
                }
            }
    }
    fn	EvalWarpLanes( 
        warp: &FastWarp, startLane: u32, endLane: u32, triggers: &mut TriggerWad< u64>,
    )
    {
        let  	op = warp._Op;
        let  	mask = warp._Mask;
        let  	mut l = startLane;
        while l < endLane {
            let  	in1Trig = warp._In1[l];
            let  	in2Trig = warp._In2[l];
            let  	outTrig = warp._Out[l];
            let  	in1 = triggers._CurrentVals[in1Trig];
            let  	in2 = triggers._CurrentVals[in2Trig];
            let  	f1 = triggers._Flags[in1Trig];
            let  	f2 = triggers._Flags[in2Trig];
            if ( ( f1 | f2) & CURR_MASK) == 0 {
                let  	raw = op.EvalRaw( in1, in2, mask);
                triggers._FutureVals[outTrig] = raw;
                triggers._Flags[outTrig] &= !FUTR_MASK;
            } else {
                let  	x1 = ( f1 & CURR_X) != 0;
                let  	i1 = ( f1 & CURR_I) != 0;
                let  	x2 = ( f2 & CURR_X) != 0;
                let  	i2 = ( f2 & CURR_I) != 0;
                let  	res = Eval4State( op, in1, x1, i1, in2, x2, i2, mask);
                triggers._FutureVals[outTrig] = res._Val;
                let  	mut f = triggers._Flags[outTrig] & !FUTR_MASK;
                if res._IsX {
                    f |= FUTR_X;
                }
                if res._IsI {
                    f |= FUTR_I;
                }
                triggers._Flags[outTrig] = f;
            }
            l += 1;
        }
    }
    fn	EvalCoroWarpLanes( 
        warp: &CoroWarp, startLane: u32, endLane: u32, triggers: &mut TriggerWad< u64>,
    )
    {
        let  	mut l = startLane;
        while l < endLane {
            let  	inTrigs = &warp._InTriggers[l];
            let  	outTrigs = &warp._OutTriggers[l];
            let  	coroCell = &warp._Instances[l];
            Self::EvalCoroInstance( coroCell, inTrigs, outTrigs, triggers);
            l += 1;
        }
    }
    pub fn	Drive( &mut self) -> usize
    {
        let  	isParallel = match self._Mode {
            SimEngineMode::Parallel( workers) => workers > 1,
            SimEngineMode::Serial => false,
        };
        if isParallel {
            // In parallel mode, execute warps across worker threads.
            // Each lane in a warp writes to its dedicated trigger index.
            let  	triggersPtr = &mut self._Triggers as *mut TriggerWad< u64> as usize;
            let  	numWorkers = match self._Mode {
                SimEngineMode::Parallel( w) => w,
                SimEngineMode::Serial => 1,
            };
            let  	atelier = Atelier::Reset( numWorkers);
            USeg::FromLen( self._FastWarps.Size()).Traverse( |wIdx| {
                let  	warp = &self._FastWarps[wIdx];
                let  	count = warp._Count;
                let  	chunkSize = 64u32;
                let  	numChunks = count.div_ceil(chunkSize);
                USeg::FromLen( numChunks).Traverse( |c| {
                    let  	start = c * chunkSize;
                    let  	end = ( start + chunkSize).min( count);
                    let  	warpClone = warp.clone();
                    atelier.MainMaestro().Post( move |_w| {
                        let  	triggers = unsafe { &mut *( triggersPtr as *mut TriggerWad< u64>) };
                        Self::EvalWarpLanes( &warpClone, start, end, triggers);
                    });
                });
            });
            USeg::FromLen( self._CoroWarps.Size()).Traverse( |wIdx| {
                let  	warp = &self._CoroWarps[wIdx];
                Self::EvalCoroWarpLanes( warp, 0, warp._Count, &mut self._Triggers);
            });
            atelier.DoLaunch();
        } else {
            USeg::FromLen( self._FastWarps.Size()).Traverse( |wIdx| {
                let  	warp = &self._FastWarps[wIdx];
                Self::EvalWarpLanes( warp, 0, warp._Count, &mut self._Triggers);
            });
            USeg::FromLen( self._CoroWarps.Size()).Traverse( |wIdx| {
                let  	warp = &self._CoroWarps[wIdx];
                Self::EvalCoroWarpLanes( warp, 0, warp._Count, &mut self._Triggers);
            });
        }
        self._Triggers.AdvanceAll();
        self._CycleCount += 1;
        self._CycleCount
    }
    pub fn	Settle( &mut self, maxCycles: u32) -> u32
    {
        let  	mut cycles = 0u32;
        while cycles < maxCycles {
            self.Drive();
            cycles += 1;
            let  	mut anyEdge = false;
            let  	sz = self._Triggers.Size();
            let  	mut t = 0u32;
            while t < sz {
                if self._Triggers.IsEdge( t) {
                    anyEdge = true;
                    break;
                }
                t += 1;
            }
            if !anyEdge {
                break;
            }
        }
        cycles
    }
    pub fn	AdvanceClock( &mut self, clkPort: PortId, ticks: u32, maxSettleCycles: u32) -> u32
    {
        if !clkPort.IsValid() {
            return 0;
        }
        let  	mut totalCycles = 0u32;
        USeg::FromLen( ticks).Traverse( |_| {
            let  	baseline = self.GetBool( clkPort);
            self.SetBool( clkPort, !baseline);
            totalCycles += self.Settle( maxSettleCycles);
            self.SetBool( clkPort, baseline);
            totalCycles += self.Settle( maxSettleCycles);
        });
        totalCycles
    }
    pub fn	AdvanceTrigger( &mut self, clkTrig: TriggerId, ticks: u32, maxSettleCycles: u32) -> u32
    {
        if clkTrig == u32::MAX || clkTrig >= self._Triggers.Size() {
            return 0;
        }
        let  	mut totalCycles = 0u32;
        USeg::FromLen( ticks).Traverse( |_| {
            let  	baseline = ( self.GetTrigger( clkTrig) & 1) != 0;
            self.SetTriggerImmediate( clkTrig, if baseline { 0 } else { 1 }, false, false);
            totalCycles += self.Settle( maxSettleCycles);
            self.SetTriggerImmediate( clkTrig, if baseline { 1 } else { 0 }, false, false);
            totalCycles += self.Settle( maxSettleCycles);
        });
        totalCycles
    }
    #[inline]
    pub fn	AdvanceTriggerTicks( &mut self, clkTrig: TriggerId, ticks: u32) -> u32
    {
        self.AdvanceTrigger( clkTrig, ticks, 100)
    }
    #[inline]
    pub fn	Advance( &mut self) -> u32
    {
        self.AdvanceTicks( 1)
    }
    #[inline]
    pub fn	AdvanceTicks( &mut self, ticks: u32) -> u32
    {
        self.AdvanceWithMax( ticks, 100)
    }
    #[inline]
    pub fn	AdvanceWithMax( &mut self, ticks: u32, maxSettleCycles: u32) -> u32
    {
        if !self._ClkPort.IsValid() {
            return 0;
        }
        self.AdvanceClock( self._ClkPort, ticks, maxSettleCycles)
    }
}
