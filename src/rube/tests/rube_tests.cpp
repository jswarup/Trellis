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
    auto runProtocolTest = [&]( bool parallelMode) {
        Layout layout;
        PortDesc inPorts[2] = {
            PortDesc( "Req", PortType::Bool()),
            PortDesc( "Data", PortType::U32Val())
        };
        PortDesc outPorts[2] = {
            PortDesc( "Ack", PortType::Bool()),
            PortDesc( "Result", PortType::U32Val())
        };

        ModuleId modId = layout.AddCoroModule(
            "ProtocolServer",
            ModuleId{},
            silo::Arr< const PortDesc>( inPorts, 2),
            silo::Arr< const PortDesc>( outPorts, 2),
            []() -> CoroTask {
                CoroPorts inPorts = co_await CoroIn{};
                while ( true) {
                    // Idle state: Ack = 0, Result = 0
                    while ( !inPorts.Get< bool>( 0)) {
                        inPorts = co_yield CoroPorts::Pair( false, 0ULL);
                    }
                    // Req received: compute result and Ack = 1
                    const uint64_t dataVal = inPorts[1];
                    const uint64_t res = dataVal * 2;
                    while ( inPorts.Get< bool>( 0)) {
                        inPorts = co_yield CoroPorts::Pair( true, res);
                    }
                    // Req de-asserted: return to Ack = 0
                    inPorts = co_yield CoroPorts::Pair( false, 0ULL);
                }
            }
        );
        layout.Freeze();

        SimEngine engine = SimEngine::Create( layout);
        if ( parallelMode) {
            trellis::heist::Atelier::Reset( 4);
            engine.WithMode( SimEngineMode::Parallel( 4));
        }

        PortId reqPort = layout.InPort( modId, 0);
        PortId dataPort = layout.InPort( modId, 1);
        PortId ackPort = layout.OutPort( modId, 0);
        PortId resultPort = layout.OutPort( modId, 1);

        // Cycle 0: initial idle state
        engine.Drive();
        const bool c0Ack = engine.Get< bool>( ackPort);
        const uint64_t c0Res = engine.Get( resultPort);
        JEEVES_ASSERT_EQ( c0Ack, false);
        JEEVES_ASSERT_EQ( c0Res, 0ULL);

        // Cycle 1: Request with Data=21
        engine.Set( dataPort, 21ULL);
        engine.Set( reqPort, true);
        engine.Drive();
        const bool c1Ack = engine.Get< bool>( ackPort);
        const uint64_t c1Res = engine.Get( resultPort);
        JEEVES_ASSERT_EQ( c1Ack, true);
        JEEVES_ASSERT_EQ( c1Res, 42ULL);

        // Cycle 2: Keep Req=1
        engine.Drive();
        const bool c2Ack = engine.Get< bool>( ackPort);
        const uint64_t c2Res = engine.Get( resultPort);
        JEEVES_ASSERT_EQ( c2Ack, true);
        JEEVES_ASSERT_EQ( c2Res, 42ULL);

        // Cycle 3: Deassert Req=0
        engine.Set( reqPort, false);
        engine.Drive();
        const bool c3Ack = engine.Get< bool>( ackPort);
        const uint64_t c3Res = engine.Get( resultPort);
        JEEVES_ASSERT_EQ( c3Ack, false);
        JEEVES_ASSERT_EQ( c3Res, 0ULL);

        if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
            std::cout << "         [Coro Multi-Step Protocol Diagnostics - "
                      << ( parallelMode ? "Parallel" : "Serial") << "]\n";
            std::cout << "           Cycle 0 (Idle)     : Req=0 -> Ack="
                      << ( c0Ack ? "1" : "0") << ", Result=" << c0Res << "\n";
            std::cout << "           Cycle 1 (Request)  : Req=1, Data=21 -> Ack="
                      << ( c1Ack ? "1" : "0") << ", Result=" << c1Res << "\n";
            std::cout << "           Cycle 2 (Sustain)  : Req=1 -> Ack="
                      << ( c2Ack ? "1" : "0") << ", Result=" << c2Res << "\n";
            std::cout << "           Cycle 3 (Deassert) : Req=0 -> Ack="
                      << ( c3Ack ? "1" : "0") << ", Result=" << c3Res << "\n";
            std::cout << "           Total Cycles       : 4 cycles elapsed\n";
        }
    };

    // 1. Serial Test
    runProtocolTest( false);

    // 2. Parallel Test
    runProtocolTest( true);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, ClockedSequentialCircuit)
{
    // 1. Gate-level sequential circuit (CRSLatch)
    uint32_t crsTotalTicks = 0;
    uint32_t crsTotalDeltaCycles = 0;
    bool crsStep1Q = false, crsStep1Q1 = false;
    bool crsStep2Q = false, crsStep2Q1 = false;
    bool crsStep3Q = false, crsStep3Q1 = false;
    bool crsStep4Q = false, crsStep4Q1 = false;
    uint32_t d1 = 0, d2 = 0, d3 = 0, d4 = 0;

    {
        Layout layout;
        PortDesc clkOutDescs[1] = {PortDesc::Bool( "Clk")};
        ModuleId clkMod = layout.AddModule(
            "ClockGen",
            ModuleId{},
            silo::Arr< const PortDesc>{},
            silo::Arr< const PortDesc>( clkOutDescs, 1),
            KernelKind::None()
        );
        PortId clkPort = layout.OutPort( clkMod, 0);

        CRSLatch crs( layout, "CRS");
        layout.Connect( clkPort, crs.Clk1());
        layout.Connect( clkPort, crs.Clk2());

        layout.Freeze();

        SimEngine engine = SimEngine::Create( layout);
        engine.WithClock( clkPort);
        JEEVES_ASSERT_EQ( engine.GetClock(), clkPort);

        // Initial clock baseline is false (0)
        engine.Set( clkPort, false);

        // Set S=1, R=0 (Active Set)
        crs.SetS( engine, true);
        crs.SetR( engine, false);

        // Before advancing clock, S/R are blocked by NAND gate because Clk=0
        engine.Settle( 10);

        // Advance 1 complete clock tick (0 -> 1 -> Settle -> 0 -> Settle)
        d1 = engine.Advance();
        crsTotalTicks += 1;
        crsTotalDeltaCycles += d1;
        crsStep1Q = engine.Get< bool>( crs.Q());
        crsStep1Q1 = engine.Get< bool>( crs.Q1());
        JEEVES_ASSERT( d1 > 0);
        JEEVES_ASSERT( crsStep1Q);
        JEEVES_ASSERT( !crsStep1Q1);
        JEEVES_ASSERT( !engine.Get< bool>( clkPort));

        // Change inputs while clock is 0: S=0, R=1 (Active Reset)
        crs.SetS( engine, false);
        crs.SetR( engine, true);
        engine.Settle( 10);

        // Q should hold previous state (1) until clock advances
        JEEVES_ASSERT( engine.Get< bool>( crs.Q()));
        JEEVES_ASSERT( !engine.Get< bool>( crs.Q1()));

        // Advance 1 complete clock tick: latch updates to Q=0, Q1=1
        d2 = engine.Advance();
        crsTotalTicks += 1;
        crsTotalDeltaCycles += d2;
        crsStep2Q = engine.Get< bool>( crs.Q());
        crsStep2Q1 = engine.Get< bool>( crs.Q1());
        JEEVES_ASSERT( d2 > 0);
        JEEVES_ASSERT( !crsStep2Q);
        JEEVES_ASSERT( crsStep2Q1);
        JEEVES_ASSERT( !engine.Get< bool>( clkPort));

        // Multi-tick advance (5 ticks)
        d3 = engine.Advance( 5);
        crsTotalTicks += 5;
        crsTotalDeltaCycles += d3;
        crsStep3Q = engine.Get< bool>( crs.Q());
        crsStep3Q1 = engine.Get< bool>( crs.Q1());
        JEEVES_ASSERT( d3 > 0);
        JEEVES_ASSERT( !crsStep3Q);
        JEEVES_ASSERT( crsStep3Q1);

        // TriggerId overload
        TriggerId clkTrig = engine.GetPortTrigger( clkPort);
        crs.SetS( engine, true);
        crs.SetR( engine, false);
        d4 = engine.AdvanceTrigger( clkTrig, 1);
        crsTotalTicks += 1;
        crsTotalDeltaCycles += d4;
        crsStep4Q = engine.Get< bool>( crs.Q());
        crsStep4Q1 = engine.Get< bool>( crs.Q1());
        JEEVES_ASSERT( d4 > 0);
        JEEVES_ASSERT( crsStep4Q);
        JEEVES_ASSERT( !crsStep4Q1);
    }

    // 2. Synchronous edge-triggered counter with CoroModule
    uint32_t counterTotalTicks = 0;
    uint32_t counterTotalDeltaCycles = 0;
    uint64_t c0 = 0, c1 = 0, c2 = 0;
    uint32_t cd1 = 0, cd2 = 0;

    {
        Layout coroLayout;
        PortDesc inPorts[1] = {PortDesc::Bool( "Clk")};
        PortDesc outPorts[1] = {PortDesc::U32( "Count")};

        ModuleId counterMod = coroLayout.AddCoroModule(
            "SyncCounter",
            ModuleId{},
            silo::Arr< const PortDesc>( inPorts, 1),
            silo::Arr< const PortDesc>( outPorts, 1),
            []() -> CoroTask {
                CoroPorts inPorts = co_await CoroIn{};
                uint32_t counter = 0;
                bool lastClk = false;
                while ( true) {
                    const bool clk = inPorts.Get< bool>( 0);
                    if ( !lastClk && clk) {
                        counter += 1;
                    }
                    lastClk = clk;
                    inPorts = co_yield CoroPorts::Single( static_cast< uint64_t>( counter));
                }
            }
        );
        coroLayout.Freeze();

        SimEngine counterEngine = SimEngine::Create( coroLayout);
        PortId clkIn = coroLayout.InPort( counterMod, 0);
        PortId countOut = coroLayout.OutPort( counterMod, 0);

        counterEngine.WithClock( clkIn);
        counterEngine.Set( clkIn, false);
        counterEngine.Drive();

        c0 = counterEngine.Get( countOut);
        JEEVES_ASSERT_EQ( c0, 0ULL);

        // Advance 1 tick -> counter increments to 1
        cd1 = counterEngine.Advance();
        counterTotalTicks += 1;
        counterTotalDeltaCycles += cd1;
        c1 = counterEngine.Get( countOut);
        JEEVES_ASSERT_EQ( c1, 1ULL);

        // Advance 5 ticks -> counter increments to 6
        cd2 = counterEngine.Advance( 5);
        counterTotalTicks += 5;
        counterTotalDeltaCycles += cd2;
        c2 = counterEngine.Get( countOut);
        JEEVES_ASSERT_EQ( c2, 6ULL);

        // Clock returned to baseline
        JEEVES_ASSERT( !counterEngine.Get< bool>( clkIn));
    }

    if ( !ctx->_AssertsEnabled || ctx->_Verbosity > 0) {
        std::cout << "         [Clocked Sequential Circuit Diagnostics]\n";
        std::cout << "           --- Gate-Level CRS Latch ---\n";
        std::cout << "             Set (1 tick)      : Q=" << ( crsStep1Q ? "1" : "0")
                  << ", Q1=" << ( crsStep1Q1 ? "1" : "0")
                  << " (" << d1 << " delta cycles)\n";
        std::cout << "             Reset (1 tick)    : Q=" << ( crsStep2Q ? "1" : "0")
                  << ", Q1=" << ( crsStep2Q1 ? "1" : "0")
                  << " (" << d2 << " delta cycles)\n";
        std::cout << "             Hold (5 ticks)    : Q=" << ( crsStep3Q ? "1" : "0")
                  << ", Q1=" << ( crsStep3Q1 ? "1" : "0")
                  << " (" << d3 << " delta cycles)\n";
        std::cout << "             Trig Set (1 tick) : Q=" << ( crsStep4Q ? "1" : "0")
                  << ", Q1=" << ( crsStep4Q1 ? "1" : "0")
                  << " (" << d4 << " delta cycles)\n";
        std::cout << "             CRS Total Advance : " << crsTotalTicks << " ticks, "
                  << crsTotalDeltaCycles << " delta cycles\n";
        std::cout << "           --- Edge-Triggered Synchronous Counter ---\n";
        std::cout << "             Initial Count     : " << c0 << "\n";
        std::cout << "             Advance 1 tick    : Count=" << c1
                  << " (" << cd1 << " delta cycles)\n";
        std::cout << "             Advance 5 ticks   : Count=" << c2
                  << " (" << cd2 << " delta cycles)\n";
        std::cout << "             Counter Total     : " << counterTotalTicks << " ticks, "
                  << counterTotalDeltaCycles << " delta cycles\n";
    }
}

//-------------------------------------------------------------------------------------------------
