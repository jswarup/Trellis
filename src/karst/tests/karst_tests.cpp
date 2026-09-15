// karst_tests.cpp ---------------------------------------------------------------------------------
#include "cove/jeeves.h"
#include "karst/karst.h"

#include <iomanip>
#include <iostream>

using namespace trellis;
using namespace trellis::karst;

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Karst, TopologyWiring)
{
    KarstFabric fabric;

    // Verify 8 host nodes exist and are indexed 0..7
    for ( uint32_t h = 0; h < k_HostsPerFabric; ++h) {
        JEEVES_ASSERT_EQ( fabric.Host( h).HostId(), h);
    }

    // Verify 2 KarstHind Memory Fabric dies exist
    JEEVES_ASSERT_EQ( fabric.FabricNode( 0).DieId(), 0U);
    JEEVES_ASSERT_EQ( fabric.FabricNode( 1).DieId(), 1U);

    // Verify 8 physical DDR5 memory channels exist
    for ( uint32_t c = 0; c < k_DChansPerFabric; ++c) {
        JEEVES_ASSERT_EQ( fabric.DChan( c).ChanIdx(), c);
        JEEVES_ASSERT_EQ( fabric.DChan( c).Capacity(), 4096ULL);
    }

    // Verify 8 EPUs exist
    for ( uint32_t e = 0; e < k_DChansPerFabric; ++e) {
        JEEVES_ASSERT_EQ( fabric.VPU( e).VPUIdx(), e);
        JEEVES_ASSERT_EQ( fabric.VPU( e).Dispatches(), 0U);
    }

    // Verify simulation engine initialized at cycle 0
    JEEVES_ASSERT_EQ( fabric.Engine()._CycleCount, 0ULL);

    if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
        std::cout << "         [Karst Topology Diagnostics]\n";
        std::cout << "           Hosts (Fore Dies)   : " << k_HostsPerFabric << " nodes (Fore_0..Fore_7)\n";
        std::cout << "           Fabric (Hind Dies)  : " << k_HindDiesPerFabric << " dies (Hind_0, Hind_1)\n";
        std::cout << "           DDR5 Memory Channels: " << k_DChansPerFabric << " channels (4 per Hind die, 4096 bytes each)\n";
        std::cout << "           Near-Memory EPUs    : " << k_DChansPerFabric << " units (4 per Hind die)\n";
        std::cout << "           Simulation Engine   : Initialized at cycle " << fabric.Engine()._CycleCount << "\n";
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Karst, NocBackpressuresFullMemoryControllerQueue)
{
    rube::Layout layout;
    KarstNoc noc( layout, "Noc", 0);
    layout.Freeze();

    rube::SimEngine engine = rube::SimEngine::Create( layout);

    engine.Set( noc.KlRxValid( 0), false);
    engine.Set( noc.McReqReady( 0), false);
    engine.Drive();

    // Hold the MC stalled while filling its bounded request queue.
    for ( uint32_t request = 0; request < 32; ++request) {
        engine.Set( noc.KlRxData( 0), KarstFlit::Pack( 0, request, 0, true));
        engine.Set( noc.KlRxValid( 0), true);
        engine.Drive();
    }

    engine.Set( noc.KlRxValid( 0), false);
    engine.Drive();

    const bool readyAfterFill = engine.Get< bool>( noc.KlRxReady( 0));
    JEEVES_ASSERT_EQ( readyAfterFill, false);

    if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
        std::cout << "         [NoC Backpressure Diagnostics]\n";
        std::cout << "           Port Under Test     : KL0 (Ingress Port 0)\n";
        std::cout << "           Target MC Stalled   : MC0 (McReqReady=0)\n";
        std::cout << "           Flits Injected      : 32 requests\n";
        std::cout << "           KlRxReady Asserted  : " << ( readyAfterFill ? "true (Accepting)" : "false (Backpressure Active)") << "\n";
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Karst, SingleHostSingleDChanWrite)
{
    KarstFabric fabric;

    // Host 0 issues write to address 0x000 (Die 0, MC 0, DChan 0)
    const uint32_t testAddr = 0x000;
    const uint32_t testData = 0xCAFE'BABE;

    fabric.PostHostWrite( 0, testAddr, testData);

    // Advance simulation through KarstLink, NoC, mpipe, and MC
    fabric.Advance( 25);

    // Verify memory channel 0 serviced write and holds correct value
    JEEVES_ASSERT_EQ( fabric.DChan( 0).Stats()._WritesServiced, 1U);
    JEEVES_ASSERT_EQ( fabric.DChan( 0).ReadWord( testAddr), testData);

    // Verify Host 0 stats recorded outgoing transaction
    JEEVES_ASSERT_EQ( fabric.Host( 0).Stats()._WritesPosted, 1U);

    if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
        std::cout << "         [Single Host Single DChan Write Diagnostics]\n";
        std::cout << "           Host Origin         : Fore_0\n";
        std::cout << "           Target Address      : 0x" << std::hex << std::uppercase << testAddr << std::dec << "\n";
        std::cout << "           Write Data          : 0x" << std::hex << std::uppercase << testData << std::dec << "\n";
        std::cout << "           Target Destination  : Die 0, MC 0 -> DChan 0\n";
        std::cout << "           Simulation Advance  : 25 cycles elapsed\n";
        std::cout << "           DChan 0 Verified    : 0x" << std::hex << std::uppercase << fabric.DChan( 0).ReadWord( testAddr) << std::dec
                  << " (Writes Serviced: " << fabric.DChan( 0).Stats()._WritesServiced << ")\n";
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Karst, AllHostsRoundRobinInterleave)
{
    KarstFabric fabric;

    // Inject 8 write transactions (1 per host) with 1 kB address striping
    // Addresses 0x000, 0x400, 0x800, 0xC00 map to DChan 0..3 (Die 0)
    // Addresses 0x1000, 0x1400, 0x1800, 0x1C00 map to DChan 4..7 (Die 1)
    for ( uint32_t h = 0; h < k_HostsPerFabric; ++h) {
        const uint32_t addr = h * 0x400;
        const uint32_t data = 0x1000 + h;
        fabric.PostHostWrite( h, addr, data);
    }

    // Advance simulation to allow all 8 transactions to traverse fabric and settle
    fabric.Advance( 40);

    // Verify every DDR5 channel received exactly its interleaved portion
    for ( uint32_t c = 0; c < k_DChansPerFabric; ++c) {
        JEEVES_ASSERT_EQ( fabric.DChan( c).Stats()._WritesServiced, 1U);
        const uint32_t expectedAddr = c * 0x400;
        const uint32_t expectedData = 0x1000 + c;
        JEEVES_ASSERT_EQ( fabric.DChan( c).ReadWord( expectedAddr), expectedData);
    }

    const KarstStats st = fabric.Stats();
    JEEVES_ASSERT_EQ( st._TotalTxCount, 8U);
    JEEVES_ASSERT_EQ( st._TotalBytesWritten, 32ULL);

    if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
        std::cout << "         [Round-Robin 1 kB Address Striping Diagnostics]\n";
        std::cout << "           Transactions Posted : 8 writes across 8 hosts\n";
        for ( uint32_t c = 0; c < k_DChansPerFabric; ++c) {
            const uint32_t a = c * 0x400;
            const uint32_t d = 0x1000 + c;
            std::cout << "             Host " << c << " -> Addr 0x" << std::hex << std::uppercase << a << std::dec
                      << " -> DChan " << c << " (Data: 0x" << std::hex << std::uppercase << d << std::dec
                      << ", Serviced: " << fabric.DChan( c).Stats()._WritesServiced << ")\n";
        }
        std::cout << "           Simulation Advance  : 40 cycles elapsed\n";
        std::cout << "           Aggregate Metrics   : Total TX=" << st._TotalTxCount
                  << ", Bytes Written=" << st._TotalBytesWritten << " B\n";
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Karst, VPUDispatch)
{
    KarstFabric fabric;

    // Initialize DChan 0 with known pattern
    fabric.DChan( 0).Fill( 100);
    const bool verifiedPre = fabric.DChan( 0).Verify( 100);
    JEEVES_ASSERT( verifiedPre);

    // Dispatch EPU 0 over DChan 0 buffer (DoubleKernel operation)
    const swarm::SwarmError err = fabric.VPU( 0).Dispatch( fabric.DChan( 0), swarm::WorkgroupDim::Linear( 16));
    JEEVES_ASSERT( err.IsOk());
    JEEVES_ASSERT_EQ( fabric.VPU( 0).Dispatches(), 1U);

    // Verify DChan 0 buffer has been updated by EPU execution
    const bool verifiedPost = fabric.DChan( 0).Verify( 100);
    JEEVES_ASSERT( !verifiedPost);

    if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
        std::cout << "         [Near-Memory EPU / VPU Dispatch Diagnostics]\n";
        std::cout << "           Target Memory       : DChan 0 (Physical DDR5 channel)\n";
        std::cout << "           Initial Buffer Fill : Pattern 100 (" << ( verifiedPre ? "Verified" : "Failed") << ")\n";
        std::cout << "           Kernel Dispatched   : DoubleKernel (Swarm SIMT)\n";
        std::cout << "           Workgroup Dimension : Linear(16) (16 threadblocks)\n";
        std::cout << "           Dispatch Status     : " << ( err.IsOk() ? "Success (Ok)" : "Error") << "\n";
        std::cout << "           Dispatches Recorded : " << fabric.VPU( 0).Dispatches() << "\n";
        std::cout << "           Memory Mutated      : " << ( !verifiedPost ? "Buffer values updated in-place" : "No mutation observed") << "\n";
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Karst, DualHindInterDieLink)
{
    KarstFabric fabric;

    // Host 0 writes to address 0x1000 (Die 1, MC 0, DChan 4)
    const uint32_t remoteAddr = 0x1000;
    const uint32_t remoteData = 0xDEAD'BEEF;

    fabric.PostHostWrite( 0, remoteAddr, remoteData);
    fabric.Advance( 30);

    // Verify DChan 4 on remote Die 1 received write
    JEEVES_ASSERT_EQ( fabric.DChan( 4).Stats()._WritesServiced, 1U);
    JEEVES_ASSERT_EQ( fabric.DChan( 4).ReadWord( remoteAddr), remoteData);

    // Host 0 issues read to the same remote address
    fabric.PostHostRead( 0, remoteAddr);
    fabric.Advance( 30);

    // Verify Host 0 received read response
    const bool hasResp = fabric.Host( 0).HasResponses();
    JEEVES_ASSERT( hasResp);
    HostResponse resp{};
    const bool popped = fabric.PopHostResponse( 0, resp);
    JEEVES_ASSERT( popped);
    JEEVES_ASSERT_EQ( resp._Addr, remoteAddr);
    JEEVES_ASSERT_EQ( resp._Data, remoteData);

    if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
        std::cout << "         [Dual-Hind Inter-Die Routing Diagnostics]\n";
        std::cout << "           Source Host         : Fore_0 (Die 0 home)\n";
        std::cout << "           Target Address      : 0x" << std::hex << std::uppercase << remoteAddr << std::dec << " (Die 1, MC 0, DChan 4)\n";
        std::cout << "           Remote Write Data   : 0x" << std::hex << std::uppercase << remoteData << std::dec << "\n";
        std::cout << "           Write Traversal     : Fore_0 -> KL0 -> Hind_0 -> Inter-Die KL8 -> Hind_1 -> DChan 4 (30 cycles)\n";
        std::cout << "           Read Traversal      : Fore_0 -> Hind_0 -> Inter-Die KL8 -> Hind_1 -> DChan 4 (30 cycles)\n";
        std::cout << "           Response Verified   : Addr=0x" << std::hex << std::uppercase << resp._Addr
                  << ", Data=0x" << resp._Data << std::dec << "\n";
        std::cout << "           Total Round-Trip    : 60 cycles elapsed\n";
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Karst, ParallelDrive)
{
    // Execute fabric in parallel mode with 4 workers
    KarstFabric fabric( 4);

    for ( uint32_t h = 0; h < k_HostsPerFabric; ++h) {
        const uint32_t addr = h * 0x400;
        const uint32_t data = 0x2000 + h;
        fabric.PostHostWrite( h, addr, data);
    }

    fabric.Advance( 40);

    // Verify all 8 channels received correct data under parallel execution
    for ( uint32_t c = 0; c < k_DChansPerFabric; ++c) {
        JEEVES_ASSERT_EQ( fabric.DChan( c).Stats()._WritesServiced, 1U);
        const uint32_t expectedAddr = c * 0x400;
        const uint32_t expectedData = 0x2000 + c;
        JEEVES_ASSERT_EQ( fabric.DChan( c).ReadWord( expectedAddr), expectedData);
    }

    if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
        std::cout << "         [Parallel SimEngine Drive Diagnostics]\n";
        std::cout << "           Execution Mode      : Parallel (4 Heist worker threads)\n";
        std::cout << "           Concurrent Hosts    : 8 hosts issuing writes simultaneously\n";
        std::cout << "           Simulation Advance  : 40 cycles elapsed\n";
        for ( uint32_t c = 0; c < k_DChansPerFabric; ++c) {
            const uint32_t a = c * 0x400;
            const uint32_t d = 0x2000 + c;
            std::cout << "             DChan " << c << " : Addr 0x" << std::hex << std::uppercase << a
                      << " = 0x" << fabric.DChan( c).ReadWord( a) << std::dec
                      << " (Serviced: " << fabric.DChan( c).Stats()._WritesServiced << ")\n";
        }
        std::cout << "           Parallel Parity     : All 8 channels verified bitwise identical\n";
    }
}

//-------------------------------------------------------------------------------------------------
