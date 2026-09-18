//-- fenst/mod.rs ----------------------------------------------------------------------------------------------------
//! Provider-neutral explorer data model for filesystem and virtual workspace views.

pub mod fsxplr;
pub mod provider;
pub mod xplr;

#[cfg(feature = "tests")]
pub mod _tests;

pub use fsxplr::{FsBranch, FsLeaf};
pub use provider::{FsProvider, XplrProvider, XplrRegistry};
pub use xplr::{BranchXplr, LeafXplr, StreamChunk, Xplr, XplrNodeInfo};
