use crate::{jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test};
// src/zephyr/_test/mod.rs
// src/zephyr/_tests.rs
use crate::crew::config::CrewLinkConfig;
use crate::crew::hub::CrewHub;
use crate::crew::protocol::*;
use crate::silo::buff::Buff;
use crate::silo::USeg;
use crate::zephyr::app::ZephyrVm;
use crate::zephyr::config::ZephyrVmConfig;
use crate::zephyr::driver::ZephyrCrewDriver;
use crate::zephyr::shm::{
    Fletcher32, ShmIvcb, ShmPacket, ShmRingBuffer, SHM_IVCB_MAGIC, SHM_IVCB_SIZE, SHM_PKT_MAGIC,
    SHM_PKT_SIZE, SHM_RING_CAPACITY,
};
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------

jeeves_test!(Zephyr, DriverApi, |ctx| {
    let hub = Arc::new(CrewHub::new());
    hub.add_node(0);
    hub.add_node(1);
    hub.add_link(CrewLinkConfig {
    let hub = Arc::new(CrewHub::New());
    hub.AddNode(0);
    hub.AddNode(1);
    hub.AddLink(CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 1),
        destination_node_ids: Buff::FromDispenser(1, |_| 1),
    });
    hub.add_link(CrewLinkConfig {
    hub.AddLink(CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 0),
        destination_node_ids: Buff::FromDispenser(1, |_| 0),
    });
    let node0 = hub.find_node(0).unwrap();
    let node1 = hub.find_node(1).unwrap();
    node0.set_online(true);
    node1.set_online(true);
    let driver0 = ZephyrCrewDriver::new(hub.clone(), node0);
    let driver1 = ZephyrCrewDriver::new(hub, node1);
    jeeves_assert_eq!(ctx, driver0.get_node_id(), 0);
    jeeves_assert_eq!(ctx, driver1.get_node_id(), 1);
    let status0 = driver0.get_status();
    let node0 = hub.FindNode(0).unwrap();
    let node1 = hub.FindNode(1).unwrap();
    node0.SetOnline(true);
    node1.SetOnline(true);
    let driver0 = ZephyrCrewDriver::New(hub.clone(), node0);
    let driver1 = ZephyrCrewDriver::New(hub, node1);
    jeeves_assert_eq!(ctx, driver0.GetNodeId(), 0);
    jeeves_assert_eq!(ctx, driver1.GetNodeId(), 1);
    let status0 = driver0.GetStatus();
    jeeves_assert_eq!(ctx, status0 & STATUS_TX_READY, STATUS_TX_READY);
    jeeves_assert_eq!(ctx, status0 & STATUS_PEER_UP, STATUS_PEER_UP);
    let sent = driver0.send(b"Hello");
    let sent = driver0.Send(b"Hello");
    jeeves_assert_eq!(ctx, sent, 5);
    jeeves_assert_eq!(ctx, driver1.rx_count(), 5);
    jeeves_assert_eq!(ctx, driver1.RxCount(), 5);
    let mut buf = [0u8; 16];
    let received = driver1.recv(&mut buf);
    let received = driver1.Recv(&mut buf);
    jeeves_assert_eq!(ctx, received, 5);
    jeeves_assert_eq!(ctx, &buf[..5], b"Hello");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Zephyr, DualVmExchange, |ctx| {
    let hub = Arc::new(CrewHub::new());
    hub.add_node(0);
    hub.add_node(1);
    hub.add_link(CrewLinkConfig {
    let hub = Arc::new(CrewHub::New());
    hub.AddNode(0);
    hub.AddNode(1);
    hub.AddLink(CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 1),
        destination_node_ids: Buff::FromDispenser(1, |_| 1),
    });
    hub.add_link(CrewLinkConfig {
    hub.AddLink(CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 0),
        destination_node_ids: Buff::FromDispenser(1, |_| 0),
    });
    let node0 = hub.find_node(0).unwrap();
    let node1 = hub.find_node(1).unwrap();
    let node0 = hub.FindNode(0).unwrap();
    let node1 = hub.FindNode(1).unwrap();
    let config0 = ZephyrVmConfig {
        node_id: 0,
        ..Default::default()
    };
    let mut vm0 = ZephyrVm::new(config0, hub.clone(), node0);
    let mut vm0 = ZephyrVm::New(config0, hub.clone(), node0);
    let config1 = ZephyrVmConfig {
        node_id: 1,
        ..Default::default()
    };
    let mut vm1 = ZephyrVm::new(config1, hub.clone(), node1);
    jeeves_assert_eq!(ctx, vm0.node_id(), 0);
    jeeves_assert_eq!(ctx, vm1.node_id(), 1);
    let mut vm1 = ZephyrVm::New(config1, hub.clone(), node1);
    jeeves_assert_eq!(ctx, vm0.NodeId(), 0);
    jeeves_assert_eq!(ctx, vm1.NodeId(), 1);
    // Heartbeat ticks
    jeeves_assert_eq!(ctx, vm0.tick_heartbeat(), 1);
    jeeves_assert_eq!(ctx, vm1.tick_heartbeat(), 1);
    jeeves_assert_eq!(ctx, vm0.TickHeartbeat(), 1);
    jeeves_assert_eq!(ctx, vm1.TickHeartbeat(), 1);
    let msg0 = "Hello World from Zephyr VM0!\n";
    let msg1 = "Hello World back from Zephyr VM1!\n";
    // VM0 sends hello message
    let sent0 = vm0.send_message(msg0);
    let sent0 = vm0.SendMessage(msg0);
    jeeves_assert_eq!(ctx, sent0, msg0.len());
    // VM1 receives message
    let rx1 = vm1.recv_message(128);
    let rx1 = vm1.RecvMessage(128);
    jeeves_assert_eq!(ctx, rx1.as_str(), msg0);
    // VM1 replies back
    let sent1 = vm1.send_message(msg1);
    let sent1 = vm1.SendMessage(msg1);
    jeeves_assert_eq!(ctx, sent1, msg1.len());
    // VM0 receives reply
    let rx0 = vm0.recv_message(128);
    let rx0 = vm0.RecvMessage(128);
    jeeves_assert_eq!(ctx, rx0.as_str(), msg1);
    // Verify stats
    let s0 = hub.get_node_stats(0);
    let s1 = hub.get_node_stats(1);
    let s0 = hub.GetNodeStats(0);
    let s1 = hub.GetNodeStats(1);
    jeeves_assert_eq!(ctx, s0._BytesSent, msg0.len() as u32);
    jeeves_assert_eq!(ctx, s0._BytesReceived, msg1.len() as u32);
    jeeves_assert_eq!(ctx, s1._BytesSent, msg1.len() as u32);
    jeeves_assert_eq!(ctx, s1._BytesReceived, msg0.len() as u32);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Zephyr, ShmIvcbLayout, |ctx| {
    let mut ivcb = ShmIvcb::New(SHM_RING_CAPACITY);
    jeeves_assert!(ctx, ivcb.IsValid());
    jeeves_assert_eq!(ctx, ivcb._Magic, SHM_IVCB_MAGIC);
    jeeves_assert_eq!(ctx, ivcb._Version, 1);
    jeeves_assert_eq!(ctx, ivcb.RingSize(), SHM_RING_CAPACITY);
    jeeves_assert_eq!(ctx, ivcb.IsSenderReady(), false);
    jeeves_assert_eq!(ctx, ivcb.IsReceiverReady(), false);

    ivcb.SetSenderReady(true);
    ivcb.SetReceiverReady(true);
    ivcb.SetWriteOffset(1024);
    ivcb.SetReadOffset(512);

    jeeves_assert!(ctx, ivcb.IsSenderReady());
    jeeves_assert!(ctx, ivcb.IsReceiverReady());
    jeeves_assert_eq!(ctx, ivcb.WriteOffset(), 1024);
    jeeves_assert_eq!(ctx, ivcb.ReadOffset(), 512);

    let bytes = ivcb.ToBytes();
    jeeves_assert_eq!(ctx, bytes.len(), SHM_IVCB_SIZE);

    let deserialized = ShmIvcb::FromBytes(&bytes).unwrap();
    jeeves_assert_eq!(ctx, deserialized, ivcb);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Zephyr, Fletcher32Checksum, |ctx| {
    // 1. Empty data test: Fletcher-32 with initial sum1=0xffff, sum2=0xffff
    // Returns (sum2 << 16) | sum1 after reduction
    let emptyCsum = Fletcher32(b"");
    jeeves_assert_eq!(ctx, emptyCsum, 0xFFFF_FFFF);

    // 2. Odd-length and even-length strings
    let testMsg1 = b"A";
    let csum1 = Fletcher32(testMsg1);
    jeeves_assert!(ctx, csum1 != 0);

    let testMsg2 = b"Hello from Sender! Packet #0 [uptime 1000ms]";
    let csum2 = Fletcher32(testMsg2);

    // 3. ShmPacket creation and verification
    let pkt = ShmPacket::New(0, 1000, testMsg2);
    jeeves_assert_eq!(ctx, pkt._Magic, SHM_PKT_MAGIC);
    jeeves_assert_eq!(ctx, pkt._Checksum, csum2);
    jeeves_assert!(ctx, pkt.VerifyChecksum());
    jeeves_assert_eq!(ctx, pkt.PayloadStr(), "Hello from Sender! Packet #0 [uptime 1000ms]");

    // 4. Test serialization / deserialization of ShmPacket
    let pktBytes = pkt.ToBytes();
    jeeves_assert_eq!(ctx, pktBytes.len(), SHM_PKT_SIZE);
    let pktRecovered = ShmPacket::FromBytes(&pktBytes).unwrap();
    jeeves_assert_eq!(ctx, pktRecovered, pkt);
    jeeves_assert!(ctx, pktRecovered.VerifyChecksum());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Zephyr, PcieShmDualVmExchange, |ctx| {
    // Simulate BAR2 PCIe shared memory ring buffer between Sender (VM 0) and Receiver (VM 1)
    let ringCap = 4096u32;
    let mut shm = ShmRingBuffer::New(ringCap);

    // Sender initializes IVCB
    shm.InitSender();
    jeeves_assert!(ctx, shm.Ivcb().IsValid());
    jeeves_assert!(ctx, shm.Ivcb().IsSenderReady());

    // Receiver signals readiness
    shm.InitReceiver();
    jeeves_assert!(ctx, shm.Ivcb().IsReceiverReady());

    // Sender posts 5 packets into the ring
    let numPackets = 5u32;
    USeg::FromLen(numPackets).Traverse(|seq| {
        let msg = format!("Hello from Dual-VM PCIe SHM! Packet #{}", seq);
        let pkt = ShmPacket::New(seq, (seq as u64) * 100, msg.as_bytes());
        jeeves_assert!(ctx, pkt.VerifyChecksum());
        jeeves_assert!(ctx, shm.PushPacket(&pkt).is_ok());
    });

    jeeves_assert!(ctx, shm.HasPackets());

    // Receiver reads all 5 packets from the ring
    USeg::FromLen(numPackets).Traverse(|seq| {
        let optPkt = shm.PopPacket().unwrap();
        jeeves_assert!(ctx, optPkt.is_some());
        let pkt = optPkt.unwrap();
        jeeves_assert_eq!(ctx, pkt._Magic, SHM_PKT_MAGIC);
        jeeves_assert_eq!(ctx, pkt._SeqNum, seq);
        jeeves_assert!(ctx, pkt.VerifyChecksum());
        let expectedMsg = format!("Hello from Dual-VM PCIe SHM! Packet #{}", seq);
        jeeves_assert_eq!(ctx, pkt.PayloadStr(), expectedMsg.as_str());
    });

    // Ring should now be empty
    jeeves_assert_eq!(ctx, shm.HasPackets(), false);
    jeeves_assert!(ctx, shm.PopPacket().unwrap().is_none());
});

//-------------------------------------------------------------------------------------------------

// Console test
jeeves_test!(Zephyr, Console, Console, |ctx| {
    jeeves_println!(ctx, "         [Zephyr Console Test: Guest Driver Active]");
});

//-------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------

// Example test
jeeves_test!(Zephyr, Example, Example, |ctx| {
    let hub = Arc::new(CrewHub::new());
    hub.add_node(0);
    let node0 = hub.find_node(0).unwrap();
    let hub = Arc::new(CrewHub::New());
    hub.AddNode(0);
    let node0 = hub.FindNode(0).unwrap();
    let config0 = ZephyrVmConfig {
        node_id: 0,
        ..Default::default()
    };
    let vm = ZephyrVm::new(config0, hub, node0);
    jeeves_assert_eq!(ctx, vm.node_id(), 0);
    let vm = ZephyrVm::New(config0, hub, node0);
    jeeves_assert_eq!(ctx, vm.NodeId(), 0);
});

//-------------------------------------------------------------------------------------------------

// Renode VM Integration Test
jeeves_test!(Zephyr, RenodeVmExecution, |ctx| {
    if std::env::var("SEGUE_RUN_RENODE_TESTS").as_deref() != Ok("1") {
        jeeves_println!(
            ctx,
            "         [Skipping Renode test: set SEGUE_RUN_RENODE_TESTS=1 to enable]"
        );
        return;
    }
    let elf_path = std::path::PathBuf::from(r"out\zephyr\ae350-n25\zephyr.elf");
    let renode_exe = std::path::PathBuf::from(r"C:\Tools\Renode\renode.exe");
    if !elf_path.exists() || !renode_exe.exists() {
    let elfPath = std::path::PathBuf::from(r"out\zephyr\ae350-n25\zephyr.elf");
    let renodeExe = std::path::PathBuf::from(r"C:\Tools\Renode\renode.exe");
    if !elfPath.exists() || !renodeExe.exists() {
        jeeves_println!(
            ctx,
            "         [Skipping Renode test: Renode or zephyr.elf not found]"
        );
        return;
    }
    let hub = Arc::new(CrewHub::new());
    hub.add_node(0);
    hub.add_node(1);
    hub.add_link(CrewLinkConfig {
    let hub = Arc::new(CrewHub::New());
    hub.AddNode(0);
    hub.AddNode(1);
    hub.AddLink(CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 1),
        destination_node_ids: Buff::FromDispenser(1, |_| 1),
    });
    hub.add_link(CrewLinkConfig {
    hub.AddLink(CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 0),
        destination_node_ids: Buff::FromDispenser(1, |_| 0),
    });
    let node0 = hub.find_node(0).unwrap();
    let node1 = hub.find_node(1).unwrap();
    let node0 = hub.FindNode(0).unwrap();
    let node1 = hub.FindNode(1).unwrap();
    let config0 = ZephyrVmConfig {
        flavor: crate::zephyr::config::ZephyrFlavor::Renode,
        node_id: 0,
        machine: crate::zephyr::config::ZephyrMachineConfig {
            firmware_elf: Some(elf_path),
            firmware_elf: Some(elfPath),
            ..ZephyrVmConfig::default().machine
        },
        ..Default::default()
    };
    let mut vm0 = ZephyrVm::new(config0, hub.clone(), node0);
    let mut vm0 = ZephyrVm::New(config0, hub.clone(), node0);
    let config1 = ZephyrVmConfig {
        flavor: crate::zephyr::config::ZephyrFlavor::Lib,
        node_id: 1,
        ..Default::default()
    };
    let vm1 = ZephyrVm::new(config1, hub.clone(), node1);
    jeeves_assert_eq!(ctx, vm0.node_id(), 0);
    jeeves_assert_eq!(ctx, vm1.node_id(), 1);
    let vm1 = ZephyrVm::New(config1, hub.clone(), node1);
    jeeves_assert_eq!(ctx, vm0.NodeId(), 0);
    jeeves_assert_eq!(ctx, vm1.NodeId(), 1);
    // Start Renode VM
    let start_res = vm0.runtime().start();
    if let Err(ref e) = start_res {
        jeeves_println!(ctx, "         [start_res ERROR: {:?}]", e);
    let startRes = vm0.Runtime().start();
    if let Err(ref e) = startRes {
        jeeves_println!(ctx, "         [startRes ERROR: {:?}]", e);
    }
    jeeves_assert!(ctx, start_res.is_ok());
    jeeves_assert!(ctx, startRes.is_ok());
    // Step VM0 to allow Renode to boot and send message
    for _ in 0..200 {
        let _ = vm0.runtime().step(crate::zephyr::runtime::StepBudget {
        let _ = vm0.Runtime().step(crate::zephyr::runtime::StepBudget {
            instruction_limit: 100_000,
            time_ns: 10_000_000,
        });
        if hub.get_node_stats(0)._BytesSent > 0 {
        if hub.GetNodeStats(0)._BytesSent > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    // Read the message on VM1
    let rx_msg = vm1.recv_message(128);
    let rxMsg = vm1.RecvMessage(128);
    jeeves_assert!(
        ctx,
        rx_msg.contains("Hello from custom Renode Zephyr Guest!")
        rxMsg.contains("Hello from custom Renode Zephyr Guest!")
    );
    // Verify stats
    let s0 = hub.get_node_stats(0);
    let s1 = hub.get_node_stats(1);
    let s0 = hub.GetNodeStats(0);
    let s1 = hub.GetNodeStats(1);
    jeeves_assert!(ctx, s0._BytesSent > 0);
    jeeves_assert_eq!(ctx, s1._BytesReceived, s0._BytesSent);
    // Stop VM0 cleanly
    let _ = vm0.runtime().stop();
    let _ = vm0.Runtime().stop();
});
