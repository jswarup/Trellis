use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test };
// src/zephyr/_test/mod.rs
use	crate::crew::config::CrewLinkConfig;
use	crate::crew::hub::CrewHub;
use	crate::crew::protocol::*;
use	crate::zephyr::app::ZephyrVm;
use	crate::zephyr::config::ZephyrVmConfig;
use	crate::zephyr::driver::ZephyrCrewDriver;
use	std::sync::Arc;

//-------------------------------------------------------------------------------------------------

jeeves_test!( Zephyr, DriverApi, |ctx| {
    let  	hub = Arc::new( CrewHub::new());
    hub.add_node( 0);
    hub.add_node( 1);
    hub.add_link( CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser( 1, |_| 1),
    });
    hub.add_link( CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser( 1, |_| 0),
    });
    let  	node0 = hub.find_node( 0).unwrap();
    let  	node1 = hub.find_node( 1).unwrap();
    node0.set_online( true);
    node1.set_online( true);
    let  	driver0 = ZephyrCrewDriver::new( hub.clone(), node0);
    let  	driver1 = ZephyrCrewDriver::new( hub, node1);
    jeeves_assert_eq!( ctx, driver0.get_node_id(), 0);
    jeeves_assert_eq!( ctx, driver1.get_node_id(), 1);
    let  	status0 = driver0.get_status();
    jeeves_assert_eq!( ctx, status0 & STATUS_TX_READY, STATUS_TX_READY);
    jeeves_assert_eq!( ctx, status0 & STATUS_PEER_UP, STATUS_PEER_UP);
    let  	sent = driver0.send( b"Hello".into());
    jeeves_assert_eq!( ctx, sent, 5);
    jeeves_assert_eq!( ctx, driver1.rx_count(), 5);
    let  	mut buf = [0u8; 16];
    let  	received = driver1.recv( ( &mut buf).into());
    jeeves_assert_eq!( ctx, received, 5);
    jeeves_assert_eq!( ctx, &buf[..5], b"Hello");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Zephyr, DualVmExchange, |ctx| {
    let  	hub = Arc::new( CrewHub::new());
    hub.add_node( 0);
    hub.add_node( 1);
    hub.add_link( CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser( 1, |_| 1),
    });
    hub.add_link( CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser( 1, |_| 0),
    });
    let  	node0 = hub.find_node( 0).unwrap();
    let  	node1 = hub.find_node( 1).unwrap();
    let  	config0 = ZephyrVmConfig {
        node_id: 0,
        ..Default::default()
    };
    let  	mut vm0 = ZephyrVm::new( config0, hub.clone(), node0);
    let  	config1 = ZephyrVmConfig {
        node_id: 1,
        ..Default::default()
    };
    let  	mut vm1 = ZephyrVm::new( config1, hub.clone(), node1);
    jeeves_assert_eq!( ctx, vm0.node_id(), 0);
    jeeves_assert_eq!( ctx, vm1.node_id(), 1);
    // Heartbeat ticks
    jeeves_assert_eq!( ctx, vm0.tick_heartbeat(), 1);
    jeeves_assert_eq!( ctx, vm1.tick_heartbeat(), 1);
    let  	msg0 = "Hello World from Zephyr VM0!\n";
    let  	msg1 = "Hello World back from Zephyr VM1!\n";
    // VM0 sends hello message
    let  	sent0 = vm0.send_message( msg0);
    jeeves_assert_eq!( ctx, sent0, msg0.len());
    // VM1 receives message
    let  	rx1 = vm1.recv_message( 128);
    jeeves_assert_eq!( ctx, rx1.as_str(), msg0);
    // VM1 replies back
    let  	sent1 = vm1.send_message( msg1);
    jeeves_assert_eq!( ctx, sent1, msg1.len());
    // VM0 receives reply
    let  	rx0 = vm0.recv_message( 128);
    jeeves_assert_eq!( ctx, rx0.as_str(), msg1);
    // Verify stats
    let  	s0 = hub.get_node_stats( 0);
    let  	s1 = hub.get_node_stats( 1);
    jeeves_assert_eq!( ctx, s0._BytesSent, msg0.len() as u32);
    jeeves_assert_eq!( ctx, s0._BytesReceived, msg1.len() as u32);
    jeeves_assert_eq!( ctx, s1._BytesSent, msg1.len() as u32);
    jeeves_assert_eq!( ctx, s1._BytesReceived, msg0.len() as u32);
});

