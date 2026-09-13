// swarm_tests.cpp --------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "swarm/swarm.h"
#include "heist/atelier.h"

#include <cmath>
#include <vector>

using namespace trellis::swarm;
using namespace trellis::silo;

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Swarm, CpuBufferLifecycle)
{
    CpuBuffer buf( "test_buf", 64, BufferUsage::Storage() | BufferUsage::ReadWrite());
    JEEVES_ASSERT_EQ( buf.Size(), 64u);
    JEEVES_ASSERT_EQ( std::string( buf.Label()), "test_buf");

    uint8_t data[4] = {10, 20, 30, 40};
    buf.Write( Arr< const uint8_t>( data, 4));

    Buff< uint8_t> readBack = buf.Read();
    JEEVES_ASSERT( readBack.Size() >= 4u);
    JEEVES_ASSERT_EQ( readBack[0], 10);
    JEEVES_ASSERT_EQ( readBack[1], 20);
    JEEVES_ASSERT_EQ( readBack[2], 30);
    JEEVES_ASSERT_EQ( readBack[3], 40);

    // Verify Read() returns an independent copy
    readBack[0] = 99;
    Buff< uint8_t> secondRead = buf.Read();
    JEEVES_ASSERT_EQ( secondRead[0], 10);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Swarm, CpuDeviceDoubleOp)
{
    CpuDevice dev( 1);

    const uint32_t count = 64;
    Buff< float> values( count, []( uint32_t i) { return static_cast< float>( i + 1); });

    auto buf = dev.CreateBufferInit(
        "data",
        Arr< const uint8_t>( reinterpret_cast< const uint8_t*>( values.Data()), count * sizeof( float)),
        BufferUsage::Storage() | BufferUsage::ReadWrite()
    );

    auto kernel = dev.CompileKernel(
        StandardOpLabel( StandardOp::Double),
        "main",
        StandardOpKernelSource( StandardOp::Double, BackendKind::Cpu)
    );

    ComputeBuffer* bufArray[1] = { buf.get() };
    SwarmError err = dev.Dispatch( *kernel, Arr< ComputeBuffer*>( bufArray, 1), WorkgroupDim::Linear( 1));
    JEEVES_ASSERT( err.IsOk());

    Buff< uint8_t> resultBytes = buf->Read();
    const float* resultFloats = reinterpret_cast< const float*>( resultBytes.Data());
    for ( uint32_t i = 0; i < count; ++i) {
        JEEVES_ASSERT_EQ( resultFloats[i], static_cast< float>( ( i + 1) * 2));
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Swarm, CpuDeviceVectorAddOp)
{
    CpuDevice dev( 1);

    const uint32_t count = 64;
    Buff< float> a( count, []( uint32_t i) { return static_cast< float>( i); });
    Buff< float> b( count, []( uint32_t i) { return static_cast< float>( i * 10); });
    Buff< float> c( count, static_cast< float>( 0));

    auto bufA = dev.CreateBufferInit( "a", Arr< const uint8_t>( reinterpret_cast< const uint8_t*>( a.Data()), count * sizeof( float)), BufferUsage::Storage());
    auto bufB = dev.CreateBufferInit( "b", Arr< const uint8_t>( reinterpret_cast< const uint8_t*>( b.Data()), count * sizeof( float)), BufferUsage::Storage());
    auto bufC = dev.CreateBufferInit( "c", Arr< const uint8_t>( reinterpret_cast< const uint8_t*>( c.Data()), count * sizeof( float)), BufferUsage::Storage());

    auto kernel = dev.CompileKernel(
        StandardOpLabel( StandardOp::VectorAdd),
        "main",
        StandardOpKernelSource( StandardOp::VectorAdd, BackendKind::Cpu)
    );

    ComputeBuffer* bufArray[3] = { bufA.get(), bufB.get(), bufC.get() };
    SwarmError err = dev.Dispatch( *kernel, Arr< ComputeBuffer*>( bufArray, 3), WorkgroupDim::Linear( 1));
    JEEVES_ASSERT( err.IsOk());

    Buff< uint8_t> resultBytes = bufC->Read();
    const float* resultFloats = reinterpret_cast< const float*>( resultBytes.Data());
    for ( uint32_t i = 0; i < count; ++i) {
        JEEVES_ASSERT_EQ( resultFloats[i], static_cast< float>( i * 11));
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Swarm, SwarmEngineCollatzAndParallelDispatch)
{
    // Test with parallel Atelier threads
    trellis::heist::Atelier::Reset( 4);

    SwarmEngine engine( BackendKind::Cpu);

    const uint32_t count = 128;
    Buff< uint32_t> inVals( count, []( uint32_t i) { return ( i % 10) + 1; });
    Buff< uint32_t> outVals( count, static_cast< uint32_t>( 0));

    auto inBuf = engine.Device().CreateBufferInit(
        "in",
        Arr< const uint8_t>( reinterpret_cast< const uint8_t*>( inVals.Data()), count * sizeof( uint32_t)),
        BufferUsage::Storage()
    );
    auto outBuf = engine.Device().CreateBufferInit(
        "out",
        Arr< const uint8_t>( reinterpret_cast< const uint8_t*>( outVals.Data()), count * sizeof( uint32_t)),
        BufferUsage::Storage()
    );

    ComputeBuffer* bufs[2] = { inBuf.get(), outBuf.get() };
    SwarmError err = engine.ExecuteOp( StandardOp::Collatz, Arr< ComputeBuffer*>( bufs, 2), WorkgroupDim::Linear( 2));
    JEEVES_ASSERT( err.IsOk());

    Buff< uint8_t> resBytes = outBuf->Read();
    const uint32_t* resU32 = reinterpret_cast< const uint32_t*>( resBytes.Data());

    for ( uint32_t i = 0; i < count; ++i) {
        JEEVES_ASSERT_EQ( resU32[i], trellis::symph::Collatz( inVals[i]));
    }
}

//-------------------------------------------------------------------------------------------------

