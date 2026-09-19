//-- _tests.rs -----------------------------------------------------------------------------------------------------
use	crate::heist::Atelier;
use	crate::rube::*;
use	crate::stalks::Coro;
use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test };
use	std::sync::Arc;
use	std::sync::atomic::{ AtomicU64, Ordering };

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, FourStateLogicOperations, |ctx| {
    // NOT
    {
        let  	r1 = Eval4State( KernelOp::Not, 1, false, false, 0, false, false, 1);
        jeeves_assert_eq!( ctx, r1._Val, 0);
        jeeves_assert!( ctx, !r1._IsX);
        jeeves_assert!( ctx, !r1._IsI);
        let  	r2 = Eval4State( KernelOp::Not, 0, false, false, 0, false, false, 1);
        jeeves_assert_eq!( ctx, r2._Val, 1);
        jeeves_assert!( ctx, !r2._IsX);
        let  	rx = Eval4State( KernelOp::Not, 0, true, false, 0, false, false, 1);
        jeeves_assert!( ctx, rx._IsX);
        let  	rz = Eval4State( KernelOp::Not, 0, false, true, 0, false, false, 1);
        jeeves_assert!( ctx, rz._IsX);
    }
    // AND
    {
        let  	r11 = Eval4State( KernelOp::And, 1, false, false, 1, false, false, 1);
        jeeves_assert_eq!( ctx, r11._Val, 1);
        jeeves_assert!( ctx, !r11._IsX);
        let  	r10 = Eval4State( KernelOp::And, 1, false, false, 0, false, false, 1);
        jeeves_assert_eq!( ctx, r10._Val, 0);
        jeeves_assert!( ctx, !r10._IsX);
        let  	r01 = Eval4State( KernelOp::And, 0, false, false, 1, false, false, 1);
        jeeves_assert_eq!( ctx, r01._Val, 0);
        jeeves_assert!( ctx, !r01._IsX);
        let  	r00 = Eval4State( KernelOp::And, 0, false, false, 0, false, false, 1);
        jeeves_assert_eq!( ctx, r00._Val, 0);
        jeeves_assert!( ctx, !r00._IsX);
        // Dominant 0 over X and Z
        let  	r0x = Eval4State( KernelOp::And, 0, false, false, 0, true, false, 1);
        jeeves_assert_eq!( ctx, r0x._Val, 0);
        jeeves_assert!( ctx, !r0x._IsX);
        let  	r1x = Eval4State( KernelOp::And, 1, false, false, 0, true, false, 1);
        jeeves_assert!( ctx, r1x._IsX);
    }
    // OR
    {
        let  	r11 = Eval4State( KernelOp::Or, 1, false, false, 1, false, false, 1);
        jeeves_assert_eq!( ctx, r11._Val, 1);
        jeeves_assert!( ctx, !r11._IsX);
        let  	r10 = Eval4State( KernelOp::Or, 1, false, false, 0, false, false, 1);
        jeeves_assert_eq!( ctx, r10._Val, 1);
        jeeves_assert!( ctx, !r10._IsX);
        // Dominant 1 over X
        let  	r1x = Eval4State( KernelOp::Or, 1, false, false, 0, true, false, 1);
        jeeves_assert_eq!( ctx, r1x._Val, 1);
        jeeves_assert!( ctx, !r1x._IsX);
        let  	r01 = Eval4State( KernelOp::Or, 0, false, false, 1, false, false, 1);
        jeeves_assert_eq!( ctx, r01._Val, 1);
        jeeves_assert!( ctx, !r01._IsX);
        let  	r00 = Eval4State( KernelOp::Or, 0, false, false, 0, false, false, 1);
        jeeves_assert_eq!( ctx, r00._Val, 0);
        jeeves_assert!( ctx, !r00._IsX);
        let  	r0x = Eval4State( KernelOp::Or, 0, false, false, 0, true, false, 1);
        jeeves_assert!( ctx, r0x._IsX);
    }
    // XOR
    {
        let  	r11 = Eval4State( KernelOp::Xor, 1, false, false, 1, false, false, 1);
        jeeves_assert_eq!( ctx, r11._Val, 0);
        jeeves_assert!( ctx, !r11._IsX);
        let  	r10 = Eval4State( KernelOp::Xor, 1, false, false, 0, false, false, 1);
        jeeves_assert_eq!( ctx, r10._Val, 1);
        jeeves_assert!( ctx, !r10._IsX);
        let  	r00 = Eval4State( KernelOp::Xor, 0, false, false, 0, false, false, 1);
        jeeves_assert_eq!( ctx, r00._Val, 0);
        jeeves_assert!( ctx, !r00._IsX);
        let  	r1x = Eval4State( KernelOp::Xor, 1, false, false, 0, true, false, 1);
        jeeves_assert!( ctx, r1x._IsX);
    }
    // Engine port flag inspection
    {
        let  	mut layout = Layout::New();
        let  	andG = AndGate::New( &mut layout, "AndG");
        layout.Freeze();
        let  	mut engine = SimEngine::Create( &mut layout);
        jeeves_assert!( ctx, engine.IsValid( andG.In1()));
        engine.Set( andG.In1(), 0, true, false);                       // Set X
        jeeves_assert!( ctx, engine.IsX( andG.In1()));
        jeeves_assert!( ctx, !engine.IsValid( andG.In1()));
        engine.Set( andG.In2(), 0, false, true);                       // Set Z / I
        jeeves_assert!( ctx, engine.IsZ( andG.In2()));
        jeeves_assert!( ctx, engine.IsI( andG.In2()));
        jeeves_assert!( ctx, !engine.IsValid( andG.In2()));
        engine.Set( andG.In1(), 1, false, false);
        jeeves_assert!( ctx, engine.IsValid( andG.In1()));
        jeeves_assert_eq!( ctx, engine.Get( andG.In1()), 1);
    }
});

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, KernelOpFullSet, |ctx| {
    let  	mask = 0xFFFFu64;
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::And, 0x00FF, 0x0F0F, mask), 0x000F);
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::Or, 0x00F0, 0x0F00, mask), 0x0FF0);
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::Xor, 0x00FF, 0x000F, mask), 0x00F0);
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::Nand, 0x0001, 0x0001, 1), 0);
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::Not, 0x0000, 0x0000, 1), 1);
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::Nor, 0, 0, 1), 1);
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::Xnor, 1, 1, 1), 1);
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::Add, 100, 200, mask), 300);
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::Sub, 200, 50, mask), 150);
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::Shl, 1, 4, mask), 16);
    jeeves_assert_eq!( ctx, EvalRaw( KernelOp::Shr, 16, 2, mask), 4);
});

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, BasicLogicGates, |ctx| {
    let  	mut layout = Layout::New();
    let  	andG = AndGate::New( &mut layout, "AndG");
    let  	orG = OrGate::New( &mut layout, "OrG");
    let  	xorG = XorGate::New( &mut layout, "XorG");
    let  	notG = NotGate::New( &mut layout, "NotG");
    layout.Freeze();
    let  	mut engine = SimEngine::Create( &mut layout);
    // Set inputs
    engine.SetBool( andG.In1(), true);
    engine.SetBool( andG.In2(), false);
    engine.SetBool( orG.In1(), false);
    engine.SetBool( orG.In2(), true);
    engine.SetBool( xorG.In1(), true);
    engine.SetBool( xorG.In2(), true);
    engine.SetBool( notG.In(), false);
    engine.Drive();
    jeeves_assert!( ctx, !engine.GetBool( andG.Out()));
    jeeves_assert!( ctx, engine.GetBool( orG.Out()));
    jeeves_assert!( ctx, !engine.GetBool( xorG.Out()));
    jeeves_assert!( ctx, engine.GetBool( notG.Out()));
});

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, RSLatchSettle, |ctx| {
    let  	mut layout = Layout::New();
    let  	latch = RSLatch::New( &mut layout, "L1");
    layout.Freeze();
    let  	mut engine = SimEngine::Create( &mut layout);
    // Active-low Set (S=0, R=1) -> Q=1, Q1=0
    latch.SetS( &mut engine, false);
    latch.SetR( &mut engine, true);
    engine.Settle( 10);
    jeeves_assert!( ctx, engine.GetBool( latch.Q()));
    jeeves_assert!( ctx, !engine.GetBool( latch.Q1()));
    // Release (S=1, R=1) -> Hold previous state (Q=1, Q1=0)
    latch.SetS( &mut engine, true);
    latch.SetR( &mut engine, true);
    engine.Settle( 10);
    jeeves_assert!( ctx, engine.GetBool( latch.Q()));
    jeeves_assert!( ctx, !engine.GetBool( latch.Q1()));
    // Active-low Reset (S=1, R=0) -> Q=0, Q1=1
    latch.SetS( &mut engine, true);
    latch.SetR( &mut engine, false);
    engine.Settle( 10);
    jeeves_assert!( ctx, !engine.GetBool( latch.Q()));
    jeeves_assert!( ctx, engine.GetBool( latch.Q1()));
});

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, Adder16SerialAndParallel, |ctx| {
    let  	mut layout = Layout::New();
    let  	adder = Adder::< 16>::New( &mut layout, "Adder16");
    layout.Freeze();
    // 1. Serial Test
    {
        let  	mut engine = SimEngine::Create( &mut layout);
        engine.WithMode( SimEngineMode::Serial);
        adder.SetA( &mut engine, 1234);
        adder.SetB( &mut engine, 5678);
        adder.SetCarryIn( &mut engine, false);
        // 16-bit ripple carry requires up to 32 delta cycles to settle
        let  	cycles = engine.Settle( 64);
        jeeves_assert!( ctx, cycles > 0);
        let  	sum = adder.GetSum( &engine);
        jeeves_assert_eq!( ctx, sum, 1234 + 5678);
        jeeves_assert!( ctx, !engine.GetBool( adder.Carry()));
        // Test carry out
        adder.SetA( &mut engine, 0xFFFF);
        adder.SetB( &mut engine, 1);
        adder.SetCarryIn( &mut engine, false);
        engine.Settle( 64);
        let  	sum2 = adder.GetSum( &engine);
        jeeves_assert_eq!( ctx, sum2, 0);
        jeeves_assert!( ctx, engine.GetBool( adder.Carry()));
    }
    // 2. Parallel Test
    {
        let  	_ = Atelier::Reset( 4);
        let  	mut engine = SimEngine::Create( &mut layout);
        engine.WithMode( SimEngineMode::Parallel( 4));
        adder.SetA( &mut engine, 20000);
        adder.SetB( &mut engine, 15000);
        adder.SetCarryIn( &mut engine, false);
        engine.Settle( 64);
        let  	sum = adder.GetSum( &engine);
        jeeves_assert_eq!( ctx, sum, 35000);
        jeeves_assert!( ctx, !engine.GetBool( adder.Carry()));
    }
});

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, CoroModuleSinkMonitor, |ctx| {
    let  	received = Arc::new( AtomicU64::new( 0));
    let  	mut layout = Layout::New();
    let  	inPorts = [PortDesc::U32( "DataIn")];
    let  	recClone = received.clone();
    let  	modId = layout.AddCoroModule( 
        "SinkMonitor",
        ModuleId::None(),
        &inPorts[..],
        &[][..],
        move || {
            let  	rec = recClone.clone();
            Coro::New( move |yielder, mut inPorts: CoroPorts| {
                loop {
                    let  	val = inPorts[0usize];
                    rec.store( val, Ordering::SeqCst);
                    inPorts = yielder.Suspend( CoroPorts::Empty());
                }
            })
        },
    );
    layout.Freeze();
    let  	mut engine = SimEngine::Create( &mut layout);
    let  	inPortId = layout.InPort( modId, 0);
    // Cycle 0: initial evaluation (inport is 0)
    engine.Drive();
    jeeves_assert_eq!( ctx, received.load( Ordering::SeqCst), 0);
    // Cycle 1: send 42
    engine.Set( inPortId, 42, false, false);
    engine.Drive();
    jeeves_assert_eq!( ctx, received.load( Ordering::SeqCst), 42);
    // Cycle 2: unchanged
    engine.Drive();
    jeeves_assert_eq!( ctx, received.load( Ordering::SeqCst), 42);
    // Cycle 3: send 99
    engine.Set( inPortId, 99, false, false);
    engine.Drive();
    jeeves_assert_eq!( ctx, received.load( Ordering::SeqCst), 99);
});

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, CoroModuleMultiStepProtocol, |ctx| {
    let  	mut runProtocolTest = |parallelMode: bool| {
        let  	mut layout = Layout::New();
        let  	inPorts = [PortDesc::Bool( "Req"), PortDesc::U32( "Data")];
        let  	outPorts = [PortDesc::Bool( "Ack"), PortDesc::U32( "Result")];
        let  	modId = layout.AddCoroModule( 
            "ProtocolServer",
            ModuleId::None(),
            &inPorts[..],
            &outPorts[..],
            || {
                Coro::New( move |yielder, mut inPorts: CoroPorts| {
                    loop {
                        // Idle state: Ack = 0, Result = 0
                        while !inPorts.GetBool( 0) {
                            inPorts = yielder.Suspend( CoroPorts::Pair( 0u64, 0u64));
                        }
                        // Req received: compute result and Ack = 1
                        let  	dataVal = inPorts[1usize];
                        let  	res = dataVal * 2;
                        while inPorts.GetBool( 0) {
                            inPorts = yielder.Suspend( CoroPorts::Pair( 1u64, res));
                        }
                        // Req de-asserted: return to Ack = 0
                        inPorts = yielder.Suspend( CoroPorts::Pair( 0u64, 0u64));
                    }
                })
            },
        );
        layout.Freeze();
        let  	mut engine = SimEngine::Create( &mut layout);
        if parallelMode {
            let  	_ = Atelier::Reset( 4);
            engine.WithMode( SimEngineMode::Parallel( 4));
        }
        let  	reqPort = layout.InPort( modId, 0);
        let  	dataPort = layout.InPort( modId, 1);
        let  	ackPort = layout.OutPort( modId, 0);
        let  	resultPort = layout.OutPort( modId, 1);
        // Cycle 0: initial idle state
        engine.Drive();
        let  	c0Ack = engine.GetBool( ackPort);
        let  	c0Res = engine.Get( resultPort);
        jeeves_assert_eq!( ctx, c0Ack, false);
        jeeves_assert_eq!( ctx, c0Res, 0);
        // Cycle 1: Request with Data=21
        engine.Set( dataPort, 21, false, false);
        engine.SetBool( reqPort, true);
        engine.Drive();
        let  	c1Ack = engine.GetBool( ackPort);
        let  	c1Res = engine.Get( resultPort);
        jeeves_assert_eq!( ctx, c1Ack, true);
        jeeves_assert_eq!( ctx, c1Res, 42);
        // Cycle 2: Keep Req=1
        engine.Drive();
        let  	c2Ack = engine.GetBool( ackPort);
        let  	c2Res = engine.Get( resultPort);
        jeeves_assert_eq!( ctx, c2Ack, true);
        jeeves_assert_eq!( ctx, c2Res, 42);
        // Cycle 3: Deassert Req=0
        engine.SetBool( reqPort, false);
        engine.Drive();
        let  	c3Ack = engine.GetBool( ackPort);
        let  	c3Res = engine.Get( resultPort);
        jeeves_assert_eq!( ctx, c3Ack, false);
        jeeves_assert_eq!( ctx, c3Res, 0);
    };
    // 1. Serial Test
    runProtocolTest( false);
    // 2. Parallel Test
    runProtocolTest( true);
});

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, ClockedSequentialCircuit, |ctx| {
    // 1. Gate-level sequential circuit (CRSLatch)
    let  	mut crsTotalTicks = 0u32;
    let  	mut crsTotalDeltaCycles = 0u32;
    {
        let  	mut layout = Layout::New();
        let  	clkOutDescs = [PortDesc::Bool( "Clk")];
        let  	clkMod = layout.AddModule( 
            "ClockGen",
            ModuleId::None(),
            &[][..],
            &clkOutDescs[..],
            KernelKind::None,
        );
        let  	clkPort = layout.OutPort( clkMod, 0);
        let  	crs = CRSLatch::New( &mut layout, "CRS");
        layout.Connect( clkPort, crs.Clk1());
        layout.Connect( clkPort, crs.Clk2());
        layout.Freeze();
        let  	mut engine = SimEngine::Create( &mut layout);
        engine.WithClock( clkPort);
        jeeves_assert_eq!( ctx, engine.GetClock(), clkPort);
        // Initial clock baseline is false (0)
        engine.SetBool( clkPort, false);
        // Set S=1, R=0 (Active Set)
        crs.SetS( &mut engine, true);
        crs.SetR( &mut engine, false);
        // Before advancing clock, S/R are blocked by NAND gate because Clk=0
        engine.Settle( 10);
        // Advance 1 complete clock tick (0 -> 1 -> Settle -> 0 -> Settle)
        let  	d1 = engine.Advance();
        crsTotalTicks += 1;
        crsTotalDeltaCycles += d1;
        let  	crsStep1Q = engine.GetBool( crs.Q());
        let  	crsStep1Q1 = engine.GetBool( crs.Q1());
        jeeves_assert!( ctx, d1 > 0);
        jeeves_assert!( ctx, crsStep1Q);
        jeeves_assert!( ctx, !crsStep1Q1);
        jeeves_assert!( ctx, !engine.GetBool( clkPort));
        // Change inputs while clock is 0: S=0, R=1 (Active Reset)
        crs.SetS( &mut engine, false);
        crs.SetR( &mut engine, true);
        engine.Settle( 10);
        // Q should hold previous state (1) until clock advances
        jeeves_assert!( ctx, engine.GetBool( crs.Q()));
        jeeves_assert!( ctx, !engine.GetBool( crs.Q1()));
        // Advance 1 complete clock tick: latch updates to Q=0, Q1=1
        let  	d2 = engine.Advance();
        crsTotalTicks += 1;
        crsTotalDeltaCycles += d2;
        let  	crsStep2Q = engine.GetBool( crs.Q());
        let  	crsStep2Q1 = engine.GetBool( crs.Q1());
        jeeves_assert!( ctx, d2 > 0);
        jeeves_assert!( ctx, !crsStep2Q);
        jeeves_assert!( ctx, crsStep2Q1);
        jeeves_assert!( ctx, !engine.GetBool( clkPort));
        // Multi-tick advance (5 ticks)
        let  	d3 = engine.AdvanceTicks( 5);
        crsTotalTicks += 5;
        crsTotalDeltaCycles += d3;
        let  	crsStep3Q = engine.GetBool( crs.Q());
        let  	crsStep3Q1 = engine.GetBool( crs.Q1());
        jeeves_assert!( ctx, d3 > 0);
        jeeves_assert!( ctx, !crsStep3Q);
        jeeves_assert!( ctx, crsStep3Q1);
        // TriggerId overload
        let  	clkTrig = engine.GetPortTrigger( clkPort);
        crs.SetS( &mut engine, true);
        crs.SetR( &mut engine, false);
        let  	d4 = engine.AdvanceTriggerTicks( clkTrig, 1);
        crsTotalTicks += 1;
        crsTotalDeltaCycles += d4;
        let  	crsStep4Q = engine.GetBool( crs.Q());
        let  	crsStep4Q1 = engine.GetBool( crs.Q1());
        jeeves_assert!( ctx, d4 > 0);
        jeeves_assert!( ctx, crsStep4Q);
        jeeves_assert!( ctx, !crsStep4Q1);
    }
    // 2. Synchronous edge-triggered counter with CoroModule
    let  	mut counterTotalTicks = 0u32;
    let  	mut counterTotalDeltaCycles = 0u32;
    {
        let  	mut coroLayout = Layout::New();
        let  	inPorts = [PortDesc::Bool( "Clk")];
        let  	outPorts = [PortDesc::U32( "Count")];
        let  	counterMod = coroLayout.AddCoroModule( 
            "SyncCounter",
            ModuleId::None(),
            &inPorts[..],
            &outPorts[..],
            || {
                Coro::New( move |yielder, mut inPorts: CoroPorts| {
                    let  	mut counter = 0u32;
                    let  	mut lastClk = false;
                    loop {
                        let  	clk = inPorts.GetBool( 0);
                        if !lastClk && clk {
                            counter += 1;
                        }
                        lastClk = clk;
                        inPorts = yielder.Suspend( CoroPorts::Single( counter as u64));
                    }
                })
            },
        );
        coroLayout.Freeze();
        let  	mut counterEngine = SimEngine::Create( &mut coroLayout);
        let  	clkIn = coroLayout.InPort( counterMod, 0);
        let  	countOut = coroLayout.OutPort( counterMod, 0);
        counterEngine.WithClock( clkIn);
        counterEngine.SetBool( clkIn, false);
        counterEngine.Drive();
        let  	c0 = counterEngine.Get( countOut);
        jeeves_assert_eq!( ctx, c0, 0);
        // Advance 1 tick -> counter increments to 1
        let  	cd1 = counterEngine.Advance();
        counterTotalTicks += 1;
        counterTotalDeltaCycles += cd1;
        let  	c1 = counterEngine.Get( countOut);
        jeeves_assert_eq!( ctx, c1, 1);
        // Advance 5 ticks -> counter increments to 6
        let  	cd2 = counterEngine.AdvanceTicks( 5);
        counterTotalTicks += 5;
        counterTotalDeltaCycles += cd2;
        let  	c2 = counterEngine.Get( countOut);
        jeeves_assert_eq!( ctx, c2, 6);
        // Clock returned to baseline
        jeeves_assert!( ctx, !counterEngine.GetBool( clkIn));
    }
    let  	_ = ( 
        crsTotalTicks,
        crsTotalDeltaCycles,
        counterTotalTicks,
        counterTotalDeltaCycles,
    );
});

