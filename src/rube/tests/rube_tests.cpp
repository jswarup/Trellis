//-------------------------------------------------------------------------------------------------

#include "cove/jeeves.h"
#include "rube/rube.h"

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
