//-- vm_runner.rs ----------------------------------------------------------------------------------

//--------------------------------------------------------------------------------------------------

use crate::rube::{CoroInstance, CoroPorts, Layout, ModuleId, PortDesc, PortId, PortType};

//--------------------------------------------------------------------------------------------------

/// VmBus — helper for constructing zero-allocation CoroPorts transactions for VM MMIO bus.
pub struct VmBus;

impl VmBus
{
    #[inline]
    pub fn Idle() -> CoroPorts
    {
        let mut p = CoroPorts::New();
        p.Push(0u64); // Req = false
        p.Push(0u64); // Write = false
        p.Push(0u64); // Addr = 0
        p.Push(0u64); // WData = 0
        p
    }

    #[inline]
    pub fn Read(addr: u32) -> CoroPorts
    {
        let mut p = CoroPorts::New();
        p.Push(1u64); // Req = true
        p.Push(0u64); // Write = false
        p.Push(addr as u64);
        p.Push(0u64);
        p
    }

    #[inline]
    pub fn Write(addr: u32, val: u32) -> CoroPorts
    {
        let mut p = CoroPorts::New();
        p.Push(1u64); // Req = true
        p.Push(1u64); // Write = true
        p.Push(addr as u64);
        p.Push(val as u64);
        p
    }
}

//--------------------------------------------------------------------------------------------------

/// VMRunner — encapsulates a virtual machine guest executing as a Rube CoroModule.
/// Communicates with its local VMAdaptor via 4 outputs (Req, Write, Addr, WData) and 2 inputs (Ack, RData).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VMRunner
{
    _Id:       ModuleId,
    _ReqOut:   PortId,
    _WriteOut: PortId,
    _AddrOut:  PortId,
    _WDataOut: PortId,
    _AckIn:    PortId,
    _RDataIn:  PortId,
}

impl VMRunner
{
    pub fn New(
        layout:  &mut Layout,
        name:    &str,
        factory: impl Fn() -> CoroInstance + Send + Sync + 'static,
        parent:  ModuleId,
    ) -> Self
    {
        let inDescs = [
            PortDesc::New("Ack",   PortType::Bool(),   ModuleId::None()),
            PortDesc::New("RData", PortType::U32Val(), ModuleId::None()),
        ];
        let outDescs = [
            PortDesc::New("Req",   PortType::Bool(),   ModuleId::None()),
            PortDesc::New("Write", PortType::Bool(),   ModuleId::None()),
            PortDesc::New("Addr",  PortType::U32Val(), ModuleId::None()),
            PortDesc::New("WData", PortType::U32Val(), ModuleId::None()),
        ];

        let id = layout.AddCoroModule(
            name,
            parent,
            &inDescs[..],
            &outDescs[..],
            factory,
        );

        let ackIn    = layout.InPort(id, 0);
        let rDataIn  = layout.InPort(id, 1);
        let reqOut   = layout.OutPort(id, 0);
        let writeOut = layout.OutPort(id, 1);
        let addrOut  = layout.OutPort(id, 2);
        let wDataOut = layout.OutPort(id, 3);

        Self {
            _Id:       id,
            _ReqOut:   reqOut,
            _WriteOut: writeOut,
            _AddrOut:  addrOut,
            _WDataOut: wDataOut,
            _AckIn:    ackIn,
            _RDataIn:  rDataIn,
        }
    }

    #[inline]
    pub const fn Id(&self) -> ModuleId
    {
        self._Id
    }

    #[inline]
    pub const fn Ack(&self) -> PortId
    {
        self._AckIn
    }

    #[inline]
    pub const fn RData(&self) -> PortId
    {
        self._RDataIn
    }

    #[inline]
    pub const fn Req(&self) -> PortId
    {
        self._ReqOut
    }

    #[inline]
    pub const fn Write(&self) -> PortId
    {
        self._WriteOut
    }

    #[inline]
    pub const fn Addr(&self) -> PortId
    {
        self._AddrOut
    }

    #[inline]
    pub const fn WData(&self) -> PortId
    {
        self._WDataOut
    }
}

