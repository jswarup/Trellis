// src/crew/protocol.rs

//-------------------------------------------------------------------------------------------------

// Renode CoSimulation plugin action types.
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CoSimAction {
    Invalid = 0,
    TickClock = 1,
    WriteBus = 2,
    ReadBus = 3,
    ResetPeripheral = 4,
    LogMessage = 5,
    Interrupt = 6,
    Disconnect = 7,
    Error = 8,
    Ok = 9,
    Handshake = 10,
    PushDword = 11,
    GetDword = 12,
    PushWord = 13,
    GetWord = 14,
    PushByte = 15,
    GetByte = 16,
    IsHalted = 17,
    RegisterGet = 18,
    RegisterSet = 19,
    SingleStep = 20,
    ReadBusByte = 21,
    ReadBusWord = 22,
    ReadBusDword = 23,
    ReadBusQword = 24,
    WriteBusByte = 25,
    WriteBusWord = 26,
    WriteBusDword = 27,
    WriteBusQword = 28,
    PushQword = 29,
    GetQword = 30,
    PushConfirmation = 31,
}
impl From<i32> for CoSimAction {
    fn from(val: i32) -> Self {
        match val {
            0 => CoSimAction::Invalid,
            1 => CoSimAction::TickClock,
            2 => CoSimAction::WriteBus,
            3 => CoSimAction::ReadBus,
            4 => CoSimAction::ResetPeripheral,
            5 => CoSimAction::LogMessage,
            6 => CoSimAction::Interrupt,
            7 => CoSimAction::Disconnect,
            8 => CoSimAction::Error,
            9 => CoSimAction::Ok,
            10 => CoSimAction::Handshake,
            11 => CoSimAction::PushDword,
            12 => CoSimAction::GetDword,
            13 => CoSimAction::PushWord,
            14 => CoSimAction::GetWord,
            15 => CoSimAction::PushByte,
            16 => CoSimAction::GetByte,
            17 => CoSimAction::IsHalted,
            18 => CoSimAction::RegisterGet,
            19 => CoSimAction::RegisterSet,
            20 => CoSimAction::SingleStep,
            21 => CoSimAction::ReadBusByte,
            22 => CoSimAction::ReadBusWord,
            23 => CoSimAction::ReadBusDword,
            24 => CoSimAction::ReadBusQword,
            25 => CoSimAction::WriteBusByte,
            26 => CoSimAction::WriteBusWord,
            27 => CoSimAction::WriteBusDword,
            28 => CoSimAction::WriteBusQword,
            29 => CoSimAction::PushQword,
            30 => CoSimAction::GetQword,
            31 => CoSimAction::PushConfirmation,
            _ => CoSimAction::Invalid,
        }
    }
}

//-------------------------------------------------------------------------------------------------

// Renode socket co-simulation protocol message packet (24 bytes packed).
#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ProtocolMessage {
    pub _ActionId: i32,
    pub _Addr: u64,
    pub _Value: u64,
    pub _PeripheralIndex: i32,
}
impl ProtocolMessage {
    pub const fn new(action_id: i32, addr: u64, value: u64, peripheral_index: i32) -> Self {
        Self {
            _ActionId: action_id,
            _Addr: addr,
            _Value: value,
            _PeripheralIndex: peripheral_index,
        }
    }
    #[inline]
    pub fn action(&self) -> CoSimAction {
        // Read unaligned field safely by value
        let id = { self._ActionId };
        CoSimAction::from(id)
    }
    #[inline]
    pub fn addr(&self) -> u64 {
        self._Addr
    }
    #[inline]
    pub fn value(&self) -> u64 {
        self._Value
    }
    #[inline]
    pub fn peripheral_index(&self) -> i32 {
        self._PeripheralIndex
    }
    pub fn to_le_bytes(&self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        buf[0..4].copy_from_slice(&self._ActionId.to_le_bytes());
        buf[4..12].copy_from_slice(&self._Addr.to_le_bytes());
        buf[12..20].copy_from_slice(&self._Value.to_le_bytes());
        buf[20..24].copy_from_slice(&self._PeripheralIndex.to_le_bytes());
        buf
    }
    pub fn from_le_bytes(buf: &[u8; 24]) -> Result<Self, &'static str> {
        let action = i32::from_le_bytes(buf[0..4].try_into().unwrap());
        let addr = u64::from_le_bytes(buf[4..12].try_into().unwrap());
        let value = u64::from_le_bytes(buf[12..20].try_into().unwrap());
        let periph = i32::from_le_bytes(buf[20..24].try_into().unwrap());
        let msg = Self {
            _ActionId: action,
            _Addr: addr,
            _Value: value,
            _PeripheralIndex: periph,
        };
        if !msg.is_valid() {
            return Err("Invalid protocol action id");
        }
        Ok(msg)
    }
    pub fn is_valid(&self) -> bool {
        let action = self.action();
        action != CoSimAction::Invalid
    }
}

//-------------------------------------------------------------------------------------------------

// MMIO register offsets mapped at 0x50000000 in Zephyr VM.
pub const REG_NODE_ID: u32 = 0x000;
pub const REG_STATUS: u32 = 0x004;
pub const REG_TX_DATA: u32 = 0x008;
pub const REG_RX_DATA: u32 = 0x00C;
pub const REG_RX_COUNT: u32 = 0x010;

//-------------------------------------------------------------------------------------------------

// Status register bit flags.
pub const STATUS_TX_READY: u32 = 1 << 0;
pub const STATUS_RX_READY: u32 = 1 << 1;
pub const STATUS_PEER_UP: u32 = 1 << 2;
