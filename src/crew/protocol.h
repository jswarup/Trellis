// protocol.h -----------------------------------------------------------------------------------------------------
#pragma once

#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::crew {

//-------------------------------------------------------------------------------------------------
// Renode CoSimulation plugin action types.

enum class CoSimAction : int32_t
{
    Invalid            = 0,
    TickClock          = 1,
    WriteBus           = 2,
    ReadBus            = 3,
    ResetPeripheral    = 4,
    LogMessage         = 5,
    Interrupt          = 6,
    Disconnect         = 7,
    Error              = 8,
    Ok                 = 9,
    Handshake          = 10,
    PushDword          = 11,
    GetDword           = 12,
    PushWord           = 13,
    GetWord            = 14,
    PushByte           = 15,
    GetByte            = 16,
    IsHalted           = 17,
    RegisterGet        = 18,
    RegisterSet        = 19,
    SingleStep         = 20,
    ReadBusByte        = 21,
    ReadBusWord        = 22,
    ReadBusDword       = 23,
    ReadBusQword       = 24,
    WriteBusByte       = 25,
    WriteBusWord       = 26,
    WriteBusDword      = 27,
    WriteBusQword      = 28,
    PushQword          = 29,
    GetQword           = 30,
    PushConfirmation   = 31
};

//-------------------------------------------------------------------------------------------------
// Renode socket co-simulation protocol message packet (24 bytes packed).

#pragma pack(push, 1)
struct ProtocolMessage
{
    int32_t             _ActionId{0};
    uint64_t            _Addr{0};
    uint64_t            _Value{0};
    int32_t             _PeripheralIndex{0};
};
#pragma pack(pop)

static_assert( sizeof( ProtocolMessage) == 24, "ProtocolMessage must be exactly 24 bytes packed");

//-------------------------------------------------------------------------------------------------
// MMIO register offsets mapped at 0x50000000 in Zephyr VM.

constexpr uint32_t      REG_NODE_ID     = 0x000;                        // [RO] Current Node ID (0 or 1)
constexpr uint32_t      REG_STATUS      = 0x004;                        // [RO] Status flags
constexpr uint32_t      REG_TX_DATA     = 0x008;                        // [WO] Transmit byte to peer VM
constexpr uint32_t      REG_RX_DATA     = 0x00C;                        // [RO] Receive byte from peer VM
constexpr uint32_t      REG_RX_COUNT    = 0x010;                        // [RO] Number of bytes pending

//-------------------------------------------------------------------------------------------------
// Status register bit flags.

constexpr uint32_t      STATUS_TX_READY = ( 1U << 0);
constexpr uint32_t      STATUS_RX_READY = ( 1U << 1);
constexpr uint32_t      STATUS_PEER_UP  = ( 1U << 2);

//-------------------------------------------------------------------------------------------------
} // namespace trellis::crew
