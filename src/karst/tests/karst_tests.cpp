// karst_tests.cpp ---------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "karst/karst.h"

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

    JEEVES_ASSERT_EQ( engine.Get< bool>( noc.KlRxReady( 0)), false);
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
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Karst, VPUDispatch)
{
    KarstFabric fabric;

    // Initialize DChan 0 with known pattern
    fabric.DChan( 0).Fill( 100);
    JEEVES_ASSERT( fabric.DChan( 0).Verify( 100));

    // Dispatch EPU 0 over DChan 0 buffer (DoubleKernel operation)
    const swarm::SwarmError err = fabric.VPU( 0).Dispatch( fabric.DChan( 0), swarm::WorkgroupDim::Linear( 16));
    JEEVES_ASSERT( err.IsOk());
    JEEVES_ASSERT_EQ( fabric.VPU( 0).Dispatches(), 1U);

    // Verify DChan 0 buffer has been updated by EPU execution
    JEEVES_ASSERT( !fabric.DChan( 0).Verify( 100));
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
    JEEVES_ASSERT( fabric.Host( 0).HasResponses());
    HostResponse resp{};
    JEEVES_ASSERT( fabric.PopHostResponse( 0, resp));
    JEEVES_ASSERT_EQ( resp._Addr, remoteAddr);
    JEEVES_ASSERT_EQ( resp._Data, remoteData);
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
}

//-------------------------------------------------------------------------------------------------
