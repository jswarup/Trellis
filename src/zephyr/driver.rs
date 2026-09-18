// src/zephyr/driver.rs
use crate::crew::hub::CrewHub;
use crate::crew::node::CrewNode;
use crate::crew::protocol::*;
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------

// Zephyr MMIO Driver for Crew Co-Simulation Peripheral (0x50000000)
pub struct ZephyrCrewDriver {
    _hub: Arc<CrewHub>,
    _node: Arc<CrewNode>,
    _base_addr: u64,
pub struct ZephyrCrewDriver
{
    _Hub: Arc<CrewHub>,
    _Node: Arc<CrewNode>,
    _BaseAddr: u64,
}
impl ZephyrCrewDriver {

impl ZephyrCrewDriver
{
    pub const DEFAULT_BASE_ADDR: u64 = 0x50000000;
    pub fn new(hub: Arc<CrewHub>, node: Arc<CrewNode>) -> Self {

    pub fn New(hub: Arc<CrewHub>, node: Arc<CrewNode>) -> Self
    {
        Self {
            _hub: hub,
            _node: node,
            _base_addr: Self::DEFAULT_BASE_ADDR,
            _Hub: hub,
            _Node: node,
            _BaseAddr: Self::DEFAULT_BASE_ADDR,
        }
    }
    pub fn with_base_addr(hub: Arc<CrewHub>, node: Arc<CrewNode>, base_addr: u64) -> Self {

    #[inline]
    pub fn new(hub: Arc<CrewHub>, node: Arc<CrewNode>) -> Self
    {
        Self::New(hub, node)
    }

    pub fn WithBaseAddr(hub: Arc<CrewHub>, node: Arc<CrewNode>, baseAddr: u64) -> Self
    {
        Self {
            _hub: hub,
            _node: node,
            _base_addr: base_addr,
            _Hub: hub,
            _Node: node,
            _BaseAddr: baseAddr,
        }
    }

    #[inline]
    pub fn reg_read(&self, offset: u32) -> u32 {
    pub fn with_base_addr(hub: Arc<CrewHub>, node: Arc<CrewNode>, baseAddr: u64) -> Self
    {
        Self::WithBaseAddr(hub, node, baseAddr)
    }

    #[inline]
    pub fn RegRead(&self, offset: u32) -> u32
    {
        let req = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusDword as i32,
            _Addr: self._base_addr | (offset as u64),
            _Addr: self._BaseAddr | (offset as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let resp = self._hub.handle_request(&self._node, &req);
        resp.value() as u32
        let resp = self._Hub.HandleRequest(&self._Node, &req);
        resp.Value() as u32
    }

    #[inline]
    pub fn reg_write(&self, offset: u32, val: u32) {
    pub fn reg_read(&self, offset: u32) -> u32
    {
        self.RegRead(offset)
    }

    #[inline]
    pub fn RegWrite(&self, offset: u32, val: u32)
    {
        let req = ProtocolMessage {
            _ActionId: CoSimAction::WriteBusByte as i32,
            _Addr: self._base_addr | (offset as u64),
            _Addr: self._BaseAddr | (offset as u64),
            _Value: val as u64,
            _PeripheralIndex: 0,
        };
        self._hub.handle_request(&self._node, &req);
        self._Hub.HandleRequest(&self._Node, &req);
    }
    pub fn get_node_id(&self) -> u32 {
        self.reg_read(REG_NODE_ID)

    #[inline]
    pub fn reg_write(&self, offset: u32, val: u32)
    {
        self.RegWrite(offset, val);
    }
    pub fn get_status(&self) -> u32 {
        self.reg_read(REG_STATUS)

    #[inline]
    pub fn GetNodeId(&self) -> u32
    {
        self.RegRead(REG_NODE_ID)
    }
    pub fn rx_count(&self) -> u32 {
        self.reg_read(REG_RX_COUNT)

    #[inline]
    pub fn get_node_id(&self) -> u32
    {
        self.GetNodeId()
    }
    pub fn send(&self, data: &[u8]) -> usize {

    #[inline]
    pub fn GetStatus(&self) -> u32
    {
        self.RegRead(REG_STATUS)
    }

    #[inline]
    pub fn get_status(&self) -> u32
    {
        self.GetStatus()
    }

    #[inline]
    pub fn RxCount(&self) -> u32
    {
        self.RegRead(REG_RX_COUNT)
    }

    #[inline]
    pub fn rx_count(&self) -> u32
    {
        self.RxCount()
    }

    pub fn Send(&self, data: &[u8]) -> usize
    {
        let mut sent = 0;
        for &b in data {
            // Wait for TX ready (in memory simulation, always ready)
            if (self.get_status() & STATUS_TX_READY) != 0 {
                self.reg_write(REG_TX_DATA, b as u32);
            if (self.GetStatus() & STATUS_TX_READY) != 0 {
                self.RegWrite(REG_TX_DATA, b as u32);
                sent += 1;
            } else {
                break;
            }
        }
        sent
    }
    pub fn recv(&self, buf: &mut [u8]) -> usize {

    #[inline]
    pub fn send(&self, data: &[u8]) -> usize
    {
        self.Send(data)
    }

    pub fn Recv(&self, buf: &mut [u8]) -> usize
    {
        let mut count = 0;
        while count < buf.len() {
            if (self.get_status() & STATUS_RX_READY) != 0 {
                let val = self.reg_read(REG_RX_DATA);
            if (self.GetStatus() & STATUS_RX_READY) != 0 {
                let val = self.RegRead(REG_RX_DATA);
                buf[count] = (val & 0xFF) as u8;
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    #[inline]
    pub fn recv(&self, buf: &mut [u8]) -> usize
    {
        self.Recv(buf)
    }
}
