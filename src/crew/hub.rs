// src/crew/hub.rs
use crate::crew::config::CrewLinkConfig;
use crate::crew::node::{CrewNode, NodeStats};
use crate::crew::protocol::{
    CoSimAction, ProtocolMessage, REG_NODE_ID, REG_RX_COUNT, REG_RX_DATA, REG_STATUS, REG_TX_DATA,
    STATUS_PEER_UP, STATUS_RX_READY, STATUS_TX_READY,
};
use crate::stalks::work::SpinMutex;
use std::collections::HashMap;
use std::sync::Arc;

use crate::silo::buff::Buff;
use crate::silo::stash::Stash;

//-------------------------------------------------------------------------------------------------

// Callback type for monitoring byte-level routing between VM nodes.
pub type MessageCallback = Arc<dyn Fn(u32, u32, u8) + Send + Sync>;

//-------------------------------------------------------------------------------------------------

// Concrete co-simulation hub managing VM nodes and in-memory protocol dispatch.
pub struct CrewHub {
    _nodes: SpinMutex<Stash<Arc<CrewNode>>>,
    _message_cb: SpinMutex<Option<MessageCallback>>,
    _routes: SpinMutex<HashMap<u32, Buff<u32>>>,
}
impl Default for CrewHub {
    fn default() -> Self {
        Self::new()
    }
}
impl CrewHub {
    pub fn new() -> Self {
        Self {
            _nodes: SpinMutex::New(Stash::New()),
            _message_cb: SpinMutex::New(None),
            _routes: SpinMutex::New(HashMap::new()),
        }
    }
    pub fn add_link(&self, config: CrewLinkConfig) {
        let mut routes = self._routes.Lock();
        routes.insert(config.source_node_id, config.destination_node_ids);
    }
    pub fn add_node(&self, id: u32) {
        let mut nodes = self._nodes.Lock();
        nodes.Push(Arc::new(CrewNode::new(id)));
    }
    pub fn node_count(&self) -> u32 {
        let nodes = self._nodes.Lock();
        nodes.Size()
    }
    pub fn find_node(&self, id: u32) -> Option<Arc<CrewNode>> {
        let nodes = self._nodes.Lock();
        for i in 0..nodes.Size() {
            let n = &nodes[i];
            if n.id() == id {
                return Some(n.clone());
            }
        }
        None
    }
    pub fn is_node_online(&self, id: u32) -> bool {
        if let Some(node) = self.find_node(id) {
            node.is_online()
        } else {
            false
        }
    }
    pub fn get_node_stats(&self, id: u32) -> NodeStats {
        if let Some(node) = self.find_node(id) {
            node.get_stats()
        } else {
            NodeStats::default()
        }
    }
    pub fn set_message_callback<F>(&self, cb: F)
    where
        F: Fn(u32, u32, u8) + Send + Sync + 'static,
    {
        let mut cb_guard = self._message_cb.Lock();
        *cb_guard = Some(Arc::new(cb));
    }
    pub fn handle_request(&self, node: &CrewNode, req: &ProtocolMessage) -> ProtocolMessage {
        let mut resp = ProtocolMessage {
            _ActionId: CoSimAction::Ok as i32,
            _Addr: req.addr(),
            _Value: 0,
            _PeripheralIndex: req.peripheral_index(),
        };
        let reg = (req.addr() & 0xFFF) as u32;
        let action = req.action();
        match action {
            CoSimAction::ReadBus
            | CoSimAction::ReadBusByte
            | CoSimAction::ReadBusWord
            | CoSimAction::ReadBusDword
            | CoSimAction::ReadBusQword => {
                node.record_read();
                match reg {
                    REG_NODE_ID => {
                        resp._Value = node.id() as u64;
                    }
                    REG_STATUS => {
                        let mut status = STATUS_TX_READY;
                        if node.rx_count() > 0 {
                            status |= STATUS_RX_READY;
                        }
                        let peers = {
                            let routes = self._routes.Lock();
                            routes.get(&node.id()).cloned().unwrap_or_default()
                        };
                        let mut any_online = false;
                        for i in 0..peers.Len() {
                            if self.is_node_online(peers[i]) {
                                any_online = true;
                                break;
                            }
                        }
                        if any_online {
                            status |= STATUS_PEER_UP;
                        }
                        resp._Value = status as u64;
                    }
                    REG_TX_DATA => {
                        resp._Value = 0;
                    }
                    REG_RX_DATA => {
                        let mut byte = 0u8;
                        if node.pop_rx(&mut byte) {
                            resp._Value = byte as u64;
                        } else {
                            resp._Value = 0;
                        }
                    }
                    REG_RX_COUNT => {
                        resp._Value = node.rx_count() as u64;
                    }
                    _ => {
                        resp._Value = 0;
                    }
                }
            }
            CoSimAction::WriteBusByte
            | CoSimAction::WriteBusWord
            | CoSimAction::WriteBusDword
            | CoSimAction::WriteBusQword => {
                node.record_write();
                if reg == REG_TX_DATA {
                    let byte = (req.value() & 0xFF) as u8;
                    node.record_byte_sent();
                    let peers = {
                        let routes = self._routes.Lock();
                        routes.get(&node.id()).cloned().unwrap_or_default()
                    };
                    for i in 0..peers.Len() {
                        let peer_id = peers[i];
                        if let Some(peer) = self.find_node(peer_id) {
                            peer.push_rx(byte);
                            let cb_opt = {
                                let guard = self._message_cb.Lock();
                                guard.clone()
                            };
                            if let Some(cb) = cb_opt {
                                cb(node.id(), peer_id, byte);
                            }
                        }
                    }
                }
            }
            CoSimAction::ResetPeripheral => {
                node.clear_rx();
            }
            _ => {}
        }
        resp
    }
}
