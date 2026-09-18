//-- vm_adaptor.rs ---------------------------------------------------------------------------------

//--------------------------------------------------------------------------------------------------

use crate::crew::node::NodeStats;
use crate::crew::protocol::{
    REG_NODE_ID, REG_RX_COUNT, REG_RX_DATA, REG_STATUS, REG_TX_DATA, STATUS_PEER_UP,
    STATUS_RX_READY, STATUS_TX_READY,
};
use crate::crew::vm_runner::VMRunner;
use crate::rube::{CoroPorts, Layout, ModuleId, PortDesc, PortId, PortType};
use crate::silo::fifo::Fifo;
use crate::stalks::Coro;
use crate::stalks::work::SpinMutex;
use std::sync::Arc;

//--------------------------------------------------------------------------------------------------

/// VMAdaptor — hardware adapter connecting a VMRunner guest to the Rube inter-VM network.
/// Implements MMIO registers (REG_NODE_ID, REG_STATUS, REG_TX_DATA, REG_RX_DATA, REG_RX_COUNT)
/// and bridges them to valid/ready streaming links across Rube netlist interconnects.
#[derive(Clone)]
pub struct VMAdaptor
{
    _Id:             ModuleId,
    _NodeId:         u32,
    _Stats:          Arc<SpinMutex<NodeStats>>,

    // Local VM bus ports
    _VmReqIn:        PortId,
    _VmWriteIn:      PortId,
    _VmAddrIn:       PortId,
    _VmWDataIn:      PortId,
    _VmAckOut:       PortId,
    _VmRDataOut:     PortId,

    // Inter-VM link ports
    _LinkRxValidIn:  PortId,
    _LinkRxDataIn:   PortId,
    _LinkTxReadyIn:  PortId,
    _LinkTxValidOut: PortId,
    _LinkTxDataOut:  PortId,
    _LinkRxReadyOut: PortId,
}

