// main.rs ---------------------------------------------------------------------------------------------------------

#![allow(non_snake_case)]

use segue::cove::runner::{RunOptions, run_all};
use std::env;

//-------------------------------------------------------------------------------------------------

fn print_usage() {
    println!("Segue systems & algorithms framework");
    println!();
    println!("Usage:");
    println!(
        "  segue -test [filter]    Run all tests (or matching filter) with assertions enabled"
    );
    println!(
        "  segue -c [filter]       Run console tests (assertions bypassed unless -test is specified)"
    );
    println!(
        "  segue -e [filter]       Run example tests (assertions bypassed unless -test is specified)"
    );
    println!(
        "  segue <filter>          Run tests matching <filter> (assertions bypassed unless -test is specified)"
    );
    println!("  segue -v [0|1|2]        Set test output verbosity");
    println!("  segue -h, --help        Display this help message");
}

//-------------------------------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut is_test_mode = false;
    let mut has_test_flag = false;
    let mut has_console_flag = false;
    let mut has_example_flag = false;
    let mut test_filter: Option<String> = None;
    let mut verbosity: i32 = 0;

    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];

        if arg == "-test" || arg == "--test" {
            is_test_mode = true;
            has_test_flag = true;
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                test_filter = Some(args[i + 1].clone());
                i += 1;
            }
        } else if let Some(stripped) = arg.strip_prefix("--test=") {
            is_test_mode = true;
            has_test_flag = true;
            test_filter = Some(stripped.to_string());
        } else if arg == "-c" {
            is_test_mode = true;
            has_console_flag = true;
        } else if arg == "-e" {
            is_test_mode = true;
            has_example_flag = true;
        } else if arg == "-v" {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                verbosity = args[i + 1].parse::<i32>().unwrap_or(1);
                i += 1;
            } else {
                verbosity = 1;
            }
        } else if let Some(stripped) = arg.strip_prefix("-v=") {
            verbosity = stripped.parse::<i32>().unwrap_or(1);
        } else if arg == "-h" || arg == "--help" {
            print_usage();
            return;
        } else if !arg.starts_with('-') {
            // Positional filter argument
            is_test_mode = true;
            test_filter = Some(arg.clone());
        }

        i += 1;
    }

    if is_test_mode {
        let asserts_enabled = has_test_flag;
        let console_output = has_console_flag || has_example_flag;

        let opts = RunOptions {
            filter: test_filter,
            verbosity,
            asserts_enabled,
            console_output,
            run_all_tests: has_test_flag || (!has_console_flag && !has_example_flag),
            run_console: has_console_flag,
            run_examples: has_example_flag,
        };

        let exit_code = run_all(opts);
        std::process::exit(exit_code);
    }

    print_usage();
}
