// lib.rs ----------------------------------------------------------------------------------------------------------
#![allow( non_snake_case)]
#![allow( clippy::neg_cmp_op_on_partial_ord)]
// Re-export inventory for macro hygiene
#[doc( hidden)]
pub use	inventory;
pub mod cove;
pub mod crew;
pub mod fascia;
pub mod fenst;
pub mod flux;
pub mod fresco;
pub mod heist;
pub mod karst;
pub mod rube;
pub mod shard;
pub mod silo;
pub mod stalks;
pub mod swarm;
pub mod symph;
pub mod zephyr;
// Re-export core macros and types
pub use	cove::context::{ TestCase, TestContext, TestKind };
pub use	cove::runner::{ RunOptions, run_all };
