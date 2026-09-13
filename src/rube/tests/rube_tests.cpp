// rube_tests.cpp ---------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "rube/rube.h"

#include <atomic>
#include <memory>

using namespace trellis;
using namespace trellis::silo;
using namespace trellis::rube;

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, RegLogicOperations)
{
    // Truth values
    JEEVES_ASSERT( Reg::TRUE.IsTrue());
    JEEVES_ASSERT( !Reg::TRUE.IsFalse());
    JEEVES_ASSERT( Reg::FALSE.IsFalse());
    JEEVES_ASSERT( !Reg::FALSE.IsTrue());
    JEEVES_ASSERT( Reg::X.IsX());
    JEEVES_ASSERT( Reg::Z.IsZ());

    // NOT
    JEEVES_ASSERT_EQ( ~Reg::TRUE, Reg::FALSE);
    JEEVES_ASSERT_EQ( ~Reg::FALSE, Reg::TRUE);
    JEEVES_ASSERT_EQ( ~Reg::X, Reg::X);
    JEEVES_ASSERT_EQ( ~Reg::Z, Reg::X);

    // AND
    JEEVES_ASSERT_EQ( Reg::TRUE & Reg::TRUE, Reg::TRUE);
    JEEVES_ASSERT_EQ( Reg::TRUE & Reg::FALSE, Reg::FALSE);
    JEEVES_ASSERT_EQ( Reg::FALSE & Reg::TRUE, Reg::FALSE);
    JEEVES_ASSERT_EQ( Reg::FALSE & Reg::FALSE, Reg::FALSE);
    JEEVES_ASSERT_EQ( Reg::FALSE & Reg::X, Reg::FALSE); // Dominant 0
    JEEVES_ASSERT_EQ( Reg::TRUE & Reg::X, Reg::X);

    // OR
    JEEVES_ASSERT_EQ( Reg::TRUE | Reg::TRUE, Reg::TRUE);
    JEEVES_ASSERT_EQ( Reg::TRUE | Reg::FALSE, Reg::TRUE);
    JEEVES_ASSERT_EQ( Reg::TRUE | Reg::X, Reg::TRUE);   // Dominant 1
    JEEVES_ASSERT_EQ( Reg::FALSE | Reg::TRUE, Reg::TRUE);
    JEEVES_ASSERT_EQ( Reg::FALSE | Reg::FALSE, Reg::FALSE);
    JEEVES_ASSERT_EQ( Reg::FALSE | Reg::X, Reg::X);

    // XOR
    JEEVES_ASSERT_EQ( Reg::TRUE ^ Reg::TRUE, Reg::FALSE);
    JEEVES_ASSERT_EQ( Reg::TRUE ^ Reg::FALSE, Reg::TRUE);
    JEEVES_ASSERT_EQ( Reg::FALSE ^ Reg::FALSE, Reg::FALSE);
    JEEVES_ASSERT_EQ( Reg::TRUE ^ Reg::X, Reg::X);
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, KernelOpFullSet)
{
    uint64_t mask = 0xFFFF;
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::And, 0x00FF, 0x0F0F, mask), 0x000F);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Or, 0x00F0, 0x0F00, mask), 0x0FF0);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Xor, 0x00FF, 0x000F, mask), 0x00F0);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Nand, 0x0001, 0x0001, 1), 0);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Not, 0x0000, 0x0000, 1), 1);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Nor, 0, 0, 1), 1);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Xnor, 1, 1, 1), 1);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Add, 100, 200, mask), 300);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Sub, 200, 50, mask), 150);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Shl, 1, 4, mask), 16);
    JEEVES_ASSERT_EQ( EvalRaw( KernelOp::Shr, 16, 2, mask), 4);
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
    engine.SetPortBool( andG.In1(), Reg::TRUE);
    engine.SetPortBool( andG.In2(), Reg::FALSE);

    engine.SetPortBool( orG.In1(), Reg::FALSE);
    engine.SetPortBool( orG.In2(), Reg::TRUE);

    engine.SetPortBool( xorG.In1(), Reg::TRUE);
    engine.SetPortBool( xorG.In2(), Reg::TRUE);

    engine.SetPortBool( notG.In(), Reg::FALSE);

    engine.Drive();

    JEEVES_ASSERT( engine.GetPortBool( andG.Out()).IsFalse());
    JEEVES_ASSERT( engine.GetPortBool( orG.Out()).IsTrue());
    JEEVES_ASSERT( engine.GetPortBool( xorG.Out()).IsFalse());
    JEEVES_ASSERT( engine.GetPortBool( notG.Out()).IsTrue());
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, RSLatchSettle)
{
    Layout layout;
    RSLatch latch( layout, "L1");
    layout.Freeze();

    SimEngine engine = SimEngine::Create( layout);

    // Active-low Set (S=0, R=1) -> Q=1, Q1=0
    latch.SetS( engine, Reg::FALSE);
    latch.SetR( engine, Reg::TRUE);
    engine.Settle( 10);

    JEEVES_ASSERT( engine.GetPortBool( latch.Q()).IsTrue());
    JEEVES_ASSERT( engine.GetPortBool( latch.Q1()).IsFalse());

    // Release (S=1, R=1) -> Hold previous state (Q=1, Q1=0)
    latch.SetS( engine, Reg::TRUE);
    latch.SetR( engine, Reg::TRUE);
    engine.Settle( 10);

    JEEVES_ASSERT( engine.GetPortBool( latch.Q()).IsTrue());
    JEEVES_ASSERT( engine.GetPortBool( latch.Q1()).IsFalse());

    // Active-low Reset (S=1, R=0) -> Q=0, Q1=1
    latch.SetS( engine, Reg::TRUE);
    latch.SetR( engine, Reg::FALSE);
    engine.Settle( 10);

    JEEVES_ASSERT( engine.GetPortBool( latch.Q()).IsFalse());
    JEEVES_ASSERT( engine.GetPortBool( latch.Q1()).IsTrue());
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

        adder.SetA( engine, 1234);
        adder.SetB( engine, 5678);
        adder.SetCarryIn( engine, Reg::FALSE);

        // 16-bit ripple carry requires up to 32 delta cycles to settle
        uint32_t cycles = engine.Settle( 64);
        JEEVES_ASSERT( cycles > 0);

        uint32_t sum = adder.GetSum( engine);
        JEEVES_ASSERT_EQ( sum, 1234u + 5678u);
        JEEVES_ASSERT( engine.GetPortBool( adder.Carry()).IsFalse());

        // Test carry out
        adder.SetA( engine, 0xFFFF);
        adder.SetB( engine, 1);
        adder.SetCarryIn( engine, Reg::FALSE);
        engine.Settle( 64);

        sum = adder.GetSum( engine);
        JEEVES_ASSERT_EQ( sum, 0u);
        JEEVES_ASSERT( engine.GetPortBool( adder.Carry()).IsTrue());
    }

    // 2. Parallel Test
    {
        trellis::heist::Atelier::Reset( 4);
        SimEngine engine = SimEngine::Create( layout);
        engine.WithMode( SimEngineMode::Parallel( 4));

        adder.SetA( engine, 20000);
        adder.SetB( engine, 15000);
        adder.SetCarryIn( engine, Reg::FALSE);
        engine.Settle( 64);

        uint32_t sum = adder.GetSum( engine);
        JEEVES_ASSERT_EQ( sum, 35000u);
        JEEVES_ASSERT( engine.GetPortBool( adder.Carry()).IsFalse());
    }
}