impl VMAdaptor
{
    pub fn New(
        layout:     &mut Layout,
        name:       &str,
        nodeId:     u32,
        attachedVm: Option<&VMRunner>,
        parent:     ModuleId,
    ) -> Self
    {
        let inDescs = [
            PortDesc::New("VmReq",       PortType::Bool(),   ModuleId::None()),
            PortDesc::New("VmWrite",     PortType::Bool(),   ModuleId::None()),
            PortDesc::New("VmAddr",      PortType::U32Val(), ModuleId::None()),
            PortDesc::New("VmWData",     PortType::U32Val(), ModuleId::None()),
            PortDesc::New("LinkRxValid", PortType::Bool(),   ModuleId::None()),
            PortDesc::New("LinkRxData",  PortType::U32Val(), ModuleId::None()),
            PortDesc::New("LinkTxReady", PortType::Bool(),   ModuleId::None()),
        ];

        let outDescs = [
            PortDesc::New("VmAck",       PortType::Bool(),   ModuleId::None()),
            PortDesc::New("VmRData",     PortType::U32Val(), ModuleId::None()),
            PortDesc::New("LinkTxValid", PortType::Bool(),   ModuleId::None()),
            PortDesc::New("LinkTxData",  PortType::U32Val(), ModuleId::None()),
            PortDesc::New("LinkRxReady", PortType::Bool(),   ModuleId::None()),
        ];

        let stats = Arc::new(SpinMutex::New(NodeStats::default()));
        let statsClone = stats.clone();

        let id = layout.AddCoroModule(
            name,
            parent,
            &inDescs[..],
            &outDescs[..],
            move || {
                let stats = statsClone.clone();
                Coro::New(move |yielder, mut inPorts: CoroPorts| {
                    let mut rxQueue: Fifo<u8, 128> = Fifo::New();
                    let mut txQueue: Fifo<u8, 128> = Fifo::New();
                    let mut wasReq = false;
                    let mut lastTxPresented = false;
                    let mut lastRData: u32 = 0;

                    loop {
                        // 1. Link TX completion (if peer asserted ready, byte was transferred)
                        if lastTxPresented && inPorts.GetBool(6) && !txQueue.IsEmpty() {
                            let _ = txQueue.PopFront();
                            let mut s = stats.Lock();
                            s._BytesSent += 1;
                        }

                        // 2. Link RX sampling (if peer asserted valid and we have room)
                        if inPorts.GetBool(4) && !rxQueue.IsFull() {
                            let inByte = (inPorts[5usize] & 0xFF) as u8;
                            let _ = rxQueue.PushBack(inByte);
                            let mut s = stats.Lock();
                            s._BytesReceived += 1;
                        }

                        // 3. VM MMIO processing
                        let (vmAck, vmRData) = if inPorts.GetBool(0) {
                            if !wasReq {
                                wasReq = true;
                                let isWrite = inPorts.GetBool(1);
                                let addr = (inPorts[2usize] & 0xFFFF) as u32;
                                let wdata = inPorts[3usize] as u32;
                                let rdata: u32;

                                if isWrite {
                                    {
                                        let mut s = stats.Lock();
                                        s._WritesServiced += 1;
                                    }
                                    if addr == REG_TX_DATA && !txQueue.IsFull() {
                                        let _ = txQueue.PushBack((wdata & 0xFF) as u8);
                                    }
                                    rdata = 0;
                                } else {
                                    {
                                        let mut s = stats.Lock();
                                        s._ReadsServiced += 1;
                                    }
                                    if addr == REG_NODE_ID {
                                        rdata = nodeId;
                                    } else if addr == REG_STATUS {
                                        let mut st = STATUS_PEER_UP;
                                        if !txQueue.IsFull() {
                                            st |= STATUS_TX_READY;
                                        }
                                        if !rxQueue.IsEmpty() {
                                            st |= STATUS_RX_READY;
                                        }
                                        rdata = st;
                                    } else if addr == REG_RX_COUNT {
                                        rdata = rxQueue.Size();
                                    } else if addr == REG_RX_DATA {
                                        rdata = rxQueue.PopFront().map(|b| b as u32).unwrap_or(0);
                                    } else {
                                        rdata = 0;
                                    }
                                }
                                lastRData = rdata;
                                (true, rdata)
                            } else {
                                // Request is still held high by VM
                                (true, lastRData)
                            }
                        } else {
                            wasReq = false;
                            lastRData = 0;
                            (false, 0)
                        };

                        // 4. Link TX presentation
                        let mut linkTxValid = false;
                        let mut linkTxData = 0u32;
                        if !txQueue.IsEmpty() {
                            linkTxValid = true;
                            linkTxData = *txQueue.Front().unwrap() as u32;
                        }
                        lastTxPresented = linkTxValid;

                        // 5. Link RX readiness
                        let linkRxReady = !rxQueue.IsFull();

                        // 6. Yield outputs
                        let mut out = CoroPorts::New();
                        out.Push(if vmAck { 1u64 } else { 0u64 });
                        out.Push(vmRData as u64);
                        out.Push(if linkTxValid { 1u64 } else { 0u64 });
                        out.Push(linkTxData as u64);
                        out.Push(if linkRxReady { 1u64 } else { 0u64 });

                        inPorts = yielder.Suspend(out);
                    }
                })
            },
        );

        let vmReqIn        = layout.InPort(id, 0);
        let vmWriteIn      = layout.InPort(id, 1);
        let vmAddrIn       = layout.InPort(id, 2);
        let vmWDataIn      = layout.InPort(id, 3);
        let linkRxValidIn  = layout.InPort(id, 4);
        let linkRxDataIn   = layout.InPort(id, 5);
        let linkTxReadyIn  = layout.InPort(id, 6);

        let vmAckOut       = layout.OutPort(id, 0);
        let vmRDataOut     = layout.OutPort(id, 1);
        let linkTxValidOut = layout.OutPort(id, 2);
        let linkTxDataOut  = layout.OutPort(id, 3);
        let linkRxReadyOut = layout.OutPort(id, 4);

        let adaptor = Self {
            _Id:             id,
            _NodeId:         nodeId,
            _Stats:          stats,
            _VmReqIn:        vmReqIn,
            _VmWriteIn:      vmWriteIn,
            _VmAddrIn:       vmAddrIn,
            _VmWDataIn:      vmWDataIn,
            _VmAckOut:       vmAckOut,
            _VmRDataOut:     vmRDataOut,
            _LinkRxValidIn:  linkRxValidIn,
            _LinkRxDataIn:   linkRxDataIn,
            _LinkTxReadyIn:  linkTxReadyIn,
            _LinkTxValidOut: linkTxValidOut,
            _LinkTxDataOut:  linkTxDataOut,
            _LinkRxReadyOut: linkRxReadyOut,
        };

        if let Some(vm) = attachedVm {
            adaptor.ConnectVm(layout, vm);
        }

        adaptor
    }

