// jeeves.rs ------------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------

// jeeves_assert — boolean assertion.
// When assertions are enabled (-test), the condition is evaluated and recorded.
// When assertions are disabled (-c/-e without -test), the assertion is bypassed.
#[macro_export]
macro_rules! jeeves_assert {
    ( $ctx:expr, $cond:expr $( , $msg:expr)?) => {{
        $ctx.assert_count += 1;
        if $ctx.asserts_enabled
        {
            if !($cond)
            {
                $ctx.fail_count += 1;
                if $ctx.verbosity >= 1
                {
                    eprintln!(
                        "         ASSERT( {} ) FAILED ({}:{})",
                        stringify!($cond),
                        file!(),
                        line!()
                    );
                }
            } else
            {
                $ctx.pass_count += 1;
                if $ctx.verbosity >= 2
                {
                    println!( "         ASSERT( {} ) ... ok", stringify!($cond));
                }
            }
        }
    }};
}

//-------------------------------------------------------------------------------------------------

// jeeves_assert_eq — equality assertion.
#[macro_export]
macro_rules! jeeves_assert_eq {
    ( $ctx:expr, $a:expr, $b:expr $( , $msg:expr)?) => {{
        $ctx.assert_count += 1;
        if $ctx.asserts_enabled
        {
            let  val_a = &$a;
            let  val_b = &$b;
            if !( val_a == val_b)
            {
                $ctx.fail_count += 1;
                if $ctx.verbosity >= 1
                {
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
            } else
            {
                $ctx.pass_count += 1;
                if $ctx.verbosity >= 2
                {
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
    ( $ctx:expr, $a:expr, $b:expr $( , $msg:expr)?) => {{
        $ctx.assert_count += 1;
        if $ctx.asserts_enabled
        {
            let  val_a = &$a;
            let  val_b = &$b;
            if !( val_a != val_b)
            {
                $ctx.fail_count += 1;
                if $ctx.verbosity >= 1
                {
                    eprintln!(
                        "         ASSERT_NE( {}, {} ) FAILED: both equal `{:?}` ({}:{})",
                        stringify!($a),
                        stringify!($b),
                        val_a,
                        file!(),
                        line!()
                    );
                }
            } else
            {
                $ctx.pass_count += 1;
                if $ctx.verbosity >= 2
                {
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
    ( $ctx:expr, $( $arg:tt)*) => {{
        if $ctx.console_output || $ctx.verbosity >= 1
        {
            println!( $( $arg)*);
        }
    }};
}

//-------------------------------------------------------------------------------------------------

// jeeves_test — declares and auto-registers a test case (Standard, Console, or Example).
#[macro_export]
macro_rules! jeeves_test {
    // Console test variant
    ( $suite:ident, $name:ident, Console, |$ctx:ident| $body:block) => {
        $crate::jeeves_test!( @impl $suite, $name, Console, |$ctx| $body);
    };
    // Example test variant
    ( $suite:ident, $name:ident, Example, |$ctx:ident| $body:block) => {
        $crate::jeeves_test!( @impl $suite, $name, Example, |$ctx| $body);
    };
    // Standard test variant (default)
    ( $suite:ident, $name:ident, |$ctx:ident| $body:block) => {
        $crate::jeeves_test!( @impl $suite, $name, Test, |$ctx| $body);
    };
    // Internal implementation
    ( @impl $suite:ident, $name:ident, $kind:ident, |$ctx:ident| $body:block) =>
    {
        #[allow( non_snake_case)]
        mod $name {
            use super::*;
            pub fn  inner( $ctx: &mut $crate::cove::context::TestContext)
            {
                $body
            }
        }
        #[test]
        #[allow( non_snake_case)]
        fn  $name()
        {
            let  mut ctx = $crate::cove::context::TestContext::new(
                stringify!( $suite),
                stringify!( $name),
                1,
                true,
                false,
            );
            $name::inner( &mut ctx);
            assert_eq!( ctx.fail_count, 0, "Test failed with {} assertion errors", ctx.fail_count);
        }
        $crate::inventory::submit! {
            $crate::cove::context::TestCase {
                suite: stringify!( $suite),
                name: stringify!( $name),
                kind: $crate::cove::context::TestKind::$kind,
                func: $name::inner,
            }
        }
    };
}
// Module re-exports
pub use {jeeves_assert_eq, jeeves_assert_ne, jeeves_println, jeeves_test};
