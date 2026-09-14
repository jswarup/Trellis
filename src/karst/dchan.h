// dchan.h --------------------------------------------------------------------------------------------------------
#pragma once

#include "silo/arr.h"
#include "silo/buff.h"
#include "swarm/cpu.h"
#include "swarm/traits.h"

#include <cstdint>
#include <cstring>
#include <memory>
#include <string>

//-------------------------------------------------------------------------------------------------

namespace trellis::karst {

//-------------------------------------------------------------------------------------------------
// Channel performance and traffic metrics.

struct DChanStats
{
    uint32_t            _ReadsServiced{0};
    uint32_t            _WritesServiced{0};
    uint64_t            _BytesWritten{0};
    uint64_t            _BytesRead{0};
};

//-------------------------------------------------------------------------------------------------
// KarstDChan — simulates a DDR5-8800 physical memory channel backed by a Swarm ComputeBuffer.

class KarstDChan
{
private:
    uint32_t                                _ChanIdx{0};
    uint64_t                                _Capacity{4096};
    std::unique_ptr< swarm::ComputeBuffer>  _Buffer{};
    DChanStats                              _Stats{};

public:
    KarstDChan( void) = default;

    KarstDChan(
        swarm::ComputeDevice& device,
        uint32_t chanIdx,
        uint64_t capacity = 4096)
        : _ChanIdx( chanIdx),
          _Capacity( capacity),
          _Stats{}
    {
        const std::string label = "dchan_" + std::to_string( chanIdx);
        _Buffer = device.CreateBuffer(
            label.c_str(),
            capacity,
            swarm::BufferUsage::Storage() | swarm::BufferUsage::ReadWrite()
        );
    }

    uint32_t ChanIdx( void) const noexcept
    {
        return _ChanIdx;
    }

    uint64_t Capacity( void) const noexcept
    {
        return _Capacity;
    }

    swarm::ComputeBuffer* Buffer( void) noexcept
    {
        return _Buffer.get();
    }

    const swarm::ComputeBuffer* Buffer( void) const noexcept
    {
        return _Buffer.get();
    }

    void WriteWord( uint32_t byteAddr, uint32_t val)
    {
        if ( !_Buffer) {
            return;
        }
        const uint32_t offset = byteAddr % static_cast< uint32_t>( _Capacity);
        silo::Buff< uint8_t> current = _Buffer->Read();
        if ( offset + 4 <= current.Size()) {
            std::memcpy( current.Data() + offset, &val, sizeof( val));
            _Buffer->Write( current.AsArr());
            _Stats._WritesServiced++;
            _Stats._BytesWritten += sizeof( val);
        }
    }

    uint32_t ReadWord( uint32_t byteAddr)
    {
        if ( !_Buffer) {
            return 0;
        }
        const uint32_t offset = byteAddr % static_cast< uint32_t>( _Capacity);
        silo::Buff< uint8_t> current = _Buffer->Read();
        uint32_t val = 0;
        if ( offset + 4 <= current.Size()) {
            std::memcpy( &val, current.Data() + offset, sizeof( val));
            _Stats._ReadsServiced++;
            _Stats._BytesRead += sizeof( val);
        }
        return val;
    }

    void Fill( uint32_t pattern)
    {
        if ( !_Buffer) {
            return;
        }
        const uint32_t numWords = static_cast< uint32_t>( _Capacity / 4);
        silo::Buff< uint32_t> words( numWords, [&]( uint32_t i) {
            return pattern + i;
        });
        silo::Arr< const uint8_t> byteView(
            reinterpret_cast< const uint8_t*>( words.Data()),
            numWords * 4
        );
        _Buffer->Write( byteView);
        _Stats._BytesWritten += numWords * 4;
    }

    bool Verify( uint32_t pattern) const
    {
        if ( !_Buffer) {
            return false;
        }
        const uint32_t numWords = static_cast< uint32_t>( _Capacity / 4);
        silo::Buff< uint8_t> raw = _Buffer->Read();
        if ( raw.Size() < numWords * 4) {
            return false;
        }
        const auto* words = reinterpret_cast< const uint32_t*>( raw.Data());
        for ( uint32_t i = 0; i < numWords; ++i) {
            if ( words[i] != pattern + i) {
                return false;
            }
        }
        return true;
    }

    DChanStats Stats( void) const noexcept
    {
        return _Stats;
    }
};

//-------------------------------------------------------------------------------------------------

} // namespace trellis::karst