//-------------------------------------------------------------------------------------------------
// Console test
jeeves_test!( Zephyr, Console, Console, |ctx| {
    jeeves_println!( ctx, "         [Zephyr Console Test: Guest Driver Active]");
});

//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
// Example test
jeeves_test!( Zephyr, Example, Example, |ctx| {
    let  	hub = Arc::new( CrewHub::new());
    hub.add_node( 0);
    let  	node0 = hub.find_node( 0).unwrap();
    let  	config0 = ZephyrVmConfig {
        node_id: 0,
        ..Default::default()
    };
    let  	vm = ZephyrVm::new( config0, hub, node0);
    jeeves_assert_eq!( ctx, vm.node_id(), 0);
});

//-------------------------------------------------------------------------------------------------
// Renode VM Integration Test
jeeves_test!( Zephyr, RenodeVmExecution, |ctx| {
    if std::env::var( "TRELLIS_RUN_RENODE_TESTS").as_deref() != Ok( "1") {
        jeeves_println!( 
            ctx,
            "         [Skipping Renode test: set TRELLIS_RUN_RENODE_TESTS=1 to enable]"
        );
        return;
    }
    let  	elf_path = std::path::PathBuf::from( r"out\zephyr\ae350-n25\zephyr.elf");
    let  	renode_exe = std::path::PathBuf::from( r"C:\Tools\Renode\renode.exe");
    if !elf_path.exists() || !renode_exe.exists() {
        jeeves_println!( 
            ctx,
            "         [Skipping Renode test: Renode or zephyr.elf not found]"
        );
        return;
    }
    let  	hub = Arc::new( CrewHub::new());
    hub.add_node( 0);
    hub.add_node( 1);
    hub.add_link( CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser( 1, |_| 1),
    });
    hub.add_link( CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser( 1, |_| 0),
    });
    let  	node0 = hub.find_node( 0).unwrap();
    let  	node1 = hub.find_node( 1).unwrap();
    let  	config0 = ZephyrVmConfig {
        flavor: crate::zephyr::config::ZephyrFlavor::Renode,
        node_id: 0,
        machine: crate::zephyr::config::ZephyrMachineConfig {
            firmware_elf: Some( elf_path),
            ..ZephyrVmConfig::default().machine
        },
        ..Default::default()
    };
    let  	mut vm0 = ZephyrVm::new( config0, hub.clone(), node0);
    let  	config1 = ZephyrVmConfig {
        flavor: crate::zephyr::config::ZephyrFlavor::Lib,
        node_id: 1,
        ..Default::default()
    };
    let  	vm1 = ZephyrVm::new( config1, hub.clone(), node1);
    jeeves_assert_eq!( ctx, vm0.node_id(), 0);
    jeeves_assert_eq!( ctx, vm1.node_id(), 1);
    // Start Renode VM
    let  	start_res = vm0.runtime().start();
    if let  	Err( ref e) = start_res {
        jeeves_println!( ctx, "         [start_res ERROR: {:?}]", e);
    }
    jeeves_assert!( ctx, start_res.is_ok());
    // Step VM0 to allow Renode to boot and send message
    for _ in 0..200 {
        let  	_ = vm0.runtime().step( crate::zephyr::runtime::StepBudget {
            instruction_limit: 100_000,
            time_ns: 10_000_000,
        });
        if hub.get_node_stats( 0)._BytesSent > 0 {
            break;
        }
        std::thread::sleep( std::time::Duration::from_millis( 100));
    }
    // Read the message on VM1
    let  	rx_msg = vm1.recv_message( 128);
    jeeves_assert!( 
        ctx,
        rx_msg.contains( "Hello from custom Renode Zephyr Guest!")
    );
    // Verify stats
    let  	s0 = hub.get_node_stats( 0);
    let  	s1 = hub.get_node_stats( 1);
    jeeves_assert!( ctx, s0._BytesSent > 0);
    jeeves_assert_eq!( ctx, s1._BytesReceived, s0._BytesSent);
    // Stop VM0 cleanly
    let  	_ = vm0.runtime().stop();
});

