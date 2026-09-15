// runner.rs ------------------------------------------------------------------------------------------------------

use crate::cove::context::{TestCase, TestContext, TestKind};

//-------------------------------------------------------------------------------------------------
// RunOptions — execution parameters for Cove test runner.

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub filter: Option<String>,
    pub verbosity: i32,
    pub asserts_enabled: bool,
    pub console_output: bool,
    pub run_all_tests: bool,
    pub run_console: bool,
    pub run_examples: bool,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            filter: None,
            verbosity: 0,
            asserts_enabled: true,
            console_output: false,
            run_all_tests: true,
            run_console: false,
            run_examples: false,
        }
    }
}

//-------------------------------------------------------------------------------------------------
// CaseInsensitiveContains — substring search matching Trellis cove runner.

fn case_insensitive_contains(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let haystack_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();
    haystack_lower.contains(&needle_lower)
}

//-------------------------------------------------------------------------------------------------
// run_all — executes all matching registered tests. Returns 0 on success, 1 on failure.

pub fn run_all(opts: RunOptions) -> i32 {
    let mut total_tests = 0;
    let mut passed_tests = 0;
    let mut failed_tests = 0;
    let mut skipped_tests = 0;

    let filter_str = opts.filter.as_deref().unwrap_or("");

    for tc in inventory::iter::<TestCase> {
        let full_name = format!("{}::{}", tc.suite, tc.name);

        // Filter by test kind
        let kind_matches = if opts.run_examples {
            tc.kind == TestKind::Example
        } else if opts.run_console {
            tc.kind == TestKind::Console || tc.kind == TestKind::Example
        } else {
            true
        };

        // Filter by name pattern
        let name_matches =
            filter_str.is_empty() || case_insensitive_contains(&full_name, filter_str);

        if !kind_matches || !name_matches {
            skipped_tests += 1;
            continue;
        }

        total_tests += 1;

        let mut ctx = TestContext::new(
            tc.suite,
            tc.name,
            opts.verbosity,
            opts.asserts_enabled,
            opts.console_output,
        );

        if opts.verbosity >= 2 {
            println!("[ RUN  ] {}", full_name);
        }

        (tc.func)(&mut ctx);

        let is_pass = (!opts.asserts_enabled) || (ctx.fail_count == 0);

        if is_pass {
            passed_tests += 1;
            if opts.verbosity >= 1 {
                print!("[ PASS ] {}", full_name);
                if !opts.asserts_enabled {
                    print!(" (assertions disabled)");
                }
                println!();
            }
        } else {
            failed_tests += 1;
            if opts.verbosity >= 1 {
                println!("[ FAIL ] {}", full_name);
            }
        }
    }

    print!(
        "{} tests run, {} passed, {} failed.",
        total_tests, passed_tests, failed_tests
    );

    if skipped_tests > 0 {
        print!(" ({} skipped)", skipped_tests);
    }

    if !opts.asserts_enabled {
        print!(" (assertions disabled)");
    }

    println!();

    if failed_tests == 0 { 0 } else { 1 }
}
