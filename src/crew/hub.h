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
// Concrete co-simulation hub managing VM nodes, worker threads, and socket MMIO dispatch.

class CrewHub
{
private:
    std::vector< std::shared_ptr< CrewNode>> _Nodes{};
    std::vector< std::thread>               _Workers{};
    std::atomic< bool>                      _IsRunning{false};
    MessageCallback                         _MessageCb{nullptr};
    mutable std::mutex                      _HubMutex{};

public:
    CrewHub();
    ~CrewHub();

    void                AddNode( uint32_t id, uint32_t mainPort, uint32_t asyncPort);
    bool                Start();
    void                Stop();
    bool                IsNodeOnline( uint32_t id) const;
    NodeStats           GetNodeStats( uint32_t id) const;
    void                SetMessageCallback( MessageCallback cb);

    size_t              NodeCount() const;
    std::shared_ptr< CrewNode> FindNode( uint32_t id) const;
    ProtocolMessage     HandleRequest( std::shared_ptr< CrewNode> node, const ProtocolMessage& req);

private:
    void                WorkerLoop( std::shared_ptr< CrewNode> node);
};

} // namespace trellis::crew
