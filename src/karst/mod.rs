// src/karst/mod.rs
#[cfg(feature = "tests")]
pub mod _test;
pub mod config;
pub mod fabric;
pub mod fabric_node;
pub mod host_node;
pub mod link;
pub mod memchan;
pub mod noc;
pub mod pipe;
pub mod vpu;
pub use config::*;
pub use fabric::{KarstEngineInfo, KarstFabric, KarstStats};
pub use fabric_node::KarstFabricNode;
pub use host_node::{HostResponse, HostStats, HostTransaction, KarstHostNode};
pub use link::{KarstFlit, KarstLink, KarstLinkChannel};
pub use memchan::{MemChan, MemChanStats};
pub use noc::KarstNoc;
pub use pipe::KarstPipe;
pub use vpu::Vpu;
// Compatibility aliases
pub type DChan = MemChan;
pub type DChanStats = MemChanStats;
pub type KarstDChan = MemChan;
pub type KarstVPU = Vpu;
pub type Epu = Vpu;
