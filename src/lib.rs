// lib.rs ----------------------------------------------------------------------------------------------------------
#![allow( non_snake_case)]
#![allow( clippy::neg_cmp_op_on_partial_ord)]
#![allow( clippy::needless_return)]                                    // Trellis requires explicit return statements.
// Re-export inventory for macro hygiene
#[doc( hidden)]
pub use	inventory;

mod pedestel;
mod salvus;
mod nexome;
mod parvis;

pub use pedestel::cove;
pub use pedestel::flux;
pub use pedestel::shard;
pub use pedestel::silo;
pub use pedestel::stalks;

pub use salvus::drove;
pub use salvus::flock;
pub use salvus::heist;
pub use salvus::swarm;
pub use salvus::symph;

pub use nexome::crew;
pub use nexome::fleck;
pub use nexome::fresco;
pub use nexome::karst;
pub use nexome::rube;
pub use nexome::zephyr;

pub use parvis::fascia;
pub use parvis::fenst;
#[cfg( feature = "python")]
pub use parvis::python;

// Re-export core macros and types
pub use	cove::context::{ TestCase, TestContext, TestKind };
pub use	cove::runner::{ RunOptions, run_all };
