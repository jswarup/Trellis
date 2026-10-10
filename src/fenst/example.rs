// example.rs -----------------------------------------------------------------------------------------------------

use crate::cove::{TestContext, TestKind};
use crate::fenst::Xplr;
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

impl XplrExample
{
    pub fn Registered() -> Buff<&'static Self>
    {
        let mut examples = Stash::New();
        inventory::iter::<Self>.into_iter()
                               .for_each(|example| examples.Push(example));
        examples.MutArr()
                .QSort(|a, b| (a.suite, a.name) < (b.suite, b.name));
        return examples.IntoBuff();
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

//-------------------------------------------------------------------------------------------------
