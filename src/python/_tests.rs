// _tests.rs -----------------------------------------------------------------------------------------

use crate::{ jeeves_assert, jeeves_assert_eq, jeeves_test };
use pyo3::prelude::*;

//-------------------------------------------------------------------------------------------------

fn with_python<F, R>( f: F) -> R
where
    F: FnOnce( Python<'_>) -> R,
{
    crate::python::ensure_initialized();
    Python::with_gil( f)
}

jeeves_test!( Python, InProcessRuntimeProbe, |ctx| {
    let result = crate::python::probe_runtime();
    jeeves_assert!( ctx, result.is_ok());
    let version = result.unwrap();
    jeeves_assert_eq!( ctx, version, env!( "CARGO_PKG_VERSION"));
    return;
});

jeeves_test!( Python, SessionAndGeometryObj, |ctx| {
    with_python( |py| {
        let trellis = py.import( "trellis").expect( "Failed to import trellis");
        let session = trellis.getattr( "Session").unwrap().call0().unwrap();

        let obj_data = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n";
        let geom_service = session.getattr( "geometry").unwrap();
        let asset = geom_service.call_method1( "parse_obj", ( obj_data,)).unwrap();

        let vertex_count: u32 = asset.getattr( "vertex_count").unwrap().extract().unwrap();
        let face_count: u32 = asset.getattr( "face_count").unwrap().extract().unwrap();
        let is_point_cloud: bool = asset.getattr( "is_point_cloud").unwrap().extract().unwrap();

        jeeves_assert_eq!( ctx, vertex_count, 3);
        jeeves_assert_eq!( ctx, face_count, 1);
        jeeves_assert!( ctx, !is_point_cloud);

        let positions: Vec<[f32; 3]> = asset.call_method0( "vertex_positions").unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, positions.len(), 3);

        let faces: Vec<[u32; 3]> = asset.call_method0( "faces").unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, faces.len(), 1);
        jeeves_assert_eq!( ctx, faces[0], [0, 1, 2]);

        let repr: String = asset.call_method0( "__repr__").unwrap().extract().unwrap();
        jeeves_assert!( ctx, repr.contains( "vertices=3"));
        jeeves_assert!( ctx, repr.contains( "faces=1"));

        // Direct session shortcut:
        let asset_shortcut = session.call_method1( "parse_obj", ( obj_data,)).unwrap();
        let vcount: u32 = asset_shortcut.getattr( "vertex_count").unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, vcount, 3);
    });
});

jeeves_test!( Python, SessionAndGeometryPts, |ctx| {
    with_python( |py| {
        let trellis = py.import( "trellis").expect( "Failed to import trellis");
        let session = trellis.getattr( "Session").unwrap().call0().unwrap();

        let pts_data = "2\n1.0 2.0 3.0 200 255 0 0\n4.0 5.0 6.0 100 0 255 0\n";
        let geom_service = session.getattr( "geometry").unwrap();
        let asset = geom_service.call_method1( "parse_pts", ( pts_data,)).unwrap();

        let point_count: u32 = asset.getattr( "point_count").unwrap().extract().unwrap();
        let is_point_cloud: bool = asset.getattr( "is_point_cloud").unwrap().extract().unwrap();

        jeeves_assert_eq!( ctx, point_count, 2);
        jeeves_assert!( ctx, is_point_cloud);

        let colors: Vec<[f32; 4]> = asset.call_method0( "vertex_colors").unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, colors.len(), 2);
    });
});

jeeves_test!( Python, GeometryErrorHandling, |ctx| {
    with_python( |py| {
        let trellis = py.import( "trellis").expect( "Failed to import trellis");
        let session = trellis.getattr( "Session").unwrap().call0().unwrap();
        let geom_service = session.getattr( "geometry").unwrap();

        // 1. Non-existent file raises FileNotFoundError:
        let err = geom_service.call_method1( "load", ( "non_existent_model_12345.obj",)).unwrap_err();
        jeeves_assert!( ctx, err.is_instance_of::<pyo3::exceptions::PyFileNotFoundError>( py));

        // 2. Invalid syntax in parse_obj raises ValueError:
        let err2 = geom_service.call_method1( "parse_obj", ( "not a valid obj",)).unwrap_err();
        jeeves_assert!( ctx, err2.is_instance_of::<pyo3::exceptions::PyValueError>( py));
    });
});

jeeves_test!( Python, LifetimeAndRepeatedSessions, |ctx| {
    with_python( |py| {
        let trellis = py.import( "trellis").expect( "Failed to import trellis");
        let obj_data = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nf 1 2 3\n";

        // Create asset inside an inner scope, drop session, verify asset persists:
        let asset = {
            let session = trellis.getattr( "Session").unwrap().call0().unwrap();
            let asset = session.call_method1( "parse_obj", ( obj_data,)).unwrap();
            asset
        };
        let vertex_count: u32 = asset.getattr( "vertex_count").unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, vertex_count, 3);
    });
    return;
});

jeeves_test!( Python, ConsoleEngineExecution, |ctx| {
    crate::python::ensure_initialized();
    let engine = crate::python::console::PythonConsoleEngine::new().expect( "Failed to create console engine");

    let res1 = engine.execute_line( "x = 40 + 2");
    jeeves_assert!( ctx, !res1.is_error);
    jeeves_assert!( ctx, !res1.needs_more_input);

    let res2 = engine.execute_line( "print(x)");
    jeeves_assert!( ctx, !res2.is_error);
    jeeves_assert!( ctx, res2.output.contains( "42"));

    // Verify persistent namespace:
    let res3 = engine.execute_line( "s = trellis.Session()");
    jeeves_assert!( ctx, !res3.is_error);

    let res4 = engine.execute_line( "print(s.version())");
    jeeves_assert!( ctx, !res4.is_error);
    jeeves_assert!( ctx, res4.output.contains( env!( "CARGO_PKG_VERSION")));
    return;
});

jeeves_test!( Python, ConsoleEngineMultiline, |ctx| {
    crate::python::ensure_initialized();
    let engine = crate::python::console::PythonConsoleEngine::new().expect( "Failed to create console engine");

    let res1 = engine.execute_line( "def multiply(a, b):");
    jeeves_assert!( ctx, res1.needs_more_input);

    let res2 = engine.execute_line( "    return a * b");
    jeeves_assert!( ctx, res2.needs_more_input);

    let res3 = engine.execute_line( "");
    jeeves_assert!( ctx, !res3.needs_more_input);

    let res4 = engine.execute_line( "print(multiply(6, 7))");
    jeeves_assert!( ctx, !res4.is_error);
    jeeves_assert!( ctx, res4.output.contains( "42"));
    return;
});

jeeves_test!( Python, ConsoleEngineErrorAndInterrupt, |ctx| {
    crate::python::ensure_initialized();
    let engine = crate::python::console::PythonConsoleEngine::new().expect( "Failed to create console engine");

    let res1 = engine.execute_line( "1 / 0");
    jeeves_assert!( ctx, res1.is_error);
    jeeves_assert!( ctx, res1.output.contains( "ZeroDivisionError"));

    let res2 = engine.execute_line( "def syntax error :");
    jeeves_assert!( ctx, res2.is_error);
    jeeves_assert!( ctx, res2.output.contains( "SyntaxError"));

    // Interrupt while idle should be safe:
    engine.interrupt();
    jeeves_assert!( ctx, engine.cancelled.load( std::sync::atomic::Ordering::SeqCst));
    return;
});

