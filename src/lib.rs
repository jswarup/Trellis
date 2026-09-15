// lib.rs ----------------------------------------------------------------------------------------------------------

#![allow(non_snake_case)]
#![allow(clippy::neg_cmp_op_on_partial_ord)]

// Re-export inventory for macro hygiene
#[doc(hidden)]
pub use inventory;

pub mod cove;
pub mod crew;
pub mod heist;
pub mod karst;
pub mod silo;
pub mod stalks;
pub mod swarm;
pub mod symph;
pub mod zephyr;

// Re-export core macros and types
pub use cove::context::{TestCase, TestContext, TestKind};
pub use cove::runner::{RunOptions, run_all};

//-------------------------------------------------------------------------------------------------
// Standard Cargo Test Integration
// Enables `cargo test` to execute all registered Cove tests seamlessly.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cove_test_suite() {
        let opts = RunOptions {
            verbosity: 1,
            asserts_enabled: true,
            run_all_tests: true,
            ..Default::default()
        };
        let code = run_all(opts);
        assert_eq!(code, 0, "Cove test suite failed");
    }
}
