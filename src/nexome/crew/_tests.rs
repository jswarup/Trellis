//-- _test/mod.rs ----------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::crew::config::CrewLinkConfig;
use	crate::crew::hub::CrewHub;
use	crate::crew::node::CrewNode;
use	crate::crew::protocol::*;
use	crate::crew::vm_adaptor::VMAdaptor;
use	crate::crew::vm_runner::{ VMRunner, VmBus };
use	crate::rube::{ CoroPorts, Layout, ModuleId, SimEngine, SimEngineMode };
use	crate::silo::buff::Buff;
use	crate::silo::stash::Stash;
use	crate::silo::useg::USeg;
use	crate::stalks::Coro;
use	crate::stalks::work::SpinMutex;
use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test };
use	std::sync::Arc;
use	std::sync::atomic::{ AtomicBool, AtomicU8, AtomicU32, Ordering };

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, ProtocolStructure, |ctx| {
    jeeves_assert_eq!( ctx, std::mem::size_of::< ProtocolMessage>(), 24);
    let  	msg = ProtocolMessage {
        _ActionId: CoSimAction::Handshake as i32,
        _Addr: 0x50000000,
        _Value: 0x12345678,
        _PeripheralIndex: -1,
    };
    jeeves_assert_eq!( ctx, std::mem::size_of_val( &msg), 24);
    jeeves_assert_eq!( ctx, msg.Action(), CoSimAction::Handshake);
    jeeves_assert_eq!( ctx, msg.Addr(), 0x50000000);
    jeeves_assert_eq!( ctx, msg.Value(), 0x12345678);
    jeeves_assert_eq!( ctx, msg.PeripheralIndex(), -1);
    jeeves_assert_eq!( ctx, REG_NODE_ID, 0x000);
    jeeves_assert_eq!( ctx, REG_STATUS, 0x004);
    jeeves_assert_eq!( ctx, REG_TX_DATA, 0x008);
    jeeves_assert_eq!( ctx, REG_RX_DATA, 0x00C);
    jeeves_assert_eq!( ctx, REG_RX_COUNT, 0x010);
    jeeves_assert_eq!( ctx, STATUS_TX_READY, 1);
    jeeves_assert_eq!( ctx, STATUS_RX_READY, 2);
    jeeves_assert_eq!( ctx, STATUS_PEER_UP, 4);
    // Codec roundtrip
    let  	bytes = msg.ToLeBytes();
    jeeves_assert_eq!( ctx, bytes.len(), 24);
    jeeves_assert_eq!( ctx, bytes[0], 10);                             // Handshake action id
    let  	decoded = ProtocolMessage::FromLeBytes( &bytes).expect( "Valid protocol decode");
    jeeves_assert_eq!( ctx, decoded.Action(), CoSimAction::Handshake);
    jeeves_assert_eq!( ctx, decoded.Addr(), 0x50000000);
    jeeves_assert_eq!( ctx, decoded.Value(), 0x12345678);
    jeeves_assert_eq!( ctx, decoded.PeripheralIndex(), -1);
    // Rejection of invalid action id
    let  	mut invalidBytes = bytes;
    invalidBytes[0..4].copy_from_slice( &0i32.to_le_bytes());          // ActionId::Invalid
    jeeves_assert!( ctx, ProtocolMessage::FromLeBytes( &invalidBytes).is_err());
    invalidBytes[0..4].copy_from_slice( &9999i32.to_le_bytes());       // Unknown ActionId
    jeeves_assert!( ctx, ProtocolMessage::FromLeBytes( &invalidBytes).is_err());
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, NodeOperations, |ctx| {
    let  	node = CrewNode::New( 0);
    jeeves_assert_eq!( ctx, node.Id(), 0);
    jeeves_assert!( ctx, !node.IsOnline());
    node.SetOnline( true);
    jeeves_assert!( ctx, node.IsOnline());
    jeeves_assert_eq!( ctx, node.RxCount(), 0);
    node.PushRx( 0x41);
    node.PushRx( 0x42);
    jeeves_assert_eq!( ctx, node.RxCount(), 2);
    let  	mut outByte = 0u8;
    let  	pop1 = node.PopRx( &mut outByte);
    jeeves_assert!( ctx, pop1);
    jeeves_assert_eq!( ctx, outByte, 0x41);
    jeeves_assert_eq!( ctx, node.RxCount(), 1);
    let  	pop2 = node.PopRx( &mut outByte);
    jeeves_assert!( ctx, pop2);
    jeeves_assert_eq!( ctx, outByte, 0x42);
    jeeves_assert_eq!( ctx, node.RxCount(), 0);
    let  	pop3 = node.PopRx( &mut outByte);
    jeeves_assert!( ctx, !pop3);
    node.PushRx( 0x43);
    node.ClearRx();
    jeeves_assert_eq!( ctx, node.RxCount(), 0);
    // Node stats
    node.RecordRead();
    node.RecordWrite();
    node.RecordByteSent();
    let  	stats = node.GetStats();
    jeeves_assert_eq!( ctx, stats._ReadsServiced, 1);
    jeeves_assert_eq!( ctx, stats._WritesServiced, 1);
    jeeves_assert_eq!( ctx, stats._BytesSent, 1);
    jeeves_assert_eq!( ctx, stats._BytesReceived, 3);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, FullRxQueueRejectsDelivery, |ctx| {
    let  	hub = CrewHub::New();
    hub.AddNode( 0);
    hub.AddNode( 1);
    let  	node0 = hub.FindNode( 0).unwrap();
    let  	node1 = hub.FindNode( 1).unwrap();
    node0.SetOnline( true);
    node1.SetOnline( true);
    for value in 0..256 {
        jeeves_assert!( ctx, node1.PushRx( value as u8));
    }
    let  	request = ProtocolMessage::New( 
        CoSimAction::WriteBusByte as i32,
        0x50000000 | REG_TX_DATA as u64,
        b'X' as u64,
        0,
    );
    let  	response = hub.HandleRequest( &node0, &request);
    jeeves_assert_eq!( ctx, response.Action(), CoSimAction::Error);
    jeeves_assert_eq!( ctx, node1.RxCount(), 256);
    jeeves_assert_eq!( ctx, node0.GetStats()._BytesSent, 0);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, HubNodeManagement, |ctx| {
    let  	hub = CrewHub::New();
    jeeves_assert_eq!( ctx, hub.NodeCount(), 0);
    hub.AddNode( 0);
    hub.AddNode( 1);
    jeeves_assert_eq!( ctx, hub.NodeCount(), 2);
    let  	node0 = hub.FindNode( 0);
    let  	node1 = hub.FindNode( 1);
    let  	node2 = hub.FindNode( 2);
    jeeves_assert!( ctx, node0.is_some());
    jeeves_assert!( ctx, node1.is_some());
    jeeves_assert!( ctx, node2.is_none());
    let  	n0 = node0.unwrap();
    let  	n1 = node1.unwrap();
    jeeves_assert_eq!( ctx, n0.Id(), 0);
    jeeves_assert_eq!( ctx, n1.Id(), 1);
    jeeves_assert!( ctx, !hub.IsNodeOnline( 0));
    jeeves_assert!( ctx, !hub.IsNodeOnline( 1));
    n0.SetOnline( true);
    jeeves_assert!( ctx, hub.IsNodeOnline( 0));
    jeeves_assert!( ctx, !hub.IsNodeOnline( 1));
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, MmioReadRegisters, |ctx| {
    let  	hub = CrewHub::New();
    hub.AddNode( 0);
    hub.AddNode( 1);
    let  	node0 = hub.FindNode( 0).unwrap();
    let  	node1 = hub.FindNode( 1).unwrap();
    node0.SetOnline( true);
    node1.SetOnline( true);
    // Read Node ID
    let  	reqNodeId = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | ( REG_NODE_ID as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let  	respNodeId = hub.HandleRequest( &node0, &reqNodeId);
    jeeves_assert_eq!( ctx, respNodeId.Action(), CoSimAction::Ok);
    jeeves_assert_eq!( ctx, respNodeId.Value(), 0);
    // Read Status (TX ready + Peer Up, no RX yet)
    let  	reqStatus = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | ( REG_STATUS as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let  	respStatus = hub.HandleRequest( &node0, &reqStatus);
    jeeves_assert_eq!( 
        ctx,
        respStatus.Value(),
        ( STATUS_TX_READY | STATUS_PEER_UP) as u64
    );
    // Enqueue byte into node 0
    node0.PushRx( 0x7E);
    // Status now has RX_READY
    let  	respStatus2 = hub.HandleRequest( &node0, &reqStatus);
    jeeves_assert_eq!( 
        ctx,
        respStatus2.Value(),
        ( STATUS_TX_READY | STATUS_RX_READY | STATUS_PEER_UP) as u64
    );
    // Read RX count
    let  	reqRxCount = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | ( REG_RX_COUNT as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let  	respRxCount = hub.HandleRequest( &node0, &reqRxCount);
    jeeves_assert_eq!( ctx, respRxCount.Value(), 1);
    // Read RX Data
    let  	reqRxData = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | ( REG_RX_DATA as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let  	respRxData = hub.HandleRequest( &node0, &reqRxData);
    jeeves_assert_eq!( ctx, respRxData.Value(), 0x7E);
    jeeves_assert_eq!( ctx, node0.RxCount(), 0);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, MmioInterVmRouting, |ctx| {
    let  	hub = CrewHub::New();
    hub.AddNode( 0);
    hub.AddNode( 1);
    let  	node0 = hub.FindNode( 0).unwrap();
    let  	node1 = hub.FindNode( 1).unwrap();
    node0.SetOnline( true);
    node1.SetOnline( true);
    let  	lastSrc = Arc::new( AtomicU32::new( 99));
    let  	lastDst = Arc::new( AtomicU32::new( 99));
    let  	lastByte = Arc::new( AtomicU8::new( 0));
    let  	ls = lastSrc.clone();
    let  	ld = lastDst.clone();
    let  	lb = lastByte.clone();
    hub.SetMessageCallback( move |src, dst, byte| {
        ls.store( src, Ordering::Relaxed);
        ld.store( dst, Ordering::Relaxed);
        lb.store( byte, Ordering::Relaxed);
    });
    // Node 0 writes a byte to TX_DATA
    let  	writeReq = ProtocolMessage {
        _ActionId: CoSimAction::WriteBusByte as i32,
        _Addr: 0x50000000 | ( REG_TX_DATA as u64),
        _Value: b'Z' as u64,
        _PeripheralIndex: 0,
    };
    let  	writeResp = hub.HandleRequest( &node0, &writeReq);
    jeeves_assert_eq!( ctx, writeResp.Action(), CoSimAction::Ok);
    // Callback fired
    jeeves_assert_eq!( ctx, lastSrc.load( Ordering::Relaxed), 0);
    jeeves_assert_eq!( ctx, lastDst.load( Ordering::Relaxed), 1);
    jeeves_assert_eq!( ctx, lastByte.load( Ordering::Relaxed), b'Z');
    // Node 1 received the byte in RX queue
    jeeves_assert_eq!( ctx, node1.RxCount(), 1);
    // Node 1 reads RX_DATA
    let  	readReq = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusByte as i32,
        _Addr: 0x50000000 | ( REG_RX_DATA as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let  	readResp = hub.HandleRequest( &node1, &readReq);
    jeeves_assert_eq!( ctx, readResp.Value(), b'Z' as u64);
    let  	s0 = hub.GetNodeStats( 0);
    let  	s1 = hub.GetNodeStats( 1);
    jeeves_assert_eq!( ctx, s0._BytesSent, 1);
    jeeves_assert_eq!( ctx, s0._WritesServiced, 1);
    jeeves_assert_eq!( ctx, s1._BytesReceived, 1);
    jeeves_assert_eq!( ctx, s1._ReadsServiced, 1);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, VirtualExchangeProtocol, |ctx| {
    let  	hub = CrewHub::New();
    hub.AddNode( 0);
    hub.AddNode( 1);
    let  	node0 = hub.FindNode( 0).unwrap();
    let  	node1 = hub.FindNode( 1).unwrap();
    node0.SetOnline( true);
    node1.SetOnline( true);
    let  	msgVm0 = "Hello World from Zephyr VM0!\n";
    let  	msgVm1 = "Hello World back from Zephyr VM1!\n";
    // 1. VM0 sends msgVm0
    USeg::FromLen( msgVm0.len() as u32).Traverse( |i| {
        let  	b = msgVm0.as_bytes()[i as usize];
        let  	req = ProtocolMessage {
            _ActionId: CoSimAction::WriteBusByte as i32,
            _Addr: 0x50000000 | ( REG_TX_DATA as u64),
            _Value: b as u64,
            _PeripheralIndex: 0,
        };
        hub.HandleRequest( &node0, &req);
    });
    // 2. VM1 reads message
    let  	mut receivedByVm1 = Stash::New();
    loop {
        let  	statusReq = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusDword as i32,
            _Addr: 0x50000000 | ( REG_STATUS as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let  	statusResp = hub.HandleRequest( &node1, &statusReq);
        if ( statusResp.Value() as u32 & STATUS_RX_READY) == 0 {
            break;
        }
        let  	rxReq = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusByte as i32,
            _Addr: 0x50000000 | ( REG_RX_DATA as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let  	rxResp = hub.HandleRequest( &node1, &rxReq);
        receivedByVm1.Push( ( rxResp.Value() & 0xFF) as u8);
    }
    let  	vm0Bytes = msgVm0.as_bytes();
    USeg::FromLen( receivedByVm1.Size()).Traverse( |i| {
        jeeves_assert_eq!( ctx, receivedByVm1[i], vm0Bytes[i as usize]);
    });
    // 3. VM1 replies msgVm1
    USeg::FromLen( msgVm1.len() as u32).Traverse( |i| {
        let  	b = msgVm1.as_bytes()[i as usize];
        let  	req = ProtocolMessage {
            _ActionId: CoSimAction::WriteBusByte as i32,
            _Addr: 0x50000000 | ( REG_TX_DATA as u64),
            _Value: b as u64,
            _PeripheralIndex: 0,
        };
        hub.HandleRequest( &node1, &req);
    });
    // 4. VM0 reads reply
    let  	mut receivedByVm0 = Stash::New();
    loop {
        let  	statusReq = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusDword as i32,
            _Addr: 0x50000000 | ( REG_STATUS as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let  	statusResp = hub.HandleRequest( &node0, &statusReq);
        if ( statusResp.Value() as u32 & STATUS_RX_READY) == 0 {
            break;
        }
        let  	rxReq = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusByte as i32,
            _Addr: 0x50000000 | ( REG_RX_DATA as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let  	rxResp = hub.HandleRequest( &node0, &rxReq);
        receivedByVm0.Push( ( rxResp.Value() & 0xFF) as u8);
    }
    let  	vm1Bytes = msgVm1.as_bytes();
    USeg::FromLen( receivedByVm0.Size()).Traverse( |i| {
        jeeves_assert_eq!( ctx, receivedByVm0[i], vm1Bytes[i as usize]);
    });
    let  	s0 = hub.GetNodeStats( 0);
    let  	s1 = hub.GetNodeStats( 1);
    jeeves_assert_eq!( ctx, s0._BytesSent, msgVm0.len() as u32);
    jeeves_assert_eq!( ctx, s0._BytesReceived, msgVm1.len() as u32);
    jeeves_assert_eq!( ctx, s1._BytesSent, msgVm1.len() as u32);
    jeeves_assert_eq!( ctx, s1._BytesReceived, msgVm0.len() as u32);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, RubeMultiVmHelloWorldExchange, |ctx| {
    #[allow( unused_assignments)]
    let  	msgVm0 = "Hello World from Zephyr VM0!\n";
    let  	msgVm1 = "Hello World back from Zephyr VM1!\n";
    #[allow( unused_assignments)]
    let  	mut runExchangeTest = |parallelMode: bool| {
        let  	receivedByVm1 = Arc::new( SpinMutex::New( Stash::< u8>::New()));
        let  	receivedByVm0 = Arc::new( SpinMutex::New( Stash::< u8>::New()));
        let  	vm0Done = Arc::new( AtomicBool::new( false));
        let  	vm1Done = Arc::new( AtomicBool::new( false));
        let  	mut layout = Layout::New();
        // 1. VM 0 Coroutine: sends msgVm0, then reads reply until '\n'
        let  	vm0DoneClone = vm0Done.clone();
        let  	r0Clone = receivedByVm0.clone();
        let  	vm0 = VMRunner::New( 
            &mut layout,
            "VM0",
            move || {
                let  	vm0Done = vm0DoneClone.clone();
                let  	r0 = r0Clone.clone();
                Coro::New( move |yielder, mut inPorts: CoroPorts| {
                    // Step 1: Send msgVm0 byte-by-byte
                    let  	bytes = msgVm0.as_bytes();
                    let  	mut i = 0usize;
                    while i < bytes.len() {
                        let  	c = bytes[i];
                        inPorts = yielder.Suspend( VmBus::Write( REG_TX_DATA, c as u32));
                        while !inPorts.GetBool( 0) {
                            inPorts = yielder.Suspend( VmBus::Write( REG_TX_DATA, c as u32));
                        }
                        inPorts = yielder.Suspend( VmBus::Idle());
                        i += 1;
                    }
                    // Step 2: Read reply from VM 1
                    loop {
                        inPorts = yielder.Suspend( VmBus::Read( REG_STATUS));
                        while !inPorts.GetBool( 0) {
                            inPorts = yielder.Suspend( VmBus::Read( REG_STATUS));
                        }
                        let  	status = inPorts[1usize] as u32;
                        inPorts = yielder.Suspend( VmBus::Idle());
                        if ( status & STATUS_RX_READY) != 0 {
                            inPorts = yielder.Suspend( VmBus::Read( REG_RX_DATA));
                            while !inPorts.GetBool( 0) {
                                inPorts = yielder.Suspend( VmBus::Read( REG_RX_DATA));
                            }
                            let  	byteVal = ( inPorts[1usize] & 0xFF) as u8;
                            inPorts = yielder.Suspend( VmBus::Idle());
                            r0.Lock().Push( byteVal);
                            if byteVal == b'\n' {
                                break;
                            }
                        }
                    }
                    vm0Done.store( true, Ordering::Release);
                    loop {
                        inPorts = yielder.Suspend( VmBus::Idle());
                    }
                })
            },
            ModuleId::None(),
        );
        // 2. VM 1 Coroutine: reads message from VM 0 until '\n', then sends msgVm1
        let  	vm1DoneClone = vm1Done.clone();
        let  	r1Clone = receivedByVm1.clone();
        let  	vm1 = VMRunner::New( 
            &mut layout,
            "VM1",
            move || {
                let  	vm1Done = vm1DoneClone.clone();
                let  	r1 = r1Clone.clone();
                Coro::New( move |yielder, mut inPorts: CoroPorts| {
                    // Step 1: Read incoming message from VM 0
                    loop {
                        inPorts = yielder.Suspend( VmBus::Read( REG_STATUS));
                        while !inPorts.GetBool( 0) {
                            inPorts = yielder.Suspend( VmBus::Read( REG_STATUS));
                        }
                        let  	status = inPorts[1usize] as u32;
                        inPorts = yielder.Suspend( VmBus::Idle());
                        if ( status & STATUS_RX_READY) != 0 {
                            inPorts = yielder.Suspend( VmBus::Read( REG_RX_DATA));
                            while !inPorts.GetBool( 0) {
                                inPorts = yielder.Suspend( VmBus::Read( REG_RX_DATA));
                            }
                            let  	byteVal = ( inPorts[1usize] & 0xFF) as u8;
                            inPorts = yielder.Suspend( VmBus::Idle());
                            r1.Lock().Push( byteVal);
                            if byteVal == b'\n' {
                                break;
                            }
                        }
                    }
                    // Step 2: Reply with msgVm1
                    let  	bytes = msgVm1.as_bytes();
                    let  	mut i = 0usize;
                    while i < bytes.len() {
                        let  	c = bytes[i];
                        inPorts = yielder.Suspend( VmBus::Write( REG_TX_DATA, c as u32));
                        while !inPorts.GetBool( 0) {
                            inPorts = yielder.Suspend( VmBus::Write( REG_TX_DATA, c as u32));
                        }
                        inPorts = yielder.Suspend( VmBus::Idle());
                        i += 1;
                    }
                    vm1Done.store( true, Ordering::Release);
                    loop {
                        inPorts = yielder.Suspend( VmBus::Idle());
                    }
                })
            },
            ModuleId::None(),
        );
        // 3. Instantiate Adaptors and connect to VMs
        let  	ad0 = VMAdaptor::New( &mut layout, "Adaptor0", 0, Some( &vm0), ModuleId::None());
        let  	ad1 = VMAdaptor::New( &mut layout, "Adaptor1", 1, Some( &vm1), ModuleId::None());
        // 4. Interconnect Adaptors
        VMAdaptor::Connect( &mut layout, &ad0, &ad1);
        layout.Freeze();
        // 5. Drive simulation
        let  	mut engine = SimEngine::Create( &layout);
        if parallelMode {
            engine.WithMode( SimEngineMode::Parallel( 4));
        }
        let  	mut cycles = 0u32;
        let  	maxCycles = 1500u32;
        while cycles < maxCycles
            && ( !vm0Done.load( Ordering::Acquire) || !vm1Done.load( Ordering::Acquire))
        {
            engine.Drive();
            cycles += 1;
        }
        jeeves_assert!( ctx, vm0Done.load( Ordering::Acquire));
        jeeves_assert!( ctx, vm1Done.load( Ordering::Acquire));
        let  	r0Bytes = receivedByVm0.Lock();
        let  	r1Bytes = receivedByVm1.Lock();
        let  	m0 = msgVm0.as_bytes();
        jeeves_assert_eq!( ctx, r1Bytes.Size(), m0.len() as u32);
        USeg::FromLen( m0.len() as u32).Traverse( |i| {
            jeeves_assert_eq!( ctx, r1Bytes[i], m0[i as usize]);
        });
        let  	m1 = msgVm1.as_bytes();
        jeeves_assert_eq!( ctx, r0Bytes.Size(), m1.len() as u32);
        USeg::FromLen( m1.len() as u32).Traverse( |i| {
            jeeves_assert_eq!( ctx, r0Bytes[i], m1[i as usize]);
        });
        let  	s0 = ad0.GetStats();
        let  	s1 = ad1.GetStats();
        jeeves_assert_eq!( ctx, s0._BytesSent, msgVm0.len() as u32);
        jeeves_assert_eq!( ctx, s0._BytesReceived, msgVm1.len() as u32);
        jeeves_assert_eq!( ctx, s1._BytesSent, msgVm1.len() as u32);
        jeeves_assert_eq!( ctx, s1._BytesReceived, msgVm0.len() as u32);
    };
    // Run in Serial mode
    runExchangeTest( false);
    // Run in Parallel mode
    runExchangeTest( true);
});

//--------------------------------------------------------------------------------------------------
// Console test
jeeves_test!( Crew, Console, Console, |ctx| {
    jeeves_println!( 
        ctx,
        "         [Crew Console Test: In-Memory Co-Simulation Hub Active]"
    );
});

//--------------------------------------------------------------------------------------------------
// Example test
jeeves_test!( Crew, Example, Example, |ctx| {
    let  	hub = CrewHub::New();
    hub.AddNode( 0);
    jeeves_assert_eq!( ctx, hub.NodeCount(), 1);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, CrewBackpressureStallQueueFull, |ctx| {
    let  	hub = CrewHub::New();
    hub.AddNode( 0);
    hub.AddNode( 1);
    let  	node0 = hub.FindNode( 0).unwrap();
    let  	node1 = hub.FindNode( 1).unwrap();
    node0.SetOnline( true);
    node1.SetOnline( true);
    // Node 0 checks initial status: peer is up and TX ready
    let  	status_req = ProtocolMessage::New( 
        CoSimAction::ReadBusDword as i32,
        0x50000000 | ( REG_STATUS as u64),
        0,
        0,
    );
    let  	resp = hub.HandleRequest( &node0, &status_req);
    jeeves_assert_eq!( 
        ctx,
        resp.Value() & ( STATUS_TX_READY as u64),
        STATUS_TX_READY as u64
    );
    jeeves_assert_eq!( 
        ctx,
        resp.Value() & ( STATUS_PEER_UP as u64),
        STATUS_PEER_UP as u64
    );
    // Fill Node 1's RX queue (256 bytes)
    for i in 0..256 {
        let  	write_req = ProtocolMessage::New( 
            CoSimAction::WriteBusByte as i32,
            0x50000000 | ( REG_TX_DATA as u64),
            ( i & 0xFF) as u64,
            0,
        );
        let  	resp = hub.HandleRequest( &node0, &write_req);
        jeeves_assert_eq!( ctx, resp.Action(), CoSimAction::Ok);
    }
    jeeves_assert_eq!( ctx, node1.RxCount(), 256);
    jeeves_assert!( ctx, !node1.CanPushRx());
    // When Node 1's queue is saturated: STATUS_TX_READY must NOT be asserted!
    let  	status_saturated = hub.HandleRequest( &node0, &status_req);
    jeeves_assert_eq!( ctx, status_saturated.Value() & ( STATUS_TX_READY as u64), 0);
    // Attempting to write another byte must stall/reject with CoSimAction::Error
    let  	overflow_req = ProtocolMessage::New( 
        CoSimAction::WriteBusByte as i32,
        0x50000000 | ( REG_TX_DATA as u64),
        0xEE,
        0,
    );
    let  	overflow_resp = hub.HandleRequest( &node0, &overflow_req);
    jeeves_assert_eq!( ctx, overflow_resp.Action(), CoSimAction::Error);
    // Node 1 pops 1 byte, freeing space
    let  	mut popped = 0u8;
    jeeves_assert!( ctx, node1.PopRx( &mut popped));
    jeeves_assert_eq!( ctx, node1.RxCount(), 255);
    jeeves_assert!( ctx, node1.CanPushRx());
    // STATUS_TX_READY must now be reasserted
    let  	status_recovered = hub.HandleRequest( &node0, &status_req);
    jeeves_assert_eq!( 
        ctx,
        status_recovered.Value() & ( STATUS_TX_READY as u64),
        STATUS_TX_READY as u64
    );
    // Node 0 can now write again successfully
    let  	retry_resp = hub.HandleRequest( &node0, &overflow_req);
    jeeves_assert_eq!( ctx, retry_resp.Action(), CoSimAction::Ok);
    jeeves_assert_eq!( ctx, node1.RxCount(), 256);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, CrewPushRxOutcomePropagation, |ctx| {
    use	crate::crew::RxPushOutcome;
    let  	node = CrewNode::New( 42);
    // Offline node must return Offline
    jeeves_assert_eq!( ctx, node.TryPushRx( 0xAA), RxPushOutcome::Offline);
    // Online node with available space must return Delivered
    node.SetOnline( true);
    jeeves_assert_eq!( ctx, node.TryPushRx( 0xBB), RxPushOutcome::Delivered);
    jeeves_assert_eq!( ctx, node.RxCount(), 1);
    // Fill the remaining 255 slots
    for _ in 0..255 {
        jeeves_assert_eq!( ctx, node.TryPushRx( 0xCC), RxPushOutcome::Delivered);
    }
    jeeves_assert_eq!( ctx, node.RxCount(), 256);
    // Saturated node must return QueueFull
    jeeves_assert_eq!( ctx, node.TryPushRx( 0xDD), RxPushOutcome::QueueFull);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, CrewCompetingSendersToSingleReceiver, |ctx| {
    let  	hub = CrewHub::New();
    hub.AddNode( 0); // sender A
    hub.AddNode( 1); // sender B
    hub.AddNode( 2); // shared receiver
    hub.AddLink( CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: Buff::FromDispenser( 1, |_| 2),
    });
    hub.AddLink( CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: Buff::FromDispenser( 1, |_| 2),
    });

    let  	node0 = hub.FindNode( 0).unwrap();
    let  	node1 = hub.FindNode( 1).unwrap();
    let  	node2 = hub.FindNode( 2).unwrap();
    node0.SetOnline( true);
    node1.SetOnline( true);
    node2.SetOnline( true);

    // Pre-fill node 2 to 255/256 bytes (1 slot remaining)
    for i in 0..255 {
        jeeves_assert!( ctx, node2.PushRx( i as u8));
    }
    jeeves_assert_eq!( ctx, node2.RxCount(), 255);

    // Node 0 checks status: TX ready is true
    let  	status_req = ProtocolMessage::New( 
        CoSimAction::ReadBusDword as i32,
        0x50000000 | ( REG_STATUS as u64),
        0,
        0,
    );
    let  	resp0 = hub.HandleRequest( &node0, &status_req);
    jeeves_assert_eq!( ctx, resp0.Value() & ( STATUS_TX_READY as u64), STATUS_TX_READY as u64);

    // Competing sender 1 writes a byte, consuming the last slot
    let  	write_req1 = ProtocolMessage::New( 
        CoSimAction::WriteBusByte as i32,
        0x50000000 | ( REG_TX_DATA as u64),
        0x42,
        0,
    );
    let  	write_resp1 = hub.HandleRequest( &node1, &write_req1);
    jeeves_assert_eq!( ctx, write_resp1.Action(), CoSimAction::Ok);
    jeeves_assert_eq!( ctx, node2.RxCount(), 256);

    // Now sender 0 attempts to write: queue is full, so it gets Error
    let  	write_req0 = ProtocolMessage::New( 
        CoSimAction::WriteBusByte as i32,
        0x50000000 | ( REG_TX_DATA as u64),
        0x99,
        0,
    );
    let  	write_resp0 = hub.HandleRequest( &node0, &write_req0);
    jeeves_assert_eq!( ctx, write_resp0.Action(), CoSimAction::Error);
    jeeves_assert_eq!( ctx, node2.RxCount(), 256);

    // Receiver drains one byte
    let  	mut out = 0u8;
    jeeves_assert!( ctx, node2.PopRx( &mut out));
    jeeves_assert_eq!( ctx, node2.RxCount(), 255);

    // Sender 0 retries and succeeds
    let  	retry_resp0 = hub.HandleRequest( &node0, &write_req0);
    jeeves_assert_eq!( ctx, retry_resp0.Action(), CoSimAction::Ok);
    jeeves_assert_eq!( ctx, node2.RxCount(), 256);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!( Crew, CrewMulticastSemantics, |ctx| {
    let  	hub = CrewHub::New();
    hub.AddNode( 0); // sender
    hub.AddNode( 1); // receiver A
    hub.AddNode( 2); // receiver B
    let  	mut dests = Stash::New();
    dests.Push( 1);
    dests.Push( 2);
    hub.AddLink( CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: dests.IntoBuff(),
    });

    let  	node0 = hub.FindNode( 0).unwrap();
    let  	node1 = hub.FindNode( 1).unwrap();
    let  	node2 = hub.FindNode( 2).unwrap();
    node0.SetOnline( true);
    node1.SetOnline( true);
    // Node 2 is offline initially!

    let  	status_req = ProtocolMessage::New( 
        CoSimAction::ReadBusDword as i32,
        0x50000000 | ( REG_STATUS as u64),
        0,
        0,
    );
    // TX_READY must NOT be asserted because node 2 is offline
    let  	status_resp1 = hub.HandleRequest( &node0, &status_req);
    jeeves_assert_eq!( ctx, status_resp1.Value() & ( STATUS_TX_READY as u64), 0);

    // Attempting to write must fail atomically: node 1 should NOT receive the byte!
    let  	write_req = ProtocolMessage::New( 
        CoSimAction::WriteBusByte as i32,
        0x50000000 | ( REG_TX_DATA as u64),
        0xAA,
        0,
    );
    let  	write_resp1 = hub.HandleRequest( &node0, &write_req);
    jeeves_assert_eq!( ctx, write_resp1.Action(), CoSimAction::Error);
    jeeves_assert_eq!( ctx, node1.RxCount(), 0);
    jeeves_assert_eq!( ctx, node2.RxCount(), 0);

    // Bring node 2 online
    node2.SetOnline( true);
    let  	status_resp2 = hub.HandleRequest( &node0, &status_req);
    jeeves_assert_eq!( ctx, status_resp2.Value() & ( STATUS_TX_READY as u64), STATUS_TX_READY as u64);

    // Write now delivers to both peers
    let  	write_resp2 = hub.HandleRequest( &node0, &write_req);
    jeeves_assert_eq!( ctx, write_resp2.Action(), CoSimAction::Ok);
    jeeves_assert_eq!( ctx, node1.RxCount(), 1);
    jeeves_assert_eq!( ctx, node2.RxCount(), 1);

    // Now saturate node 2 (fill all 256 slots)
    for _ in 0..255 {
        jeeves_assert!( ctx, node2.PushRx( 0xFF));
    }
    jeeves_assert_eq!( ctx, node2.RxCount(), 256);
    jeeves_assert!( ctx, !node2.CanPushRx());

    // TX_READY should now be false
    let  	status_resp3 = hub.HandleRequest( &node0, &status_req);
    jeeves_assert_eq!( ctx, status_resp3.Value() & ( STATUS_TX_READY as u64), 0);

    // Write fails atomically: node 1 must NOT receive the byte even though it has space!
    let  	write_resp3 = hub.HandleRequest( &node0, &write_req);
    jeeves_assert_eq!( ctx, write_resp3.Action(), CoSimAction::Error);
    jeeves_assert_eq!( ctx, node1.RxCount(), 1);
    jeeves_assert_eq!( ctx, node2.RxCount(), 256);
});

//-------------------------------------------------------------------------------------------------
// XplrExample protocol implementation for Crew VM interconnect hub

#[derive(Clone)]
enum CrewExampleChild
{
    Leaf(CrewExampleLeaf),
    Branch(CrewExampleBranch),
}

#[derive(Clone)]
struct CrewExampleLeaf
{
    name:      String,
    path:      String,
    content:   String,
    extension: String,
}

impl CrewExampleLeaf
{
    fn new(name: impl Into<String>, path: impl Into<String>, content: impl Into<String>, extension: impl Into<String>) -> Self
    {
        Self {
            name:      name.into(),
            path:      path.into(),
            content:   content.into(),
            extension: extension.into(),
        }
    }
}

impl crate::fenst::Xplr for CrewExampleLeaf
{
    fn Name(&self) -> &str { &self.name }
    fn Path(&self) -> &str { &self.path }
    fn IsLeaf(&self) -> bool { true }
    fn Leaf(&self) -> Option<&dyn crate::fenst::LeafXplr> { Some(self) }
}

impl crate::fenst::LeafXplr for CrewExampleLeaf
{
    fn Size(&self) -> u64 { self.content.len() as u64 }
    fn Extension(&self) -> &str { &self.extension }
    fn ReadChunk(&self, offset: u64, length: u32) -> Result<crate::fenst::StreamChunk, String>
    {
        let start = usize::try_from(offset).unwrap_or(usize::MAX).min(self.content.len());
        let end = start.saturating_add(length as usize).min(self.content.len());
        Ok(crate::fenst::StreamChunk::New(
            self.path.clone(),
            offset,
            self.Size(),
            self.content[start..end].to_string(),
        ))
    }
}

#[derive(Clone)]
struct CrewExampleBranch
{
    name:     String,
    path:     String,
    children: Vec<CrewExampleChild>,
}

impl CrewExampleBranch
{
    fn new(name: impl Into<String>, path: impl Into<String>) -> Self
    {
        Self {
            name:     name.into(),
            path:     path.into(),
            children: Vec::new(),
        }
    }

    fn with_child(mut self, child: CrewExampleChild) -> Self
    {
        self.children.push(child);
        self
    }
}

impl crate::fenst::Xplr for CrewExampleBranch
{
    fn Name(&self) -> &str { &self.name }
    fn Path(&self) -> &str { &self.path }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn crate::fenst::BranchXplr> { Some(self) }
}

impl crate::fenst::BranchXplr for CrewExampleBranch
{
    fn Children(&self) -> Result<crate::silo::Buff<Box<dyn crate::fenst::Xplr>>, String>
    {
        let mut stash = crate::silo::Stash::New();
        for child in &self.children {
            match child {
                CrewExampleChild::Leaf(l) => {
                    stash.Push(Box::new(l.clone()) as Box<dyn crate::fenst::Xplr>);
                }
                CrewExampleChild::Branch(b) => {
                    stash.Push(Box::new(b.clone()) as Box<dyn crate::fenst::Xplr>);
                }
            }
        }
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(self.children.len() as u32)
    }
}

//-------------------------------------------------------------------------------------------------
// 1. Inspectable example conforming to XplrExample protocol in Crew

jeeves_test!(Crew, HubTopology, XplrExample, |ctx| {
    use crate::fenst::Xplr;

    let hub = CrewHub::New();
    hub.AddNode(0);
    hub.AddNode(1);
    hub.AddNode(2);

    let node0 = hub.FindNode(0).unwrap();
    let node1 = hub.FindNode(1).unwrap();
    let node2 = hub.FindNode(2).unwrap();

    node0.SetOnline(true);
    node1.SetOnline(true);
    node2.SetOnline(true);

    // Multicast link from Node 0 (Coordinator) -> Node 1 & Node 2 (Workers)
    let mut dests = Stash::New();
    dests.Push(1);
    dests.Push(2);
    hub.AddLink(CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: dests.IntoBuff(),
    });

    // Unicast return channels
    let mut ret1 = Stash::New();
    ret1.Push(0);
    hub.AddLink(CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: ret1.IntoBuff(),
    });

    let mut ret2 = Stash::New();
    ret2.Push(0);
    hub.AddLink(CrewLinkConfig {
        source_node_id: 2,
        destination_node_ids: ret2.IntoBuff(),
    });

    // Node 0 broadcasts a command byte 0x42 to all workers
    let write_req = ProtocolMessage::New(
        CoSimAction::WriteBusByte as i32,
        0x50000000 | (REG_TX_DATA as u64),
        0x42,
        0,
    );
    let resp = hub.HandleRequest(&node0, &write_req);
    jeeves_assert_eq!(ctx, resp.Action(), CoSimAction::Ok);
    jeeves_assert_eq!(ctx, node1.RxCount(), 1);
    jeeves_assert_eq!(ctx, node2.RxCount(), 1);

    // Node 1 replies with 0xA1, Node 2 replies with 0xB2
    let write_resp1 = ProtocolMessage::New(
        CoSimAction::WriteBusByte as i32,
        0x50000000 | (REG_TX_DATA as u64),
        0xA1,
        0,
    );
    hub.HandleRequest(&node1, &write_resp1);

    let write_resp2 = ProtocolMessage::New(
        CoSimAction::WriteBusByte as i32,
        0x50000000 | (REG_TX_DATA as u64),
        0xB2,
        0,
    );
    hub.HandleRequest(&node2, &write_resp2);

    jeeves_assert_eq!(ctx, node0.RxCount(), 2);

    // Assemble inspectable Xplr tree conforming to XplrExample protocol
    let root_path = "example://Crew/HubTopology";

    let nodes_branch = CrewExampleBranch::new("Nodes", format!("{}/Nodes", root_path))
        .with_child(CrewExampleChild::Leaf(CrewExampleLeaf::new(
            "Node0_Coordinator",
            format!("{}/Nodes/Node0_Coordinator", root_path),
            format!("Node ID: 0\nRole: Coordinator\nOnline: true\nRxPending: {}\nTxBytes: 1\nRxBytes: 2", node0.RxCount()),
            "node",
        )))
        .with_child(CrewExampleChild::Leaf(CrewExampleLeaf::new(
            "Node1_WorkerA",
            format!("{}/Nodes/Node1_WorkerA", root_path),
            format!("Node ID: 1\nRole: WorkerA\nOnline: true\nRxPending: {}\nTxBytes: 1\nRxBytes: 1", node1.RxCount()),
            "node",
        )))
        .with_child(CrewExampleChild::Leaf(CrewExampleLeaf::new(
            "Node2_WorkerB",
            format!("{}/Nodes/Node2_WorkerB", root_path),
            format!("Node ID: 2\nRole: WorkerB\nOnline: true\nRxPending: {}\nTxBytes: 1\nRxBytes: 1", node2.RxCount()),
            "node",
        )));

    let routing_branch = CrewExampleBranch::new("Routing", format!("{}/Routing", root_path))
        .with_child(CrewExampleChild::Leaf(CrewExampleLeaf::new(
            "Multicast_0_to_1_2",
            format!("{}/Routing/Multicast_0_to_1_2", root_path),
            "Source: Node 0 -> Destinations: [Node 1, Node 2]\nMode: Atomic Multicast\nStatus: Active",
            "link",
        )))
        .with_child(CrewExampleChild::Leaf(CrewExampleLeaf::new(
            "Unicast_1_to_0",
            format!("{}/Routing/Unicast_1_to_0", root_path),
            "Source: Node 1 -> Destinations: [Node 0]\nMode: Point-to-Point\nStatus: Active",
            "link",
        )))
        .with_child(CrewExampleChild::Leaf(CrewExampleLeaf::new(
            "Unicast_2_to_0",
            format!("{}/Routing/Unicast_2_to_0", root_path),
            "Source: Node 2 -> Destinations: [Node 0]\nMode: Point-to-Point\nStatus: Active",
            "link",
        )));

    let traffic_leaf = CrewExampleLeaf::new(
        "TrafficReport",
        format!("{}/TrafficReport", root_path),
        "{\n  \"protocol\": \"CrewCoSim\",\n  \"active_nodes\": 3,\n  \"configured_links\": 3,\n  \"broadcast_packets\": 1,\n  \"unicast_responses\": 2,\n  \"status\": \"healthy\"\n}",
        "json",
    );

    let root = CrewExampleBranch::new("HubTopology", root_path)
        .with_child(CrewExampleChild::Branch(nodes_branch))
        .with_child(CrewExampleChild::Branch(routing_branch))
        .with_child(CrewExampleChild::Leaf(traffic_leaf));

    jeeves_assert_eq!(ctx, root.Name(), "HubTopology");
    jeeves_assert_eq!(ctx, root.Path(), root_path);

    return Ok(Box::new(root) as Box<dyn Xplr + Send>);
});

//-------------------------------------------------------------------------------------------------
// 2. Comprehensive verification of the Crew XplrExample protocol registration and lifecycle

jeeves_test!(Crew, XplrExampleProtocolDemonstration, |ctx| {
    use crate::fenst::example::XplrExample;
    use crate::silo::IArr;

    let all = XplrExample::Registered();
    let mut found = false;
    all.Arr().Traverse(|ex| {
        if ex.suite == "Crew" && ex.name == "HubTopology" {
            found = true;
        }
    });
    jeeves_assert!(ctx, found, "HubTopology must be registered in inventory under Crew suite");

    let mut target_ex = None;
    all.Arr().Traverse(|ex| {
        if ex.suite == "Crew" && ex.name == "HubTopology" {
            target_ex = Some(*ex);
        }
    });
    let ex = target_ex.expect("Crew::HubTopology example must exist");
    let result = ex.Run();
    jeeves_assert!(ctx, result.is_ok(), "Crew::HubTopology XplrExample execution must succeed");

    let root = result.unwrap();
    jeeves_assert_eq!(ctx, root.Name(), "HubTopology");
    jeeves_assert_eq!(ctx, root.Path(), "example://Crew/HubTopology");
    jeeves_assert!(ctx, !root.IsLeaf());
    jeeves_assert!(ctx, root.Branch().is_some());

    let branch = root.Branch().unwrap();
    let children = branch.Children().expect("HubTopology children lookup");
    jeeves_assert_eq!(ctx, children.Size(), 3);

    // Verify first child branch "Nodes"
    let nodes_child = &children[0];
    jeeves_assert_eq!(ctx, nodes_child.Name(), "Nodes");
    jeeves_assert_eq!(ctx, nodes_child.Path(), "example://Crew/HubTopology/Nodes");
    let nodes_branch = nodes_child.Branch().expect("Nodes must be a branch");
    let node_leaves = nodes_branch.Children().expect("Nodes children");
    jeeves_assert_eq!(ctx, node_leaves.Size(), 3);

    // Verify reading chunk from TrafficReport leaf
    let traffic_child = &children[2];
    jeeves_assert_eq!(ctx, traffic_child.Name(), "TrafficReport");
    let traffic_leaf = traffic_child.Leaf().expect("TrafficReport must be a leaf");
    jeeves_assert_eq!(ctx, traffic_leaf.Extension(), "json");
    let chunk = traffic_leaf.ReadChunk(0, 64).expect("ReadChunk");
    jeeves_assert!(ctx, chunk.Content().contains("CrewCoSim"));
});