//-------------------------------------------------------------------------------------------------
// ShmRingBuffer Lossless & Protocol Tests
jeeves_test!( Zephyr, ShmRingLosslessWrap, |ctx| {
    use	crate::zephyr::shm::{ ShmPacket, ShmRingBuffer };
    use	crate::silo::Arr;

    // 1000 bytes capacity: fits 3 packets of 280 bytes (840 bytes), then wraps
    let  	mut ring = ShmRingBuffer::New( 1000);
    ring.InitSender();
    ring.InitReceiver();

    for round in 0..2000u32 {
        let  	msg = format!( "Packet payload for round {}", round);
        let  	pkt = ShmPacket::New( round, round as u64 * 10, Arr::New( msg.as_ptr(), msg.len() as u32));
        jeeves_assert!( ctx, ring.PushPacket( &pkt).is_ok());

        let  	popped = ring.PopPacket();
        jeeves_assert!( ctx, popped.is_ok());
        let  	rx = popped.unwrap();
        jeeves_assert!( ctx, rx.is_some());
        let  	rx_pkt = rx.unwrap();
        jeeves_assert_eq!( ctx, rx_pkt._SeqNum, round);
        jeeves_assert_eq!( ctx, rx_pkt.PayloadStr(), msg.as_str());
    }
});

jeeves_test!( Zephyr, ShmRingFullRejection, |ctx| {
    use	crate::zephyr::shm::{ ShmPacket, ShmRingBuffer };
    use	crate::silo::Arr;

    let  	mut ring = ShmRingBuffer::New( 1000);
    ring.InitSender();
    ring.InitReceiver();

    let  	p0 = ShmPacket::New( 0, 0, Arr::New( b"Pkt0".as_ptr(), 4));
    let  	p1 = ShmPacket::New( 1, 1, Arr::New( b"Pkt1".as_ptr(), 4));
    let  	p2 = ShmPacket::New( 2, 2, Arr::New( b"Pkt2".as_ptr(), 4));
    let  	p3 = ShmPacket::New( 3, 3, Arr::New( b"Pkt3".as_ptr(), 4));

    jeeves_assert!( ctx, ring.PushPacket( &p0).is_ok());
    jeeves_assert!( ctx, ring.PushPacket( &p1).is_ok());
    jeeves_assert!( ctx, ring.PushPacket( &p2).is_ok());

    // 4th packet must be rejected because ring is full
    let  	full_res = ring.PushPacket( &p3);
    jeeves_assert!( ctx, full_res.is_err());
    jeeves_assert!( ctx, ring.IsFull());

    // Pop one packet
    let  	rx0 = ring.PopPacket().unwrap().unwrap();
    jeeves_assert_eq!( ctx, rx0._SeqNum, 0);

    // Now p3 can be pushed (wrapping around to offset 0)
    jeeves_assert!( ctx, ring.PushPacket( &p3).is_ok());

    // Pop remaining: p1, p2, p3
    let  	rx1 = ring.PopPacket().unwrap().unwrap();
    jeeves_assert_eq!( ctx, rx1._SeqNum, 1);
    let  	rx2 = ring.PopPacket().unwrap().unwrap();
    jeeves_assert_eq!( ctx, rx2._SeqNum, 2);
    let  	rx3 = ring.PopPacket().unwrap().unwrap();
    jeeves_assert_eq!( ctx, rx3._SeqNum, 3);

    // Now empty
    jeeves_assert!( ctx, ring.PopPacket().unwrap().is_none());
});

jeeves_test!( Zephyr, ShmRingCorruptPacketHandling, |ctx| {
    use	crate::zephyr::shm::{ ShmPacket, ShmRingBuffer };
    use	crate::silo::Arr;

    let  	mut ring = ShmRingBuffer::New( 1000);
    ring.InitSender();
    ring.InitReceiver();

    let  	p0 = ShmPacket::New( 42, 100, Arr::New( b"Corruptible".as_ptr(), 11));
    jeeves_assert!( ctx, ring.PushPacket( &p0).is_ok());

    // Corrupt a byte in payload (offset 30)
    ring.CorruptByteForTest( 30, 0xAA);

    // PopPacket must detect checksum mismatch and return Err
    let  	pop_res = ring.PopPacket();
    jeeves_assert!( ctx, pop_res.is_err());
    // Read offset must NOT advance on corruption
    jeeves_assert_eq!( ctx, ring.Ivcb()._ReadOffset, 0);

    // Recover by skipping the corrupt packet
    ring.SkipPacket();
    jeeves_assert_eq!( ctx, ring.Ivcb()._ReadOffset, 280);

    // Now push another valid packet
    let  	p1 = ShmPacket::New( 43, 200, Arr::New( b"ValidAfterSkip".as_ptr(), 14));
    jeeves_assert!( ctx, ring.PushPacket( &p1).is_ok());

    let  	rx1 = ring.PopPacket().unwrap().unwrap();
    jeeves_assert_eq!( ctx, rx1._SeqNum, 43);
    jeeves_assert_eq!( ctx, rx1.PayloadStr(), "ValidAfterSkip");
});
