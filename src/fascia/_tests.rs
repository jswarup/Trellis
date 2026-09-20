use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test };
// src/fascia/_test/mod.rs
use	crate::fascia::app::{ AppMessage, AppState };
#[allow( unused_imports)]
use	crate::fascia::explorer::{ default_initial_dir, detect_system_roots, is_text_file };
#[allow( unused_imports)]
use	crate::fascia::{ ActivityTab, ExplorerAction, ExplorerState, FasciaTheme, FileTreeNode, MenuAction, StatusBarInfo, TabBarAction, TabId, TabItem, TabKind, TabManager, ToolBarAction, WaveformAction, WaveformState, default_code_font, default_system_font };
use	crate::rube::{ ParseVcd, VcdDisplayModel };
use	std::path::PathBuf;

//-------------------------------------------------------------------------------------------------
// TabManager lifecycle and operations
jeeves_test!( Fascia, TabManagerLifecycle, |ctx| {
    let  	mut manager = TabManager::new();
    // Initial state should have Welcome tab opened
    jeeves_assert_eq!( ctx, manager.tabs().len(), 1);
    jeeves_assert_eq!( ctx, manager.active_index(), Some( 0));
    jeeves_assert_eq!( ctx, manager.active_tab().unwrap().kind, TabKind::Welcome);
    jeeves_assert_eq!( ctx, manager.active_tab().unwrap().title, "Welcome");
    // Open settings tab
    manager.open_settings();
    jeeves_assert_eq!( ctx, manager.tabs().len(), 2);
    jeeves_assert_eq!( ctx, manager.active_index(), Some( 1));
    jeeves_assert_eq!( ctx, manager.active_tab().unwrap().kind, TabKind::Settings);
    // Opening settings again should not duplicate, but focus it
    manager.select_tab( 0);
    jeeves_assert_eq!( ctx, manager.active_index(), Some( 0));
    manager.open_settings();
    jeeves_assert_eq!( ctx, manager.tabs().len(), 2);
    jeeves_assert_eq!( ctx, manager.active_index(), Some( 1));
    // Open a file
    let  	file_path = PathBuf::from( "src/main.rs");
    let  	( idx, newly_opened) = manager.open_file( file_path.clone());
    jeeves_assert!( ctx, newly_opened);
    jeeves_assert_eq!( ctx, idx, 2);
    jeeves_assert_eq!( ctx, manager.active_index(), Some( 2));
    jeeves_assert_eq!( ctx, manager.active_tab().unwrap().kind, TabKind::FileEditor);
    jeeves_assert_eq!( ctx, manager.active_tab().unwrap().title, "main.rs");
    // Open same file should focus existing
    let  	( idx2, newly_opened2) = manager.open_file( file_path);
    jeeves_assert!( ctx, !newly_opened2);
    jeeves_assert_eq!( ctx, idx2, 2);
    // Mark dirty
    jeeves_assert!( ctx, !manager.active_tab().unwrap().is_dirty);
    manager.mark_active_dirty( true);
    jeeves_assert!( ctx, manager.active_tab().unwrap().is_dirty);
    manager.mark_active_dirty( false);
    jeeves_assert!( ctx, !manager.active_tab().unwrap().is_dirty);
    // Close tab
    let  	closed = manager.close_tab( 2);
    jeeves_assert!( ctx, closed.is_some());
    jeeves_assert_eq!( ctx, closed.unwrap().title, "main.rs");
    jeeves_assert_eq!( ctx, manager.tabs().len(), 2);
    jeeves_assert_eq!( ctx, manager.active_index(), Some( 1));
    // Close all
    manager.close_all();
    jeeves_assert_eq!( ctx, manager.tabs().len(), 0);
    jeeves_assert_eq!( ctx, manager.active_index(), None);
    jeeves_assert!( ctx, manager.active_tab().is_none());
});

