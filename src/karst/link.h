// link.h ---------------------------------------------------------------------------------------------------------
#pragma once

#include "rube/layout.h"
#include "rube/port.h"

#include <cstdint>

//-------------------------------------------------------------------------------------------------

namespace trellis::karst {

//-------------------------------------------------------------------------------------------------
// KarstFlit — packed 64-bit transaction word carried over Karst Tiger-links.
// Bit [63]:    IsWrite (1 = write, 0 = read)
// Bits [62:56]: Source node ID (7 bits: 0..127)
// Bits [55:32]: Target byte address (24 bits: up to 16 MB addressable space)
// Bits [31:0]: Payload data word (32 bits)

struct KarstFlit
{
    uint32_t            _Addr{0};
    uint32_t            _Data{0};
    uint8_t             _SrcId{0};
    bool                _IsWrite{false};

    constexpr KarstFlit( void) noexcept = default;

    constexpr KarstFlit( uint32_t addr, uint32_t data, uint8_t srcId, bool isWrite) noexcept
        : _Addr( addr),
          _Data( data),
          _SrcId( srcId),
          _IsWrite( isWrite)
    {
    }

    static constexpr uint64_t Pack( uint32_t addr, uint32_t data, uint8_t srcId, bool isWrite) noexcept
    {
        uint64_t raw = isWrite ? ( 1ULL << 63) : 0ULL;
        raw |= ( static_cast< uint64_t>( srcId & 0x7FU) << 56);
        raw |= ( static_cast< uint64_t>( addr & 0x00FF'FFFFU) << 32);
        raw |= static_cast< uint64_t>( data);
        return raw;
    }

    static constexpr KarstFlit Unpack( uint64_t raw) noexcept
    {
        KarstFlit f;
        f._IsWrite = ( raw & ( 1ULL << 63)) != 0;
        f._SrcId   = static_cast< uint8_t>( ( raw >> 56) & 0x7FU);
        f._Addr    = static_cast< uint32_t>( ( raw >> 32) & 0x00FF'FFFFU);
        f._Data    = static_cast< uint32_t>( raw & 0xFFFF'FFFFU);
        return f;
    }
};

//-------------------------------------------------------------------------------------------------
// KarstLink — encapsulates port IDs for a bidirectional Tiger-link streaming connection.

struct KarstLink
{
    // Outbound (Transmit) interface
    rube::PortId        _TxValid{};
    rube::PortId        _TxData{};
    rube::PortId        _TxReady{};

    // Inbound (Receive) interface
    rube::PortId        _RxValid{};
    rube::PortId        _RxData{};
    rube::PortId        _RxReady{};

    constexpr KarstLink( void) noexcept = default;

    constexpr KarstLink(
        rube::PortId txValid,
        rube::PortId txData,
        rube::PortId txReady,
        rube::PortId rxValid,
        rube::PortId rxData,
        rube::PortId rxReady) noexcept
        : _TxValid( txValid),
          _TxData( txData),
          _TxReady( txReady),
          _RxValid( rxValid),
          _RxData( rxData),
          _RxReady( rxReady)
    {
    }

    static void Connect( rube::Layout& layout, const KarstLink& a, const KarstLink& b)
    {
        // a -> b
        layout.Connect( a._TxValid, b._RxValid);
        layout.Connect( a._TxData,  b._RxData);
        layout.Connect( b._RxReady, a._TxReady);

        // b -> a
        layout.Connect( b._TxValid, a._RxValid);
        layout.Connect( b._TxData,  a._RxData);
        layout.Connect( a._RxReady, b._TxReady);
    }
};

//-------------------------------------------------------------------------------------------------

} // namespace trellis::karst

