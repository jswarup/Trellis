//-- driver.rs ------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::crew::hub::{ AsNodeId, CrewHub };
use	crate::crew::protocol::{ CoSimAction, ProtocolMessage, REG_NODE_ID, REG_RX_COUNT, REG_RX_DATA, REG_STATUS, REG_TX_DATA, STATUS_RX_READY, STATUS_TX_READY };
use	crate::silo::{ Arr, MutArr };
use	std::sync::Arc;

//-------------------------------------------------------------------------------------------------
// Zephyr MMIO Driver for Crew Co-Simulation Peripheral (0x50000000)
pub struct ZephyrCrewDriver
{
    _hub:       Arc< CrewHub>,
    _node_id:   u32,
    _base_addr: u64,
}

impl ZephyrCrewDriver
{
    pub const DEFAULT_BASE_ADDR: u64 = 0x50000000;

    pub fn	new( hub: Arc< CrewHub>, node: impl AsNodeId) -> Self
    {
        Self {
            _hub:       hub,
            _node_id:   node.AsNodeId(),
            _base_addr: Self::DEFAULT_BASE_ADDR,
        }
    }

    pub fn	with_base_addr( hub: Arc< CrewHub>, node: impl AsNodeId, base_addr: u64) -> Self
    {
        Self {
            _hub:       hub,
            _node_id:   node.AsNodeId(),
            _base_addr: base_addr,
        }
    }

    #[inline]
    pub fn	reg_read( &self, offset: u32) -> u32
    {
        let  	req = ProtocolMessage {
            _ActionId:        CoSimAction::ReadBusDword as i32,
            _Addr:            self._base_addr | ( offset as u64),
            _Value:           0,
            _PeripheralIndex: 0,
        };
        let  	resp = self._hub.handle_request( &self._node_id, &req);
        resp.value() as u32
    }

    #[inline]
    pub fn	reg_write( &self, offset: u32, val: u32)
    {
        let  	req = ProtocolMessage {
            _ActionId:        CoSimAction::WriteBusByte as i32,
            _Addr:            self._base_addr | ( offset as u64),
            _Value:           val as u64,
            _PeripheralIndex: 0,
        };
        self._hub.handle_request( &self._node_id, &req);
    }

    pub fn	get_node_id( &self) -> u32
    {
        self.reg_read( REG_NODE_ID)
    }

    pub fn	get_status( &self) -> u32
    {
        self.reg_read( REG_STATUS)
    }

    pub fn	rx_count( &self) -> u32
    {
        self.reg_read( REG_RX_COUNT)
    }

    pub fn	send( &self, data: Arr< '_, u8>) -> usize
    {
        let  	mut sent = 0u32;
        data.USeg().Span( |i| {
            // Wait for TX ready (in memory simulation, always ready)
            if ( self.get_status() & STATUS_TX_READY) != 0
            {
                self.reg_write( REG_TX_DATA, data[i] as u32);
                sent += 1;
                true
            }
            else
            {
                false
            }
        });
        sent as usize
    }

    pub fn	recv( &self, mut buf: MutArr< '_, u8>) -> usize
    {
        let  	mut count = 0u32;
        while count < buf.Len()
        {
            if ( self.get_status() & STATUS_RX_READY) != 0
            {
                let  	val = self.reg_read( REG_RX_DATA);
                buf[count] = ( val & 0xFF) as u8;
                count += 1;
            }
            else
            {
                break;
            }
        }
        count as usize
    }
}

//--------------------------------------------------------------------------------------------------
