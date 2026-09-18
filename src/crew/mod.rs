// src/crew/mod.rs
//-- mod.rs ----------------------------------------------------------------------------------------

//--------------------------------------------------------------------------------------------------

#[cfg(feature = "tests")]
pub mod _tests;
pub mod config;
pub mod hub;
pub mod node;
pub mod protocol;
pub mod vm_adaptor;
pub mod vm_runner;

pub use config::CrewLinkConfig;
pub use hub::{CrewHub, MessageCallback};
pub use node::{CrewNode, NodeStats, RxPushOutcome};
pub use protocol::{
    CoSimAction, ProtocolMessage, REG_NODE_ID, REG_RX_COUNT, REG_RX_DATA, REG_STATUS, REG_TX_DATA,
    STATUS_PEER_UP, STATUS_RX_READY, STATUS_TX_READY,
};
pub use vm_adaptor::VMAdaptor;
pub use vm_runner::{VMRunner, VmBus};