//-------------------------------------------------------------------------------------------------

JEEVES_TEST( Rube, CoroModuleSinkMonitor)
{
    auto received = std::make_shared< std::atomic< uint64_t>>( 0);

    Layout layout;
    PortDesc inPorts[1] = {PortDesc( "DataIn", PortType::U32Val())};

    ModuleId modId = layout.AddCoroModule(
        "SinkMonitor",
        ModuleId{},
        silo::Arr< const PortDesc>( inPorts, 1),
        silo::Arr< const PortDesc>{},
        [received]() -> CoroTask {
            CoroPorts inPorts = co_await CoroIn{};
            while ( true)
            {
                const uint64_t val = inPorts[0].Val();
                received->store( val, std::memory_order_seq_cst);
                inPorts = co_yield CoroPorts::Empty();
            }
        }
    );

    layout.Freeze();
    SimEngine engine = SimEngine::Create( layout);
    PortId inPortId = layout.InPort( modId, 0);

    // Cycle 0: initial evaluation (inport is 0)
    engine.Drive();
    JEEVES_ASSERT_EQ( received->load( std::memory_order_seq_cst), 0ull);

    // Cycle 1: send 42
    engine.SetPortValue( inPortId, Reg::Known( 42));
    engine.Drive();
    JEEVES_ASSERT_EQ( received->load( std::memory_order_seq_cst), 42ull);

    // Cycle 2: unchanged
    engine.Drive();
    JEEVES_ASSERT_EQ( received->load( std::memory_order_seq_cst), 42ull);

    // Cycle 3: send 99
    engine.SetPortValue( inPortId, Reg::Known( 99));
    engine.Drive();
    JEEVES_ASSERT_EQ( received->load( std::memory_order_seq_cst), 99ull);
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
                    while ( !inPorts[0].IsTrue())
                    {
                        inPorts = co_yield CoroPorts::Pair( Reg::FALSE, Reg::Known( 0));
                    }
                    // Req received: compute result and Ack = 1
                    const uint64_t dataVal = inPorts[1].Val();
                    const uint64_t res = dataVal * 2;
                    while ( inPorts[0].IsTrue())
                    {
                        inPorts = co_yield CoroPorts::Pair( Reg::TRUE, Reg::Known( res));
                    }
                    // Req de-asserted: return to Ack = 0
                    inPorts = co_yield CoroPorts::Pair( Reg::FALSE, Reg::Known( 0));
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
        JEEVES_ASSERT_EQ( engine.GetPortBool( ackPort), Reg::FALSE);
        JEEVES_ASSERT_EQ( engine.GetPortValue( resultPort), Reg::Known( 0));

        // Cycle 1: Request with Data=21
        engine.SetPortValue( dataPort, Reg::Known( 21));
        engine.SetPortBool( reqPort, Reg::TRUE);
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.GetPortBool( ackPort), Reg::TRUE);
        JEEVES_ASSERT_EQ( engine.GetPortValue( resultPort), Reg::Known( 42));

        // Cycle 2: Keep Req=1
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.GetPortBool( ackPort), Reg::TRUE);
        JEEVES_ASSERT_EQ( engine.GetPortValue( resultPort), Reg::Known( 42));

        // Cycle 3: Deassert Req=0
        engine.SetPortBool( reqPort, Reg::FALSE);
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.GetPortBool( ackPort), Reg::FALSE);
        JEEVES_ASSERT_EQ( engine.GetPortValue( resultPort), Reg::Known( 0));
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
        JEEVES_ASSERT_EQ( engine.GetPortBool( ackPort), Reg::FALSE);
        JEEVES_ASSERT_EQ( engine.GetPortValue( resultPort), Reg::Known( 0));

        // Cycle 1: Request with Data=21
        engine.SetPortValue( dataPort, Reg::Known( 21));
        engine.SetPortBool( reqPort, Reg::TRUE);
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.GetPortBool( ackPort), Reg::TRUE);
        JEEVES_ASSERT_EQ( engine.GetPortValue( resultPort), Reg::Known( 42));

        // Cycle 2: Keep Req=1
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.GetPortBool( ackPort), Reg::TRUE);
        JEEVES_ASSERT_EQ( engine.GetPortValue( resultPort), Reg::Known( 42));

        // Cycle 3: Deassert Req=0
        engine.SetPortBool( reqPort, Reg::FALSE);
        engine.Drive();
        JEEVES_ASSERT_EQ( engine.GetPortBool( ackPort), Reg::FALSE);
        JEEVES_ASSERT_EQ( engine.GetPortValue( resultPort), Reg::Known( 0));
    }
}

//-------------------------------------------------------------------------------------------------
