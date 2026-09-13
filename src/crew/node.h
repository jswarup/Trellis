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
    uint32_t            _MainPort{0};
    uint32_t            _AsyncPort{0};
    stalks::Atm< bool>  _IsOnline{false};
    std::deque< uint8_t> _RxQueue{};
    mutable stalks::Spinlock _Lock{};
    NodeStats           _Stats{};

public:
    CrewNode( uint32_t id, uint32_t mainPort, uint32_t asyncPort)
        : _Id( id)
        , _MainPort( mainPort)
        , _AsyncPort( asyncPort)
    {
    }

    ~CrewNode() = default;

    uint32_t Id() const noexcept
    {
        return _Id;
    }

    uint32_t MainPort() const noexcept
    {
        return _MainPort;
    }

    uint32_t AsyncPort() const noexcept
    {
        return _AsyncPort;
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
