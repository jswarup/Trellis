// crew_tests.cpp ---------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "crew/crew.h"
#include "stalks/atm.h"

#include <cstdlib>
#include <fstream>
#include <iostream>
#include <memory>
#include <string>

#if defined(_WIN32)
#include <windows.h>
#endif

using namespace trellis::crew;
using namespace trellis::stalks;

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Crew, ProtocolStructure)
{
    static_assert( sizeof( ProtocolMessage) == 24, "ProtocolMessage must be 24 bytes");

    ProtocolMessage msg{};
    msg._ActionId = static_cast< int32_t>( CoSimAction::Handshake);
    msg._Addr = 0x50000000ULL;
    msg._Value = 0x12345678ULL;
    msg._PeripheralIndex = -1;

    JEEVES_ASSERT_EQ( sizeof( msg), 24u);
    JEEVES_ASSERT_EQ( msg._ActionId, 10);
    JEEVES_ASSERT_EQ( msg._Addr, 0x50000000ULL);
    JEEVES_ASSERT_EQ( msg._Value, 0x12345678ULL);
    JEEVES_ASSERT_EQ( msg._PeripheralIndex, -1);

    JEEVES_ASSERT_EQ( REG_NODE_ID, 0x000u);
    JEEVES_ASSERT_EQ( REG_STATUS, 0x004u);
    JEEVES_ASSERT_EQ( REG_TX_DATA, 0x008u);
    JEEVES_ASSERT_EQ( REG_RX_DATA, 0x00Cu);
    JEEVES_ASSERT_EQ( REG_RX_COUNT, 0x010u);

    JEEVES_ASSERT_EQ( STATUS_TX_READY, 1u);
    JEEVES_ASSERT_EQ( STATUS_RX_READY, 2u);
    JEEVES_ASSERT_EQ( STATUS_PEER_UP, 4u);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Crew, NodeOperations)
{
    CrewNode node( 0);

    JEEVES_ASSERT_EQ( node.Id(), 0u);
    JEEVES_ASSERT( !node.IsOnline());

    node.SetOnline( true);
    JEEVES_ASSERT( node.IsOnline());

    JEEVES_ASSERT_EQ( node.RxCount(), 0u);

    node.PushRx( 0x41);
    node.PushRx( 0x42);
    JEEVES_ASSERT_EQ( node.RxCount(), 2u);

    uint8_t outByte = 0;
    bool pop1 = node.PopRx( outByte);
    JEEVES_ASSERT( pop1);
    JEEVES_ASSERT_EQ( outByte, 0x41);
    JEEVES_ASSERT_EQ( node.RxCount(), 1u);

    bool pop2 = node.PopRx( outByte);
    JEEVES_ASSERT( pop2);
    JEEVES_ASSERT_EQ( outByte, 0x42);
    JEEVES_ASSERT_EQ( node.RxCount(), 0u);

    bool pop3 = node.PopRx( outByte);
    JEEVES_ASSERT( !pop3);

    node.PushRx( 0x99);
    node.ClearRx();
    JEEVES_ASSERT_EQ( node.RxCount(), 0u);

    node.RecordRead();
    node.RecordWrite();
    node.RecordByteSent();

    NodeStats stats = node.GetStats();
    JEEVES_ASSERT_EQ( stats._ReadsServiced, 1u);
    JEEVES_ASSERT_EQ( stats._WritesServiced, 1u);
    JEEVES_ASSERT_EQ( stats._BytesSent, 1u);
    JEEVES_ASSERT_EQ( stats._BytesReceived, 2u);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Crew, HubNodeManagement)
{
    CrewHub hub;
    JEEVES_ASSERT_EQ( hub.NodeCount(), 0u);

    hub.AddNode( 0);
    hub.AddNode( 1);

    JEEVES_ASSERT_EQ( hub.NodeCount(), 2u);

    auto node0 = hub.FindNode( 0);
    auto node1 = hub.FindNode( 1);
    auto node2 = hub.FindNode( 2);

    JEEVES_ASSERT( node0 != nullptr);
    JEEVES_ASSERT( node1 != nullptr);
    JEEVES_ASSERT( node2 == nullptr);

    JEEVES_ASSERT_EQ( node0->Id(), 0u);
    JEEVES_ASSERT_EQ( node1->Id(), 1u);

    JEEVES_ASSERT( !hub.IsNodeOnline( 0));
    JEEVES_ASSERT( !hub.IsNodeOnline( 1));

    node0->SetOnline( true);
    JEEVES_ASSERT( hub.IsNodeOnline( 0));
    JEEVES_ASSERT( !hub.IsNodeOnline( 1));
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Crew, MmioReadRegisters)
{
    CrewHub hub;
    hub.AddNode( 0);
    hub.AddNode( 1);

    auto node0 = hub.FindNode( 0);
    auto node1 = hub.FindNode( 1);
    node0->SetOnline( true);
    node1->SetOnline( true);

    // Read Node ID
    ProtocolMessage reqNodeId{};
    reqNodeId._ActionId = static_cast< int32_t>( CoSimAction::ReadBusDword);
    reqNodeId._Addr = 0x50000000ULL | REG_NODE_ID;

    ProtocolMessage respNodeId = hub.HandleRequest( node0, reqNodeId);
    JEEVES_ASSERT_EQ( respNodeId._ActionId, static_cast< int32_t>( CoSimAction::Ok));
    JEEVES_ASSERT_EQ( respNodeId._Value, 0ULL);

    // Read Status (TX ready + Peer Up, no RX yet)
    ProtocolMessage reqStatus{};
    reqStatus._ActionId = static_cast< int32_t>( CoSimAction::ReadBusDword);
    reqStatus._Addr = 0x50000000ULL | REG_STATUS;

    ProtocolMessage respStatus = hub.HandleRequest( node0, reqStatus);
    JEEVES_ASSERT_EQ( respStatus._Value, static_cast< uint64_t>( STATUS_TX_READY | STATUS_PEER_UP));

    // Enqueue byte into node 0
    node0->PushRx( 0x7E);

    // Status now has RX_READY
    respStatus = hub.HandleRequest( node0, reqStatus);
    JEEVES_ASSERT_EQ( respStatus._Value, static_cast< uint64_t>( STATUS_TX_READY | STATUS_RX_READY | STATUS_PEER_UP));

    // Read RX count
    ProtocolMessage reqRxCount{};
    reqRxCount._ActionId = static_cast< int32_t>( CoSimAction::ReadBusDword);
    reqRxCount._Addr = 0x50000000ULL | REG_RX_COUNT;

    ProtocolMessage respRxCount = hub.HandleRequest( node0, reqRxCount);
    JEEVES_ASSERT_EQ( respRxCount._Value, 1ULL);

    // Read RX Data
    ProtocolMessage reqRxData{};
    reqRxData._ActionId = static_cast< int32_t>( CoSimAction::ReadBusDword);
    reqRxData._Addr = 0x50000000ULL | REG_RX_DATA;

    ProtocolMessage respRxData = hub.HandleRequest( node0, reqRxData);
    JEEVES_ASSERT_EQ( respRxData._Value, 0x7EULL);
    JEEVES_ASSERT_EQ( node0->RxCount(), 0u);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Crew, MmioInterVmRouting)
{
    CrewHub hub;
    hub.AddNode( 0);
    hub.AddNode( 1);

    auto node0 = hub.FindNode( 0);
    auto node1 = hub.FindNode( 1);

    uint32_t lastSrc = 99;
    uint32_t lastDst = 99;
    uint8_t  lastByte = 0;

    hub.SetMessageCallback( [&]( uint32_t src, uint32_t dst, uint8_t byte) {
        lastSrc = src;
        lastDst = dst;
        lastByte = byte;
    });

    // Node 0 writes a byte to TX_DATA
    ProtocolMessage writeReq{};
    writeReq._ActionId = static_cast< int32_t>( CoSimAction::WriteBusByte);
    writeReq._Addr = 0x50000000ULL | REG_TX_DATA;
    writeReq._Value = 'Z';

    ProtocolMessage writeResp = hub.HandleRequest( node0, writeReq);
    JEEVES_ASSERT_EQ( writeResp._ActionId, static_cast< int32_t>( CoSimAction::Ok));

    // Callback fired
    JEEVES_ASSERT_EQ( lastSrc, 0u);
    JEEVES_ASSERT_EQ( lastDst, 1u);
    JEEVES_ASSERT_EQ( lastByte, static_cast< uint8_t>( 'Z'));

    // Node 1 received the byte in RX queue
    JEEVES_ASSERT_EQ( node1->RxCount(), 1u);

    // Node 1 reads RX_DATA
    ProtocolMessage readReq{};
    readReq._ActionId = static_cast< int32_t>( CoSimAction::ReadBusByte);
    readReq._Addr = 0x50000000ULL | REG_RX_DATA;

    ProtocolMessage readResp = hub.HandleRequest( node1, readReq);
    JEEVES_ASSERT_EQ( readResp._Value, static_cast< uint64_t>( 'Z'));

    NodeStats s0 = hub.GetNodeStats( 0);
    NodeStats s1 = hub.GetNodeStats( 1);

    JEEVES_ASSERT_EQ( s0._BytesSent, 1u);
    JEEVES_ASSERT_EQ( s0._WritesServiced, 1u);
    JEEVES_ASSERT_EQ( s1._BytesReceived, 1u);
    JEEVES_ASSERT_EQ( s1._ReadsServiced, 1u);

    if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
        std::cout << "         [Crew Routing Diagnostics]\n";
        std::cout << "           Routed Byte : 0x" << std::hex << static_cast< int>( lastByte)
                  << " ('" << static_cast< char>( lastByte) << "')\n";
        std::cout << "           Route Path  : VM" << std::dec << lastSrc << " -> VM" << lastDst << "\n";
        std::cout << "           VM0 Stats   : Reads=" << s0._ReadsServiced
                  << ", Writes=" << s0._WritesServiced
                  << ", Sent=" << s0._BytesSent
                  << ", Recv=" << s0._BytesReceived << "\n";
        std::cout << "           VM1 Stats   : Reads=" << s1._ReadsServiced
                  << ", Writes=" << s1._WritesServiced
                  << ", Sent=" << s1._BytesSent
                  << ", Recv=" << s1._BytesReceived << "\n";
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Crew, VirtualExchangeProtocol)
{
    CrewHub     hub;
    hub.AddNode( 0);
    hub.AddNode( 1);

    auto node0 = hub.FindNode( 0);
    auto node1 = hub.FindNode( 1);
    node0->SetOnline( true);
    node1->SetOnline( true);

    const std::string msgVm0 = "Hello World from Zephyr VM0!\n";
    const std::string msgVm1 = "Hello World back from Zephyr VM1!\n";

    // 1. VM0 sends msgVm0
    for ( char c : msgVm0) {
        ProtocolMessage req{};
        req._ActionId = static_cast< int32_t>( CoSimAction::WriteBusByte);
        req._Addr = 0x50000000ULL | REG_TX_DATA;
        req._Value = static_cast< uint8_t>( c);
        hub.HandleRequest( node0, req);
    }

    // 2. VM1 reads message
    std::string receivedByVm1;
    while ( true) {
        ProtocolMessage statusReq{};
        statusReq._ActionId = static_cast< int32_t>( CoSimAction::ReadBusDword);
        statusReq._Addr = 0x50000000ULL | REG_STATUS;
        ProtocolMessage statusResp = hub.HandleRequest( node1, statusReq);

        if ( !( statusResp._Value & STATUS_RX_READY)) {
            break;
        }

        ProtocolMessage rxReq{};
        rxReq._ActionId = static_cast< int32_t>( CoSimAction::ReadBusByte);
        rxReq._Addr = 0x50000000ULL | REG_RX_DATA;
        ProtocolMessage rxResp = hub.HandleRequest( node1, rxReq);

        receivedByVm1.push_back( static_cast< char>( rxResp._Value & 0xFF));
    }

    JEEVES_ASSERT_EQ( receivedByVm1, msgVm0);

    // 3. VM1 replies msgVm1
    for ( char c : msgVm1) {
        ProtocolMessage req{};
        req._ActionId = static_cast< int32_t>( CoSimAction::WriteBusByte);
        req._Addr = 0x50000000ULL | REG_TX_DATA;
        req._Value = static_cast< uint8_t>( c);
        hub.HandleRequest( node1, req);
    }

    // 4. VM0 reads reply
    std::string receivedByVm0;
    while ( true) {
        ProtocolMessage statusReq{};
        statusReq._ActionId = static_cast< int32_t>( CoSimAction::ReadBusDword);
        statusReq._Addr = 0x50000000ULL | REG_STATUS;
        ProtocolMessage statusResp = hub.HandleRequest( node0, statusReq);

        if ( !( statusResp._Value & STATUS_RX_READY)) {
            break;
        }

        ProtocolMessage rxReq{};
        rxReq._ActionId = static_cast< int32_t>( CoSimAction::ReadBusByte);
        rxReq._Addr = 0x50000000ULL | REG_RX_DATA;
        ProtocolMessage rxResp = hub.HandleRequest( node0, rxReq);

        receivedByVm0.push_back( static_cast< char>( rxResp._Value & 0xFF));
    }

    JEEVES_ASSERT_EQ( receivedByVm0, msgVm1);

    NodeStats s0 = hub.GetNodeStats( 0);
    NodeStats s1 = hub.GetNodeStats( 1);

    JEEVES_ASSERT_EQ( s0._BytesSent, static_cast< uint32_t>( msgVm0.size()));
    JEEVES_ASSERT_EQ( s0._BytesReceived, static_cast< uint32_t>( msgVm1.size()));
    JEEVES_ASSERT_EQ( s1._BytesSent, static_cast< uint32_t>( msgVm1.size()));
    JEEVES_ASSERT_EQ( s1._BytesReceived, static_cast< uint32_t>( msgVm0.size()));

    if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
        std::cout << "         [Crew Transfer Diagnostics]\n";
        std::cout << "           VM0 -> VM1 Message : \"" << msgVm0.substr( 0, msgVm0.size() - 1) << "\"\n";
        std::cout << "           VM1 Bytes Received : " << receivedByVm1.size() << " bytes\n";
        std::cout << "           VM1 -> VM0 Reply   : \"" << msgVm1.substr( 0, msgVm1.size() - 1) << "\"\n";
        std::cout << "           VM0 Bytes Received : " << receivedByVm0.size() << " bytes\n";
        std::cout << "           Telemetry VM0      : Reads=" << s0._ReadsServiced
                  << ", Writes=" << s0._WritesServiced
                  << ", Sent=" << s0._BytesSent
                  << ", Recv=" << s0._BytesReceived << "\n";
        std::cout << "           Telemetry VM1      : Reads=" << s1._ReadsServiced
                  << ", Writes=" << s1._WritesServiced
                  << ", Sent=" << s1._BytesSent
                  << ", Recv=" << s1._BytesReceived << "\n";
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Crew, RubeMultiVmHelloWorldExchange)
{
    const std::string msgVm0 = "Hello World from Zephyr VM0!\n";
    const std::string msgVm1 = "Hello World back from Zephyr VM1!\n";

    auto runExchangeTest = [&]( bool parallelMode) {
        std::string receivedByVm1;
        std::string receivedByVm0;
        Atm< bool>  vm0Done{false};
        Atm< bool>  vm1Done{false};

        trellis::rube::Layout layout;

        // 1. VM 0 Coroutine: sends msgVm0, then reads reply until '\n'
        VMRunner vm0( layout, "VM0", [&msgVm0, &receivedByVm0, &vm0Done]() -> trellis::rube::CoroTask {
            trellis::rube::CoroPorts in = co_await trellis::rube::CoroIn{};

            // Step 1: Send msgVm0 byte-by-byte
            for ( char c : msgVm0) {
                VM_MMIO_WRITE( in, REG_TX_DATA, static_cast< uint8_t>( c));
            }

            // Step 2: Read reply from VM 1
            std::string reply;
            while ( true) {
                uint32_t status = 0;
                VM_MMIO_READ( in, REG_STATUS, status);
                if ( status & STATUS_RX_READY) {
                    uint32_t byteVal = 0;
                    VM_MMIO_READ( in, REG_RX_DATA, byteVal);
                    reply.push_back( static_cast< char>( byteVal & 0xFF));
                    if ( byteVal == '\n') {
                        break;
                    }
                }
            }
            receivedByVm0 = reply;
            vm0Done.Store( true, std::memory_order_release);

            while ( true) {
                in = co_yield VmBus::Idle();
            }
        });

        // 2. VM 1 Coroutine: reads message from VM 0 until '\n', then sends msgVm1
        VMRunner vm1( layout, "VM1", [&msgVm1, &receivedByVm1, &vm1Done]() -> trellis::rube::CoroTask {
            trellis::rube::CoroPorts in = co_await trellis::rube::CoroIn{};

            // Step 1: Read incoming message from VM 0
            std::string msg;
            while ( true) {
                uint32_t status = 0;
                VM_MMIO_READ( in, REG_STATUS, status);
                if ( status & STATUS_RX_READY) {
                    uint32_t byteVal = 0;
                    VM_MMIO_READ( in, REG_RX_DATA, byteVal);
                    msg.push_back( static_cast< char>( byteVal & 0xFF));
                    if ( byteVal == '\n') {
                        break;
                    }
                }
            }
            receivedByVm1 = msg;

            // Step 2: Reply with msgVm1
            for ( char c : msgVm1) {
                VM_MMIO_WRITE( in, REG_TX_DATA, static_cast< uint8_t>( c));
            }

            vm1Done.Store( true, std::memory_order_release);

            while ( true) {
                in = co_yield VmBus::Idle();
            }
        });

        // 3. Instantiate Adaptors and connect to VMs
        VMAdaptor ad0( layout, "Adaptor0", 0, &vm0);
        VMAdaptor ad1( layout, "Adaptor1", 1, &vm1);

        // 4. Interconnect Adaptors
        VMAdaptor::Connect( layout, ad0, ad1);

        layout.Freeze();

        // 5. Drive simulation
        trellis::rube::SimEngine engine = trellis::rube::SimEngine::Create( layout);
        if ( parallelMode) {
            trellis::heist::Atelier::Reset( 4);
            engine.WithMode( trellis::rube::SimEngineMode::Parallel( 4));
        }

        uint32_t cycles = 0;
        const uint32_t maxCycles = 1500;
        while ( cycles < maxCycles && ( !vm0Done.Load( std::memory_order_acquire) ||
                                        !vm1Done.Load( std::memory_order_acquire)))
        {
            engine.Drive();
            cycles++;
        }

        JEEVES_ASSERT( vm0Done.Load( std::memory_order_acquire));
        JEEVES_ASSERT( vm1Done.Load( std::memory_order_acquire));
        JEEVES_ASSERT_EQ( receivedByVm1, msgVm0);
        JEEVES_ASSERT_EQ( receivedByVm0, msgVm1);

        NodeStats s0 = ad0.GetStats();
        NodeStats s1 = ad1.GetStats();

        JEEVES_ASSERT_EQ( s0._BytesSent, static_cast< uint32_t>( msgVm0.size()));
        JEEVES_ASSERT_EQ( s0._BytesReceived, static_cast< uint32_t>( msgVm1.size()));
        JEEVES_ASSERT_EQ( s1._BytesSent, static_cast< uint32_t>( msgVm1.size()));
        JEEVES_ASSERT_EQ( s1._BytesReceived, static_cast< uint32_t>( msgVm0.size()));

        if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
            std::cout << "         [Rube Multi-VM Exchange Diagnostics - "
                      << ( parallelMode ? "Parallel" : "Serial") << "]\n";
            std::cout << "           Cycles Elapsed     : " << cycles << " delta cycles\n";
            std::cout << "           VM0 -> VM1 Message : \"" << msgVm0.substr( 0, msgVm0.size() - 1) << "\"\n";
            std::cout << "           VM1 Bytes Received : " << receivedByVm1.size() << " bytes\n";
            std::cout << "           VM1 -> VM0 Reply   : \"" << msgVm1.substr( 0, msgVm1.size() - 1) << "\"\n";
            std::cout << "           VM0 Bytes Received : " << receivedByVm0.size() << " bytes\n";
            std::cout << "           Telemetry Adaptor0 : Reads=" << s0._ReadsServiced
                      << ", Writes=" << s0._WritesServiced
                      << ", Sent=" << s0._BytesSent
                      << ", Recv=" << s0._BytesReceived << "\n";
            std::cout << "           Telemetry Adaptor1 : Reads=" << s1._ReadsServiced
                      << ", Writes=" << s1._WritesServiced
                      << ", Sent=" << s1._BytesSent
                      << ", Recv=" << s1._BytesReceived << "\n";
        }
    };

    // Run in Serial mode
    runExchangeTest( false);

    // Run in Parallel mode
    runExchangeTest( true);
}

//-------------------------------------------------------------------------------------------------
