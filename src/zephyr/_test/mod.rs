// src/zephyr/_test/mod.rs

use crate::crew::hub::CrewHub;
use crate::crew::protocol::*;
use crate::zephyr::app::ZephyrVm;
use crate::zephyr::driver::ZephyrCrewDriver;
use crate::{segue_assert_eq, segue_console_test, segue_example_test, segue_println, segue_test};
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------

segue_test!(Zephyr, DriverApi, |ctx| {
    let hub = Arc::new(CrewHub::new());
    hub.add_node(0);
    hub.add_node(1);

    let node0 = hub.find_node(0).unwrap();
    let node1 = hub.find_node(1).unwrap();
    node0.set_online(true);
    node1.set_online(true);

    let driver0 = ZephyrCrewDriver::new(hub.clone(), node0);
    let driver1 = ZephyrCrewDriver::new(hub, node1);

    segue_assert_eq!(ctx, driver0.get_node_id(), 0);
    segue_assert_eq!(ctx, driver1.get_node_id(), 1);

    let status0 = driver0.get_status();
    segue_assert_eq!(ctx, status0 & STATUS_TX_READY, STATUS_TX_READY);
    segue_assert_eq!(ctx, status0 & STATUS_PEER_UP, STATUS_PEER_UP);

    let sent = driver0.send(b"Hello");
    segue_assert_eq!(ctx, sent, 5);

    segue_assert_eq!(ctx, driver1.rx_count(), 5);

    let mut buf = [0u8; 16];
    let received = driver1.recv(&mut buf);
    segue_assert_eq!(ctx, received, 5);
    segue_assert_eq!(ctx, &buf[..5], b"Hello");
});

//-------------------------------------------------------------------------------------------------

segue_test!(Zephyr, DualVmExchange, |ctx| {
    let hub = Arc::new(CrewHub::new());
    hub.add_node(0);
    hub.add_node(1);

    let node0 = hub.find_node(0).unwrap();
    let node1 = hub.find_node(1).unwrap();

    let mut vm0 = ZephyrVm::new(hub.clone(), node0);
    let mut vm1 = ZephyrVm::new(hub.clone(), node1);

    segue_assert_eq!(ctx, vm0.node_id(), 0);
    segue_assert_eq!(ctx, vm1.node_id(), 1);

    // Heartbeat ticks
    segue_assert_eq!(ctx, vm0.tick_heartbeat(), 1);
    segue_assert_eq!(ctx, vm1.tick_heartbeat(), 1);

    let msg0 = "Hello World from Zephyr VM0!\n";
    let msg1 = "Hello World back from Zephyr VM1!\n";

    // VM0 sends hello message
    let sent0 = vm0.send_message(msg0);
    segue_assert_eq!(ctx, sent0, msg0.len());

    // VM1 receives message
    let rx1 = vm1.recv_message(128);
    segue_assert_eq!(ctx, rx1.as_str(), msg0);

    // VM1 replies back
    let sent1 = vm1.send_message(msg1);
    segue_assert_eq!(ctx, sent1, msg1.len());

    // VM0 receives reply
    let rx0 = vm0.recv_message(128);
    segue_assert_eq!(ctx, rx0.as_str(), msg1);

    // Verify stats
    let s0 = hub.get_node_stats(0);
    let s1 = hub.get_node_stats(1);
    segue_assert_eq!(ctx, s0._BytesSent, msg0.len() as u32);
    segue_assert_eq!(ctx, s0._BytesReceived, msg1.len() as u32);
    segue_assert_eq!(ctx, s1._BytesSent, msg1.len() as u32);
    segue_assert_eq!(ctx, s1._BytesReceived, msg0.len() as u32);
});

//-------------------------------------------------------------------------------------------------
// Console test

segue_console_test!(Zephyr, Console, |ctx| {
    segue_println!(ctx, "         [Zephyr Console Test: Guest Driver Active]");
});

//-------------------------------------------------------------------------------------------------
// Example test

segue_example_test!(Zephyr, Example, |ctx| {
    let hub = Arc::new(CrewHub::new());
    hub.add_node(0);
    let node0 = hub.find_node(0).unwrap();
    let vm = ZephyrVm::new(hub, node0);
    segue_assert_eq!(ctx, vm.node_id(), 0);
});
