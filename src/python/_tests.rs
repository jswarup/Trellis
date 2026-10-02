// _tests.rs -----------------------------------------------------------------------------------------

use crate::{ jeeves_assert, jeeves_assert_eq, jeeves_test };

//-------------------------------------------------------------------------------------------------

jeeves_test!( Python, InProcessRuntimeProbe, |ctx| {
    let result = crate::python::probe_runtime();
    jeeves_assert!( ctx, result.is_ok());
    let version = result.unwrap();
    jeeves_assert_eq!( ctx, version, env!( "CARGO_PKG_VERSION"));
    return;
});