    pub fn ConnectVm(&self, layout: &mut Layout, vm: &VMRunner)
    {
        layout.Connect(vm.Req(),   self._VmReqIn);
        layout.Connect(vm.Write(), self._VmWriteIn);
        layout.Connect(vm.Addr(),  self._VmAddrIn);
        layout.Connect(vm.WData(), self._VmWDataIn);
        layout.Connect(self._VmAckOut,   vm.Ack());
        layout.Connect(self._VmRDataOut, vm.RData());
    }

    pub fn Connect(layout: &mut Layout, a: &VMAdaptor, b: &VMAdaptor)
    {
        // a -> b
        layout.Connect(a.LinkTxValid(), b.LinkRxValid());
        layout.Connect(a.LinkTxData(),  b.LinkRxData());
        layout.Connect(b.LinkRxReady(), a.LinkTxReady());

        // b -> a
        layout.Connect(b.LinkTxValid(), a.LinkRxValid());
        layout.Connect(b.LinkTxData(),  a.LinkRxData());
        layout.Connect(a.LinkRxReady(), b.LinkTxReady());
    }

    #[inline]
    pub const fn Id(&self) -> ModuleId
    {
        self._Id
    }

    #[inline]
    pub const fn NodeId(&self) -> u32
    {
        self._NodeId
    }

    #[inline]
    pub const fn VmReq(&self) -> PortId
    {
        self._VmReqIn
    }

    #[inline]
    pub const fn VmWrite(&self) -> PortId
    {
        self._VmWriteIn
    }

    #[inline]
    pub const fn VmAddr(&self) -> PortId
    {
        self._VmAddrIn
    }

    #[inline]
    pub const fn VmWData(&self) -> PortId
    {
        self._VmWDataIn
    }

    #[inline]
    pub const fn VmAck(&self) -> PortId
    {
        self._VmAckOut
    }

    #[inline]
    pub const fn VmRData(&self) -> PortId
    {
        self._VmRDataOut
    }

    #[inline]
    pub const fn LinkRxValid(&self) -> PortId
    {
        self._LinkRxValidIn
    }

    #[inline]
    pub const fn LinkRxData(&self) -> PortId
    {
        self._LinkRxDataIn
    }

    #[inline]
    pub const fn LinkTxReady(&self) -> PortId
    {
        self._LinkTxReadyIn
    }

    #[inline]
    pub const fn LinkTxValid(&self) -> PortId
    {
        self._LinkTxValidOut
    }

    #[inline]
    pub const fn LinkTxData(&self) -> PortId
    {
        self._LinkTxDataOut
    }

    #[inline]
    pub const fn LinkRxReady(&self) -> PortId
    {
        self._LinkRxReadyOut
    }

    pub fn GetStats(&self) -> NodeStats
    {
        *self._Stats.Lock()
    }
}
