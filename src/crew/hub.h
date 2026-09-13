// hub.h ----------------------------------------------------------------------------------------------------------
#pragma once

#include "crew/node.h"
#include "crew/protocol.h"

#include <atomic>
#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <thread>
#include <vector>

//-------------------------------------------------------------------------------------------------

namespace trellis::crew {

//-------------------------------------------------------------------------------------------------
// Callback signature for monitoring byte-level routing between VM nodes.

using MessageCallback = std::function< void( uint32_t srcNode, uint32_t dstNode, uint8_t byte)>;

//-------------------------------------------------------------------------------------------------
// Interface contract for the co-simulation coordinator hub.

class ICrewHub
{
public:
    virtual             ~ICrewHub() = default;

    virtual void        AddNode( uint32_t id, uint32_t mainPort, uint32_t asyncPort) = 0;
    virtual bool        Start() = 0;
    virtual void        Stop() = 0;
    virtual bool        IsNodeOnline( uint32_t id) const = 0;
    virtual NodeStats   GetNodeStats( uint32_t id) const = 0;
    virtual void        SetMessageCallback( MessageCallback cb) = 0;
};

//-------------------------------------------------------------------------------------------------
// Concrete co-simulation hub managing VM nodes, worker threads, and socket MMIO dispatch.

class CrewHub : public ICrewHub
{
private:
    std::vector< std::shared_ptr< CrewNode>> _Nodes{};
    std::vector< std::thread>               _Workers{};
    std::atomic< bool>                      _IsRunning{false};
    MessageCallback                         _MessageCb{nullptr};
    mutable std::mutex                      _HubMutex{};

public:
    CrewHub();
    virtual ~CrewHub() override;

    virtual void        AddNode( uint32_t id, uint32_t mainPort, uint32_t asyncPort) override;
    virtual bool        Start() override;
    virtual void        Stop() override;
    virtual bool        IsNodeOnline( uint32_t id) const override;
    virtual NodeStats   GetNodeStats( uint32_t id) const override;
    virtual void        SetMessageCallback( MessageCallback cb) override;

    size_t              NodeCount() const;
    std::shared_ptr< CrewNode> FindNode( uint32_t id) const;
    ProtocolMessage     HandleRequest( std::shared_ptr< CrewNode> node, const ProtocolMessage& req);

private:
    void                WorkerLoop( std::shared_ptr< CrewNode> node);
};

} // namespace trellis::crew
