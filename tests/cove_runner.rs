use segue::cove::runner::{RunOptions, run_all};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut opts = RunOptions::from_args(&args);

    // When executing via cargo test, defaults apply:
    if !opts.run_console && !opts.run_examples && opts.filter.is_none() {
        opts.run_all_tests = true;
    }
    opts.asserts_enabled = true; // Always enable asserts during tests

    let exit_code = run_all(opts);
    std::process::exit(exit_code);
}
