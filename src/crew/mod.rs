// src/crew/mod.rs
#[cfg(feature = "tests")]
pub mod _test;
pub mod config;
pub mod hub;
pub mod node;
pub mod protocol;
pub use config::CrewLinkConfig;
pub use hub::{CrewHub, MessageCallback};
pub use node::{CrewNode, NodeStats};
pub use protocol::{
    CoSimAction, ProtocolMessage, REG_NODE_ID, REG_RX_COUNT, REG_RX_DATA, REG_STATUS, REG_TX_DATA,
    STATUS_PEER_UP, STATUS_RX_READY, STATUS_TX_READY,
};
