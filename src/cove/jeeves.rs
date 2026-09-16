// jeeves.rs ------------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------
// jeeves_assert — boolean assertion.
// When assertions are enabled (-test), the condition is evaluated and recorded.
// When assertions are disabled (-c/-e without -test), the assertion is bypassed.

#[macro_export]
macro_rules! jeeves_assert {
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
// jeeves_assert_eq — equality assertion.

#[macro_export]
macro_rules! jeeves_assert_eq {
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
// jeeves_assert_ne — inequality assertion.

#[macro_export]
macro_rules! jeeves_assert_ne {
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
// jeeves_println — prints output when console output is active or verbosity is elevated.

#[macro_export]
macro_rules! jeeves_println {
    ($ctx:expr, $($arg:tt)*) => {{
        if $ctx.console_output || $ctx.verbosity >= 1 {
            println!($($arg)*);
        }
    }};
}

//-------------------------------------------------------------------------------------------------
// jeeves_test — declares and auto-registers a standard test case.

#[macro_export]
macro_rules! jeeves_test {
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
// jeeves_console_test — declares and auto-registers a console-marked test.

#[macro_export]
macro_rules! jeeves_console_test {
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
// jeeves_example_test — declares and auto-registers an example test.

#[macro_export]
macro_rules! jeeves_example_test {
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

//-------------------------------------------------------------------------------------------------
// Segue backward compatibility macro aliases

#[macro_export]
macro_rules! segue_assert {
    ($($arg:tt)*) => {
        $crate::jeeves_assert!($($arg)*)
    };
}

#[macro_export]
macro_rules! segue_assert_eq {
    ($($arg:tt)*) => {
        $crate::jeeves_assert_eq!($($arg)*)
    };
}

#[macro_export]
macro_rules! segue_assert_ne {
    ($($arg:tt)*) => {
        $crate::jeeves_assert_ne!($($arg)*)
    };
}

#[macro_export]
macro_rules! segue_println {
    ($($arg:tt)*) => {
        $crate::jeeves_println!($($arg)*)
    };
}

#[macro_export]
macro_rules! segue_test {
    ($($arg:tt)*) => {
        $crate::jeeves_test! { $($arg)* }
    };
}

#[macro_export]
macro_rules! segue_console_test {
    ($($arg:tt)*) => {
        $crate::jeeves_console_test! { $($arg)* }
    };
}

#[macro_export]
macro_rules! segue_example_test {
    ($($arg:tt)*) => {
        $crate::jeeves_example_test! { $($arg)* }
    };
}

//-------------------------------------------------------------------------------------------------
// Cove aliases matching Trellis

#[macro_export]
macro_rules! cove_assert {
    ($($arg:tt)*) => {
        $crate::jeeves_assert!($($arg)*)
    };
}

#[macro_export]
macro_rules! cove_assert_eq {
    ($($arg:tt)*) => {
        $crate::jeeves_assert_eq!($($arg)*)
    };
}

#[macro_export]
macro_rules! cove_assert_ne {
    ($($arg:tt)*) => {
        $crate::jeeves_assert_ne!($($arg)*)
    };
}

#[macro_export]
macro_rules! cove_println {
    ($($arg:tt)*) => {
        $crate::jeeves_println!($($arg)*)
    };
}

#[macro_export]
macro_rules! cove_test {
    ($($arg:tt)*) => {
        $crate::jeeves_test! { $($arg)* }
    };
}

#[macro_export]
macro_rules! cove_console_test {
    ($($arg:tt)*) => {
        $crate::jeeves_console_test! { $($arg)* }
    };
}

#[macro_export]
macro_rules! cove_example_test {
    ($($arg:tt)*) => {
        $crate::jeeves_example_test! { $($arg)* }
    };
}

// Module re-exports
pub use {
    cove_assert, cove_assert_eq, cove_assert_ne, cove_console_test, cove_example_test,
    cove_println, cove_test, jeeves_assert, jeeves_assert_eq, jeeves_assert_ne,
    jeeves_console_test, jeeves_example_test, jeeves_println, jeeves_test, segue_assert,
    segue_assert_eq, segue_assert_ne, segue_console_test, segue_example_test, segue_println,
    segue_test,
};
