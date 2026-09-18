//-- _test/mod.rs ----------------------------------------------------------------------------------

//--------------------------------------------------------------------------------------------------

use crate::crew::hub::CrewHub;
use crate::crew::node::CrewNode;
use crate::crew::protocol::*;
use crate::crew::vm_adaptor::VMAdaptor;
use crate::crew::vm_runner::{VMRunner, VmBus};
use crate::heist::Atelier;
use crate::rube::{CoroPorts, Layout, ModuleId, SimEngine, SimEngineMode};
use crate::silo::stash::Stash;
use crate::silo::useg::USeg;
use crate::stalks::Coro;
use crate::stalks::work::SpinMutex;
use crate::{jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

//--------------------------------------------------------------------------------------------------

jeeves_test!(Crew, ProtocolStructure, |ctx| {
    jeeves_assert_eq!(ctx, std::mem::size_of::<ProtocolMessage>(), 24);
    let msg = ProtocolMessage {
        _ActionId: CoSimAction::Handshake as i32,
        _Addr: 0x50000000,
        _Value: 0x12345678,
        _PeripheralIndex: -1,
    };
    jeeves_assert_eq!(ctx, std::mem::size_of_val(&msg), 24);
    jeeves_assert_eq!(ctx, msg.Action(), CoSimAction::Handshake);
    jeeves_assert_eq!(ctx, msg.Addr(), 0x50000000);
    jeeves_assert_eq!(ctx, msg.Value(), 0x12345678);
    jeeves_assert_eq!(ctx, msg.PeripheralIndex(), -1);
    jeeves_assert_eq!(ctx, REG_NODE_ID, 0x000);
    jeeves_assert_eq!(ctx, REG_STATUS, 0x004);
    jeeves_assert_eq!(ctx, REG_TX_DATA, 0x008);
    jeeves_assert_eq!(ctx, REG_RX_DATA, 0x00C);
    jeeves_assert_eq!(ctx, REG_RX_COUNT, 0x010);
    jeeves_assert_eq!(ctx, STATUS_TX_READY, 1);
    jeeves_assert_eq!(ctx, STATUS_RX_READY, 2);
    jeeves_assert_eq!(ctx, STATUS_PEER_UP, 4);

    // Codec roundtrip
    let bytes = msg.ToLeBytes();
    jeeves_assert_eq!(ctx, bytes.len(), 24);
    jeeves_assert_eq!(ctx, bytes[0], 10); // Handshake action id
    let decoded = ProtocolMessage::FromLeBytes(&bytes).expect("Valid protocol decode");
    jeeves_assert_eq!(ctx, decoded.Action(), CoSimAction::Handshake);
    jeeves_assert_eq!(ctx, decoded.Addr(), 0x50000000);
    jeeves_assert_eq!(ctx, decoded.Value(), 0x12345678);
    jeeves_assert_eq!(ctx, decoded.PeripheralIndex(), -1);

    // Rejection of invalid action id
    let mut invalidBytes = bytes;
    invalidBytes[0..4].copy_from_slice(&0i32.to_le_bytes()); // ActionId::Invalid
    jeeves_assert!(ctx, ProtocolMessage::FromLeBytes(&invalidBytes).is_err());
    invalidBytes[0..4].copy_from_slice(&9999i32.to_le_bytes()); // Unknown ActionId
    jeeves_assert!(ctx, ProtocolMessage::FromLeBytes(&invalidBytes).is_err());
});

//--------------------------------------------------------------------------------------------------

jeeves_test!(Crew, NodeOperations, |ctx| {
    let node = CrewNode::New(0);
    jeeves_assert_eq!(ctx, node.Id(), 0);
    jeeves_assert!(ctx, !node.IsOnline());
    node.SetOnline(true);
    jeeves_assert!(ctx, node.IsOnline());
    jeeves_assert_eq!(ctx, node.RxCount(), 0);

    node.PushRx(0x41);
    node.PushRx(0x42);
    jeeves_assert_eq!(ctx, node.RxCount(), 2);

    let mut outByte = 0u8;
    let pop1 = node.PopRx(&mut outByte);
    jeeves_assert!(ctx, pop1);
    jeeves_assert_eq!(ctx, outByte, 0x41);
    jeeves_assert_eq!(ctx, node.RxCount(), 1);

    let pop2 = node.PopRx(&mut outByte);
    jeeves_assert!(ctx, pop2);
    jeeves_assert_eq!(ctx, outByte, 0x42);
    jeeves_assert_eq!(ctx, node.RxCount(), 0);

    let pop3 = node.PopRx(&mut outByte);
    jeeves_assert!(ctx, !pop3);

    node.PushRx(0x43);
    node.ClearRx();
    jeeves_assert_eq!(ctx, node.RxCount(), 0);

    // Node stats
    node.RecordRead();
    node.RecordWrite();
    node.RecordByteSent();
    let stats = node.GetStats();
    jeeves_assert_eq!(ctx, stats._ReadsServiced, 1);
    jeeves_assert_eq!(ctx, stats._WritesServiced, 1);
    jeeves_assert_eq!(ctx, stats._BytesSent, 1);
    jeeves_assert_eq!(ctx, stats._BytesReceived, 3);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!(Crew, FullRxQueueRejectsDelivery, |ctx| {
    let hub = CrewHub::New();
    hub.AddNode(0);
    hub.AddNode(1);
    let node0 = hub.FindNode(0).unwrap();
    let node1 = hub.FindNode(1).unwrap();
    node0.SetOnline(true);
    node1.SetOnline(true);

    for value in 0..256 {
        jeeves_assert!(ctx, node1.PushRx(value as u8));
    }
    let request = ProtocolMessage::New(
        CoSimAction::WriteBusByte as i32,
        0x50000000 | REG_TX_DATA as u64,
        b'X' as u64,
        0,
    );
    let response = hub.HandleRequest(&node0, &request);
    jeeves_assert_eq!(ctx, response.Action(), CoSimAction::Error);
    jeeves_assert_eq!(ctx, node1.RxCount(), 256);
    jeeves_assert_eq!(ctx, node0.GetStats()._BytesSent, 0);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!(Crew, HubNodeManagement, |ctx| {
    let hub = CrewHub::New();
    jeeves_assert_eq!(ctx, hub.NodeCount(), 0);
    hub.AddNode(0);
    hub.AddNode(1);
    jeeves_assert_eq!(ctx, hub.NodeCount(), 2);

    let node0 = hub.FindNode(0);
    let node1 = hub.FindNode(1);
    let node2 = hub.FindNode(2);
    jeeves_assert!(ctx, node0.is_some());
    jeeves_assert!(ctx, node1.is_some());
    jeeves_assert!(ctx, node2.is_none());

    let n0 = node0.unwrap();
    let n1 = node1.unwrap();
    jeeves_assert_eq!(ctx, n0.Id(), 0);
    jeeves_assert_eq!(ctx, n1.Id(), 1);
    jeeves_assert!(ctx, !hub.IsNodeOnline(0));
    jeeves_assert!(ctx, !hub.IsNodeOnline(1));

    n0.SetOnline(true);
    jeeves_assert!(ctx, hub.IsNodeOnline(0));
    jeeves_assert!(ctx, !hub.IsNodeOnline(1));
});

//--------------------------------------------------------------------------------------------------

jeeves_test!(Crew, MmioReadRegisters, |ctx| {
    let hub = CrewHub::New();
    hub.AddNode(0);
    hub.AddNode(1);

    let node0 = hub.FindNode(0).unwrap();
    let node1 = hub.FindNode(1).unwrap();
    node0.SetOnline(true);
    node1.SetOnline(true);

    // Read Node ID
    let reqNodeId = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | (REG_NODE_ID as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let respNodeId = hub.HandleRequest(&node0, &reqNodeId);
    jeeves_assert_eq!(ctx, respNodeId.Action(), CoSimAction::Ok);
    jeeves_assert_eq!(ctx, respNodeId.Value(), 0);

    // Read Status (TX ready + Peer Up, no RX yet)
    let reqStatus = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | (REG_STATUS as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let respStatus = hub.HandleRequest(&node0, &reqStatus);
    jeeves_assert_eq!(
        ctx,
        respStatus.Value(),
        (STATUS_TX_READY | STATUS_PEER_UP) as u64
    );

    // Enqueue byte into node 0
    node0.PushRx(0x7E);

    // Status now has RX_READY
    let respStatus2 = hub.HandleRequest(&node0, &reqStatus);
    jeeves_assert_eq!(
        ctx,
        respStatus2.Value(),
        (STATUS_TX_READY | STATUS_RX_READY | STATUS_PEER_UP) as u64
    );

    // Read RX count
    let reqRxCount = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | (REG_RX_COUNT as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let respRxCount = hub.HandleRequest(&node0, &reqRxCount);
    jeeves_assert_eq!(ctx, respRxCount.Value(), 1);

    // Read RX Data
    let reqRxData = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | (REG_RX_DATA as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let respRxData = hub.HandleRequest(&node0, &reqRxData);
    jeeves_assert_eq!(ctx, respRxData.Value(), 0x7E);
    jeeves_assert_eq!(ctx, node0.RxCount(), 0);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!(Crew, MmioInterVmRouting, |ctx| {
    let hub = CrewHub::New();
    hub.AddNode(0);
    hub.AddNode(1);

    let node0 = hub.FindNode(0).unwrap();
    let node1 = hub.FindNode(1).unwrap();
    node0.SetOnline(true);
    node1.SetOnline(true);

    let lastSrc = Arc::new(AtomicU32::new(99));
    let lastDst = Arc::new(AtomicU32::new(99));
    let lastByte = Arc::new(AtomicU8::new(0));

    let ls = lastSrc.clone();
    let ld = lastDst.clone();
    let lb = lastByte.clone();

    hub.SetMessageCallback(move |src, dst, byte| {
        ls.store(src, Ordering::Relaxed);
        ld.store(dst, Ordering::Relaxed);
        lb.store(byte, Ordering::Relaxed);
    });

    // Node 0 writes a byte to TX_DATA
    let writeReq = ProtocolMessage {
        _ActionId: CoSimAction::WriteBusByte as i32,
        _Addr: 0x50000000 | (REG_TX_DATA as u64),
        _Value: b'Z' as u64,
        _PeripheralIndex: 0,
    };
    let writeResp = hub.HandleRequest(&node0, &writeReq);
    jeeves_assert_eq!(ctx, writeResp.Action(), CoSimAction::Ok);

    // Callback fired
    jeeves_assert_eq!(ctx, lastSrc.load(Ordering::Relaxed), 0);
    jeeves_assert_eq!(ctx, lastDst.load(Ordering::Relaxed), 1);
    jeeves_assert_eq!(ctx, lastByte.load(Ordering::Relaxed), b'Z');

    // Node 1 received the byte in RX queue
    jeeves_assert_eq!(ctx, node1.RxCount(), 1);

    // Node 1 reads RX_DATA
    let readReq = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusByte as i32,
        _Addr: 0x50000000 | (REG_RX_DATA as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let readResp = hub.HandleRequest(&node1, &readReq);
    jeeves_assert_eq!(ctx, readResp.Value(), b'Z' as u64);

    let s0 = hub.GetNodeStats(0);
    let s1 = hub.GetNodeStats(1);
    jeeves_assert_eq!(ctx, s0._BytesSent, 1);
    jeeves_assert_eq!(ctx, s0._WritesServiced, 1);
    jeeves_assert_eq!(ctx, s1._BytesReceived, 1);
    jeeves_assert_eq!(ctx, s1._ReadsServiced, 1);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!(Crew, VirtualExchangeProtocol, |ctx| {
    let hub = CrewHub::New();
    hub.AddNode(0);
    hub.AddNode(1);

    let node0 = hub.FindNode(0).unwrap();
    let node1 = hub.FindNode(1).unwrap();
    node0.SetOnline(true);
    node1.SetOnline(true);

    let msgVm0 = "Hello World from Zephyr VM0!\n";
    let msgVm1 = "Hello World back from Zephyr VM1!\n";

    // 1. VM0 sends msgVm0
    USeg::FromLen(msgVm0.len() as u32).Traverse(|i| {
        let b = msgVm0.as_bytes()[i as usize];
        let req = ProtocolMessage {
            _ActionId: CoSimAction::WriteBusByte as i32,
            _Addr: 0x50000000 | (REG_TX_DATA as u64),
            _Value: b as u64,
            _PeripheralIndex: 0,
        };
        hub.HandleRequest(&node0, &req);
    });

    // 2. VM1 reads message
    let mut receivedByVm1 = Stash::New();
    loop {
        let statusReq = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusDword as i32,
            _Addr: 0x50000000 | (REG_STATUS as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let statusResp = hub.HandleRequest(&node1, &statusReq);
        if (statusResp.Value() as u32 & STATUS_RX_READY) == 0 {
            break;
        }

        let rxReq = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusByte as i32,
            _Addr: 0x50000000 | (REG_RX_DATA as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let rxResp = hub.HandleRequest(&node1, &rxReq);
        receivedByVm1.Push((rxResp.Value() & 0xFF) as u8);
    }

    let vm0Bytes = msgVm0.as_bytes();
    USeg::FromLen(receivedByVm1.Size()).Traverse(|i| {
        jeeves_assert_eq!(ctx, receivedByVm1[i], vm0Bytes[i as usize]);
    });

    // 3. VM1 replies msgVm1
    USeg::FromLen(msgVm1.len() as u32).Traverse(|i| {
        let b = msgVm1.as_bytes()[i as usize];
        let req = ProtocolMessage {
            _ActionId: CoSimAction::WriteBusByte as i32,
            _Addr: 0x50000000 | (REG_TX_DATA as u64),
            _Value: b as u64,
            _PeripheralIndex: 0,
        };
        hub.HandleRequest(&node1, &req);
    });

    // 4. VM0 reads reply
    let mut receivedByVm0 = Stash::New();
    loop {
        let statusReq = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusDword as i32,
            _Addr: 0x50000000 | (REG_STATUS as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let statusResp = hub.HandleRequest(&node0, &statusReq);
        if (statusResp.Value() as u32 & STATUS_RX_READY) == 0 {
            break;
        }

        let rxReq = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusByte as i32,
            _Addr: 0x50000000 | (REG_RX_DATA as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let rxResp = hub.HandleRequest(&node0, &rxReq);
        receivedByVm0.Push((rxResp.Value() & 0xFF) as u8);
    }

    let vm1Bytes = msgVm1.as_bytes();
    USeg::FromLen(receivedByVm0.Size()).Traverse(|i| {
        jeeves_assert_eq!(ctx, receivedByVm0[i], vm1Bytes[i as usize]);
    });

    let s0 = hub.GetNodeStats(0);
    let s1 = hub.GetNodeStats(1);
    jeeves_assert_eq!(ctx, s0._BytesSent, msgVm0.len() as u32);
    jeeves_assert_eq!(ctx, s0._BytesReceived, msgVm1.len() as u32);
    jeeves_assert_eq!(ctx, s1._BytesSent, msgVm1.len() as u32);
    jeeves_assert_eq!(ctx, s1._BytesReceived, msgVm0.len() as u32);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!(Crew, RubeMultiVmHelloWorldExchange, |ctx| {
    #[allow(unused_assignments)]
    let msgVm0 = "Hello World from Zephyr VM0!\n";
    let msgVm1 = "Hello World back from Zephyr VM1!\n";

    #[allow(unused_assignments)]
    let mut runExchangeTest = |parallelMode: bool| {
        let receivedByVm1 = Arc::new(SpinMutex::New(Stash::<u8>::New()));
        let receivedByVm0 = Arc::new(SpinMutex::New(Stash::<u8>::New()));
        let vm0Done = Arc::new(AtomicBool::new(false));
        let vm1Done = Arc::new(AtomicBool::new(false));

        let mut layout = Layout::New();

        // 1. VM 0 Coroutine: sends msgVm0, then reads reply until '\n'
        let vm0DoneClone = vm0Done.clone();
        let r0Clone = receivedByVm0.clone();
        let vm0 = VMRunner::New(
            &mut layout,
            "VM0",
            move || {
                let vm0Done = vm0DoneClone.clone();
                let r0 = r0Clone.clone();
                Coro::New(move |yielder, mut inPorts: CoroPorts| {
                    // Step 1: Send msgVm0 byte-by-byte
                    let bytes = msgVm0.as_bytes();
                    let mut i = 0usize;
                    while i < bytes.len() {
                        let c = bytes[i];
                        inPorts = yielder.Suspend(VmBus::Write(REG_TX_DATA, c as u32));
                        while !inPorts.GetBool(0) {
                            inPorts = yielder.Suspend(VmBus::Write(REG_TX_DATA, c as u32));
                        }
                        inPorts = yielder.Suspend(VmBus::Idle());
                        i += 1;
                    }

                    // Step 2: Read reply from VM 1
                    loop {
                        inPorts = yielder.Suspend(VmBus::Read(REG_STATUS));
                        while !inPorts.GetBool(0) {
                            inPorts = yielder.Suspend(VmBus::Read(REG_STATUS));
                        }
                        let status = inPorts[1usize] as u32;
                        inPorts = yielder.Suspend(VmBus::Idle());

                        if (status & STATUS_RX_READY) != 0 {
                            inPorts = yielder.Suspend(VmBus::Read(REG_RX_DATA));
                            while !inPorts.GetBool(0) {
                                inPorts = yielder.Suspend(VmBus::Read(REG_RX_DATA));
                            }
                            let byteVal = (inPorts[1usize] & 0xFF) as u8;
                            inPorts = yielder.Suspend(VmBus::Idle());

                            r0.Lock().Push(byteVal);
                            if byteVal == b'\n' {
                                break;
                            }
                        }
                    }

                    vm0Done.store(true, Ordering::Release);

                    loop {
                        inPorts = yielder.Suspend(VmBus::Idle());
                    }
                })
            },
            ModuleId::None(),
        );

        // 2. VM 1 Coroutine: reads message from VM 0 until '\n', then sends msgVm1
        let vm1DoneClone = vm1Done.clone();
        let r1Clone = receivedByVm1.clone();
        let vm1 = VMRunner::New(
            &mut layout,
            "VM1",
            move || {
                let vm1Done = vm1DoneClone.clone();
                let r1 = r1Clone.clone();
                Coro::New(move |yielder, mut inPorts: CoroPorts| {
                    // Step 1: Read incoming message from VM 0
                    loop {
                        inPorts = yielder.Suspend(VmBus::Read(REG_STATUS));
                        while !inPorts.GetBool(0) {
                            inPorts = yielder.Suspend(VmBus::Read(REG_STATUS));
                        }
                        let status = inPorts[1usize] as u32;
                        inPorts = yielder.Suspend(VmBus::Idle());

                        if (status & STATUS_RX_READY) != 0 {
                            inPorts = yielder.Suspend(VmBus::Read(REG_RX_DATA));
                            while !inPorts.GetBool(0) {
                                inPorts = yielder.Suspend(VmBus::Read(REG_RX_DATA));
                            }
                            let byteVal = (inPorts[1usize] & 0xFF) as u8;
                            inPorts = yielder.Suspend(VmBus::Idle());

                            r1.Lock().Push(byteVal);
                            if byteVal == b'\n' {
                                break;
                            }
                        }
                    }

                    // Step 2: Reply with msgVm1
                    let bytes = msgVm1.as_bytes();
                    let mut i = 0usize;
                    while i < bytes.len() {
                        let c = bytes[i];
                        inPorts = yielder.Suspend(VmBus::Write(REG_TX_DATA, c as u32));
                        while !inPorts.GetBool(0) {
                            inPorts = yielder.Suspend(VmBus::Write(REG_TX_DATA, c as u32));
                        }
                        inPorts = yielder.Suspend(VmBus::Idle());
                        i += 1;
                    }

                    vm1Done.store(true, Ordering::Release);

                    loop {
                        inPorts = yielder.Suspend(VmBus::Idle());
                    }
                })
            },
            ModuleId::None(),
        );

        // 3. Instantiate Adaptors and connect to VMs
        let ad0 = VMAdaptor::New(&mut layout, "Adaptor0", 0, Some(&vm0), ModuleId::None());
        let ad1 = VMAdaptor::New(&mut layout, "Adaptor1", 1, Some(&vm1), ModuleId::None());

        // 4. Interconnect Adaptors
        VMAdaptor::Connect(&mut layout, &ad0, &ad1);

        layout.Freeze();

        // 5. Drive simulation
        let mut engine = SimEngine::Create(&mut layout);
        if parallelMode {
            let _ = Atelier::Reset(4);
            engine.WithMode(SimEngineMode::Parallel(4));
        }

        let mut cycles = 0u32;
        let maxCycles = 1500u32;
        while cycles < maxCycles
            && (!vm0Done.load(Ordering::Acquire) || !vm1Done.load(Ordering::Acquire))
        {
            engine.Drive();
            cycles += 1;
        }

        jeeves_assert!(ctx, vm0Done.load(Ordering::Acquire));
        jeeves_assert!(ctx, vm1Done.load(Ordering::Acquire));

        let r0Bytes = receivedByVm0.Lock();
        let r1Bytes = receivedByVm1.Lock();

        let m0 = msgVm0.as_bytes();
        jeeves_assert_eq!(ctx, r1Bytes.Size(), m0.len() as u32);
        USeg::FromLen(m0.len() as u32).Traverse(|i| {
            jeeves_assert_eq!(ctx, r1Bytes[i], m0[i as usize]);
        });

        let m1 = msgVm1.as_bytes();
        jeeves_assert_eq!(ctx, r0Bytes.Size(), m1.len() as u32);
        USeg::FromLen(m1.len() as u32).Traverse(|i| {
            jeeves_assert_eq!(ctx, r0Bytes[i], m1[i as usize]);
        });

        let s0 = ad0.GetStats();
        let s1 = ad1.GetStats();
        jeeves_assert_eq!(ctx, s0._BytesSent, msgVm0.len() as u32);
        jeeves_assert_eq!(ctx, s0._BytesReceived, msgVm1.len() as u32);
        jeeves_assert_eq!(ctx, s1._BytesSent, msgVm1.len() as u32);
        jeeves_assert_eq!(ctx, s1._BytesReceived, msgVm0.len() as u32);
    };

    // Run in Serial mode
    runExchangeTest(false);

    // Run in Parallel mode
    runExchangeTest(true);
});

//--------------------------------------------------------------------------------------------------

// Console test
jeeves_test!(Crew, Console, Console, |ctx| {
    jeeves_println!(
        ctx,
        "         [Crew Console Test: In-Memory Co-Simulation Hub Active]"
    );
});

//--------------------------------------------------------------------------------------------------

// Example test
jeeves_test!(Crew, Example, Example, |ctx| {
    let hub = CrewHub::New();
    hub.AddNode(0);
    jeeves_assert_eq!(ctx, hub.NodeCount(), 1);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!(Crew, CrewBackpressureStallQueueFull, |ctx| {
    let hub = CrewHub::New();
    hub.AddNode(0);
    hub.AddNode(1);

    let node0 = hub.FindNode(0).unwrap();
    let node1 = hub.FindNode(1).unwrap();
    node0.SetOnline(true);
    node1.SetOnline(true);

    // Node 0 checks initial status: peer is up and TX ready
    let status_req = ProtocolMessage::New(
        CoSimAction::ReadBusDword as i32,
        0x50000000 | (REG_STATUS as u64),
        0,
        0,
    );
    let resp = hub.HandleRequest(&node0, &status_req);
    jeeves_assert_eq!(
        ctx,
        resp.Value() & (STATUS_TX_READY as u64),
        STATUS_TX_READY as u64
    );
    jeeves_assert_eq!(
        ctx,
        resp.Value() & (STATUS_PEER_UP as u64),
        STATUS_PEER_UP as u64
    );

    // Fill Node 1's RX queue (256 bytes)
    for i in 0..256 {
        let write_req = ProtocolMessage::New(
            CoSimAction::WriteBusByte as i32,
            0x50000000 | (REG_TX_DATA as u64),
            (i & 0xFF) as u64,
            0,
        );
        let resp = hub.HandleRequest(&node0, &write_req);
        jeeves_assert_eq!(ctx, resp.Action(), CoSimAction::Ok);
    }
    jeeves_assert_eq!(ctx, node1.RxCount(), 256);
    jeeves_assert!(ctx, !node1.CanPushRx());

    // When Node 1's queue is saturated: STATUS_TX_READY must NOT be asserted!
    let status_saturated = hub.HandleRequest(&node0, &status_req);
    jeeves_assert_eq!(ctx, status_saturated.Value() & (STATUS_TX_READY as u64), 0);

    // Attempting to write another byte must stall/reject with CoSimAction::Error
    let overflow_req = ProtocolMessage::New(
        CoSimAction::WriteBusByte as i32,
        0x50000000 | (REG_TX_DATA as u64),
        0xEE,
        0,
    );
    let overflow_resp = hub.HandleRequest(&node0, &overflow_req);
    jeeves_assert_eq!(ctx, overflow_resp.Action(), CoSimAction::Error);

    // Node 1 pops 1 byte, freeing space
    let mut popped = 0u8;
    jeeves_assert!(ctx, node1.PopRx(&mut popped));
    jeeves_assert_eq!(ctx, node1.RxCount(), 255);
    jeeves_assert!(ctx, node1.CanPushRx());

    // STATUS_TX_READY must now be reasserted
    let status_recovered = hub.HandleRequest(&node0, &status_req);
    jeeves_assert_eq!(
        ctx,
        status_recovered.Value() & (STATUS_TX_READY as u64),
        STATUS_TX_READY as u64
    );

    // Node 0 can now write again successfully
    let retry_resp = hub.HandleRequest(&node0, &overflow_req);
    jeeves_assert_eq!(ctx, retry_resp.Action(), CoSimAction::Ok);
    jeeves_assert_eq!(ctx, node1.RxCount(), 256);
});

//--------------------------------------------------------------------------------------------------

jeeves_test!(Crew, CrewPushRxOutcomePropagation, |ctx| {
    use crate::crew::RxPushOutcome;

    let node = CrewNode::New(42);
    // Offline node must return Offline
    jeeves_assert_eq!(ctx, node.TryPushRx(0xAA), RxPushOutcome::Offline);

    // Online node with available space must return Delivered
    node.SetOnline(true);
    jeeves_assert_eq!(ctx, node.TryPushRx(0xBB), RxPushOutcome::Delivered);
    jeeves_assert_eq!(ctx, node.RxCount(), 1);

    // Fill the remaining 255 slots
    for _ in 0..255 {
        jeeves_assert_eq!(ctx, node.TryPushRx(0xCC), RxPushOutcome::Delivered);
    }
    jeeves_assert_eq!(ctx, node.RxCount(), 256);

    // Saturated node must return QueueFull
    jeeves_assert_eq!(ctx, node.TryPushRx(0xDD), RxPushOutcome::QueueFull);
});
