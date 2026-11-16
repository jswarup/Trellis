//-- cosim.rs --------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::crew::hub::CrewHub;
use	crate::rube::engine::SimEngine;
use	crate::rube::port::PortId;
use	crate::zephyr::runtime::{ IZephyrRuntime, StepBudget, ZephyrError };
use	std::sync::Arc;

//--------------------------------------------------------------------------------------------------
/// Hardware port mapping bridging digital logic simulation with Zephyr VM co-simulation.
#[derive( Copy, Clone, Debug, PartialEq, Eq)]
pub struct CosimPortMap
{
    _Clk:         PortId,
    _DataIn:      PortId,
    _DataOut:     PortId,
    _WriteEnable: PortId,
    _ReadEnable:  PortId,
    _Irq:         PortId,
    _Status:      PortId,
}

impl CosimPortMap
{
    pub fn	New(
        clk: PortId, dataIn: PortId, dataOut: PortId,
        writeEnable: PortId, readEnable: PortId, irq: PortId, status: PortId,
    ) -> Self
    {
        Self {
            _Clk:         clk,
            _DataIn:      dataIn,
            _DataOut:     dataOut,
            _WriteEnable: writeEnable,
            _ReadEnable:  readEnable,
            _Irq:         irq,
            _Status:      status,
        }
    }

    pub fn	Empty() -> Self
    {
        Self {
            _Clk:         PortId::Invalid(),
            _DataIn:      PortId::Invalid(),
            _DataOut:     PortId::Invalid(),
            _WriteEnable: PortId::Invalid(),
            _ReadEnable:  PortId::Invalid(),
            _Irq:         PortId::Invalid(),
            _Status:      PortId::Invalid(),
        }
    }

    #[inline]
    pub const fn	Clk( &self) -> PortId
    {
        self._Clk
    }

    #[inline]
    pub const fn	DataIn( &self) -> PortId
    {
        self._DataIn
    }

    #[inline]
    pub const fn	DataOut( &self) -> PortId
    {
        self._DataOut
    }

    #[inline]
    pub const fn	WriteEnable( &self) -> PortId
    {
        self._WriteEnable
    }

    #[inline]
    pub const fn	ReadEnable( &self) -> PortId
    {
        self._ReadEnable
    }

    #[inline]
    pub const fn	Irq( &self) -> PortId
    {
        self._Irq
    }

    #[inline]
    pub const fn	Status( &self) -> PortId
    {
        self._Status
    }
}

//--------------------------------------------------------------------------------------------------
/// Summary of lockstep co-simulation stepping results.
#[derive( Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct CosimStepSummary
{
    _TotalCycles:      u64,
    _VmInstructions:   u64,
    _MmioTransactions: u32,
    _IrqEvents:        u32,
}

impl CosimStepSummary
{
    pub fn	New( cycles: u64, instructions: u64, mmio: u32, irq: u32) -> Self
    {
        Self {
            _TotalCycles:      cycles,
            _VmInstructions:   instructions,
            _MmioTransactions: mmio,
            _IrqEvents:        irq,
        }
    }

    #[inline]
    pub const fn	TotalCycles( &self) -> u64
    {
        self._TotalCycles
    }

    #[inline]
    pub const fn	VmInstructions( &self) -> u64
    {
        self._VmInstructions
    }

    #[inline]
    pub const fn	MmioTransactions( &self) -> u32
    {
        self._MmioTransactions
    }

    #[inline]
    pub const fn	IrqEvents( &self) -> u32
    {
        self._IrqEvents
    }
}

//--------------------------------------------------------------------------------------------------
/// Cycle-accurate co-simulation harness synchronizing Zephyr VM execution with digital hardware simulation.
pub struct ZephyrCosimHarness
{
    _Runtime:   Box< dyn IZephyrRuntime>,
    _Engine:    SimEngine,
    _Hub:       Arc< CrewHub>,
    _NodeId:    u32,
    _PortMap:   CosimPortMap,
    _CycleCount: u64,
    _MmioCount:  u32,
    _IrqCount:   u32,
    _IrqActive:  bool,
}

