// context.rs -----------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
// TestKind — categorizes the test case execution target.
#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKind {
    Test,
    Console,
    Example,
}

//-------------------------------------------------------------------------------------------------
// TestContext — per-run state passed into every test function.
pub struct TestContext
{
    pub kind: TestKind,
    pub verbosity: i32,
    pub asserts_enabled: bool,
    pub console_output: bool,
    pub assert_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub current_suite: &'static str,
    pub current_name: &'static str,
}
impl TestContext
{
    pub fn	new( 
        suite: &'static str,
        name: &'static str,
        kind: TestKind,
        verbosity: i32,
        asserts_enabled: bool,
        console_output: bool,
    ) -> Self
    {
        Self {
            kind,
            verbosity,
            asserts_enabled,
            console_output,
            assert_count: 0,
            pass_count: 0,
            fail_count: 0,
            current_suite: suite,
            current_name: name,
        }
    }
}

//-------------------------------------------------------------------------------------------------
// TestCase — statically registered test descriptor collected via inventory.
pub struct TestCase
{
    pub suite: &'static str,
    pub name: &'static str,
    pub kind: TestKind,
    pub func: fn( &mut TestContext),
}
inventory::collect!( TestCase);
