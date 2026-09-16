// src/crew/_test/mod.rs

use crate::crew::hub::CrewHub;
use crate::crew::node::CrewNode;
use crate::crew::protocol::*;
use crate::{
    segue_assert, segue_assert_eq, segue_console_test, segue_example_test, segue_println,
    segue_test,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

//-------------------------------------------------------------------------------------------------

segue_test!(Crew, ProtocolStructure, |ctx| {
    segue_assert_eq!(ctx, std::mem::size_of::<ProtocolMessage>(), 24);

    let msg = ProtocolMessage {
        _ActionId: CoSimAction::Handshake as i32,
        _Addr: 0x50000000,
        _Value: 0x12345678,
        _PeripheralIndex: -1,
    };

    segue_assert_eq!(ctx, std::mem::size_of_val(&msg), 24);
    segue_assert_eq!(ctx, msg.action(), CoSimAction::Handshake);
    segue_assert_eq!(ctx, msg.addr(), 0x50000000);
    segue_assert_eq!(ctx, msg.value(), 0x12345678);
    segue_assert_eq!(ctx, msg.peripheral_index(), -1);

    segue_assert_eq!(ctx, REG_NODE_ID, 0x000);
    segue_assert_eq!(ctx, REG_STATUS, 0x004);
    segue_assert_eq!(ctx, REG_TX_DATA, 0x008);
    segue_assert_eq!(ctx, REG_RX_DATA, 0x00C);
    segue_assert_eq!(ctx, REG_RX_COUNT, 0x010);

    segue_assert_eq!(ctx, STATUS_TX_READY, 1);
    segue_assert_eq!(ctx, STATUS_RX_READY, 2);
    segue_assert_eq!(ctx, STATUS_PEER_UP, 4);
});

//-------------------------------------------------------------------------------------------------

segue_test!(Crew, NodeOperations, |ctx| {
    let node = CrewNode::new(0);

    segue_assert_eq!(ctx, node.id(), 0);
    segue_assert!(ctx, !node.is_online());

    node.set_online(true);
    segue_assert!(ctx, node.is_online());

    segue_assert_eq!(ctx, node.rx_count(), 0);

    node.push_rx(0x41);
    node.push_rx(0x42);
    segue_assert_eq!(ctx, node.rx_count(), 2);

    let mut out_byte = 0u8;
    let pop1 = node.pop_rx(&mut out_byte);
    segue_assert!(ctx, pop1);
    segue_assert_eq!(ctx, out_byte, 0x41);
    segue_assert_eq!(ctx, node.rx_count(), 1);

    let pop2 = node.pop_rx(&mut out_byte);
    segue_assert!(ctx, pop2);
    segue_assert_eq!(ctx, out_byte, 0x42);
    segue_assert_eq!(ctx, node.rx_count(), 0);

    let pop3 = node.pop_rx(&mut out_byte);
    segue_assert!(ctx, !pop3);

    node.push_rx(0x43);
    node.clear_rx();
    segue_assert_eq!(ctx, node.rx_count(), 0);

    // Node stats
    node.record_read();
    node.record_write();
    node.record_byte_sent();

    let stats = node.get_stats();
    segue_assert_eq!(ctx, stats._ReadsServiced, 1);
    segue_assert_eq!(ctx, stats._WritesServiced, 1);
    segue_assert_eq!(ctx, stats._BytesSent, 1);
    segue_assert_eq!(ctx, stats._BytesReceived, 2);
});

//-------------------------------------------------------------------------------------------------

segue_test!(Crew, HubNodeManagement, |ctx| {
    let hub = CrewHub::new();
    segue_assert_eq!(ctx, hub.node_count(), 0);

    hub.add_node(0);
    hub.add_node(1);
    segue_assert_eq!(ctx, hub.node_count(), 2);

    let node0 = hub.find_node(0);
    let node1 = hub.find_node(1);
    let node2 = hub.find_node(2);

    segue_assert!(ctx, node0.is_some());
    segue_assert!(ctx, node1.is_some());
    segue_assert!(ctx, node2.is_none());

    let n0 = node0.unwrap();
    let n1 = node1.unwrap();
    segue_assert_eq!(ctx, n0.id(), 0);
    segue_assert_eq!(ctx, n1.id(), 1);

    segue_assert!(ctx, !hub.is_node_online(0));
    segue_assert!(ctx, !hub.is_node_online(1));

    n0.set_online(true);
    segue_assert!(ctx, hub.is_node_online(0));
    segue_assert!(ctx, !hub.is_node_online(1));
});

//-------------------------------------------------------------------------------------------------

segue_test!(Crew, MmioReadRegisters, |ctx| {
    let hub = CrewHub::new();
    hub.add_node(0);
    hub.add_node(1);

    hub.add_link(crate::crew::config::CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: vec![1],
    });
    hub.add_link(crate::crew::config::CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: vec![0],
    });

    let node0 = hub.find_node(0).unwrap();
    let node1 = hub.find_node(1).unwrap();
    node0.set_online(true);
    node1.set_online(true);

    // Read Node ID
    let req_node_id = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | (REG_NODE_ID as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let resp_node_id = hub.handle_request(&node0, &req_node_id);
    segue_assert_eq!(ctx, resp_node_id.action(), CoSimAction::Ok);
    segue_assert_eq!(ctx, resp_node_id.value(), 0);

    // Read Status (TX ready + Peer Up, no RX yet)
    let req_status = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | (REG_STATUS as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let resp_status = hub.handle_request(&node0, &req_status);
    segue_assert_eq!(
        ctx,
        resp_status.value(),
        (STATUS_TX_READY | STATUS_PEER_UP) as u64
    );

    // Enqueue byte into node 0
    node0.push_rx(0x7E);

    // Status now has RX_READY
    let resp_status2 = hub.handle_request(&node0, &req_status);
    segue_assert_eq!(
        ctx,
        resp_status2.value(),
        (STATUS_TX_READY | STATUS_RX_READY | STATUS_PEER_UP) as u64
    );

    // Read RX count
    let req_rx_count = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | (REG_RX_COUNT as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let resp_rx_count = hub.handle_request(&node0, &req_rx_count);
    segue_assert_eq!(ctx, resp_rx_count.value(), 1);

    // Read RX Data
    let req_rx_data = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusDword as i32,
        _Addr: 0x50000000 | (REG_RX_DATA as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let resp_rx_data = hub.handle_request(&node0, &req_rx_data);
    segue_assert_eq!(ctx, resp_rx_data.value(), 0x7E);
    segue_assert_eq!(ctx, node0.rx_count(), 0);
});

//-------------------------------------------------------------------------------------------------

segue_test!(Crew, MmioInterVmRouting, |ctx| {
    let hub = CrewHub::new();
    hub.add_node(0);
    hub.add_node(1);

    hub.add_link(crate::crew::config::CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: vec![1],
    });
    hub.add_link(crate::crew::config::CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: vec![0],
    });

    let node0 = hub.find_node(0).unwrap();
    let node1 = hub.find_node(1).unwrap();

    let last_src = Arc::new(AtomicU32::new(99));
    let last_dst = Arc::new(AtomicU32::new(99));
    let last_byte = Arc::new(AtomicU8::new(0));

    let ls = last_src.clone();
    let ld = last_dst.clone();
    let lb = last_byte.clone();
    hub.set_message_callback(move |src, dst, byte| {
        ls.store(src, Ordering::Relaxed);
        ld.store(dst, Ordering::Relaxed);
        lb.store(byte, Ordering::Relaxed);
    });

    // Node 0 writes a byte to TX_DATA
    let write_req = ProtocolMessage {
        _ActionId: CoSimAction::WriteBusByte as i32,
        _Addr: 0x50000000 | (REG_TX_DATA as u64),
        _Value: b'Z' as u64,
        _PeripheralIndex: 0,
    };
    let write_resp = hub.handle_request(&node0, &write_req);
    segue_assert_eq!(ctx, write_resp.action(), CoSimAction::Ok);

    // Callback fired
    segue_assert_eq!(ctx, last_src.load(Ordering::Relaxed), 0);
    segue_assert_eq!(ctx, last_dst.load(Ordering::Relaxed), 1);
    segue_assert_eq!(ctx, last_byte.load(Ordering::Relaxed), b'Z');

    // Node 1 received the byte in RX queue
    segue_assert_eq!(ctx, node1.rx_count(), 1);

    // Node 1 reads RX_DATA
    let read_req = ProtocolMessage {
        _ActionId: CoSimAction::ReadBusByte as i32,
        _Addr: 0x50000000 | (REG_RX_DATA as u64),
        _Value: 0,
        _PeripheralIndex: 0,
    };
    let read_resp = hub.handle_request(&node1, &read_req);
    segue_assert_eq!(ctx, read_resp.value(), b'Z' as u64);

    let s0 = hub.get_node_stats(0);
    let s1 = hub.get_node_stats(1);

    segue_assert_eq!(ctx, s0._BytesSent, 1);
    segue_assert_eq!(ctx, s0._WritesServiced, 1);
    segue_assert_eq!(ctx, s1._BytesReceived, 1);
    segue_assert_eq!(ctx, s1._ReadsServiced, 1);
});

//-------------------------------------------------------------------------------------------------

segue_test!(Crew, VirtualExchangeProtocol, |ctx| {
    let hub = CrewHub::new();
    hub.add_node(0);
    hub.add_node(1);

    hub.add_link(crate::crew::config::CrewLinkConfig {
        source_node_id: 0,
        destination_node_ids: vec![1],
    });
    hub.add_link(crate::crew::config::CrewLinkConfig {
        source_node_id: 1,
        destination_node_ids: vec![0],
    });

    let node0 = hub.find_node(0).unwrap();
    let node1 = hub.find_node(1).unwrap();
    node0.set_online(true);
    node1.set_online(true);

    let msg_vm0 = "Hello World from Zephyr VM0!\n";
    let msg_vm1 = "Hello World back from Zephyr VM1!\n";

    // 1. VM0 sends msg_vm0
    for &b in msg_vm0.as_bytes() {
        let req = ProtocolMessage {
            _ActionId: CoSimAction::WriteBusByte as i32,
            _Addr: 0x50000000 | (REG_TX_DATA as u64),
            _Value: b as u64,
            _PeripheralIndex: 0,
        };
        hub.handle_request(&node0, &req);
    }

    // 2. VM1 reads message
    let mut received_by_vm1 = Vec::new();
    loop {
        let status_req = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusDword as i32,
            _Addr: 0x50000000 | (REG_STATUS as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let status_resp = hub.handle_request(&node1, &status_req);
        if (status_resp.value() as u32 & STATUS_RX_READY) == 0 {
            break;
        }

        let rx_req = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusByte as i32,
            _Addr: 0x50000000 | (REG_RX_DATA as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let rx_resp = hub.handle_request(&node1, &rx_req);
        received_by_vm1.push((rx_resp.value() & 0xFF) as u8);
    }

    segue_assert_eq!(ctx, received_by_vm1.as_slice(), msg_vm0.as_bytes());

    // 3. VM1 replies msg_vm1
    for &b in msg_vm1.as_bytes() {
        let req = ProtocolMessage {
            _ActionId: CoSimAction::WriteBusByte as i32,
            _Addr: 0x50000000 | (REG_TX_DATA as u64),
            _Value: b as u64,
            _PeripheralIndex: 0,
        };
        hub.handle_request(&node1, &req);
    }

    // 4. VM0 reads reply
    let mut received_by_vm0 = Vec::new();
    loop {
        let status_req = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusDword as i32,
            _Addr: 0x50000000 | (REG_STATUS as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let status_resp = hub.handle_request(&node0, &status_req);
        if (status_resp.value() as u32 & STATUS_RX_READY) == 0 {
            break;
        }

        let rx_req = ProtocolMessage {
            _ActionId: CoSimAction::ReadBusByte as i32,
            _Addr: 0x50000000 | (REG_RX_DATA as u64),
            _Value: 0,
            _PeripheralIndex: 0,
        };
        let rx_resp = hub.handle_request(&node0, &rx_req);
        received_by_vm0.push((rx_resp.value() & 0xFF) as u8);
    }

    segue_assert_eq!(ctx, received_by_vm0.as_slice(), msg_vm1.as_bytes());

    let s0 = hub.get_node_stats(0);
    let s1 = hub.get_node_stats(1);

    segue_assert_eq!(ctx, s0._BytesSent, msg_vm0.len() as u32);
    segue_assert_eq!(ctx, s0._BytesReceived, msg_vm1.len() as u32);
    segue_assert_eq!(ctx, s1._BytesSent, msg_vm1.len() as u32);
    segue_assert_eq!(ctx, s1._BytesReceived, msg_vm0.len() as u32);
});

//-------------------------------------------------------------------------------------------------
// Console test

segue_console_test!(Crew, Console, |ctx| {
    segue_println!(
        ctx,
        "         [Crew Console Test: In-Memory Co-Simulation Hub Active]"
    );
});

//-------------------------------------------------------------------------------------------------
// Example test

segue_example_test!(Crew, Example, |ctx| {
    let hub = CrewHub::new();
    hub.add_node(0);
    segue_assert_eq!(ctx, hub.node_count(), 1);
});
