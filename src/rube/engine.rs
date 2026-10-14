//-- engine.rs ------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------------

use crate::heist::ScopedExecutor;
use	crate::rube::coro_kernel::{ CORO_MAX_PORTS, CoroCell, CoroWarp };
use crate::rube::eval::{ITriggerEval, ParallelTriggers};
use	crate::rube::layout::Layout;
use	crate::rube::module::{ Eval4State, FastWarp };
use	crate::rube::port::PortId;
use	crate::rube::trigger::{ CURR_I, CURR_MASK, CURR_X, FUTR_I, FUTR_X, TriggerId, TriggerWad };
use	crate::silo::{ Buff, USeg };
use	crate::stalks::coro::ICoro;

const K_FAST_LANES_PER_WORKER: u32 = 16384;

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
    _Triggers: TriggerWad< u64>,
    _FastWarps: Buff< FastWarp>,
    _CoroWarps: Buff< CoroWarp>,
    _PortToTrigger: Buff< TriggerId>,
    _CycleCount: usize,
    _Mode: SimEngineMode,
    _ClkPort: PortId,
    _FastLaneCount: u32,
    _ParallelOutputs: u8,
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
            _FastLaneCount: 0,
            _ParallelOutputs: 0,
        }
    }
}
impl SimEngine
{

    #[inline] pub fn Triggers(&self) -> &TriggerWad<u64> { &self._Triggers }
    #[inline] pub fn CycleCount(&self) -> usize { self._CycleCount }

    pub fn	Create( layout: &Layout) -> Self
    {
        let  	portToTrigger = layout.PortToTrigger();
        let  	triggers = layout.BuildTriggers( &portToTrigger);
        let  	fastWarps = layout.CompileWarps( &portToTrigger);
        let  	coroWarps = layout.CompileCoroWarps( &portToTrigger);
        let mut fastLaneCount = 0u32;
        fastWarps.Traverse(|warp| {
            fastLaneCount = fastLaneCount.checked_add(warp._Count).expect("too many fast lanes");
        });
        Self {
            _Triggers: triggers,
            _FastWarps: fastWarps,
            _CoroWarps: coroWarps,
            _PortToTrigger: portToTrigger,
            _CycleCount: 0,
            _Mode: SimEngineMode::Serial,
            _ClkPort: PortId::Invalid(),
            _FastLaneCount: fastLaneCount,
            _ParallelOutputs: 0,
        }
    }
    #[inline]
    pub fn	WithMode( &mut self, mode: SimEngineMode)
    {
        if matches!(mode, SimEngineMode::Parallel(workers) if workers > 1)
            && self._ParallelOutputs == 0
        {
            self._ParallelOutputs = if self.OutputsAreDisjoint() { 1 } else { 2 };
        }
        self._Mode = mode;
    }

    pub(super) fn OutputsAreDisjoint(&self) -> bool
    {
        let mut seen = Buff::FromDispenser(self._Triggers.Size().div_ceil(64), |_| 0u64);
        let mut mark = |id: &TriggerId| {
            let word = &mut seen[*id / 64];
            let bit = 1u64 << (*id % 64);
            if *word & bit != 0 {
                return false;
            }
            *word |= bit;
            return true;
        };
        if !self._FastWarps.Span(|warp| warp._Out.Span(&mut mark)) {
            return false;
        }
        self._CoroWarps.Span(|warp| warp._OutTriggers.Span(|ids| ids.Span(&mut mark)))
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
        coroCell: &mut CoroCell, inTriggers: &Buff< TriggerId>, outTriggers: &Buff< TriggerId>,
        triggers: &mut TriggerWad< u64>,
    )
    {
        Self::EvalCoro(coroCell, inTriggers, outTriggers, triggers);
    }

