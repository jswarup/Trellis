// hub.cpp -----------------------------------------------------------------------------------------------------

#include "crew/hub.h"

//-------------------------------------------------------------------------------------------------

namespace trellis::crew {

//-------------------------------------------------------------------------------------------------

void CrewHub::AddNode( uint32_t id)
{
    auto lock = _HubLock.Lock();
    _Nodes.PushBack( std::make_unique< CrewNode>( id));
}

//-------------------------------------------------------------------------------------------------

size_t CrewHub::NodeCount() const
{
    auto lock = _HubLock.Lock();
    return _Nodes.Size();
}

//-------------------------------------------------------------------------------------------------

CrewNode* CrewHub::FindNode( uint32_t id) const
{
    auto lock = _HubLock.Lock();
    for ( const auto& node : _Nodes) {
        if ( node->Id() == id) {
            return node.get();
        }
    }
    return nullptr;
}

//-------------------------------------------------------------------------------------------------

bool CrewHub::IsNodeOnline( uint32_t id) const
{
    auto node = FindNode( id);
    if ( node != nullptr) {
        return node->IsOnline();
    }
    return false;
}

//-------------------------------------------------------------------------------------------------

NodeStats CrewHub::GetNodeStats( uint32_t id) const
{
    auto node = FindNode( id);
    if ( node != nullptr) {
        return node->GetStats();
    }
    return NodeStats{};
}

//-------------------------------------------------------------------------------------------------

void CrewHub::SetMessageCallback( MessageCallback cb)
{
    _MessageCb = cb;
}

//-------------------------------------------------------------------------------------------------

ProtocolMessage CrewHub::HandleRequest( CrewNode* node, const ProtocolMessage& req)
{
    ProtocolMessage resp;
    resp._ActionId = static_cast< int32_t>( CoSimAction::Ok);
    resp._Addr = req._Addr;
    resp._Value = 0;
    resp._PeripheralIndex = req._PeripheralIndex;

    const uint64_t reg = req._Addr & 0xFFF;
    const auto action = static_cast< CoSimAction>( req._ActionId);

    if ( action == CoSimAction::ReadBus ||
         action == CoSimAction::ReadBusByte ||
         action == CoSimAction::ReadBusWord ||
         action == CoSimAction::ReadBusDword ||
         action == CoSimAction::ReadBusQword) {

        node->RecordRead();

        switch ( reg) {
        case REG_NODE_ID:
            resp._Value = static_cast< uint64_t>( node->Id());
            break;

        case REG_STATUS: {
            uint32_t status = STATUS_TX_READY;
            if ( node->RxCount() > 0) {
                status |= STATUS_RX_READY;
            }
            const uint32_t peerId = 1 - node->Id();
            if ( IsNodeOnline( peerId)) {
                status |= STATUS_PEER_UP;
            }
            resp._Value = static_cast< uint64_t>( status);
            break;
        }

        case REG_TX_DATA:
            resp._Value = 0;
            break;

        case REG_RX_DATA: {
            uint8_t byte = 0;
            if ( node->PopRx( byte)) {
                resp._Value = static_cast< uint64_t>( byte);
            } else {
                resp._Value = 0;
            }
            break;
        }

        case REG_RX_COUNT: {
            resp._Value = static_cast< uint64_t>( node->RxCount());
            break;
        }

        default:
            resp._Value = 0;
            break;
        }
    } else if ( action == CoSimAction::WriteBusByte ||
                action == CoSimAction::WriteBusWord ||
                action == CoSimAction::WriteBusDword ||
                action == CoSimAction::WriteBusQword) {

        node->RecordWrite();

        if ( reg == REG_TX_DATA) {
            const uint8_t byte = static_cast< uint8_t>( req._Value & 0xFF);
            node->RecordByteSent();

            const uint32_t peerId = 1 - node->Id();
            auto peer = FindNode( peerId);
            if ( peer != nullptr) {
                peer->PushRx( byte);
                if ( _MessageCb) {
                    _MessageCb( node->Id(), peerId, byte);
                }
            }
        }
    } else if ( action == CoSimAction::ResetPeripheral) {
        node->ClearRx();
    }

    return resp;
}

} // namespace trellis::crew
