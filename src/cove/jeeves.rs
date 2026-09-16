// macros.rs ------------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------
// segue_assert — boolean assertion.
// When assertions are enabled (-test), the condition is evaluated and recorded.
// When assertions are disabled (-c/-e without -test), the assertion is bypassed.

#[macro_export]
macro_rules! segue_assert {
    ($ctx:expr, $cond:expr $(, $msg:expr)?) => {{
        $ctx.assert_count += 1;
        if $ctx.asserts_enabled {
            if !($cond) {
                $ctx.fail_count += 1;
                if $ctx.verbosity >= 1 {
                    eprintln!(
                        "         ASSERT( {} ) FAILED ({}:{})",
                        stringify!($cond),
                        file!(),
                        line!()
                    );
                }
            } else {
                $ctx.pass_count += 1;
                if $ctx.verbosity >= 2 {
                    println!("         ASSERT( {} ) ... ok", stringify!($cond));
                }
            }
        }
    }};
}

//-------------------------------------------------------------------------------------------------
// segue_assert_eq — equality assertion.

#[macro_export]
macro_rules! segue_assert_eq {
    ($ctx:expr, $a:expr, $b:expr $(, $msg:expr)?) => {{
        $ctx.assert_count += 1;
        if $ctx.asserts_enabled {
            let val_a = &$a;
            let val_b = &$b;
            if !(val_a == val_b) {
                $ctx.fail_count += 1;
                if $ctx.verbosity >= 1 {
                    eprintln!(
                        "         ASSERT_EQ( {}, {} ) FAILED: `{:?}` vs `{:?}` ({}:{})",
                        stringify!($a),
                        stringify!($b),
                        val_a,
                        val_b,
                        file!(),
                        line!()
                    );
                }
            } else {
                $ctx.pass_count += 1;
                if $ctx.verbosity >= 2 {
                    println!(
                        "         ASSERT_EQ( {}, {} ) ... ok",
                        stringify!($a),
                        stringify!($b)
                    );
                }
            }
        }
    }};
}

//-------------------------------------------------------------------------------------------------
// segue_assert_ne — inequality assertion.

#[macro_export]
macro_rules! segue_assert_ne {
    ($ctx:expr, $a:expr, $b:expr $(, $msg:expr)?) => {{
        $ctx.assert_count += 1;
        if $ctx.asserts_enabled {
            let val_a = &$a;
            let val_b = &$b;
            if !(val_a != val_b) {
                $ctx.fail_count += 1;
                if $ctx.verbosity >= 1 {
                    eprintln!(
                        "         ASSERT_NE( {}, {} ) FAILED: both equal `{:?}` ({}:{})",
                        stringify!($a),
                        stringify!($b),
                        val_a,
                        file!(),
                        line!()
                    );
                }
            } else {
                $ctx.pass_count += 1;
                if $ctx.verbosity >= 2 {
                    println!(
                        "         ASSERT_NE( {}, {} ) ... ok",
                        stringify!($a),
                        stringify!($b)
                    );
                }
            }
        }
    }};
}

//-------------------------------------------------------------------------------------------------
// segue_println — prints output when console output is active or verbosity is elevated.

#[macro_export]
macro_rules! segue_println {
    ($ctx:expr, $($arg:tt)*) => {{
        if $ctx.console_output || $ctx.verbosity >= 1 {
            println!($($arg)*);
        }
    }};
}

//-------------------------------------------------------------------------------------------------
// segue_test — declares and auto-registers a standard test case.

#[macro_export]
macro_rules! segue_test {
    ($suite:ident, $name:ident, |$ctx:ident| $body:block) => {
        #[allow(non_snake_case)]
        fn $name($ctx: &mut $crate::cove::context::TestContext) {
            $body
        }

        $crate::inventory::submit! {
            $crate::cove::context::TestCase {
                suite: stringify!($suite),
                name: stringify!($name),
                kind: $crate::cove::context::TestKind::Test,
                func: $name,
            }
        }
    };
}

//-------------------------------------------------------------------------------------------------
// segue_console_test — declares and auto-registers a console-marked test.

#[macro_export]
macro_rules! segue_console_test {
    ($suite:ident, $name:ident, |$ctx:ident| $body:block) => {
        #[allow(non_snake_case)]
        fn $name($ctx: &mut $crate::cove::context::TestContext) {
            $body
        }

        $crate::inventory::submit! {
            $crate::cove::context::TestCase {
                suite: stringify!($suite),
                name: stringify!($name),
                kind: $crate::cove::context::TestKind::Console,
                func: $name,
            }
        }
    };
}

//-------------------------------------------------------------------------------------------------
// segue_example_test — declares and auto-registers an example test.

#[macro_export]
macro_rules! segue_example_test {
    ($suite:ident, $name:ident, |$ctx:ident| $body:block) => {
        #[allow(non_snake_case)]
        fn $name($ctx: &mut $crate::cove::context::TestContext) {
            $body
        }

        $crate::inventory::submit! {
            $crate::cove::context::TestCase {
                suite: stringify!($suite),
                name: stringify!($name),
                kind: $crate::cove::context::TestKind::Example,
                func: $name,
            }
        }
    };
}

// Cove aliases matching Trellis
pub use segue_assert as cove_assert;
pub use segue_assert_eq as cove_assert_eq;
pub use segue_assert_ne as cove_assert_ne;
pub use segue_console_test as cove_console_test;
pub use segue_example_test as cove_example_test;
pub use segue_println as cove_println;
pub use segue_test as cove_test;
