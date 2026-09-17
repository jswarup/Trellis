// mod.rs ---------------------------------------------------------------------------------------------------------
use crate::{
    segue_assert, segue_assert_eq, segue_console_test, segue_example_test, segue_println,
    segue_test,
};

//-------------------------------------------------------------------------------------------------

// Cove Self Tests
segue_test!( Cove, TruthTest, |ctx| {
    segue_assert!( ctx, true);
});
segue_test!( Cove, EqualityTest, |ctx| {
    let  a: i32 = 42;
    let  b: i32 = 40 + 2;
    segue_assert_eq!( ctx, a, b);
});
segue_console_test!( Cove, ConsoleLogTest, |ctx| {
    segue_println!( ctx, "         [Console] Cove console log verification");
    segue_assert_eq!( ctx, 1 + 1, 2);
});
segue_example_test!( Cove, BasicUsageExample, |ctx| {
    segue_println!( ctx, "         [Example] Cove runner example execution");
    segue_assert_eq!( ctx, 10 * 2, 20);
});