impl ZephyrCosimHarness
{
    pub fn	New(
        runtime: Box< dyn IZephyrRuntime>,
        engine: SimEngine,
        hub: Arc< CrewHub>,
        nodeId: u32,
        portMap: CosimPortMap,
    ) -> Self
    {
        Self {
            _Runtime:    runtime,
            _Engine:     engine,
            _Hub:        hub,
            _NodeId:     nodeId,
            _PortMap:    portMap,
            _CycleCount: 0,
            _MmioCount:  0,
            _IrqCount:   0,
            _IrqActive:  false,
        }
    }

    #[inline]
    pub fn	Engine( &self) -> &SimEngine
    {
        &self._Engine
    }

    #[inline]
    pub fn	EngineMut( &mut self) -> &mut SimEngine
    {
        &mut self._Engine
    }

    #[inline]
    pub fn	Runtime( &self) -> &dyn IZephyrRuntime
    {
        &*self._Runtime
    }

    #[inline]
    pub fn	RuntimeMut( &mut self) -> &mut ( dyn IZephyrRuntime + 'static)
    {
        &mut *self._Runtime
    }

    #[inline]
    pub fn	Hub( &self) -> &Arc< CrewHub>
    {
        &self._Hub
    }

    #[inline]
    pub const fn	NodeId( &self) -> u32
    {
        self._NodeId
    }

    #[inline]
    pub const fn	PortMap( &self) -> &CosimPortMap
    {
        &self._PortMap
    }

    #[inline]
    pub const fn	CycleCount( &self) -> u64
    {
        self._CycleCount
    }

    #[inline]
    pub const fn	MmioCount( &self) -> u32
    {
        self._MmioCount
    }

    #[inline]
    pub const fn	IrqCount( &self) -> u32
    {
        self._IrqCount
    }

    #[inline]
    pub const fn	IsIrqActive( &self) -> bool
    {
        self._IrqActive
    }

    /// Performs an MMIO write to hardware simulation ports.
    pub fn	WriteHardwareMmio( &mut self, val: u32)
    {
        if self._PortMap._DataIn.IsValid()
        {
            self._Engine.SetU32( self._PortMap._DataIn, val);
        }
        if self._PortMap._WriteEnable.IsValid()
        {
            self._Engine.SetBool( self._PortMap._WriteEnable, true);
            self._Engine.StepCycle();
            self._Engine.SetBool( self._PortMap._WriteEnable, false);
        }
        self._MmioCount += 1;
    }

    /// Performs an MMIO read from hardware simulation ports.
    pub fn	ReadHardwareMmio( &mut self) -> u32
    {
        if self._PortMap._ReadEnable.IsValid()
        {
            self._Engine.SetBool( self._PortMap._ReadEnable, true);
            self._Engine.StepCycle();
            self._Engine.SetBool( self._PortMap._ReadEnable, false);
        }
        self._MmioCount += 1;
        if self._PortMap._DataOut.IsValid()
        {
            self._Engine.GetU32( self._PortMap._DataOut)
        }
        else
        {
            0
        }
    }

    /// Executes lockstep co-simulation for the requested number of hardware cycles.
    pub fn	StepLockstep( &mut self, cycles: u32) -> Result< CosimStepSummary, ZephyrError>
    {
        let  	mut instructionsExecuted = 0u64;
        let  	mut stepIdx = 0u32;
        while stepIdx < cycles
        {
            // 1. Advance digital hardware simulation
            self._Engine.StepCycle();
            self._CycleCount += 1;

            // 2. Sample hardware IRQ pin
            if self._PortMap._Irq.IsValid()
            {
                let  	irqSample = self._Engine.GetBool( self._PortMap._Irq);
                if irqSample && !self._IrqActive
                {
                    self._IrqCount += 1;
                    self._IrqActive = true;
                }
                else if !irqSample
                {
                    self._IrqActive = false;
                }
            }

            // 3. Step Zephyr VM execution
            let  	budget = StepBudget {
                instruction_limit: 1,
                time_ns:           10,
            };
            let  	stepRes = self._Runtime.Step( budget)?;
            instructionsExecuted += stepRes.instructions_executed;

            // 4. Update status port if bound
            if self._PortMap._Status.IsValid()
            {
                let  	statusVal = if self._IrqActive { 2 } else { 1 };
                self._Engine.SetU32( self._PortMap._Status, statusVal);
            }

            stepIdx += 1;
        }

        Ok( CosimStepSummary::New(
            self._CycleCount,
            instructionsExecuted,
            self._MmioCount,
            self._IrqCount,
        ))
    }
}
