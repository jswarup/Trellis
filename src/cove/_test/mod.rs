use crate::{jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test};
// mod.rs ---------------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------

// Cove Self Tests
jeeves_test!( Cove, TruthTest, |ctx| {
    jeeves_assert!( ctx, true);
});
jeeves_test!( Cove, EqualityTest, |ctx| {
    let  a: i32 = 42;
    let  b: i32 = 40 + 2;
    jeeves_assert_eq!( ctx, a, b);
});
jeeves_test!( Cove, ConsoleLogTest, Console, |ctx| {
    jeeves_println!( ctx, "         [Console] Cove console log verification");
    jeeves_assert_eq!( ctx, 1 + 1, 2);
});
jeeves_test!( Cove, BasicUsageExample, Example, |ctx| {
    jeeves_println!( ctx, "         [Example] Cove runner example execution");
    jeeves_assert_eq!( ctx, 10 * 2, 20);
});
