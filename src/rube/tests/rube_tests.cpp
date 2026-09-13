// rube_tests.cpp ---------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "rube/rube.h"
#include "stalks/atm.h"

using namespace trellis;
using namespace trellis::silo;
using namespace trellis::rube;

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, FourStateLogicOperations)
{
    // NOT
    {
        const auto r1 = Eval4State( KernelOp::Not, 1ULL, false, false, 0ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r1._Val, 0ULL);
        JEEVES_ASSERT( !r1._IsX);
        JEEVES_ASSERT( !r1._IsI);

        const auto r2 = Eval4State( KernelOp::Not, 0ULL, false, false, 0ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r2._Val, 1ULL);
        JEEVES_ASSERT( !r2._IsX);

        const auto rx = Eval4State( KernelOp::Not, 0ULL, true, false, 0ULL, false, false, 1);
        JEEVES_ASSERT( rx._IsX);

        const auto rz = Eval4State( KernelOp::Not, 0ULL, false, true, 0ULL, false, false, 1);
        JEEVES_ASSERT( rz._IsX);
    }

    // AND
    {
        const auto r11 = Eval4State( KernelOp::And, 1ULL, false, false, 1ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r11._Val, 1ULL);
        JEEVES_ASSERT( !r11._IsX);

        const auto r10 = Eval4State( KernelOp::And, 1ULL, false, false, 0ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r10._Val, 0ULL);
        JEEVES_ASSERT( !r10._IsX);

        const auto r01 = Eval4State( KernelOp::And, 0ULL, false, false, 1ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r01._Val, 0ULL);
        JEEVES_ASSERT( !r01._IsX);

        const auto r00 = Eval4State( KernelOp::And, 0ULL, false, false, 0ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r00._Val, 0ULL);
        JEEVES_ASSERT( !r00._IsX);

        // Dominant 0 over X and Z
        const auto r0x = Eval4State( KernelOp::And, 0ULL, false, false, 0ULL, true, false, 1);
        JEEVES_ASSERT_EQ( r0x._Val, 0ULL);
        JEEVES_ASSERT( !r0x._IsX);

        const auto r1x = Eval4State( KernelOp::And, 1ULL, false, false, 0ULL, true, false, 1);
        JEEVES_ASSERT( r1x._IsX);
    }

    // OR
    {
        const auto r11 = Eval4State( KernelOp::Or, 1ULL, false, false, 1ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r11._Val, 1ULL);
        JEEVES_ASSERT( !r11._IsX);

        const auto r10 = Eval4State( KernelOp::Or, 1ULL, false, false, 0ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r10._Val, 1ULL);
        JEEVES_ASSERT( !r10._IsX);

        // Dominant 1 over X
        const auto r1x = Eval4State( KernelOp::Or, 1ULL, false, false, 0ULL, true, false, 1);
        JEEVES_ASSERT_EQ( r1x._Val, 1ULL);
        JEEVES_ASSERT( !r1x._IsX);

        const auto r01 = Eval4State( KernelOp::Or, 0ULL, false, false, 1ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r01._Val, 1ULL);
        JEEVES_ASSERT( !r01._IsX);

        const auto r00 = Eval4State( KernelOp::Or, 0ULL, false, false, 0ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r00._Val, 0ULL);
        JEEVES_ASSERT( !r00._IsX);

        const auto r0x = Eval4State( KernelOp::Or, 0ULL, false, false, 0ULL, true, false, 1);
        JEEVES_ASSERT( r0x._IsX);
    }

    // XOR
    {
        const auto r11 = Eval4State( KernelOp::Xor, 1ULL, false, false, 1ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r11._Val, 0ULL);
        JEEVES_ASSERT( !r11._IsX);

        const auto r10 = Eval4State( KernelOp::Xor, 1ULL, false, false, 0ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r10._Val, 1ULL);
        JEEVES_ASSERT( !r10._IsX);

        const auto r00 = Eval4State( KernelOp::Xor, 0ULL, false, false, 0ULL, false, false, 1);
        JEEVES_ASSERT_EQ( r00._Val, 0ULL);
        JEEVES_ASSERT( !r00._IsX);

        const auto r1x = Eval4State( KernelOp::Xor, 1ULL, false, false, 0ULL, true, false, 1);
        JEEVES_ASSERT( r1x._IsX);
    }

    // Engine port flag inspection
    {
        Layout layout;
        AndGate andG( layout, "AndG");
        layout.Freeze();
        SimEngine engine = SimEngine::Create( layout);

        JEEVES_ASSERT( engine.IsValid( andG.In1()));
        engine.Set( andG.In1(), 0ULL, true, false); // Set X
        JEEVES_ASSERT( engine.IsX( andG.In1()));
        JEEVES_ASSERT( !engine.IsValid( andG.In1()));

        engine.Set( andG.In2(), 0ULL, false, true); // Set Z / I
        JEEVES_ASSERT( engine.IsZ( andG.In2()));
        JEEVES_ASSERT( engine.IsI( andG.In2()));
        JEEVES_ASSERT( !engine.IsValid( andG.In2()));

        engine.Set( andG.In1(), 1ULL);
        JEEVES_ASSERT( engine.IsValid( andG.In1()));
        JEEVES_ASSERT_EQ( engine.Get( andG.In1()), 1ULL);
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, KernelOpFullSet)
{
    uint64_t mask = 0xFFFF;
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::And, 0x00FFULL, 0x0F0FULL, mask), 0x000FULL);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Or, 0x00F0ULL, 0x0F00ULL, mask), 0x0FF0ULL);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Xor, 0x00FFULL, 0x000FULL, mask), 0x00F0ULL);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Nand, 0x0001ULL, 0x0001ULL, 1ULL), 0ULL);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Not, 0x0000ULL, 0x0000ULL, 1ULL), 1ULL);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Nor, 0ULL, 0ULL, 1ULL), 1ULL);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Xnor, 1ULL, 1ULL, 1ULL), 1ULL);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Add, 100ULL, 200ULL, mask), 300ULL);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Sub, 200ULL, 50ULL, mask), 150ULL);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Shl, 1ULL, 4ULL, mask), 16ULL);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Shr, 16ULL, 2ULL, mask), 4ULL);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, BasicLogicGates)
{
    Layout layout;
    AndGate andG( layout, "AndG");
    OrGate  orG( layout, "OrG");
    XorGate xorG( layout, "XorG");
    NotGate notG( layout, "NotG");

    layout.Freeze();

    SimEngine engine = SimEngine::Create( layout);

    // Set inputs
    engine.Set( andG.In1(), true);
    engine.Set( andG.In2(), false);

    engine.Set( orG.In1(), false);
    engine.Set( orG.In2(), true);

    engine.Set( xorG.In1(), true);
    engine.Set( xorG.In2(), true);

    engine.Set( notG.In(), false);

    engine.Drive();

    JEEVES_ASSERT( !engine.Get< bool>( andG.Out()));
    JEEVES_ASSERT( engine.Get< bool>( orG.Out()));
    JEEVES_ASSERT( !engine.Get< bool>( xorG.Out()));
    JEEVES_ASSERT( engine.Get< bool>( notG.Out()));
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, RSLatchSettle)
{
    Layout layout;
    RSLatch latch( layout, "L1");
    layout.Freeze();

    SimEngine engine = SimEngine::Create( layout);

    // Active-low Set (S=0, R=1) -> Q=1, Q1=0
    latch.SetS( engine, false);
    latch.SetR( engine, true);
    engine.Settle( 10);

    JEEVES_ASSERT( engine.Get< bool>( latch.Q()));
    JEEVES_ASSERT( !engine.Get< bool>( latch.Q1()));

    // Release (S=1, R=1) -> Hold previous state (Q=1, Q1=0)
    latch.SetS( engine, true);
    latch.SetR( engine, true);
    engine.Settle( 10);

    JEEVES_ASSERT( engine.Get< bool>( latch.Q()));
    JEEVES_ASSERT( !engine.Get< bool>( latch.Q1()));

    // Active-low Reset (S=1, R=0) -> Q=0, Q1=1
    latch.SetS( engine, true);
    latch.SetR( engine, false);
    engine.Settle( 10);

    JEEVES_ASSERT( !engine.Get< bool>( latch.Q()));
    JEEVES_ASSERT( engine.Get< bool>( latch.Q1()));
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, Adder16SerialAndParallel)
{
    Layout layout;
    Adder< 16> adder( layout, "Adder16");
    layout.Freeze();

    // 1. Serial Test
    {
        SimEngine engine = SimEngine::Create( layout);
        engine.WithMode( SimEngineMode::Serial());

        adder.SetA( engine, 1234u);
        adder.SetB( engine, 5678u);
        adder.SetCarryIn( engine, false);

        // 16-bit ripple carry requires up to 32 delta cycles to settle
        uint32_t cycles = engine.Settle( 64);
        JEEVES_ASSERT( cycles > 0);

        uint32_t sum = adder.GetSum( engine);
        JEEVES_ASSERT_EQ( sum, 1234u + 5678u);
        JEEVES_ASSERT( !engine.Get< bool>( adder.Carry()));

        // Test carry out
        adder.SetA( engine, 0xFFFFu);
        adder.SetB( engine, 1u);
        adder.SetCarryIn( engine, false);
        engine.Settle( 64);

        sum = adder.GetSum( engine);
        JEEVES_ASSERT_EQ( sum, 0u);
        JEEVES_ASSERT( engine.Get< bool>( adder.Carry()));
    }

    // 2. Parallel Test
    {
        trellis::heist::Atelier::Reset( 4);
        SimEngine engine = SimEngine::Create( layout);
        engine.WithMode( SimEngineMode::Parallel( 4));

        adder.SetA( engine, 20000u);
        adder.SetB( engine, 15000u);
        adder.SetCarryIn( engine, false);
        engine.Settle( 64);

        uint32_t sum = adder.GetSum( engine);
        JEEVES_ASSERT_EQ( sum, 35000u);
        JEEVES_ASSERT( !engine.Get< bool>( adder.Carry()));
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, CoroModuleSinkMonitor)
{
    stalks::Atm< uint64_t> received{0};

    Layout layout;
    PortDesc inPorts[1] = {PortDesc( "DataIn", PortType::U32Val())};

    ModuleId modId = layout.AddCoroModule(
        "SinkMonitor",
        ModuleId{},
        silo::Arr< const PortDesc>( inPorts, 1),
        silo::Arr< const PortDesc>{},
        [&received]() -> CoroTask {
            CoroPorts inPorts = co_await CoroIn{};
            while ( true)
            {
                const uint64_t val = inPorts[0];
                received.Store( val);
                inPorts = co_yield CoroPorts::Empty();
            }
        }
    );

    layout.Freeze();
    SimEngine engine = SimEngine::Create( layout);
    PortId inPortId = layout.InPort( modId, 0);

    // Cycle 0: initial evaluation (inport is 0)
    engine.Drive();
    JEEVES_ASSERT_EQ( received.Load(), 0ull);

    // Cycle 1: send 42
    engine.Set( inPortId, 42ULL);
    engine.Drive();
    JEEVES_ASSERT_EQ( received.Load(), 42ull);

    // Cycle 2: unchanged
    engine.Drive();
    JEEVES_ASSERT_EQ( received.Load(), 42ull);

    // Cycle 3: send 99
    engine.Set( inPortId, 99ULL);
    engine.Drive();
    JEEVES_ASSERT_EQ( received.Load(), 99ull);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, CoroModuleMultiStepProtocol)
{
    auto createTestLayout = []( Layout& layout, ModuleId& outMod) {
        PortDesc inPorts[2] = {
            PortDesc( "Req", PortType::Bool()),
            PortDesc( "Data", PortType::U32Val())
        };
        PortDesc outPorts[2] = {
            PortDesc( "Ack", PortType::Bool()),
            PortDesc( "Result", PortType::U32Val())
        };

        outMod = layout.AddCoroModule(
            "ProtocolServer",
            ModuleId{},
            silo::Arr< const PortDesc>( inPorts, 2),
            silo::Arr< const PortDesc>( outPorts, 2),
            []() -> CoroTask {
                CoroPorts inPorts = co_await CoroIn{};
                while ( true)
                {
                    // Idle state: Ack = 0, Result = 0
                    while ( !inPorts.Get< bool>( 0))
                    {
                        inPorts = co_yield CoroPorts::Pair( false, 0ULL);
                    }
                    // Req received: compute result and Ack = 1
                    const uint64_t dataVal = inPorts[1];
                    const uint64_t res = dataVal * 2;
                    while ( inPorts.Get< bool>( 0))
                    {
                        inPorts = co_yield CoroPorts::Pair( true, res);
                    }
                    // Req de-asserted: return to Ack = 0
                    inPorts = co_yield CoroPorts::Pair( false, 0ULL);
                }
            }
        );
        layout.Freeze();
    };

    // 1. Serial Test
    {
        Layout layout;
        ModuleId modId{};
        createTestLayout( layout, modId);

        SimEngine engine = SimEngine::Create( layout);
        PortId reqPort = layout.InPort( modId, 0);
        PortId dataPort = layout.InPort( modId, 1);
        PortId ackPort = layout.OutPort( modId, 0);
        PortId resultPort = layout.OutPort( modId, 1);

        // Cycle 0: initial idle state
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.Get< bool>( ackPort), false);
        JEEVES_ASSERT_EQ( engine.Get( resultPort), 0ULL);

        // Cycle 1: Request with Data=21
        engine.Set( dataPort, 21ULL);
        engine.Set( reqPort, true);
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.Get< bool>( ackPort), true);
        JEEVES_ASSERT_EQ( engine.Get( resultPort), 42ULL);

        // Cycle 2: Keep Req=1
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.Get< bool>( ackPort), true);
        JEEVES_ASSERT_EQ( engine.Get( resultPort), 42ULL);

        // Cycle 3: Deassert Req=0
        engine.Set( reqPort, false);
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.Get< bool>( ackPort), false);
        JEEVES_ASSERT_EQ( engine.Get( resultPort), 0ULL);
    }

    // 2. Parallel Test
    {
        trellis::heist::Atelier::Reset( 4);
        Layout layout;
        ModuleId modId{};
        createTestLayout( layout, modId);

        SimEngine engine = SimEngine::Create( layout);
        engine.WithMode( SimEngineMode::Parallel( 4));

        PortId reqPort = layout.InPort( modId, 0);
        PortId dataPort = layout.InPort( modId, 1);
        PortId ackPort = layout.OutPort( modId, 0);
        PortId resultPort = layout.OutPort( modId, 1);

        // Cycle 0: initial idle state
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.Get< bool>( ackPort), false);
        JEEVES_ASSERT_EQ( engine.Get( resultPort), 0ULL);

        // Cycle 1: Request with Data=21
        engine.Set( dataPort, 21ULL);
        engine.Set( reqPort, true);
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.Get< bool>( ackPort), true);
        JEEVES_ASSERT_EQ( engine.Get( resultPort), 42ULL);

        // Cycle 2: Keep Req=1
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.Get< bool>( ackPort), true);
        JEEVES_ASSERT_EQ( engine.Get( resultPort), 42ULL);

        // Cycle 3: Deassert Req=0
        engine.Set( reqPort, false);
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.Get< bool>( ackPort), false);
        JEEVES_ASSERT_EQ( engine.Get( resultPort), 0ULL);
    }
}

//-------------------------------------------------------------------------------------------------
