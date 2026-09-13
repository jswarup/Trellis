// hub.cpp -----------------------------------------------------------------------------------------------------

#include "crew/hub.h"

#include <chrono>
#include <cstring>
#include <iostream>

#if defined(_WIN32)
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <winsock2.h>
#include <ws2tcpip.h>
#pragma comment(lib, "ws2_32.lib")
using SocketType = SOCKET;
constexpr SocketType InvalidSocket = INVALID_SOCKET;
#else
#include <arpa/inet.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/socket.h>
#include <unistd.h>
using SocketType = int;
constexpr SocketType InvalidSocket = -1;
#define closesocket close
#endif

//-------------------------------------------------------------------------------------------------

namespace trellis::crew {

//-------------------------------------------------------------------------------------------------

static int RecvExact( SocketType s, char* buf, int len)
{
    int total = 0;
    while ( total < len) {
        int r = recv( s, buf + total, len - total, 0);
        if ( r <= 0) {
            return r;
        }
        total += r;
    }
    return total;
}

//-------------------------------------------------------------------------------------------------

static int SendExact( SocketType s, const char* buf, int len)
{
    int total = 0;
    while ( total < len) {
        int r = send( s, buf + total, len - total, 0);
        if ( r <= 0) {
            return r;
        }
        total += r;
    }
    return total;
}

//-------------------------------------------------------------------------------------------------

CrewHub::CrewHub()
{
}

//-------------------------------------------------------------------------------------------------

CrewHub::~CrewHub()
{
    Stop();
}

//-------------------------------------------------------------------------------------------------

void CrewHub::AddNode( uint32_t id, uint32_t mainPort, uint32_t asyncPort)
{
    std::lock_guard< std::mutex> lock( _HubMutex);
    _Nodes.push_back( std::make_shared< CrewNode>( id, mainPort, asyncPort));
}

//-------------------------------------------------------------------------------------------------

size_t CrewHub::NodeCount() const
{
    std::lock_guard< std::mutex> lock( _HubMutex);
    return _Nodes.size();
}

//-------------------------------------------------------------------------------------------------

std::shared_ptr< CrewNode> CrewHub::FindNode( uint32_t id) const
{
    std::lock_guard< std::mutex> lock( _HubMutex);
    for ( const auto& node : _Nodes) {
        if ( node->Id() == id) {
            return node;
        }
    }
    return nullptr;
}

//-------------------------------------------------------------------------------------------------

bool CrewHub::Start()
{
    if ( _IsRunning.load()) {
        return true;
    }

#if defined(_WIN32)
    WSADATA wsaData;
    if ( WSAStartup( MAKEWORD( 2, 2), &wsaData) != 0) {
        return false;
    }
#endif

    _IsRunning.store( true);

    std::lock_guard< std::mutex> lock( _HubMutex);
    for ( auto& node : _Nodes) {
        _Workers.emplace_back( &CrewHub::WorkerLoop, this, node);
    }

    return true;
}

//-------------------------------------------------------------------------------------------------

void CrewHub::Stop()
{
    if ( !_IsRunning.load()) {
        return;
    }
    _IsRunning.store( false);

    for ( auto& worker : _Workers) {
        if ( worker.joinable()) {
            worker.join();
        }
    }
    _Workers.clear();

#if defined(_WIN32)
    WSACleanup();
#endif
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

void CrewHub::WorkerLoop( std::shared_ptr< CrewNode> node)
{
    SocketType sMain = InvalidSocket;
    sockaddr_in addr;
    std::memset( &addr, 0, sizeof( addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons( static_cast< uint16_t>( node->MainPort()));
    inet_pton( AF_INET, "127.0.0.1", &addr.sin_addr);

    const int maxRetries = 100;
    for ( int retry = 0; retry < maxRetries && _IsRunning.load(); ++retry) {
        sMain = socket( AF_INET, SOCK_STREAM, IPPROTO_TCP);
        if ( sMain == InvalidSocket) {
            std::this_thread::sleep_for( std::chrono::milliseconds( 100));
            continue;
        }
        if ( connect( sMain, reinterpret_cast< sockaddr*>( &addr), sizeof( addr)) == 0) {
            break;
        }
        closesocket( sMain);
        sMain = InvalidSocket;
        std::this_thread::sleep_for( std::chrono::milliseconds( 100));
    }

    if ( sMain == InvalidSocket || !_IsRunning.load()) {
        return;
    }

    int noDelay = 1;
    setsockopt( sMain, IPPROTO_TCP, TCP_NODELAY, reinterpret_cast< const char*>( &noDelay), sizeof( noDelay));

    SocketType sAsync = InvalidSocket;
    sockaddr_in asyncAddr;
    std::memset( &asyncAddr, 0, sizeof( asyncAddr));
    asyncAddr.sin_family = AF_INET;
    asyncAddr.sin_port = htons( static_cast< uint16_t>( node->AsyncPort()));
    inet_pton( AF_INET, "127.0.0.1", &asyncAddr.sin_addr);

    for ( int retry = 0; retry < maxRetries && _IsRunning.load(); ++retry) {
        sAsync = socket( AF_INET, SOCK_STREAM, IPPROTO_TCP);
        if ( sAsync == InvalidSocket) {
            std::this_thread::sleep_for( std::chrono::milliseconds( 100));
            continue;
        }
        if ( connect( sAsync, reinterpret_cast< sockaddr*>( &asyncAddr), sizeof( asyncAddr)) == 0) {
            break;
        }
        closesocket( sAsync);
        sAsync = InvalidSocket;
        std::this_thread::sleep_for( std::chrono::milliseconds( 100));
    }

    if ( sAsync == InvalidSocket || !_IsRunning.load()) {
        closesocket( sMain);
        return;
    }

    // Renode Handshake
    ProtocolMessage hsReq;
    int r = RecvExact( sMain, reinterpret_cast< char*>( &hsReq), sizeof( hsReq));
    if ( r != sizeof( hsReq) || hsReq._ActionId != static_cast< int32_t>( CoSimAction::Handshake)) {
        closesocket( sMain);
        closesocket( sAsync);
        return;
    }

    ProtocolMessage hsResp;
    hsResp._ActionId = static_cast< int32_t>( CoSimAction::Handshake);
    hsResp._Addr = 0;
    hsResp._Value = 0;
    hsResp._PeripheralIndex = -1;
    SendExact( sMain, reinterpret_cast< const char*>( &hsResp), sizeof( hsResp));
    node->SetOnline( true);

    while ( _IsRunning.load()) {
        ProtocolMessage req;
        int rec = RecvExact( sMain, reinterpret_cast< char*>( &req), sizeof( req));
        if ( rec <= 0) {
            break;
        }

        if ( req._ActionId == static_cast< int32_t>( CoSimAction::Disconnect)) {
            break;
        }

        ProtocolMessage resp = HandleRequest( node, req);
        if ( SendExact( sMain, reinterpret_cast< const char*>( &resp), sizeof( resp)) <= 0) {
            break;
        }
    }

    node->SetOnline( false);
    closesocket( sMain);
    closesocket( sAsync);
}

//-------------------------------------------------------------------------------------------------

ProtocolMessage CrewHub::HandleRequest( std::shared_ptr< CrewNode> node, const ProtocolMessage& req)
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
