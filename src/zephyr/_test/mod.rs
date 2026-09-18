use crate::{jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test};
// src/zephyr/_test/mod.rs
use crate::crew::config::CrewLinkConfig;
use crate::crew::hub::CrewHub;
use crate::crew::protocol::*;
use crate::zephyr::app::ZephyrVm;
use crate::zephyr::config::ZephyrVmConfig;
use crate::zephyr::driver::ZephyrCrewDriver;
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------

jeeves_test!(Zephyr, DriverApi, |ctx| {
    let hub = Arc::new(CrewHub::new());
    hub.add_node(0);
    hub.add_node(1);
    hub.add_link(CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 1),
    });
    hub.add_link(CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 0),
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
    jeeves_assert_eq!(ctx, status0 & STATUS_TX_READY, STATUS_TX_READY);
    jeeves_assert_eq!(ctx, status0 & STATUS_PEER_UP, STATUS_PEER_UP);
    let sent = driver0.send(b"Hello");
    jeeves_assert_eq!(ctx, sent, 5);
    jeeves_assert_eq!(ctx, driver1.rx_count(), 5);
    let mut buf = [0u8; 16];
    let received = driver1.recv(&mut buf);
    jeeves_assert_eq!(ctx, received, 5);
    jeeves_assert_eq!(ctx, &buf[..5], b"Hello");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Zephyr, DualVmExchange, |ctx| {
    let hub = Arc::new(CrewHub::new());
    hub.add_node(0);
    hub.add_node(1);
    hub.add_link(CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 1),
    });
    hub.add_link(CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 0),
    });
    let node0 = hub.find_node(0).unwrap();
    let node1 = hub.find_node(1).unwrap();
    let config0 = ZephyrVmConfig {
        node_id: 0,
        ..Default::default()
    };
    let mut vm0 = ZephyrVm::new(config0, hub.clone(), node0);
    let config1 = ZephyrVmConfig {
        node_id: 1,
        ..Default::default()
    };
    let mut vm1 = ZephyrVm::new(config1, hub.clone(), node1);
    jeeves_assert_eq!(ctx, vm0.node_id(), 0);
    jeeves_assert_eq!(ctx, vm1.node_id(), 1);
    // Heartbeat ticks
    jeeves_assert_eq!(ctx, vm0.tick_heartbeat(), 1);
    jeeves_assert_eq!(ctx, vm1.tick_heartbeat(), 1);
    let msg0 = "Hello World from Zephyr VM0!\n";
    let msg1 = "Hello World back from Zephyr VM1!\n";
    // VM0 sends hello message
    let sent0 = vm0.send_message(msg0);
    jeeves_assert_eq!(ctx, sent0, msg0.len());
    // VM1 receives message
    let rx1 = vm1.recv_message(128);
    jeeves_assert_eq!(ctx, rx1.as_str(), msg0);
    // VM1 replies back
    let sent1 = vm1.send_message(msg1);
    jeeves_assert_eq!(ctx, sent1, msg1.len());
    // VM0 receives reply
    let rx0 = vm0.recv_message(128);
    jeeves_assert_eq!(ctx, rx0.as_str(), msg1);
    // Verify stats
    let s0 = hub.get_node_stats(0);
    let s1 = hub.get_node_stats(1);
    jeeves_assert_eq!(ctx, s0._BytesSent, msg0.len() as u32);
    jeeves_assert_eq!(ctx, s0._BytesReceived, msg1.len() as u32);
    jeeves_assert_eq!(ctx, s1._BytesSent, msg1.len() as u32);
    jeeves_assert_eq!(ctx, s1._BytesReceived, msg0.len() as u32);
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
    let config0 = ZephyrVmConfig {
        node_id: 0,
        ..Default::default()
    };
    let vm = ZephyrVm::new(config0, hub, node0);
    jeeves_assert_eq!(ctx, vm.node_id(), 0);
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
        source_node_id: 0,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 1),
    });
    hub.add_link(CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: crate::silo::buff::Buff::FromDispenser(1, |_| 0),
    });
    let node0 = hub.find_node(0).unwrap();
    let node1 = hub.find_node(1).unwrap();
    let config0 = ZephyrVmConfig {
        flavor: crate::zephyr::config::ZephyrFlavor::Renode,
        node_id: 0,
        machine: crate::zephyr::config::ZephyrMachineConfig {
            firmware_elf: Some(elf_path),
            ..ZephyrVmConfig::default().machine
        },
        ..Default::default()
    };
    let mut vm0 = ZephyrVm::new(config0, hub.clone(), node0);
    let config1 = ZephyrVmConfig {
        flavor: crate::zephyr::config::ZephyrFlavor::Lib,
        node_id: 1,
        ..Default::default()
    };
    let vm1 = ZephyrVm::new(config1, hub.clone(), node1);
    jeeves_assert_eq!(ctx, vm0.node_id(), 0);
    jeeves_assert_eq!(ctx, vm1.node_id(), 1);
    // Start Renode VM
    let start_res = vm0.runtime().start();
    if let Err(ref e) = start_res {
        jeeves_println!(ctx, "         [start_res ERROR: {:?}]", e);
    }
    jeeves_assert!(ctx, start_res.is_ok());
    // Step VM0 to allow Renode to boot and send message
    for _ in 0..200 {
        let _ = vm0.runtime().step(crate::zephyr::runtime::StepBudget {
            instruction_limit: 100_000,
            time_ns: 10_000_000,
        });
        if hub.get_node_stats(0)._BytesSent > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    // Read the message on VM1
    let rx_msg = vm1.recv_message(128);
    jeeves_assert!(
        ctx,
        rx_msg.contains("Hello from custom Renode Zephyr Guest!")
    );
    // Verify stats
    let s0 = hub.get_node_stats(0);
    let s1 = hub.get_node_stats(1);
    jeeves_assert!(ctx, s0._BytesSent > 0);
    jeeves_assert_eq!(ctx, s1._BytesReceived, s0._BytesSent);
    // Stop VM0 cleanly
    let _ = vm0.runtime().stop();
});
