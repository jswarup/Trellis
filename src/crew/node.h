// node.h ---------------------------------------------------------------------------------------------------------
#pragma once

#include "crew/protocol.h"
#include "stalks/atm.h"

#include <cstdint>
#include <deque>

//-------------------------------------------------------------------------------------------------

namespace trellis::crew {

//-------------------------------------------------------------------------------------------------
// Node performance and co-simulation metrics.

struct NodeStats
{
    uint32_t            _BytesSent{0};
    uint32_t            _BytesReceived{0};
    uint32_t            _ReadsServiced{0};
    uint32_t            _WritesServiced{0};
};

//-------------------------------------------------------------------------------------------------
// Concrete representation of a co-simulated VM node endpoint.

class CrewNode
{
private:
    uint32_t            _Id{0};
    stalks::Atm< bool>  _IsOnline{false};
    std::deque< uint8_t> _RxQueue{};
    mutable stalks::Spinlock _Lock{};
    NodeStats           _Stats{};

public:
    explicit CrewNode( uint32_t id)
        : _Id( id)
    {
    }

    ~CrewNode() = default;

    uint32_t Id() const noexcept
    {
        return _Id;
    }

    bool IsOnline() const noexcept
    {
        return _IsOnline.Load();
    }

    void SetOnline( bool online) noexcept
    {
        _IsOnline.Store( online);
    }

    void PushRx( uint8_t byte)
    {
        auto lock = _Lock.Lock();
        _RxQueue.push_back( byte);
    }

    bool PopRx( uint8_t& outByte)
    {
        auto lock = _Lock.Lock();
        if ( _RxQueue.empty()) {
            return false;
        }
        outByte = _RxQueue.front();
        _RxQueue.pop_front();
        _Stats._BytesReceived++;
        return true;
    }

    uint32_t RxCount() const
    {
        auto lock = _Lock.Lock();
        return static_cast< uint32_t>( _RxQueue.size());
    }

    void ClearRx()
    {
        auto lock = _Lock.Lock();
        _RxQueue.clear();
    }

    NodeStats GetStats() const
    {
        auto lock = _Lock.Lock();
        return _Stats;
    }

    void ResetStats()
    {
        auto lock = _Lock.Lock();
        _Stats = NodeStats{};
    }

    void RecordRead()
    {
        auto lock = _Lock.Lock();
        _Stats._ReadsServiced++;
    }

    void RecordWrite()
    {
        auto lock = _Lock.Lock();
        _Stats._WritesServiced++;
    }

    void RecordByteSent()
    {
        auto lock = _Lock.Lock();
        _Stats._BytesSent++;
    }
};

//-------------------------------------------------------------------------------------------------
} // namespace trellis::crew
