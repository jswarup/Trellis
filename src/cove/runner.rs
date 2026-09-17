// runner.rs ------------------------------------------------------------------------------------------------------
use crate::cove::context::{TestCase, TestContext, TestKind};

//-------------------------------------------------------------------------------------------------

// RunOptions — execution parameters for Cove test runner.
#[derive( Debug, Clone)]
pub struct RunOptions
{
    pub filter: Option< String>,
    pub verbosity: i32,
    pub asserts_enabled: bool,
    pub console_output: bool,
    pub run_all_tests: bool,
    pub run_console: bool,
    pub run_examples: bool,
}
impl Default for RunOptions
{
    fn  default() -> Self
    {
        Self {
            filter: None,
            verbosity: 1,
            asserts_enabled: false,
            console_output: false,
            run_all_tests: true,
            run_console: false,
            run_examples: false,
        }
    }
}
impl RunOptions
{
    pub fn  from_args( args: &[String]) -> Self
    {
        let  mut opts = Self::default();
        let  mut i = 1;
        let  mut has_test_flag = false;
        while i < args.len()
        {
            let  arg = &args[i];
            if arg == "-test" || arg == "--test" || arg == "-t"
            {
                has_test_flag = true;
                if i + 1 < args.len() && !args[i + 1].starts_with( '-')
                {
                    opts.filter = Some( args[i + 1].clone());
                    i += 1;
                }
            } else if let  Some( stripped) = arg.strip_prefix( "--test=")
            {
                has_test_flag = true;
                opts.filter = Some( stripped.to_string());
            } else if arg == "-c"
            {
                opts.run_console = true;
                opts.console_output = true;
                if i + 1 < args.len() && !args[i + 1].starts_with( '-')
                {
                    opts.filter = Some( args[i + 1].clone());
                    i += 1;
                }
            } else if arg == "-e"
            {
                opts.run_examples = true;
                opts.console_output = true;
                if i + 1 < args.len() && !args[i + 1].starts_with( '-')
                {
                    opts.filter = Some( args[i + 1].clone());
                    i += 1;
                }
            } else if arg == "-v"
            {
                if i + 1 < args.len() && !args[i + 1].starts_with( '-')
                {
                    opts.verbosity = args[i + 1].parse::<i32>().unwrap_or( 1);
                    i += 1;
                } else
                {
                    opts.verbosity = 1;
                }
            } else if let  Some( stripped) = arg.strip_prefix( "-v=")
            {
                opts.verbosity = stripped.parse::<i32>().unwrap_or( 1);
            } else if !arg.starts_with( '-') && !arg.starts_with( "--")
            {
                // Heuristic: If it doesn't look like a cargo libtest flag, assume it's a positional filter argument.
                // But libtest intercepts some flags, so we just take the first non-flag as filter if not set.
                if opts.filter.is_none()
                {
                    opts.filter = Some( arg.clone());
                }
            }
            i += 1;
        }
        opts.asserts_enabled = has_test_flag || opts.asserts_enabled;
        opts.run_all_tests = has_test_flag || ( !opts.run_console && !opts.run_examples);
        opts
    }
}

//-------------------------------------------------------------------------------------------------

// CaseInsensitiveContains — substring search matching Trellis cove runner.
fn  case_insensitive_contains( haystack: &str, needle: &str) -> bool
{
    if needle.is_empty()
    {
        return true;
    }
    let  haystack_lower = haystack.to_lowercase();
    let  needle_lower = needle.to_lowercase();
    haystack_lower.contains( &needle_lower)
}

//-------------------------------------------------------------------------------------------------

// run_all — executes all matching registered tests. Returns 0 on success, 1 on failure.
pub fn  run_all( opts: RunOptions) -> i32
{
    let  mut total_tests = 0;
    let  mut passed_tests = 0;
    let  mut failed_tests = 0;
    let  mut skipped_tests = 0;
    let  filter_str = opts.filter.as_deref().unwrap_or( "");
    for tc in inventory::iter::<TestCase>
    {
        let  full_name = format!( "{}::{}", tc.suite, tc.name);
        // Filter by test kind
        let  kind_matches = if opts.run_examples {
            tc.kind == TestKind::Example
        } else if opts.run_console
        {
            tc.kind == TestKind::Console || tc.kind == TestKind::Example
        } else
        {
            true
        };
        // Filter by name pattern
        let  name_matches =
            filter_str.is_empty() || case_insensitive_contains( &full_name, filter_str);
        if !kind_matches || !name_matches
        {
            skipped_tests += 1;
            continue;
        }
        total_tests += 1;
        let  mut ctx = TestContext::new(
            tc.suite,
            tc.name,
            tc.kind,
            opts.verbosity,
            opts.asserts_enabled,
            opts.console_output,
        );
        if opts.verbosity >= 1
        {
            println!( "[ RUN  ] {}", full_name);
        }
        ( tc.func)( &mut ctx);
        let  is_pass = ( !opts.asserts_enabled) || ( ctx.fail_count == 0);
        if is_pass
        {
            passed_tests += 1;
            if opts.verbosity >= 1
            {
                print!( "[ PASS ] {}", full_name);
                if !opts.asserts_enabled
                {
                    print!( " (assertions disabled)");
                }
                println!();
            }
        } else
        {
            failed_tests += 1;
            if opts.verbosity >= 1
            {
                println!( "[ FAIL ] {}", full_name);
            }
        }
    }
    print!(
        "{} tests run, {} passed, {} failed.",
        total_tests, passed_tests, failed_tests
    );
    if skipped_tests > 0
    {
        print!( " ({} skipped)", skipped_tests);
    }
    if !opts.asserts_enabled
    {
        print!( " (assertions disabled)");
    }
    println!();
    if failed_tests == 0 { 0 } else { 1 }
}
