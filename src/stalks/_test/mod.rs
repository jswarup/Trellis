// mod.rs ---------------------------------------------------------------------------------------------------------

use crate::{segue_assert, segue_example_test, segue_println, segue_test};

//-------------------------------------------------------------------------------------------------
// Stalks Seed Tests

segue_test!(Stalks, InitTest, |ctx| {
    segue_assert!(ctx, true);
});

segue_example_test!(Stalks, WorkerSeedExample, |ctx| {
    segue_println!(ctx, "         [Example] Stalks worker scaffold ready");
    segue_assert!(ctx, true);
});