//------------------------------------------------------------------------------------------------------------------
// VCD Tests
jeeves_test!( Rube, VcdParserBasic, |ctx| {
    let  	vcdContent = r#"
$version
   Segue Rube Engine
$end
$timescale 1ns $end
$date 2026-09-18 $end
$scope module top $end
$var wire 1 ! sig1 $end
$var wire 4 " sig2 $end
$upscope $end
$enddefinitions $end
$dumpvars
0!
b1010 "
$end
#1
1!
#2
0!
b1100 "
"#;
    let  	model = ParseVcd( vcdContent).expect( "Failed to parse VCD");
    jeeves_assert_eq!( ctx, model._Version.trim(), "Segue Rube Engine");
    jeeves_assert_eq!( ctx, model._Timescale.trim(), "1ns");
    jeeves_assert_eq!( ctx, model._Date.trim(), "2026-09-18");
    jeeves_assert_eq!( ctx, model._Scopes.Size(), 1);
    jeeves_assert_eq!( ctx, model._Scopes[0]._Type, "module");
    jeeves_assert_eq!( ctx, model._Scopes[0]._Name, "top");
    jeeves_assert_eq!( ctx, model._TimeSteps.Size(), 3);
    let  	ts0 = &model._TimeSteps[0];
    jeeves_assert_eq!( ctx, ts0._Time, 0);
    jeeves_assert_eq!( ctx, ts0._Values.Size(), 2);
    jeeves_assert_eq!( ctx, ts0._Values[0]._Id, "!");
    jeeves_assert_eq!( ctx, ts0._Values[0]._ValStr, "0");
    jeeves_assert_eq!( ctx, ts0._Values[1]._Id, "\"");
    jeeves_assert_eq!( ctx, ts0._Values[1]._ValStr, "1010");
    let  	ts1 = &model._TimeSteps[1];
    jeeves_assert_eq!( ctx, ts1._Time, 1);
    jeeves_assert_eq!( ctx, ts1._Values.Size(), 1);
    jeeves_assert_eq!( ctx, ts1._Values[0]._Id, "!");
    jeeves_assert_eq!( ctx, ts1._Values[0]._ValStr, "1");
    let  	ts2 = &model._TimeSteps[2];
    jeeves_assert_eq!( ctx, ts2._Time, 2);
    jeeves_assert_eq!( ctx, ts2._Values.Size(), 2);
    jeeves_assert_eq!( ctx, ts2._Values[0]._Id, "!");
    jeeves_assert_eq!( ctx, ts2._Values[0]._ValStr, "0");
    jeeves_assert_eq!( ctx, ts2._Values[1]._Id, "\"");
    jeeves_assert_eq!( ctx, ts2._Values[1]._ValStr, "1100");
});
jeeves_test!( Rube, VcdParserHierarchy, |ctx| {
    let  	vcdContent = r#"
$version Segue Rube Engine $end
$timescale 1ns $end
$scope module top $end
$var wire 1 ! clk $end
$scope module alu $end
$var wire 4 " a $end
$var wire 4 # b $end
$var wire 4 % out $end
$upscope $end
$upscope $end
$enddefinitions $end
$dumpvars
0!
b0000 "
b0000 #
b0000 %
$end
#5
1!
b0011 "
b0101 #
#10
0!
b1000 %
"#;
    let  	model = ParseVcd( vcdContent).expect( "Failed to parse hierarchical VCD");
    jeeves_assert_eq!( ctx, model._Scopes.Size(), 1);
    let  	top = &model._Scopes[0];
    jeeves_assert_eq!( ctx, top._Name, "top");
    jeeves_assert_eq!( ctx, top._Vars.Size(), 1);
    jeeves_assert_eq!( ctx, top._Vars[0]._Name, "clk");
    jeeves_assert_eq!( ctx, top._Scopes.Size(), 1);
    let  	alu = &top._Scopes[0];
    jeeves_assert_eq!( ctx, alu._Name, "alu");
    jeeves_assert_eq!( ctx, alu._Vars.Size(), 3);
    jeeves_assert_eq!( ctx, alu._Vars[0]._Name, "a");
    jeeves_assert_eq!( ctx, alu._Vars[1]._Name, "b");
    jeeves_assert_eq!( ctx, alu._Vars[2]._Name, "out");
});
jeeves_test!( Rube, VcdDisplayModelTimeline, |ctx| {
    let  	vcdContent = r#"
$version Segue Rube Engine $end
$timescale 1ns $end
$scope module top $end
$var wire 1 ! clk $end
$scope module alu $end
$var wire 4 " a $end
$var wire 4 % out $end
$upscope $end
$upscope $end
$enddefinitions $end
$dumpvars
0!
b0001 "
b0001 %
$end
#10
1!
b0010 "
#20
0!
b0100 %
"#;
    let  	model = ParseVcd( vcdContent).expect( "Failed to parse VCD");
    let  	display = VcdDisplayModel::FromVcdModel( &model);
    jeeves_assert_eq!( ctx, display.SignalCount(), 3);
    jeeves_assert_eq!( ctx, display._TimeMin, 0);
    jeeves_assert_eq!( ctx, display._TimeMax, 20);
    let  	clk = display.Signal( 0).unwrap();
    jeeves_assert_eq!( ctx, clk._FullName, "top.clk");
    jeeves_assert!( ctx, clk.IsSingleBit());
    jeeves_assert_eq!( ctx, clk.ValueAt( 0), "0");
    jeeves_assert_eq!( ctx, clk.ValueAt( 5), "0");
    jeeves_assert_eq!( ctx, clk.ValueAt( 10), "1");
    jeeves_assert_eq!( ctx, clk.ValueAt( 15), "1");
    jeeves_assert_eq!( ctx, clk.ValueAt( 20), "0");
    jeeves_assert_eq!( ctx, clk.ValueAt( 100), "0");
    let  	aluA = display.Signal( 1).unwrap();
    jeeves_assert_eq!( ctx, aluA._FullName, "top.alu.a");
    jeeves_assert!( ctx, !aluA.IsSingleBit());
    jeeves_assert_eq!( ctx, aluA.ValueAt( 0), "0001");
    jeeves_assert_eq!( ctx, aluA.ValueAt( 9), "0001");
    jeeves_assert_eq!( ctx, aluA.ValueAt( 10), "0010");
    jeeves_assert_eq!( ctx, aluA.ValueAt( 25), "0010");
    let  	aluOut = display.Signal( 2).unwrap();
    jeeves_assert_eq!( ctx, aluOut._FullName, "top.alu.out");
    jeeves_assert_eq!( ctx, aluOut.ValueAt( 0), "0001");
    jeeves_assert_eq!( ctx, aluOut.ValueAt( 19), "0001");
    jeeves_assert_eq!( ctx, aluOut.ValueAt( 20), "0100");
    jeeves_assert_eq!( ctx, display.ValueAt( "top.clk", 12), Some( "1"));
    jeeves_assert_eq!( ctx, display.ValueAt( "top.alu.a", 12), Some( "0010"));
    jeeves_assert_eq!( ctx, display.ValueAt( "top.nonexistent", 0), None);
});
jeeves_test!( Rube, VcdSerializeRoundTrip, |ctx| {
    let  	vcdContent = r#"
$version Segue Rube Engine $end
$timescale 1ns $end
$date 2026-09-18 $end
$scope module top $end
$var wire 1 ! clk $end
$var wire 4 " data $end
$upscope $end
$enddefinitions $end
$dumpvars
0!
b0001 "
$end
#10
1!
b0010 "
#20
0!
b0100 "
"#;
    let  	model1 = ParseVcd( vcdContent).expect( "Failed to parse initial VCD");
    let  	mut serialized = String::new();
    SerializeVcd( &model1, &mut serialized);
    let  	model2 = ParseVcd( &serialized).expect( "Failed to parse serialized VCD");
    jeeves_assert_eq!( ctx, model1._Version, model2._Version);
    jeeves_assert_eq!( ctx, model1._Timescale, model2._Timescale);
    jeeves_assert_eq!( ctx, model1._Date, model2._Date);
    jeeves_assert_eq!( ctx, model1._Scopes.Size(), model2._Scopes.Size());
    jeeves_assert_eq!( ctx, model1._TimeSteps.Size(), model2._TimeSteps.Size());
    let  	d1 = VcdDisplayModel::FromVcdModel( &model1);
    let  	d2 = VcdDisplayModel::FromVcdModel( &model2);
    jeeves_assert_eq!( ctx, d1.ValueAt( "top.clk", 0), d2.ValueAt( "top.clk", 0));
    jeeves_assert_eq!( ctx, d1.ValueAt( "top.clk", 10), d2.ValueAt( "top.clk", 10));
    jeeves_assert_eq!( ctx, d1.ValueAt( "top.clk", 20), d2.ValueAt( "top.clk", 20));
    jeeves_assert_eq!( ctx, d1.ValueAt( "top.data", 0), d2.ValueAt( "top.data", 0));
    jeeves_assert_eq!( ctx, d1.ValueAt( "top.data", 10), d2.ValueAt( "top.data", 10));
    jeeves_assert_eq!( ctx, d1.ValueAt( "top.data", 20), d2.ValueAt( "top.data", 20));
});
jeeves_test!( Rube, VcdWriterSimulation, |ctx| {
    use	crate::silo::useg::USeg;
    let  	mut layout = Layout::New();
    let  	dLatch = DLatch::New( &mut layout, "DLatch");
    layout.Freeze();
    let  	mut engine = SimEngine::Create( &mut layout);
    let  	vcdWriter = VcdWriter::New( &layout, &engine);
    let  	mut vcdStr = String::new();
    vcdWriter.WriteHeader( &layout, &engine, &mut vcdStr);
    dLatch.SetEnable( &mut engine, false);
    dLatch.SetD( &mut engine, true);
    USeg::FromLen( 4).Traverse( |_| {
        engine.Drive();
        vcdWriter.DumpCycle( &engine, &mut vcdStr);
    });
    jeeves_assert!( ctx, vcdStr.contains( "$timescale 1ns $end"));
    jeeves_assert!( ctx, vcdStr.contains( "$scope module DLatch.Inv $end"));
    jeeves_assert!( ctx, vcdStr.contains( "$var wire 1"));
    jeeves_assert!( ctx, vcdStr.contains( "$dumpvars"));
    jeeves_assert!( ctx, vcdStr.contains( "#1"));
    let  	parsed = ParseVcd( &vcdStr);
    jeeves_assert!( ctx, parsed.is_ok());
    let  	model = parsed.unwrap();
    jeeves_assert!( ctx, model._Scopes.Size() > 0);
});

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, Adder8ConsoleExample, Console, |ctx| {
    use	std::fs;
    // 1. Build an 8-bit ripple-carry adder
    let  	mut layout = Layout::New();
    let  	adder = Adder::< 8>::New( &mut layout, "Adder8");
    layout.Freeze();
    let  	mut engine = SimEngine::Create( &mut layout);
    // 2. Initialize VCD writer and write header
    let  	vcd_writer = VcdWriter::New( &layout, &engine);
    let  	mut vcd_str = String::new();
    vcd_writer.WriteHeader( &layout, &engine, &mut vcd_str);
    // 3. Set operands A = 35, B = 78, CarryIn = false
    let  	a: u64 = 35;
    let  	b: u64 = 78;
    adder.SetA( &mut engine, a);
    adder.SetB( &mut engine, b);
    adder.SetCarryIn( &mut engine, false);
    // 4. Simulate delta cycles to allow carry propagation, recording each cycle into VCD
    let  	mut settled_cycles = 0;
    let  	max_cycles = 32;
    for cycle in 1..=max_cycles {
        engine.Drive();
        vcd_writer.DumpCycle( &engine, &mut vcd_str);
        let  	mut any_edge = false;
        let  	sz = engine._Triggers.Size();
        for t in 0..sz {
            if engine._Triggers.IsEdge( t) {
                any_edge = true;
                break;
            }
        }
        if !any_edge && settled_cycles == 0 {
            settled_cycles = cycle;
        }
    }
    // 5. Verify the arithmetic result (35 + 78 = 113)
    let  	sum = adder.GetSum( &engine);
    let  	carry_out = engine.GetBool( adder.Carry());
    jeeves_assert_eq!( ctx, sum, a + b);
    jeeves_assert_eq!( ctx, sum, 113);
    jeeves_assert!( ctx, !carry_out);
    // 6. Dump the VCD file to disk
    let  	vcd_dir = "out";
    let  	vcd_path = "out/adder8_35_plus_78.vcd";
    let  	_ = fs::create_dir_all( vcd_dir);
    let  	write_res = fs::write( vcd_path, &vcd_str);
    jeeves_assert!( ctx, write_res.is_ok());
    // 7. Verify the dumped VCD parses successfully into a VcdModel
    let  	parsed = ParseVcd( &vcd_str);
    jeeves_assert!( ctx, parsed.is_ok());
    let  	model = parsed.unwrap();
    jeeves_assert!( ctx, model._Scopes.Size() > 0);
    jeeves_assert!( ctx, model._TimeSteps.Size() > 0);
    // 8. Console report diagnostics
    jeeves_println!( ctx, "         [Rube 8-Bit Adder Console Example]");
    jeeves_println!( ctx, "           =================================================");
    jeeves_println!( ctx, "           Operand A (35)   : 0b{:08b} (0x{:02X})", a, a);
    jeeves_println!( ctx, "           Operand B (78)   : 0b{:08b} (0x{:02X})", b, b);
    jeeves_println!( ctx, "           Carry In         : false");
    jeeves_println!( ctx, "           -------------------------------------------------");
    jeeves_println!( ctx, "           Expected Sum     : {} (0b{:08b}, 0x{:02X})", a + b, a + b, a + b);
    jeeves_println!( ctx, "           Calculated Sum   : {} (0b{:08b}, 0x{:02X})", sum, sum, sum);
    jeeves_println!( ctx, "           Carry Out        : {}", carry_out);
    jeeves_println!( ctx, "           Settled Cycles   : {}", settled_cycles);
    jeeves_println!( ctx, "           Verification     : PASS (35 + 78 = 113)");
    jeeves_println!( ctx, "           -------------------------------------------------");
    jeeves_println!( ctx, "           VCD Dump File    : {}", vcd_path);
    jeeves_println!( ctx, "           VCD File Size    : {} bytes", vcd_str.len());
    jeeves_println!( ctx, "           VCD Scopes       : {}", model._Scopes.Size());
    jeeves_println!( ctx, "           VCD Time Steps   : {}", model._TimeSteps.Size());
    jeeves_println!( ctx, "           =================================================");
});

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, Console, Console, |ctx| {
    jeeves_println!( 
        ctx,
        "         [Rube Console Test: Digital Circuit Simulation Active]"
    );
});

//------------------------------------------------------------------------------------------------------------------

jeeves_test!( Rube, Example, Example, |ctx| {
    let  	mut layout = Layout::New();
    let  	adder = Adder::< 8>::New( &mut layout, "Adder8");
    layout.Freeze();
    let  	mut engine = SimEngine::Create( &mut layout);
    adder.SetA( &mut engine, 35);
    adder.SetB( &mut engine, 78);
    engine.Settle( 32);
    jeeves_assert_eq!( ctx, adder.GetSum( &engine), 113);
});
