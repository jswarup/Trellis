//-- fenst/mod.rs ----------------------------------------------------------------------------------------------------
//! Provider-neutral explorer data model for filesystem and virtual workspace views.
#[cfg( feature = "tests")]
pub mod _tests;
pub mod cask;
pub mod fsxplr;
pub mod provider;
pub mod xplr;
pub use	cask::*;
pub use	fsxplr::{ FsBranch, FsLeaf };
pub use	provider::{ FsProvider, XplrProvider, XplrRegistry };
pub use	xplr::{ BranchXplr, LeafXplr, StreamChunk, TraverseDepth, TraverseDepthRoots, Xplr, XplrNodeInfo };
