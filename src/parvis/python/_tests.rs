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

jeeves_test!( Python, ConsoleEarlyImportFailureClearsCancellationTarget, |ctx| {
    let engine = crate::python::console::PythonConsoleEngine::new().unwrap();
    with_python( |py| {
        let sys = py.import( "sys").unwrap();
        let modules = sys.getattr( "modules").unwrap();
        let oldIo = py.import( "io").unwrap();
        modules.set_item( "io", py.None()).unwrap();
        let result = engine.execute_line( "1 + 1");
        modules.set_item( "io", &oldIo).unwrap();
        jeeves_assert!( ctx, result.is_error);
        jeeves_assert_eq!( ctx, engine.ActiveThreadId(), 0);
    });
});

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
            session.call_method1( "parse_obj", ( obj_data,)).unwrap()
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
    jeeves_assert!( ctx, engine.IsCancelled());
    return;
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Python, CircuitAndSketchIntegration, |ctx| {
    with_python( |py| {
        let  	trellis = py.import( "trellis").expect( "Failed to import trellis");

        // 1. Digital Circuit Construction & Simulation via Python
        let  	layout_cls = trellis.getattr( "Layout").unwrap();
        let  	layout = layout_cls.call0().unwrap();
        let  	ports: ( u32, u32, u32) = layout.call_method1( "add_and_gate", ( "and0",)).unwrap().extract().unwrap();
        layout.call_method0( "freeze").unwrap();

        let  	engine_cls = trellis.getattr( "SimEngine").unwrap();
        let  	engine = engine_cls.call1( ( &layout,)).unwrap();

        let  	in1 = ports.0;
        let  	in2 = ports.1;
        let  	out = ports.2;

        engine.call_method1( "set_bool", ( in1, true)).unwrap();
        engine.call_method1( "set_bool", ( in2, true)).unwrap();
        engine.call_method0( "step_cycle").unwrap();

        let  	out_val: bool = engine.call_method1( "get_bool", ( out,)).unwrap().extract().unwrap();
        jeeves_assert!( ctx, out_val);

        // 2. 2D Sketch Creation, Constraint Solving & 3D Extrusion via Python
        let  	doc_cls = trellis.getattr( "SketchDocument").unwrap();
        let  	doc = doc_cls.call0().unwrap();

        doc.call_method1( "add_line", ( 0.0, 0.0, 10.0, 0.0)).unwrap();
        doc.call_method1( "add_line", ( 10.0, 0.0, 10.0, 10.0)).unwrap();
        doc.call_method1( "add_line", ( 10.0, 10.0, 0.0, 10.0)).unwrap();
        doc.call_method1( "add_line", ( 0.0, 10.0, 0.0, 0.0)).unwrap();

        let  	entity_count: u32 = doc.getattr( "entity_count").unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, entity_count, 4);

        let  	_iters: u32 = doc.call_method1( "solve_constraints", ( 50u32,)).unwrap().extract().unwrap();

        let  	mesh = doc.call_method1( "extrude", ( 5.0f64,)).unwrap();
        let  	face_count: u32 = mesh.getattr( "face_count").unwrap().extract().unwrap();
        jeeves_assert!( ctx, face_count > 0);
    });
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Python, SessionGoldenWorkflowsParity, |ctx| {
    with_python( |py| {
        let trellis = py.import( "trellis").expect( "Failed to import trellis");
        let session = trellis.getattr( "Session").unwrap().call0().unwrap();

        // 1. Resource and document opening
        let doc = session.call_method1( "open", ( "example://Fresco/MathView",)).unwrap();
        let uri: String = doc.getattr( "uri").unwrap().extract().unwrap();
        let kind: String = doc.getattr( "kind").unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, uri, "example://Fresco/MathView");
        jeeves_assert_eq!( ctx, kind, "fresco");

        // 2. Inspecting properties and selection
        let info = session.call_method1( "inspect", ( &doc,)).unwrap();
        let name: String = info.getattr( "name").unwrap().extract().unwrap();
        let shape: Option<String> = info.getattr( "shape").unwrap().extract().unwrap();
        let formula: String = info.getattr( "formula").unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, name, "Matrix_A");
        jeeves_assert_eq!( ctx, shape, Some("Matrix [2 × 2]".to_string()));
        jeeves_assert!( ctx, formula.contains("Matrix_A"));

        // 3. Running explicit operations and jobs
        let job = session.call_method1( "run", ( "example://Rube/DaddaMultiplier4x4",)).unwrap();
        let status: String = job.getattr( "status").unwrap().extract().unwrap();
        let result: String = job.call_method0( "wait").unwrap().extract().unwrap();
        let link: Option<String> = job.getattr( "result_link").unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, status, "Completed");
        jeeves_assert!( ctx, result.contains("DaddaMultiplier4x4"));
        jeeves_assert_eq!( ctx, link, Some("waveform://Rube/DaddaMultiplier4x4".to_string()));

        // 4. Settings read/write
        let settings = session.getattr( "settings").unwrap();
        settings.call_method1( "set", ( "layout/sidebar", true)).unwrap();
        let val: Option<String> = settings.call_method1( "get", ( "layout/sidebar",)).unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, val, Some("True".to_string()));

        // Top-level function shortcuts
        let top_doc = trellis.call_method1( "open", ( "example://Rube/DaddaMultiplier4x4",)).unwrap();
        let top_info = trellis.call_method1( "inspect", ( &top_doc,)).unwrap();
        let ports: Option<u32> = top_info.getattr( "ports").unwrap().extract().unwrap();
        let submods: Option<u32> = top_info.getattr( "submodules").unwrap().extract().unwrap();
        jeeves_assert_eq!( ctx, ports, Some(16));
        jeeves_assert_eq!( ctx, submods, Some(28));
    });
});