//-------------------------------------------------------------------------------------------------
// Filesystem explorer operations and tree nodes
jeeves_test!( Fascia, ExplorerTreeOps, |ctx| {
    let  	roots = detect_system_roots();
    jeeves_assert!( ctx, !roots.is_empty());
    let  	init_dir = default_initial_dir();
    jeeves_assert!( ctx, init_dir.exists());
    let  	mut state = ExplorerState::new( init_dir.clone());
    state.refresh();
    jeeves_assert_eq!( ctx, state.root_node.path, init_dir);
    jeeves_assert!( ctx, state.root_node.is_expanded);
    jeeves_assert!( ctx, state.root_node.is_dir);
    // Extension text check
    jeeves_assert!( ctx, is_text_file( &PathBuf::from( "test.rs")));
    jeeves_assert!( ctx, is_text_file( &PathBuf::from( "config.toml")));
    jeeves_assert!( ctx, is_text_file( &PathBuf::from( "README.md")));
    // Node icon checks
    let  	rs_node = FileTreeNode::new( PathBuf::from( "lib.rs"), 0);
    jeeves_assert_eq!( ctx, rs_node.icon(), "🦀");
    let  	toml_node = FileTreeNode::new( PathBuf::from( "Cargo.toml"), 0);
    jeeves_assert_eq!( ctx, toml_node.icon(), "⚙");
    let  	dir_node = FileTreeNode::new( PathBuf::from( "src"), 0);
    jeeves_assert_eq!( ctx, dir_node.icon(), "📁");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fascia, VcdWaveformViewer, |ctx| {
    let  	vcd = r#"
$timescale 1ns $end
$scope module top $end
$var wire 1 ! clk $end
$upscope $end
$enddefinitions $end
$dumpvars
0!
$end
#10
1!
#20
0!
"#;
    let  	model = ParseVcd( vcd).expect( "VCD should parse");
    let  	mut waveform = WaveformState::New( VcdDisplayModel::FromVcdModel( &model));
    waveform.Update( WaveformAction::SelectSignal( 0));
    waveform.Update( WaveformAction::NextChange);
    jeeves_assert_eq!( ctx, waveform.CursorTime(), 10);
    waveform.Update( WaveformAction::NextChange);
    jeeves_assert_eq!( ctx, waveform.CursorTime(), 20);
    waveform.Update( WaveformAction::PreviousChange);
    jeeves_assert_eq!( ctx, waveform.CursorTime(), 10);
    let  	tab = TabItem::new_file( TabId( 99), PathBuf::from( "trace.vcd"));
    jeeves_assert_eq!( ctx, tab.kind, TabKind::VcdViewer);
    jeeves_assert_eq!( ctx, tab.icon, "VCD");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fascia, ExplorerOpensVcdWaveform, |ctx| {
    let  	path = std::env::temp_dir().join( format!( "segue-fascia-{}.vcd", std::process::id()));
    std::fs::write( 
        &path,
        "$timescale 1ns $end\n$scope module top $end\n$var wire 1 ! clk $end\n$upscope $end\n$enddefinitions $end\n#0\n0!\n",
    )
    .expect( "temporary VCD should be writable");
    let  	mut app = AppState::new();
    let  	_ = app.update( AppMessage::Explorer( ExplorerAction::OpenFile( path.clone())));
    jeeves_assert_eq!( ctx, app.explorer.selected_path, Some( path.clone()));
    jeeves_assert_eq!( 
        ctx,
        app.tab_manager.active_tab().unwrap().kind,
        TabKind::VcdViewer
    );
    jeeves_assert!( 
        ctx,
        app.open_waveforms
            .contains_key( &app.tab_manager.active_tab().unwrap().id)
    );
    std::fs::remove_file( path).expect( "temporary VCD should be removable");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fascia, ExplorerOpensPtsViewer, |ctx| {
    let  	path = std::env::temp_dir().join( format!( "segue-fascia-{}.pts", std::process::id()));
    std::fs::write( &path, "2\n0 0 0\n10 20 30\n").expect( "temporary PTS should be writable");
    let  	mut app = AppState::new();
    let  	_ = app.update( AppMessage::Explorer( ExplorerAction::OpenFile( path.clone())));
    let  	tab = app.tab_manager.active_tab().unwrap();
    jeeves_assert_eq!( ctx, tab.kind, TabKind::PtsViewer);
    let id = tab.id;
    jeeves_assert!( ctx, app.Geometry( id).unwrap().Asset().is_none());
    let loaded = iced::futures::executor::block_on( crate::fascia::geometry_load::Load( path.clone(), app.Geometry( id).unwrap().Cancellation()));
    let _ = app.update( AppMessage::GeometryLoaded( id, loaded));
    jeeves_assert_eq!( ctx, app.Geometry( id).unwrap().Asset().unwrap().VertexCount(), 2);
    std::fs::remove_file( path).expect( "temporary PTS should be removable");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fascia, ExplorerOpensObjViewer, |ctx| {
    let  	path = std::env::temp_dir().join( format!( "segue-fascia-{}.obj", std::process::id()));
    std::fs::write( &path, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")
        .expect( "temporary OBJ should be writable");
    let  	mut app = AppState::new();
    let  	_ = app.update( AppMessage::Explorer( ExplorerAction::OpenFile( path.clone())));
    let  	tab = app.tab_manager.active_tab().unwrap();
    jeeves_assert_eq!( ctx, tab.kind, TabKind::ObjViewer);
    let id = tab.id;
    jeeves_assert!( ctx, app.Geometry( id).unwrap().Asset().is_none());
    let loaded = iced::futures::executor::block_on( crate::fascia::geometry_load::Load( path.clone(), app.Geometry( id).unwrap().Cancellation()));
    let _ = app.update( AppMessage::GeometryLoaded( id, loaded));
    jeeves_assert_eq!( ctx, app.Geometry( id).unwrap().Asset().unwrap().VertexCount(), 3);
    jeeves_assert_eq!( ctx, app.Geometry( id).unwrap().Asset().unwrap().FaceCount(), 1);
    std::fs::remove_file( path).expect( "temporary OBJ should be removable");
});

//-------------------------------------------------------------------------------------------------
jeeves_test!( Fascia, GeometryCancellationAndStaleResults, |ctx| {
    use crate::fleck::{ ParsePts, geometry::GeometryAsset };
    use std::sync::{ Arc, atomic::Ordering };
    let mut app = AppState::new();
    let _ = app.update( AppMessage::OpenFile( PathBuf::from( "pending.pts")));
    let id = app.tab_manager.active_tab().unwrap().id;
    let token = app.Geometry( id).unwrap().Cancellation();
    let _ = app.update( AppMessage::CloseActiveTab);
    jeeves_assert!( ctx, token.load( Ordering::Acquire));
    let asset = Arc::new( GeometryAsset::FromPts( ParsePts( "0 0 0\n").unwrap()).unwrap());
    let _ = app.update( AppMessage::GeometryLoaded( id, Ok( asset)));
    jeeves_assert!( ctx, app.Geometry( id).is_none());
    let result = iced::futures::executor::block_on( crate::fascia::geometry_load::Load( PathBuf::from( "missing.pts"), token));
    jeeves_assert!( ctx, result.is_err());
});

jeeves_test!( Fascia, GeometryResultsStayWithTheirTab, |ctx| {
    use crate::fascia::geometry_view::GeometryAction;
    use crate::fleck::{ ParsePts, geometry::GeometryAsset };
    use std::sync::Arc;
    let mut app = AppState::new();
    let _ = app.update( AppMessage::OpenFile( PathBuf::from( "first.pts")));
    let first = app.tab_manager.active_tab().unwrap().id;
    let _ = app.update( AppMessage::OpenFile( PathBuf::from( "second.obj")));
    let second = app.tab_manager.active_tab().unwrap().id;
    let status = app.status_info.message.clone();
    let asset = Arc::new( GeometryAsset::FromPts( ParsePts( "0 0 0\n1 1 1\n").unwrap()).unwrap());
    let _ = app.update( AppMessage::GeometryLoaded( first, Ok( asset.clone())));
    jeeves_assert_eq!( ctx, app.tab_manager.active_tab().unwrap().id, second);
    jeeves_assert_eq!( ctx, app.status_info.message, status);
    jeeves_assert_eq!( ctx, app.Geometry( first).unwrap().Asset().unwrap().VertexCount(), 2);
    jeeves_assert!( ctx, app.Geometry( second).unwrap().Asset().is_none());
    let _ = app.update( AppMessage::Geometry( second, GeometryAction::Cancel));
    let _ = app.update( AppMessage::GeometryLoaded( second, Ok( asset)));
    jeeves_assert!( ctx, app.Geometry( second).unwrap().Asset().is_none());
    jeeves_assert!( ctx, app.Geometry( second).unwrap().Error().is_some());
    let _ = app.update( AppMessage::OpenFile( PathBuf::from( "first.pts")));
    jeeves_assert_eq!( ctx, app.tab_manager.active_tab().unwrap().id, first);
    jeeves_assert_eq!( ctx, app.Geometry( first).unwrap().Asset().unwrap().VertexCount(), 2);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fascia, GeometryCameraFitAndNavigation, |ctx| {
    use crate::fascia::camera::ViewCamera;
    use glam::{ Mat4, Vec3 };
    let mut camera = ViewCamera::default();
    [0.3f32, 1.0, 3.0].iter().copied().for_each( |aspect| {
        camera.Fit( aspect);
        let matrix = Mat4::from_cols_array( &camera.Matrix( aspect));
        [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z].iter().for_each( |p| {
            let clip = matrix.project_point3( *p);
            jeeves_assert!( ctx, clip.x.abs() < 1.0 && clip.y.abs() < 1.0 && clip.z > 0.0 && clip.z < 1.0);
        });
    });
    let before = camera.Matrix( 1.0);
    camera.Orbit( 40.0, 20.0);
    camera.Pan( 10.0, 5.0, 600.0);
    camera.Zoom( 4.0);
    camera.ToggleProjection();
    jeeves_assert!( ctx, before != camera.Matrix( 1.0));
    jeeves_assert!( ctx, camera.Matrix( 1.0).iter().all( |v| v.is_finite()));
    camera.Zoom( 10000.0);
    jeeves_assert!( ctx, camera.Matrix( 1.0).iter().all( |v| v.is_finite()));
});

//-------------------------------------------------------------------------------------------------
// Theme palettes and font selection
jeeves_test!( Fascia, ThemePaletteVariants, |ctx| {
    for &theme in FasciaTheme::ALL {
        let  	palette = theme.palette();
        if theme.is_dark() {
            jeeves_assert!( ctx, palette.text_primary.r > 0.5);
        } else {
            jeeves_assert!( ctx, palette.text_primary.r < 0.5);
        }
        let  	_sys_font = default_system_font( theme);
        let  	_code_font = default_code_font( theme);
    }
    let  	default_theme = FasciaTheme::default();
    jeeves_assert_eq!( ctx, default_theme, FasciaTheme::native_default());
    #[cfg( target_os = "windows")]
    jeeves_assert_eq!( ctx, default_theme, FasciaTheme::WindowsDark);
    #[cfg( not( target_os = "windows"))]
    jeeves_assert_eq!( ctx, default_theme, FasciaTheme::LinuxDark);
});

//-------------------------------------------------------------------------------------------------
// Console test
jeeves_test!( Fascia, Console, Console, |ctx| {
    jeeves_println!( 
        ctx,
        "         [Fascia Console Test: Modular UI Shell Active]"
    );
    jeeves_println!( 
        ctx,
        "           Supported Themes    : {}",
        FasciaTheme::ALL.len()
    );
    for &theme in FasciaTheme::ALL {
        jeeves_println!( 
            ctx,
            "             Theme: {:<20} (Dark: {})",
            theme.display_name(),
            theme.is_dark()
        );
    }
    let  	roots = detect_system_roots();
    jeeves_println!( ctx, "           System Drive Roots  : {:?}", roots);
    let  	status = StatusBarInfo::default();
    jeeves_println!( 
        ctx,
        "           Default Status      : Branch='{}', Encoding='{}', Indent='{}'",
        status.branch,
        status.encoding,
        status.indentation
    );
});

//-------------------------------------------------------------------------------------------------
// Example test
jeeves_test!( Fascia, Example, Example, |ctx| {
    // 1. Initialize Tab Manager with Welcome and a file
    let  	mut tab_manager = TabManager::new();
    let  	file_path = PathBuf::from( "src/lib.rs");
    tab_manager.open_file( file_path);
    jeeves_assert_eq!( ctx, tab_manager.tabs().len(), 2);
    jeeves_assert_eq!( ctx, tab_manager.active_index(), Some( 1));
    // 2. Setup Explorer
    let  	explorer = ExplorerState::new( PathBuf::from( "."));
    jeeves_assert!( ctx, explorer.root_node.is_expanded);
    // 3. Setup Theme and Status
    let  	theme = FasciaTheme::WindowsDark;
    let  	palette = theme.palette();
    let  	status = StatusBarInfo {
        branch: "master".to_string(),
        message: "Framework Ready".to_string(),
        line: 42,
        column: 10,
        encoding: "UTF-8",
        indentation: "Spaces: 4",
        language: "Rust".to_string(),
    };
    jeeves_assert_eq!( ctx, theme.is_dark(), true);
    jeeves_assert_eq!( ctx, status.line, 42);
    jeeves_assert_eq!( ctx, status.column, 10);
    jeeves_assert_eq!( ctx, palette.corner_radius, 5.0);
    // If invoked specifically via -e (example mode), launch the interactive GUI app
    let  	has_e_flag = std::env::args().any( |a| a == "-e");
    let  	is_headless = std::env::var( "SEGUE_HEADLESS").as_deref() == Ok( "1");
    if has_e_flag && !is_headless {
        jeeves_println!( 
            ctx,
            "         [Launching Fascia Interactive Desktop GUI Shell...]"
        );
        if let  	Err( err) = crate::fascia::run_app() {
            jeeves_println!( ctx, "         [Error launching Fascia GUI: {:?}]", err);
        }
    }
});