    fn EvalCoro<T: ITriggerEval>(
        coroCell: &mut CoroCell, inTriggers: &Buff<TriggerId>, outTriggers: &Buff<TriggerId>,
        triggers: &mut T,
    )
    {
        let  	inLen = inTriggers.Size();
        let  	outLen = outTriggers.Size();
        let  	mut inPorts = crate::rube::coro_kernel::CoroPorts::New();
        let  	inCount = inLen.min( CORO_MAX_PORTS as u32);
        let  	mut k = 0;
        while k < inCount {
            inPorts._Vals[k as usize] = triggers.Current( inTriggers[k]);
            k += 1;
        }
        inPorts._Len = inCount;
        let  	coro = coroCell.GetMut();
        if coro.IsDone() {
            return;
        }
        let  	res = coro.Resume( inPorts);
        if let  	crate::stalks::CoroRes::Yield( ports) = res
            && outLen > 0 {
                let  	outCount = outLen.min( ports.Len());
                let  	mut outK = 0;
                while outK < outCount {
                    triggers.WriteFuture(outTriggers[outK], ports._Vals[outK as usize], 0);
                    outK += 1;
                }
            }
    }
    fn	EvalWarpLanes<T: ITriggerEval>(
        warp: &FastWarp, startLane: u32, endLane: u32, triggers: &mut T,
    )
    {
        let  	op = warp._Op;
        let  	mask = warp._Mask;
        let  	mut l = startLane;
        while l < endLane {
            let  	in1Trig = warp._In1[l];
            let  	in2Trig = warp._In2[l];
            let  	outTrig = warp._Out[l];
            let  	in1 = triggers.Current(in1Trig);
            let  	in2 = triggers.Current(in2Trig);
            let  	f1 = triggers.Flags(in1Trig);
            let  	f2 = triggers.Flags(in2Trig);
            if ( ( f1 | f2) & CURR_MASK) == 0 {
                let  	raw = op.EvalRaw( in1, in2, mask);
                triggers.WriteFuture(outTrig, raw, 0);
            } else {
                let  	x1 = ( f1 & CURR_X) != 0;
                let  	i1 = ( f1 & CURR_I) != 0;
                let  	x2 = ( f2 & CURR_X) != 0;
                let  	i2 = ( f2 & CURR_I) != 0;
                let  	res = Eval4State( op, in1, x1, i1, in2, x2, i2, mask);
                let  	mut f = 0;
                if res._IsX {
                    f |= FUTR_X;
                }
                if res._IsI {
                    f |= FUTR_I;
                }
                triggers.WriteFuture(outTrig, res._Val, f);
            }
            l += 1;
        }
    }
    fn	EvalCoroWarpLanes(
        warp: &mut CoroWarp, startLane: u32, endLane: u32, triggers: &mut TriggerWad< u64>,
    )
    {
        let  	mut l = startLane;
        while l < endLane {
            let  	inTrigs = &warp._InTriggers[l];
            let  	outTrigs = &warp._OutTriggers[l];
            let  	coroCell = &mut warp._Instances[l];
            Self::EvalCoroInstance( coroCell, inTrigs, outTrigs, triggers);
            l += 1;
        }
    }
    pub fn	Drive( &mut self) -> usize
    {
        let workers = match self._Mode {
            SimEngineMode::Parallel(workers) => workers,
            SimEngineMode::Serial => 1,
        };
        if workers > 1 && self._ParallelOutputs == 1 && ParallelTriggers::SUPPORTED {
            self.DriveParallel(workers);
        } else {
            self.EvalFastWarps();
            self._CoroWarps.TraverseMut(|warp| {
                Self::EvalCoroWarpLanes(warp, 0, warp._Count, &mut self._Triggers);
            });
        }
        self._Triggers.AdvanceAll();
        self._CycleCount += 1;
        self._CycleCount
    }

    fn EvalFastWarps(&mut self)
    {
        self._FastWarps.Traverse(|warp| {
            Self::EvalWarpLanes(warp, 0, warp._Count, &mut self._Triggers);
        });
    }

    fn DriveParallel(&mut self, workers: u32)
    {
        let fastWorkers = workers.min(self._FastLaneCount / K_FAST_LANES_PER_WORKER);
        if fastWorkers > 1 {
            let warps = self._FastWarps.Arr();
            let triggers = ParallelTriggers::New(&mut self._Triggers);
            ScopedExecutor::New(fastWorkers).ForEachRange(self._FastLaneCount, |_, start, end| {
                let mut triggers = triggers;
                let mut offset = 0u32;
                warps.USeg().Span(|index| {
                    let warp = &warps[index];
                    let warpCount = warp._Count;
                    let warpEnd = offset + warpCount;
                    if warpEnd > start {
                        let startLane = start.saturating_sub(offset).min(warpCount);
                        let endLane = end.saturating_sub(offset).min(warpCount);
                        if startLane < endLane {
                            Self::EvalWarpLanes(warp, startLane, endLane, &mut triggers);
                        }
                    }
                    offset = warpEnd;
                    return offset < end;
                });
            });
        } else {
            self.EvalFastWarps();
        }
        self._CoroWarps.TraverseMut(|warp| {
            if warp._Count < 2 {
                Self::EvalCoroWarpLanes(warp, 0, warp._Count, &mut self._Triggers);
                return;
            }
            let inputs = warp._InTriggers.Arr();
            let outputs = warp._OutTriggers.Arr();
            let triggers = ParallelTriggers::New(&mut self._Triggers);
            ScopedExecutor::New(workers).ForEachMut(warp._Instances.MutArr(),
                |_, start, mut instances| {
                    let mut triggers = triggers;
                    instances.USeg().Traverse(|index| {
                        Self::EvalCoro(instances.GetMut(index).unwrap(), &inputs[start + index],
                                       &outputs[start + index], &mut triggers);
                    });
                });
        });
    }
    pub fn	Settle( &mut self, maxCycles: u32) -> u32
    {
        let  	mut cycles = 0u32;
        while cycles < maxCycles {
            self.Drive();
            cycles += 1;
            if !self._Triggers.HasAnyEdge() {
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
