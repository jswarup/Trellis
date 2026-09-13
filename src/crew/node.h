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
// Interface contract for co-simulated VM node.

class ICrewNode
{
public:
    virtual             ~ICrewNode() = default;

    virtual uint32_t    Id() const noexcept = 0;
    virtual uint32_t    MainPort() const noexcept = 0;
    virtual uint32_t    AsyncPort() const noexcept = 0;
    virtual bool        IsOnline() const noexcept = 0;
    virtual void        SetOnline( bool online) noexcept = 0;
    virtual void        PushRx( uint8_t byte) = 0;
    virtual bool        PopRx( uint8_t& outByte) = 0;
    virtual uint32_t    RxCount() const = 0;
    virtual void        ClearRx() = 0;
    virtual NodeStats   GetStats() const = 0;
    virtual void        ResetStats() = 0;
};

//-------------------------------------------------------------------------------------------------
// Concrete representation of a co-simulated VM node endpoint.

class CrewNode : public ICrewNode
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

    virtual ~CrewNode() override = default;

    virtual uint32_t Id() const noexcept override
    {
        return _Id;
    }

    virtual uint32_t MainPort() const noexcept override
    {
        return _MainPort;
    }

    virtual uint32_t AsyncPort() const noexcept override
    {
        return _AsyncPort;
    }

    virtual bool IsOnline() const noexcept override
    {
        return _IsOnline.load();
    }

    virtual void SetOnline( bool online) noexcept override
    {
        _IsOnline.store( online);
    }

    virtual void PushRx( uint8_t byte) override
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        _RxQueue.push_back( byte);
    }

    virtual bool PopRx( uint8_t& outByte) override
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

    virtual uint32_t RxCount() const override
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        return static_cast< uint32_t>( _RxQueue.size());
    }

    virtual void ClearRx() override
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        _RxQueue.clear();
    }

    virtual NodeStats GetStats() const override
    {
        std::lock_guard< std::mutex> lock( _Mutex);
        return _Stats;
    }

    virtual void ResetStats() override
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
