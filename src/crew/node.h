// node.h ---------------------------------------------------------------------------------------------------------
#pragma once

#include "crew/protocol.h"

#include <atomic>
#include <cstdint>
#include <deque>
#include <mutex>

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
    std::atomic< bool>  _IsOnline{false};
    std::deque< uint8_t> _RxQueue{};
    mutable std::mutex  _Mutex{};
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
        return _IsOnline.load();
    }

    void SetOnline( bool online) noexcept
    {
        _IsOnline.store( online);
    }

    void PushRx( uint8_t byte)
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        _RxQueue.push_back( byte);
    }

    bool PopRx( uint8_t& outByte)
    {
        std::lock_guard< std::mutex> lock( _Mutex);
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
        std::lock_guard< std::mutex> lock( _Mutex);
        return static_cast< uint32_t>( _RxQueue.size());
    }

    void ClearRx()
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        _RxQueue.clear();
    }

    NodeStats GetStats() const
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        return _Stats;
    }

    void ResetStats()
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        _Stats = NodeStats{};
    }

    void RecordRead()
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        _Stats._ReadsServiced++;
    }

    void RecordWrite()
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        _Stats._WritesServiced++;
    }

    void RecordByteSent()
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        _Stats._BytesSent++;
    }
};

} // namespace trellis::crew
