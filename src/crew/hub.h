// hub.h ----------------------------------------------------------------------------------------------------------
#pragma once

#include "crew/node.h"
#include "crew/protocol.h"

#include "silo/buff.h"
#include "silo/stash.h"
#include "stalks/atm.h"

#include <cstdint>
#include <functional>
#include <memory>
#include <thread>

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
    silo::Stash< std::unique_ptr< CrewNode>> _Nodes{};
    silo::Buff< std::thread>                _Workers{};
    stalks::Atm< bool>                      _IsRunning{false};
    MessageCallback                         _MessageCb{nullptr};
    mutable stalks::Spinlock                _HubLock{};

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
    CrewNode*           FindNode( uint32_t id) const;
    ProtocolMessage     HandleRequest( CrewNode* node, const ProtocolMessage& req);

private:
    void                WorkerLoop( CrewNode* node);
};

} // namespace trellis::crew
