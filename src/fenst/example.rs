// example.rs -----------------------------------------------------------------------------------------------------

use crate::cove::{TestContext, TestKind};
use crate::fenst::{BranchXplr, Xplr, XplrProvider};
use crate::silo::{Buff, Stash};

//-------------------------------------------------------------------------------------------------

pub type XplrExampleResult = Result<Box<dyn Xplr + Send>, String>;

/// An example registered with both Cove and the interactive explorer.
pub struct XplrExample
{
    pub suite: &'static str,
    pub name:  &'static str,
    pub func:  fn(&mut TestContext) -> XplrExampleResult,
}

inventory::collect!(XplrExample);

static REGISTERED_EXAMPLES: std::sync::OnceLock<Buff<&'static XplrExample>> = std::sync::OnceLock::new();

impl XplrExample
{
    pub fn Registered() -> Buff<&'static Self>
    {
        REGISTERED_EXAMPLES.get_or_init(|| {
            let mut examples = Stash::New();
            inventory::iter::<Self>.into_iter()
                                   .for_each(|example| examples.Push(example));
            examples.MutArr()
                    .QSort(|a, b| (a.suite, a.name) < (b.suite, b.name));
            examples.IntoBuff()
        }).clone()
    }

    /// Assertions stay enabled in the UI. Failed runs never publish their root.
    pub fn Run(&self) -> XplrExampleResult
    {
        let mut ctx = TestContext::new(self.suite, self.name, TestKind::Example, 1, true, false);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                                  (self.func)(&mut ctx)
                                              })).map_err(|panic| {
                                                     let message =
                                                         panic.downcast_ref::<String>()
                                                              .map(String::as_str)
                                                              .or_else(|| {
                                                                  panic.downcast_ref::<&str>()
                                                                       .copied()
                                                              })
                                                              .unwrap_or("Unknown panic");
                                                     return format!("Example panicked: {message}");
                                                 })?;
        if ctx.fail_count != 0 {
            return Err(format!("{} assertion(s) failed", ctx.fail_count));
        }
        return result;
    }
}

impl Xplr for XplrExample
{
    fn Name(&self) -> &str { self.name }
    fn Path(&self) -> &str { self.name }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
}

impl BranchXplr for XplrExample
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let res = self.Run()?;
        let mut stash = Stash::New();
        stash.Push(res as Box<dyn Xplr>);
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(1)
    }
}

//-------------------------------------------------------------------------------------------------
/// An Xplr node representing an individual test example under `example://<suite>/<name>`.
pub struct ExampleItemXplr
{
    _Suite:   String,
    _Name:    String,
    _Path:    String,
    _Example: &'static XplrExample,
}

impl ExampleItemXplr
{
    pub fn New(example: &'static XplrExample) -> Self
    {
        Self {
            _Suite:   example.suite.to_string(),
            _Name:    example.name.to_string(),
            _Path:    format!("example://{}/{}", example.suite, example.name),
            _Example: example,
        }
    }
}

impl Xplr for ExampleItemXplr
{
    fn Name(&self) -> &str { &self._Name }
    fn Path(&self) -> &str { &self._Path }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
}

impl BranchXplr for ExampleItemXplr
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let res = self._Example.Run()?;
        let mut stash = Stash::New();
        stash.Push(res as Box<dyn Xplr>);
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(1)
    }
}

//-------------------------------------------------------------------------------------------------
/// An Xplr branch representing an entire test suite under `example://<suite>`.
pub struct ExampleSuiteXplr
{
    _Name:     String,
    _Path:     String,
    _Examples: Buff<&'static XplrExample>,
}

impl ExampleSuiteXplr
{
    pub fn New(suite: &str) -> Self
    {
        let all = XplrExample::Registered();
        let mut matching = Stash::New();
        all.Traverse(|ex| {
            if ex.suite == suite {
                matching.Push(*ex);
            }
        });
        Self {
            _Name:     suite.to_string(),
            _Path:     format!("example://{}", suite),
            _Examples: matching.IntoBuff(),
        }
    }
}

impl Xplr for ExampleSuiteXplr
{
    fn Name(&self) -> &str { &self._Name }
    fn Path(&self) -> &str { &self._Path }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
}

impl BranchXplr for ExampleSuiteXplr
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut stash = Stash::New();
        self._Examples.Traverse(|ex| {
            stash.Push(Box::new(ExampleItemXplr::New(ex)) as Box<dyn Xplr>);
        });
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(self._Examples.Len())
    }
}

//-------------------------------------------------------------------------------------------------
/// The root Xplr branch representing all registered examples under `example://`.
pub struct ExamplesRootXplr
{
    _Path: String,
}

impl ExamplesRootXplr
{
    pub fn New() -> Self
    {
        Self { _Path: "example://".to_string() }
    }

    pub fn Suites() -> Buff<String>
    {
        let all = XplrExample::Registered();
        let mut suites = Stash::New();
        let mut last = "";
        all.Traverse(|ex| {
            if ex.suite != last {
                last = ex.suite;
                suites.Push(ex.suite.to_string());
            }
        });
        suites.IntoBuff()
    }
}

impl Default for ExamplesRootXplr
{
    fn default() -> Self { Self::New() }
}

impl Xplr for ExamplesRootXplr
{
    fn Name(&self) -> &str { "Examples" }
    fn Path(&self) -> &str { &self._Path }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
}

impl BranchXplr for ExamplesRootXplr
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let suites = Self::Suites();
        let mut stash = Stash::New();
        suites.Traverse(|s| {
            stash.Push(Box::new(ExampleSuiteXplr::New(s)) as Box<dyn Xplr>);
        });
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(Self::Suites().Len())
    }
}

//-------------------------------------------------------------------------------------------------
/// Example provider resolving `example://` URIs.
pub struct ExampleProvider;

impl ExampleProvider
{
    pub fn New() -> Self { Self }
}

impl Default for ExampleProvider
{
    fn default() -> Self { Self::New() }
}

impl XplrProvider for ExampleProvider
{
    fn Scheme(&self) -> &str { "example" }

    fn OpenRoot(&self, uri: &str) -> Result<Box<dyn BranchXplr>, String>
    {
        let stripped = uri.strip_prefix("example://").unwrap_or(uri);
        let trimmed = stripped.trim_matches('/');
        if trimmed.is_empty() {
            return Ok(Box::new(ExamplesRootXplr::New()));
        }
        if let Some((suite, name)) = trimmed.split_once('/') {
            let all = XplrExample::Registered();
            let mut found = None;
            all.Traverse(|ex| {
                if ex.suite == suite && ex.name == name {
                    found = Some(*ex);
                }
            });
            if let Some(ex) = found {
                return Ok(Box::new(ExampleItemXplr::New(ex)));
            }
            return Err(format!("Example not found: {}/{}", suite, name));
        }
        Ok(Box::new(ExampleSuiteXplr::New(trimmed)))
    }
}
