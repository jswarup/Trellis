// _tests.rs -----------------------------------------------------------------------------------------

use crate::{jeeves_assert, jeeves_assert_eq, jeeves_println, jeeves_test};
// src/fascia/_test/mod.rs
use crate::fascia::app::{AppMessage, AppState};
#[allow(unused_imports)]
use crate::fascia::explorer::{default_initial_dir, detect_system_roots, is_text_file};
#[allow(unused_imports)]
use crate::fascia::{
    ActivityTab, ExplorerAction, ExplorerState, ExplorerTab, FasciaTheme, FileTreeNode, MenuAction,
    StatusBarInfo, TabBarAction, TabId, TabItem, TabKind, TabManager, ToolBarAction,
    WaveformAction, WaveformState, default_code_font, default_system_font,
};
use crate::silo::IArr;
use std::path::PathBuf;

//-------------------------------------------------------------------------------------------------
jeeves_test!( Fascia, CircuitScenePresentationAndThemeAdherence, |ctx| {
    use crate::fascia::{circuit_scene, theme::FasciaTheme};
    use crate::fenst::cask::CaskRenderCommand;
    use crate::fenst::xplr::{CircuitXplr, PortDescriptor, PortDirection, WireDescriptor, Xplr};
    use std::sync::atomic::AtomicBool;

    struct MockGate { name: &'static str, kind: &'static str }
    impl Xplr for MockGate {
        fn Name(&self) -> &str { self.name }
        fn Path(&self) -> &str { self.name }
        fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
    }
    impl CircuitXplr for MockGate {
        fn ComponentType(&self) -> &str { self.kind }
        fn Ports(&self) -> Vec<PortDescriptor> {
            vec![
                PortDescriptor::New("in0", PortDirection::In),
                PortDescriptor::New("in1", PortDirection::In),
                PortDescriptor::New("out", PortDirection::Out),
            ]
        }
        fn Wires(&self) -> Vec<WireDescriptor> { Vec::new() }
        fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>> { Vec::new() }
    }

    struct MockCircuit;
    impl Xplr for MockCircuit {
        fn Name(&self) -> &str { "ALU" }
        fn Path(&self) -> &str { "ALU" }
        fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
    }
    impl CircuitXplr for MockCircuit {
        fn ComponentType(&self) -> &str { "Module" }
        fn Ports(&self) -> Vec<PortDescriptor> {
            vec![
                PortDescriptor::New("a", PortDirection::In),
                PortDescriptor::New("b", PortDirection::In),
                PortDescriptor::New("res", PortDirection::Out),
            ]
        }
        fn Wires(&self) -> Vec<WireDescriptor> {
            vec![
                WireDescriptor::New("ALU", "a", "G1", "in0"),
                WireDescriptor::New("ALU", "b", "G1", "in1"),
                WireDescriptor::New("G1", "out", "G2", "in0"),
                WireDescriptor::New("ALU", "b", "G2", "in1"),
                WireDescriptor::New("G2", "out", "ALU", "res"),
            ]
        }
        fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>> {
            vec![
                Box::new(MockGate { name: "G1", kind: "AND" }),
                Box::new(MockGate { name: "G2", kind: "OR" }),
            ]
        }
    }

    let circuit = MockCircuit;
    let palette = FasciaTheme::WindowsDark.palette();

    // 1. Classical 2D Circuit Schematic Presentation
    let schematic = circuit_scene::CircuitSchematic(&circuit, &palette);
    jeeves_assert_eq!(ctx, schematic.NodeCount(), 3);
    jeeves_assert_eq!(ctx, schematic.Nodes()[0].Name(), "ALU");

    let g1 = schematic.Nodes().iter().find(|n| n.Name() == "G1").unwrap();
    let g2 = schematic.Nodes().iter().find(|n| n.Name() == "G2").unwrap();
    jeeves_assert!(ctx, g1.Bounds().x < g2.Bounds().x, "G1 must be left of G2");

    let mut has_lines = false;
    let mut has_rects = false;
    for cmd in schematic.Commands() {
        match cmd {
            CaskRenderCommand::Line { .. } => has_lines = true,
            CaskRenderCommand::Rectangle { .. } => has_rects = true,
            _ => {}
        }
    }
    jeeves_assert!(ctx, has_lines, "Schematic must generate wire lines");
    jeeves_assert!(ctx, has_rects, "Schematic must generate module rectangles");
    jeeves_assert!(ctx, schematic.PinCount() >= 4, "Schematic must generate interactive pins");
    jeeves_assert!(ctx, schematic.WireCount() >= 1, "Schematic must generate interactive wires");

    let first_pin = &schematic.Pins()[0];
    let hit_pin = schematic.FindPinAt(first_pin.Point());
    jeeves_assert!(ctx, hit_pin.is_some(), "Point at pin must hit test");
    jeeves_assert_eq!(ctx, hit_pin.unwrap().PortName(), first_pin.PortName());

    let first_wire = &schematic.Wires()[0];
    let seg = first_wire.Segments()[0];
    let mid_x = (seg.0.x + seg.1.x) * 0.5;
    let mid_y = (seg.0.y + seg.1.y) * 0.5;
    let hit_wire = schematic.FindWireAt(crate::fenst::Point::New(mid_x, mid_y), 3.0);
    jeeves_assert!(ctx, hit_wire.is_some(), "Midpoint of wire segment must hit test");

    // 2. 3D Circuit Geometry Asset Generation
    let asset = circuit_scene::CircuitBuild(&circuit, &AtomicBool::new(false)).unwrap();
    jeeves_assert!(ctx, asset.MaxDepth() >= 2);
    jeeves_assert!(ctx, circuit_scene::CircuitBuild(&circuit, &AtomicBool::new(true)).is_err(), "Cancellation must be respected");
});

//-------------------------------------------------------------------------------------------------
// TabManager lifecycle and operations
#[cfg( feature = "tests")]
jeeves_test!( Fascia, XplrExampleBrowserLifecycle, |ctx| {
    use crate::fascia::examples::{ExampleAction, ExampleBrowser, ExampleCompletion};
    use crate::silo::USeg;

    let mut browser = ExampleBrowser::default();
    let mut exampleIndex = u32::MAX;
    browser.Examples().USeg().Traverse( |index| {
        if browser.Examples()[index].name == "TraverseDepthFullOrder"
        {
            exampleIndex = index;
        }
    });
    jeeves_assert!( ctx, exampleIndex != u32::MAX);
    let example = browser.Examples()[exampleIndex];
    let _ = browser.Update( ExampleAction::Select( exampleIndex));
    jeeves_assert_eq!( ctx, browser.ResultCount(), 0);
    USeg::FromLen( 2).Traverse( |_| {
        let _ = browser.Update( ExampleAction::Run( exampleIndex));
        let completed = ExampleCompletion::New( Ok( example.Run().unwrap()));
        let _ = browser.Update( ExampleAction::Completed( exampleIndex, completed.clone()));
        let count = browser.ResultCount();
        let _ = browser.Update( ExampleAction::Completed( exampleIndex, completed));
        jeeves_assert_eq!( ctx, browser.ResultCount(), count);
    });
    jeeves_assert_eq!( ctx, browser.ResultCount(), 2);
    jeeves_assert!( ctx, browser.Status().starts_with( "Passed"));
    let first = browser.ResultNode( 0);
    let second = browser.ResultNode( 1);
    jeeves_assert!( ctx, first != second);
    let _ = browser.Update( ExampleAction::Toggle( first));
    jeeves_assert_eq!( ctx, browser.Children( first).Len(), 3);
    jeeves_assert_eq!( ctx, browser.Children( second).Len(), 0);
    let directory = browser.Children( first)[0];
    let _ = browser.Update( ExampleAction::Toggle( directory));
    jeeves_assert_eq!( ctx, browser.Children( directory).Len(), 2);
    let leaf = browser.Children( directory)[0];
    let _ = browser.Update( ExampleAction::Toggle( leaf));
    jeeves_assert!( ctx, browser.Preview().contains( "FileA1"));
    jeeves_assert!( ctx, browser.Preview().contains( "6 bytes"));
    let _ = browser.Update( ExampleAction::Toggle( first));
    let _ = browser.Update( ExampleAction::Toggle( first));
    jeeves_assert_eq!( ctx, browser.Children( first)[0], directory);
    let _ = browser.Update( ExampleAction::Run( exampleIndex));
    let _ = browser.Update( ExampleAction::Completed( exampleIndex,
        ExampleCompletion::New( Err( "Deliberate failure".into()))));
    jeeves_assert_eq!( ctx, browser.ResultCount(), 2);
    jeeves_assert!( ctx, browser.Status().contains( "Deliberate failure"));
    let _ = browser.Update( ExampleAction::Run( exampleIndex));
    let _ = browser.Update( ExampleAction::Completed( exampleIndex,
        ExampleCompletion::New( Ok( Box::new( crate::fenst::FsBranch::New(
            "missing://example-fixture".into()))))));
    let missing = browser.ResultNode( 2);
    let _ = browser.Update( ExampleAction::Toggle( missing));
    jeeves_assert!( ctx, browser.Preview().starts_with( "Cannot expand"));
    jeeves_assert_eq!( ctx, browser.Children( missing).Len(), 0);
    let _view = browser.View( FasciaTheme::default().palette());
});

jeeves_test!( Fascia, ExplorerSettingsAndExamplesToggles, |ctx| {
    use crate::fascia::ExplorerState;
    use std::path::PathBuf;

    let mut explorer = ExplorerState::new( PathBuf::from( "src"));
    // Default: Filesystem ON, Examples and Settings OFF (hidden)
    jeeves_assert_eq!( ctx, explorer.show_filesystem, true);
    jeeves_assert_eq!( ctx, explorer.show_examples, false);
    jeeves_assert_eq!( ctx, explorer.show_settings, false);
    jeeves_assert!( ctx, explorer.has_visible_roots());
    jeeves_assert_eq!( ctx, explorer.root_node.name, "Filesystem");
    jeeves_assert!( ctx, explorer.examples_root.is_none());
    jeeves_assert!( ctx, explorer.settings_root.is_none());

    // Toggle examples on
    explorer.toggle_examples();
    jeeves_assert_eq!( ctx, explorer.show_examples, true);
    jeeves_assert!( ctx, explorer.examples_root.is_some());
    let ex_root = explorer.examples_root.as_ref().unwrap();
    jeeves_assert_eq!( ctx, ex_root.name, "Examples");

    // Toggle settings on
    explorer.toggle_settings();
    jeeves_assert_eq!( ctx, explorer.show_settings, true);
    jeeves_assert!( ctx, explorer.settings_root.is_some());
    let st_root = explorer.settings_root.as_ref().unwrap();
    jeeves_assert_eq!( ctx, st_root.name, "Settings");

    // Toggle back off
    explorer.toggle_examples();
    jeeves_assert_eq!( ctx, explorer.show_examples, false);
    explorer.toggle_settings();
    jeeves_assert_eq!( ctx, explorer.show_settings, false);

    // Toggle filesystem off -> all hidden
    explorer.toggle_filesystem();
    jeeves_assert_eq!( ctx, explorer.show_filesystem, false);
    jeeves_assert!( ctx, !explorer.has_visible_roots());

    // Toggle filesystem back on
    explorer.toggle_filesystem();
    jeeves_assert_eq!( ctx, explorer.show_filesystem, true);
    jeeves_assert!( ctx, explorer.has_visible_roots());
});

jeeves_test!( Fascia, ExplorerCollapseWhenAllHidden, |ctx| {
    use crate::fascia::{ AppState, AppMessage, ExplorerAction, MenuAction };

    let mut app = AppState::new();
    jeeves_assert!( ctx, app.show_sidebar);
    jeeves_assert!( ctx, app.explorer.show_filesystem);
    jeeves_assert!( ctx, !app.explorer.show_examples);
    jeeves_assert!( ctx, !app.explorer.show_settings);

    // Turning off filesystem when examples and settings are off collapses the sidebar
    let _ = app.update( AppMessage::Explorer( ExplorerAction::ToggleFilesystem));
    jeeves_assert!( ctx, !app.explorer.show_filesystem);
    jeeves_assert!( ctx, !app.explorer.has_visible_roots());
    jeeves_assert!( ctx, !app.show_sidebar);

    // Opening explorer tab via ActivityTab or Menu does not open it if all views are disabled
    let _ = app.update( AppMessage::SelectActivityTab( crate::fascia::ActivityTab::Explorer));
    jeeves_assert!( ctx, !app.show_sidebar);
    jeeves_assert!( ctx, !app.explorer.has_visible_roots());

    let _ = app.update( AppMessage::Menu( MenuAction::OpenExplorer));
    jeeves_assert!( ctx, !app.show_sidebar);
    jeeves_assert!( ctx, !app.explorer.has_visible_roots());

    // Explorer only opens if at least one view is selected from View drop-down menu
    let _ = app.update( AppMessage::Menu( MenuAction::ToggleExplorerFilesystem));
    jeeves_assert!( ctx, app.show_sidebar);
    jeeves_assert!( ctx, app.explorer.show_filesystem);
    jeeves_assert!( ctx, app.explorer.has_visible_roots());
});

jeeves_test!( Fascia, ExplorerTabsAndCurrentFolderFocus, |ctx| {
    use crate::fascia::{ AppState, AppMessage, ExplorerAction, ExplorerTab, MenuAction, default_initial_dir };

    let mut app = AppState::new();
    let current_dir = default_initial_dir();

    // 1. Current folder should be open and in focus on the filesystem by default
    jeeves_assert_eq!( ctx, app.explorer.active_tab, ExplorerTab::Filesystem);
    jeeves_assert_eq!( ctx, app.explorer.selected_path, Some(current_dir));
    jeeves_assert!( ctx, app.explorer.root_node.is_expanded);

    // 2. Select Settings view from View menu
    let _ = app.update( AppMessage::Menu( MenuAction::ToggleExplorerSettings));
    jeeves_assert!( ctx, app.explorer.show_settings);
    jeeves_assert_eq!( ctx, app.explorer.active_tab, ExplorerTab::Settings);
    jeeves_assert!( ctx, app.explorer.settings_root.is_some());

    // 3. Select Examples view from View menu
    let _ = app.update( AppMessage::Menu( MenuAction::ToggleExplorerExamples));
    jeeves_assert!( ctx, app.explorer.show_examples);
    jeeves_assert_eq!( ctx, app.explorer.active_tab, ExplorerTab::Examples);
    jeeves_assert!( ctx, app.explorer.examples_root.is_some());

    // 4. Tab switching within Explorer
    let _ = app.update( AppMessage::Explorer( ExplorerAction::SelectTab( ExplorerTab::Filesystem)));
    jeeves_assert_eq!( ctx, app.explorer.active_tab, ExplorerTab::Filesystem);

    let _ = app.update( AppMessage::Explorer( ExplorerAction::SelectTab( ExplorerTab::Settings)));
    jeeves_assert_eq!( ctx, app.explorer.active_tab, ExplorerTab::Settings);

    let _ = app.update( AppMessage::Explorer( ExplorerAction::SelectTab( ExplorerTab::Examples)));
    jeeves_assert_eq!( ctx, app.explorer.active_tab, ExplorerTab::Examples);

    // 5. If active tab is turned off via View menu, active tab shifts to another visible tab
    let _ = app.update( AppMessage::Menu( MenuAction::ToggleExplorerExamples));
    jeeves_assert!( ctx, !app.explorer.show_examples);
    jeeves_assert!( ctx, app.explorer.active_tab != ExplorerTab::Examples);
});

jeeves_test!(Fascia, TabManagerLifecycle, |ctx| {
    let mut manager = TabManager::new();
    // Initial state should have Welcome tab opened
    jeeves_assert_eq!(ctx, manager.tabs().Len(), 1);
    jeeves_assert_eq!(ctx, manager.active_index(), Some(0));
    jeeves_assert_eq!(ctx, manager.active_tab().unwrap().kind, TabKind::Welcome);
    jeeves_assert_eq!(ctx, manager.active_tab().unwrap().title, "Welcome");
    // Open settings tab
    manager.open_settings();
    jeeves_assert_eq!(ctx, manager.tabs().Len(), 2);
    jeeves_assert_eq!(ctx, manager.active_index(), Some(1));
    jeeves_assert_eq!(ctx, manager.active_tab().unwrap().kind, TabKind::Settings);
    // Opening settings again should not duplicate, but focus it
    manager.select_tab(0);
    jeeves_assert_eq!(ctx, manager.active_index(), Some(0));
    manager.open_settings();
    jeeves_assert_eq!(ctx, manager.tabs().Len(), 2);
    jeeves_assert_eq!(ctx, manager.active_index(), Some(1));
    // Open a file
    let file_path = PathBuf::from("src/main.rs");
    let (idx, newly_opened) = manager.open_file(file_path.clone());
    jeeves_assert!(ctx, newly_opened);
    jeeves_assert_eq!(ctx, idx, 2);
    jeeves_assert_eq!(ctx, manager.active_index(), Some(2));
    jeeves_assert_eq!(ctx, manager.active_tab().unwrap().kind, TabKind::FileEditor);
    jeeves_assert_eq!(ctx, manager.active_tab().unwrap().title, "main.rs");
    // Open same file should focus existing
    let (idx2, newly_opened2) = manager.open_file(file_path);
    jeeves_assert!(ctx, !newly_opened2);
    jeeves_assert_eq!(ctx, idx2, 2);
    // Mark dirty
    jeeves_assert!(ctx, !manager.active_tab().unwrap().is_dirty);
    manager.mark_active_dirty(true);
    jeeves_assert!(ctx, manager.active_tab().unwrap().is_dirty);
    manager.mark_active_dirty(false);
    jeeves_assert!(ctx, !manager.active_tab().unwrap().is_dirty);
    // Close tab
    let closed = manager.close_tab(2);
    jeeves_assert!(ctx, closed.is_some());
    jeeves_assert_eq!(ctx, closed.unwrap().title, "main.rs");
    jeeves_assert_eq!(ctx, manager.tabs().Len(), 2);
    jeeves_assert_eq!(ctx, manager.active_index(), Some(1));
    // Close all
    manager.close_all();
    jeeves_assert_eq!(ctx, manager.tabs().Len(), 0);
    jeeves_assert_eq!(ctx, manager.active_index(), None);
    jeeves_assert!(ctx, manager.active_tab().is_none());
});

//-------------------------------------------------------------------------------------------------
// Filesystem explorer operations and tree nodes
jeeves_test!(Fascia, ExplorerTreeOps, |ctx| {
    let roots = detect_system_roots();
    jeeves_assert!(ctx, !roots.is_empty());
    let init_dir = default_initial_dir();
    jeeves_assert!(ctx, init_dir.exists());
    let mut state = ExplorerState::new(init_dir.clone());
    state.refresh();
    jeeves_assert_eq!(ctx, state.root_node.name, "Filesystem");
    jeeves_assert_eq!(ctx, state.root_node.path, PathBuf::from("file://"));
    jeeves_assert!(ctx, state.root_node.is_expanded);
    jeeves_assert!(ctx, state.root_node.is_dir);
    // Setting an explicit root focuses on that directory
    state.set_root(init_dir.clone());
    jeeves_assert_eq!(ctx, state.root_node.path, init_dir);
    jeeves_assert!(ctx, state.root_node.is_expanded);
    jeeves_assert!(ctx, state.root_node.is_dir);
    // Extension text check
    jeeves_assert!(ctx, is_text_file(&PathBuf::from("test.rs")));
    jeeves_assert!(ctx, is_text_file(&PathBuf::from("config.toml")));
    jeeves_assert!(ctx, is_text_file(&PathBuf::from("README.md")));
    // Node icon checks
    let rs_node = FileTreeNode::new(PathBuf::from("lib.rs"), 0);
    jeeves_assert_eq!(ctx, rs_node.icon(), "🦀");
    let toml_node = FileTreeNode::new(PathBuf::from("Cargo.toml"), 0);
    jeeves_assert_eq!(ctx, toml_node.icon(), "⚙");
    let dir_node = FileTreeNode::new(PathBuf::from("src"), 0);
    jeeves_assert_eq!(ctx, dir_node.icon(), "📁");
    let fs_node = FileTreeNode::new(PathBuf::from("file://"), 0);
    jeeves_assert_eq!(ctx, fs_node.icon(), "💻");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, VcdWaveformViewer, |ctx| {
    use crate::fenst::waveform::ParseWaveform;
    use std::sync::Arc;

    let vcd = r#"
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
    let model = ParseWaveform("vcd", vcd).expect("VCD should parse");
    let mut waveform = WaveformState::New(Arc::from(model));
    waveform.Update(WaveformAction::SelectSignal(0));
    waveform.Update(WaveformAction::NextChange);
    jeeves_assert_eq!(ctx, waveform.CursorTime(), 10);
    waveform.Update(WaveformAction::NextChange);
    jeeves_assert_eq!(ctx, waveform.CursorTime(), 20);
    waveform.Update(WaveformAction::PreviousChange);
    jeeves_assert_eq!(ctx, waveform.CursorTime(), 10);
    let tab = TabItem::new_file(TabId(99), PathBuf::from("trace.vcd"));
    jeeves_assert_eq!(ctx, tab.kind, TabKind::VcdViewer);
    jeeves_assert_eq!(ctx, tab.icon, "VCD");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, ExplorerOpensVcdWaveform, |ctx| {
    let path = std::env::temp_dir().join(format!("trellis-fascia-{}.vcd", std::process::id()));
    std::fs::write(
        &path,
        "$timescale 1ns $end\n$scope module top $end\n$var wire 1 ! clk $end\n$upscope $end\n$enddefinitions $end\n#0\n0!\n",
    )
    .expect( "temporary VCD should be writable");
    let mut app = AppState::new();
    let _ = app.update(AppMessage::Explorer(ExplorerAction::OpenFile(path.clone())));
    jeeves_assert_eq!(ctx, app.explorer.selected_path, Some(path.clone()));
    jeeves_assert_eq!(
        ctx,
        app.tab_manager.active_tab().unwrap().kind,
        TabKind::VcdViewer
    );
    jeeves_assert!(
        ctx,
        app.Waveform(app.tab_manager.active_tab().unwrap().id).is_some()
    );
    std::fs::remove_file(path).expect("temporary VCD should be removable");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, ExplorerOpensPtsViewer, |ctx| {
    let path = std::env::temp_dir().join(format!("trellis-fascia-{}.pts", std::process::id()));
    std::fs::write(&path, "2\n0 0 0\n10 20 30\n").expect("temporary PTS should be writable");
    let mut app = AppState::new();
    let _ = app.update(AppMessage::Explorer(ExplorerAction::OpenFile(path.clone())));
    let tab = app.tab_manager.active_tab().unwrap();
    jeeves_assert_eq!(ctx, tab.kind, TabKind::PtsViewer);
    let id = tab.id;
    jeeves_assert!(ctx, app.Geometry(id).unwrap().Asset().is_none());
    let loaded = iced::futures::executor::block_on(crate::fascia::geometry_load::Load(
        path.clone(),
        app.Geometry(id).unwrap().Cancellation(),
    ));
    let _ = app.update( AppMessage::GeometryPrepared( id, app.Geometry( id).unwrap().Cancellation(), loaded));
    jeeves_assert_eq!(
        ctx,
        app.Geometry(id).unwrap().Asset().unwrap().VertexCount(),
        2
    );
    std::fs::remove_file(path).expect("temporary PTS should be removable");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, ExplorerOpensObjViewer, |ctx| {
    let path = std::env::temp_dir().join(format!("trellis-fascia-{}.obj", std::process::id()));
    std::fs::write(&path, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")
        .expect("temporary OBJ should be writable");
    let mut app = AppState::new();
    let _ = app.update(AppMessage::Explorer(ExplorerAction::OpenFile(path.clone())));
    let tab = app.tab_manager.active_tab().unwrap();
    jeeves_assert_eq!(ctx, tab.kind, TabKind::ObjViewer);
    let id = tab.id;
    jeeves_assert!(ctx, app.Geometry(id).unwrap().Asset().is_none());
    let loaded = iced::futures::executor::block_on(crate::fascia::geometry_load::Load(
        path.clone(),
        app.Geometry(id).unwrap().Cancellation(),
    ));
    let _ = app.update( AppMessage::GeometryPrepared( id, app.Geometry( id).unwrap().Cancellation(), loaded));
    jeeves_assert_eq!(
        ctx,
        app.Geometry(id).unwrap().Asset().unwrap().VertexCount(),
        3
    );
    jeeves_assert_eq!(
        ctx,
        app.Geometry(id).unwrap().Asset().unwrap().FaceCount(),
        1
    );
    std::fs::remove_file(path).expect("temporary OBJ should be removable");
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, GeometryCancellationAndStaleResults, |ctx| {
    use crate::fenst::geometry::LoadGeometry;
    use std::sync::{Arc, atomic::Ordering};
    let mut app = AppState::new();
    let _ = app.update(AppMessage::OpenFile(PathBuf::from("pending.pts")));
    let id = app.tab_manager.active_tab().unwrap().id;
    let token = app.Geometry(id).unwrap().Cancellation();
    let _ = app.update(AppMessage::CloseActiveTab);
    jeeves_assert!(ctx, token.load(Ordering::Acquire));
    let asset = Arc::new(LoadGeometry("pts", "0 0 0\n").unwrap());
    let _ = app.update( AppMessage::GeometryPrepared( id, token.clone(), Ok( asset)));
    jeeves_assert!(ctx, app.Geometry(id).is_none());
    let result = iced::futures::executor::block_on(crate::fascia::geometry_load::Load(
        PathBuf::from("missing.pts"),
        token,
    ));
    jeeves_assert!(ctx, result.is_err());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, GeometryResultsStayWithTheirTab, |ctx| {
    use crate::fascia::geometry_view::GeometryAction;
    use crate::fenst::geometry::LoadGeometry;
    use std::sync::Arc;
    let mut app = AppState::new();
    let _ = app.update(AppMessage::OpenFile(PathBuf::from("first.pts")));
    let first = app.tab_manager.active_tab().unwrap().id;
    let _ = app.update(AppMessage::OpenFile(PathBuf::from("second.obj")));
    let second = app.tab_manager.active_tab().unwrap().id;
    let status = app.status_info.message.clone();
    let asset = Arc::new(LoadGeometry("pts", "0 0 0\n1 1 1\n").unwrap());
    let _ = app.update( AppMessage::GeometryPrepared( first, app.Geometry( first).unwrap().Cancellation(), Ok( asset.clone())));
    jeeves_assert_eq!(ctx, app.tab_manager.active_tab().unwrap().id, second);
    jeeves_assert_eq!(ctx, app.status_info.message, status);
    jeeves_assert_eq!(
        ctx,
        app.Geometry(first).unwrap().Asset().unwrap().VertexCount(),
        2
    );
    jeeves_assert!(ctx, app.Geometry(second).unwrap().Asset().is_none());
    let _ = app.update(AppMessage::Geometry(second, GeometryAction::Cancel));
    let _ = app.update( AppMessage::GeometryPrepared( second, app.Geometry( second).unwrap().Cancellation(), Ok( asset)));
    jeeves_assert!(ctx, app.Geometry(second).unwrap().Asset().is_none());
    jeeves_assert!(ctx, app.Geometry(second).unwrap().Error().is_some());
    let _ = app.update(AppMessage::OpenFile(PathBuf::from("first.pts")));
    jeeves_assert_eq!(ctx, app.tab_manager.active_tab().unwrap().id, first);
    jeeves_assert_eq!(
        ctx,
        app.Geometry(first).unwrap().Asset().unwrap().VertexCount(),
        2
    );
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, GeometryCameraFitAndNavigation, |ctx| {
    use crate::fascia::camera::ViewCamera;
    use glam::{Mat4, Vec3};
    let mut camera = ViewCamera::default();
    [0.3f32, 1.0, 3.0].iter().copied().for_each(|aspect| {
        camera.Fit(aspect);
        let matrix = Mat4::from_cols_array(&camera.Matrix(aspect));
        [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z]
            .iter()
            .for_each(|p| {
                let clip = matrix.project_point3(*p);
                jeeves_assert!(
                    ctx,
                    clip.x.abs() < 1.0 && clip.y.abs() < 1.0 && clip.z > 0.0 && clip.z < 1.0
                );
            });
    });
    let before = camera.Matrix(1.0);
    camera.Orbit(40.0, 20.0);
    camera.Pan(10.0, 5.0, 600.0);
    camera.Zoom(4.0);
    camera.ToggleProjection();
    jeeves_assert!(ctx, before != camera.Matrix(1.0));
    jeeves_assert!(ctx, camera.Matrix(1.0).iter().all(|v| v.is_finite()));
    camera.Zoom(10000.0);
    jeeves_assert!(ctx, camera.Matrix(1.0).iter().all(|v| v.is_finite()));
});

//-------------------------------------------------------------------------------------------------
// Theme palettes and font selection
jeeves_test!(Fascia, ThemePaletteVariants, |ctx| {
    jeeves_assert!( ctx, !FasciaTheme::default().is_dark());
    for &theme in FasciaTheme::ALL {
        let palette = theme.palette();
        if theme.is_dark() {
            jeeves_assert!(ctx, palette.text_primary.r > 0.5);
        } else {
            jeeves_assert!(ctx, palette.text_primary.r < 0.5);
        }
        let _sys_font = default_system_font(theme);
        let _code_font = default_code_font(theme);
    }
    let default_theme = FasciaTheme::default();
    jeeves_assert_eq!(ctx, default_theme, FasciaTheme::native_default());
    #[cfg(target_os = "windows")]
    jeeves_assert_eq!(ctx, default_theme, FasciaTheme::WindowsLight);
    #[cfg(not(target_os = "windows"))]
    jeeves_assert_eq!(ctx, default_theme, FasciaTheme::LinuxLight);
});

//-------------------------------------------------------------------------------------------------
// Console test
jeeves_test!(Fascia, Console, Console, |ctx| {
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
    let roots = detect_system_roots();
    jeeves_println!(ctx, "           System Drive Roots  : {:?}", roots);
    let status = StatusBarInfo::default();
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
jeeves_test!(Fascia, Example, Example, |ctx| {
    // 1. Initialize Tab Manager with Welcome and a file
    let mut tab_manager = TabManager::new();
    let file_path = PathBuf::from("src/lib.rs");
    tab_manager.open_file(file_path);
    jeeves_assert_eq!(ctx, tab_manager.tabs().Len(), 2);
    jeeves_assert_eq!(ctx, tab_manager.active_index(), Some(1));
    // 2. Setup Explorer
    let explorer = ExplorerState::new(PathBuf::from("."));
    jeeves_assert!(ctx, explorer.root_node.is_expanded);
    // 3. Setup Theme and Status
    let theme = FasciaTheme::WindowsDark;
    let palette = theme.palette();
    let status = StatusBarInfo {
        branch: "master".to_string(),
        message: "Framework Ready".to_string(),
        line: 42,
        column: 10,
        encoding: "UTF-8",
        indentation: "Spaces: 4",
        language: "Rust".to_string(),
    };
    jeeves_assert_eq!(ctx, theme.is_dark(), true);
    jeeves_assert_eq!(ctx, status.line, 42);
    jeeves_assert_eq!(ctx, status.column, 10);
    jeeves_assert_eq!(ctx, palette.corner_radius, 5.0);
    // If invoked specifically via -e (example mode), launch the interactive GUI app
    let has_e_flag = std::env::args().any(|a| a == "-e");
    let is_headless = std::env::var("TRELLIS_HEADLESS").as_deref() == Ok("1");
    if has_e_flag && !is_headless {
        jeeves_println!(
            ctx,
            "         [Launching Fascia Interactive Desktop GUI Shell...]"
        );
        if let Err(err) = crate::fascia::run_app() {
            jeeves_println!(ctx, "         [Error launching Fascia GUI: {:?}]", err);
        }
    }
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, CaskViewWidgetRender, |ctx| {
    use crate::fascia::cask_view::{CaskRenderer, view_cask_root};
    use crate::fenst::cask::{Cask, Color, LayoutDirection, Padding, Sizing};
    use iced::Length;

    let root = Cask::NewWindow("view_test")
        .WithDirection(LayoutDirection::TopToBottom)
        .WithSize(Sizing::Fixed(300.0), Sizing::Fixed(200.0))
        .WithPadding(Padding::All(8.0))
        .WithBackground(Color::Hex(0x282A36))
        .WithBorder(Color::Hex(0x6272A4), 1.0)
        .WithChild(
            Cask::NewLabel("lbl", "Fascia Cask Canvas View")
                .WithTextColor(Color::WHITE)
                .WithFontSize(14.0)
        );

    let renderer = CaskRenderer::FromRoot(&root, 0.0, 0.0);
    jeeves_assert_eq!(ctx, renderer.Commands().len(), 3);

    let _element: iced::Element<'_, ()> = view_cask_root(&root, 0.0, 0.0, Length::Fill, Length::Fill);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, ExplorerContextMenuAndCaskAction, |ctx| {
    use crate::fascia::explorer::{ExplorerAction, ExplorerState, view_explorer};
    use crate::fascia::theme::FasciaTheme;
    use std::path::PathBuf;

    let initial_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut explorer = ExplorerState::new(initial_dir.clone());

    // Context menu initially None
    jeeves_assert_eq!(ctx, explorer.context_menu, None);

    // Right-click opens context menu on targeted path
    let target = initial_dir.join("Cargo.toml");
    explorer.open_context_menu(target.clone());
    jeeves_assert_eq!(ctx, explorer.context_menu, Some(target.clone()));

    // Rendering explorer with open context menu produces valid UI
    {
        let palette = FasciaTheme::WindowsDark.palette();
        let _ui: iced::Element<'_, ExplorerAction> = view_explorer(&explorer, palette, |act| act);
    }

    // Dismissing context menu resets state
    explorer.dismiss_context_menu();
    jeeves_assert_eq!(ctx, explorer.context_menu, None);

    // Toggle path also dismisses context menu
    explorer.open_context_menu(target.clone());
    explorer.toggle_path(&initial_dir);
    jeeves_assert_eq!(ctx, explorer.context_menu, None);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, CaskViewerTabAndState, |ctx| {
    use crate::fascia::geometry_view::{GeometryAction, GeometryViewerState, ViewGeometry};
    use crate::fascia::tabs::{TabKind, TabManager};
    use crate::fascia::theme::FasciaTheme;
    use std::path::PathBuf;

    let path = PathBuf::from("Cargo.toml");
    let mut manager = TabManager::new();

    // Open Cask viewer tab
    let (idx, is_new, id) = manager.open_cask(path.clone());
    jeeves_assert!(ctx, is_new);
    jeeves_assert_eq!(ctx, idx, 1); // After initial welcome tab

    let active_tab = manager.active_tab().unwrap();
    jeeves_assert_eq!(ctx, active_tab.kind, TabKind::CaskViewer);
    jeeves_assert_eq!(ctx, active_tab.icon, "📦");
    jeeves_assert!(ctx, active_tab.title.contains("Cask"));

    // Opening same path again focuses existing tab
    let (idx2, is_new2, id2) = manager.open_cask(path.clone());
    jeeves_assert!(ctx, !is_new2);
    jeeves_assert_eq!(ctx, idx, idx2);
    jeeves_assert_eq!(ctx, id, id2);

    let mut state = GeometryViewerState::default();
    let root = crate::fenst::cask::Cask::NewWindow( "root");
    let scene = crate::fenst::cask_scene::CaskScene::FromRoot( &root).unwrap();
    let asset = crate::fascia::cask_scene::Build( scene, &state.Cancellation()).unwrap();
    state.Complete( Ok( std::sync::Arc::new( asset)));
    jeeves_assert_eq!( ctx, state.RootDepth(), 1);
    jeeves_assert_eq!( ctx, state.MaxDepth(), 1);
    let palette = FasciaTheme::WindowsDark.palette();
    let _viewer: iced::Element<'_, GeometryAction> = ViewGeometry( id.0, &state, palette, |act| act);
});


//-------------------------------------------------------------------------------------------------
jeeves_test!( Fascia, CaskSampledMeshAndSharedControls, |ctx| {
    use crate::fascia::{
        cask_scene,
        geometry_view::{GeometryAction, GeometryViewerState, PointColor},
    };
    use crate::fenst::cask::Cask;
    use crate::fenst::cask_scene::CaskScene;
    use crate::silo::{IArr, USeg};
    use crate::swarm::viewport::RenderMode;
    use std::sync::{Arc, atomic::AtomicBool};
    let mut root = Cask::NewWindow( "leaf");
    USeg::FromLen( 6).Traverse( |i| {
        root = Cask::NewWindow( format!( "ancestor {i}")).WithChild( std::mem::replace( &mut root, Cask::NewWindow( "unused")));
    });
    let mut scene = CaskScene::FromRoot( &root).unwrap();
    scene.Layout( &AtomicBool::new( false), |_| [20.0, 20.0]).unwrap();
    let asset = cask_scene::Mesh( &scene, &AtomicBool::new( false)).unwrap();
    jeeves_assert_eq!( ctx, asset.MaxDepth(), 7);
    jeeves_assert_eq!( ctx,
                      asset.DrawCounts( 7),
                      [asset.VertexCount(),
                       asset.Triangles().Size(),
                       asset.Edges().Size()]);
    let parentStart = asset.DrawCounts( 5);
    let leafStart = asset.DrawCounts( 6);
    let last = asset.DrawCounts( 7);
    // Surfaces and outlines use 1 segment per cuboid edge (12 triangles, 12 edges).
    // Eight mesh corners and 26 independent point samples per box, at every depth.
    jeeves_assert_eq!( ctx, leafStart[0] - parentStart[0], 8);
    jeeves_assert_eq!( ctx, leafStart[1] - parentStart[1], 12);
    jeeves_assert_eq!( ctx, leafStart[2] - parentStart[2], 12);
    jeeves_assert_eq!( ctx, last[0] - leafStart[0], 8);
    jeeves_assert_eq!( ctx, last[1] - leafStart[1], 12);
    jeeves_assert_eq!( ctx, last[2] - leafStart[2], 12);
    jeeves_assert_eq!( ctx, asset.PointCount(), 7 * 26);
    jeeves_assert_eq!( ctx, asset.PointDrawCount( 0), 26);
    jeeves_assert_eq!( ctx, asset.PointDrawCount( 6) - asset.PointDrawCount( 5), 26);
    jeeves_assert_eq!( ctx, asset.PointDrawCount( u32::MAX), 7 * 26);
    let mut valid = true;
    asset.Triangles().Traverse( |t| {
                         valid &= t[0] < asset.VertexCount()
                                  && t[1] < asset.VertexCount()
                                  && t[2] < asset.VertexCount();
                         valid &= t[0] != t[1] && t[1] != t[2] && t[0] != t[2];
                     });
    jeeves_assert!( ctx, valid);
    let mut state = GeometryViewerState::default();
    let asset = Arc::new( asset);
    state.Complete( Ok( asset.clone()));
    state.Update( GeometryAction::Resize( 800.0, 600.0));
    jeeves_assert_eq!( ctx, state.RootDepth(), 7);
    jeeves_assert_eq!( ctx, state.MaxDepth(), 4);
    state.Update( GeometryAction::Orbit( 0.2, 0.1));
    let camera = state.CameraMatrix();
    USeg::FromLen( 201).Traverse( |i| {
                          state.Update( GeometryAction::Opacity( i as f32 / 200.0));
                          jeeves_assert_eq!( ctx, state.Opacity(), i as f32 / 200.0);
                      });
    state.Update( GeometryAction::MaxDepth( u32::MAX));
    jeeves_assert_eq!( ctx, state.MaxDepth(), 7);
    jeeves_assert_eq!( ctx, state.CameraMatrix(), camera);
    jeeves_assert!( ctx, std::ptr::eq( state.Asset().unwrap(), asset.as_ref()));
    state.Update( GeometryAction::MaxDepth( 0));
    jeeves_assert_eq!( ctx, state.MaxDepth(), 1);
    state.Update( GeometryAction::Mode( RenderMode::Points));
    state.Update( GeometryAction::Color( PointColor::Height));
    state.Update( GeometryAction::PointSize( 8.0));
    state.Update( GeometryAction::Pan( 5.0, 8.0));
    state.Update( GeometryAction::Zoom( 1.0));
    state.Update( GeometryAction::Projection);
    state.Update( GeometryAction::Axis( 2));
    let refreshedCamera = state.CameraMatrix();
    let oldToken = state.Cancellation();
    state.BeginReload();
    jeeves_assert!( ctx, oldToken.load( std::sync::atomic::Ordering::Acquire));
    state.Complete( Ok( asset.clone()));
    jeeves_assert_eq!( ctx, state.CameraMatrix(), refreshedCamera);
    jeeves_assert_eq!( ctx, state.MaxDepth(), 1);
    let labelled = cask_scene::Build( CaskScene::FromRoot( &root).unwrap(), &AtomicBool::new( false)).unwrap();
    let labels = labelled.Labels().unwrap();
    jeeves_assert_eq!( ctx, labels.DrawCount( 7), 42);
    jeeves_assert_eq!( ctx, labels.DrawCount( 4), 24);
    let mut ink = false;
    labels.Pixels().Traverse( |pixel| ink |= *pixel != 0);
    jeeves_assert!( ctx, ink);
    jeeves_assert!( ctx,
                   labels.Vertices()[1].Position()[0] > labels.Vertices()[0].Position()[0]);
    jeeves_assert_eq!( ctx,
                      labels.Vertices()[1].Position()[1],
                      labels.Vertices()[0].Position()[1]);
});

//-------------------------------------------------------------------------------------------------
jeeves_test!( Fascia, GeometryRefreshRejectsOldGeneration, |ctx| {
    use crate::fascia::geometry_view::GeometryAction;
    use crate::fenst::geometry::LoadGeometry;
    use std::sync::Arc;
    let mut app = AppState::new();
    let _ = app.update( AppMessage::OpenFile( PathBuf::from( "refresh.pts")));
    let id = app.tab_manager.active_tab().unwrap().id;
    let oldToken = app.Geometry( id).unwrap().Cancellation();
    let _ = app.update( AppMessage::Geometry( id, GeometryAction::Refresh));
    let asset = Arc::new( LoadGeometry( "pts", "0 0 0\n").unwrap());
    let _ = app.update( AppMessage::GeometryPrepared( id, oldToken, Ok( asset.clone())));
    jeeves_assert!( ctx, app.Geometry( id).unwrap().Asset().is_none());
    let current = app.Geometry( id).unwrap().Cancellation();
    let _ = app.update( AppMessage::GeometryPrepared( id, current, Ok( asset)));
    jeeves_assert!( ctx, app.Geometry( id).unwrap().Asset().is_some());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fascia, GeometryRefreshFailurePreservesScene, |ctx| {
    use crate::fascia::geometry_view::{GeometryAction, GeometryViewerState, ViewGeometry};
    use crate::fenst::geometry::LoadGeometry;
    use std::sync::{Arc, atomic::Ordering};
    let asset = Arc::new( LoadGeometry( "pts", "0 0 0\n").unwrap());
    let mut state = GeometryViewerState::default();
    state.Complete( Ok( asset.clone()));
    state.Update( GeometryAction::Resize( 800.0, 600.0));
    state.Update( GeometryAction::Orbit( 12.0, 8.0));
    state.Update( GeometryAction::Opacity( 0.4));
    let camera = state.CameraMatrix();
    state.BeginReload();
    jeeves_assert!( ctx, state.IsLoading());
    state.Complete( Err( "Directory is no longer readable.".into()));
    jeeves_assert!( ctx, state.Error().is_none());
    jeeves_assert_eq!( ctx, state.ReloadError(), Some( "Directory is no longer readable."));
    jeeves_assert!( ctx, !state.IsLoading());
    jeeves_assert!( ctx, std::ptr::eq( state.Asset().unwrap(), asset.as_ref()));
    jeeves_assert_eq!( ctx, state.CameraMatrix(), camera);
    jeeves_assert_eq!( ctx, state.Opacity(), 0.4);
    let _: iced::Element<'_, GeometryAction> = ViewGeometry( 1, &state,
        FasciaTheme::WindowsLight.palette(), |action| action);
    let token = state.BeginReload();
    jeeves_assert!( ctx, state.ReloadError().is_none());
    state.Update( GeometryAction::Cancel);
    jeeves_assert!( ctx, token.load( Ordering::Acquire));
    jeeves_assert!( ctx, state.Error().is_none());
    jeeves_assert!( ctx, state.ReloadError().unwrap().contains( "Refresh cancelled"));
    state.Complete( Err( "Stale failure".into()));
    jeeves_assert!( ctx, !state.ReloadError().unwrap().contains( "Stale"));
    state.BeginReload();
    state.Complete( Ok( asset));
    jeeves_assert!( ctx, state.ReloadError().is_none());
    jeeves_assert_eq!( ctx, state.CameraMatrix(), camera);
    jeeves_assert_eq!( ctx, state.Opacity(), 0.4);
    state.Update( GeometryAction::GpuError( "GPU unavailable".into()));
    jeeves_assert_eq!( ctx, state.Error(), Some( "GPU unavailable"));
    let mut initial = GeometryViewerState::default();
    initial.Complete( Err( "File missing".into()));
    jeeves_assert_eq!( ctx, initial.Error(), Some( "File missing"));
    jeeves_assert!( ctx, initial.ReloadError().is_none());
    initial.BeginReload();
    initial.Update( GeometryAction::Cancel);
    jeeves_assert!( ctx, initial.Error().unwrap().contains( "Retry"));
});

//-------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fascia, CaskBoundedConstruction, |ctx| {
    use crate::fascia::cask_scene;
    use crate::fenst::cask_scene::{CaskScene, ICaskHierarchy};
    use crate::silo::USeg;
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;

    struct Hierarchy
    {
        _Nodes: u32,
        _Chain: bool,
    }

    impl ICaskHierarchy for Hierarchy
    {
        type Node = u32;

        fn Label( &self, node: u32) -> String
        {
            format!( "node {node}")
        }

        fn SpanChildren( &self, node: u32, mut visit: impl FnMut( u32) -> bool)
        {
            if self._Chain
            {
                if node + 1 < self._Nodes
                {
                    visit( node + 1);
                }
            }
            else if node == 0
            {
                USeg::WithLen( 1, self._Nodes - 1).Span( visit);
            }
        }
    }

    let cancelled = AtomicBool::new( false);
    USeg::FromLen( 2).Traverse( |case| {
        let source = Hierarchy { _Nodes: if case == 0 { 1000 } else { 10_000 },
                                 _Chain: case == 0 };
        let started = Instant::now();
        let mut scene = CaskScene::FromHierarchy( &source, 0, &cancelled).unwrap();
        scene.Layout( &cancelled, |_| [20.0, 20.0]).unwrap();
        let asset = cask_scene::Mesh( &scene, &cancelled).unwrap();
        jeeves_assert_eq!( ctx, asset.VertexCount(), source._Nodes * 8);
        jeeves_assert_eq!( ctx, asset.PointCount(), source._Nodes * 26);
        jeeves_assert_eq!( ctx, asset.FaceCount(), source._Nodes * 12);
        jeeves_assert_eq!( ctx, asset.Edges().Size(), source._Nodes * 12);
        jeeves_assert_eq!( ctx, asset.PointDrawCount( 1), 26);
        jeeves_assert_eq!( ctx, asset.PointDrawCount( u32::MAX), source._Nodes * 26);
        jeeves_assert_eq!( ctx, asset.MaxDepth(), if source._Chain { source._Nodes } else { 2 });
        // Every generated triangle remains outward-facing after normalization.
        let mut outward = true;
        asset.Triangles().USeg().Traverse( |index| {
            let face = asset.Triangles()[index];
            let a = glam::Vec3::from_array( asset.Vertices()[face[0]].Position());
            let b = glam::Vec3::from_array( asset.Vertices()[face[1]].Position());
            let c = glam::Vec3::from_array( asset.Vertices()[face[2]].Position());
            let base = index / 12 * 8;
            let center = ( glam::Vec3::from_array( asset.Vertices()[base].Position())
                           + glam::Vec3::from_array( asset.Vertices()[base + 7].Position())) * 0.5;
            outward &= ( b - a).cross( c - a).dot( a - center) > 0.0;
        });
        jeeves_assert!( ctx, outward);
        jeeves_assert!( ctx, cask_scene::Mesh( &scene, &AtomicBool::new( true)).is_err());
        jeeves_println!( ctx, "{}: {} nodes, {} surface vertices, {} samples, {:?}",
                        if source._Chain { "Deep" } else { "Wide" }, source._Nodes,
                        asset.VertexCount(), asset.PointCount(), started.elapsed());
    });
});

jeeves_test!( Fascia, CaskConstructionPreflight, |ctx| {
    use crate::fascia::cask_scene;
    use crate::fenst::{cask::Cask, cask_scene::CaskScene};
    use std::sync::atomic::AtomicBool;
    let cancelled = AtomicBool::new( false);
    let root = Cask::NewWindow( "Unmeasured");
    let scene = CaskScene::FromRoot( &root).unwrap();
    jeeves_assert!( ctx, cask_scene::Mesh( &scene, &cancelled).is_err());
    let root = Cask::NewWindow( "W".repeat( 1000));
    let scene = CaskScene::FromRoot( &root).unwrap();
    jeeves_assert!( ctx, cask_scene::Build( scene, &cancelled).unwrap_err().contains( "atlas"));
    let scene = CaskScene::FromRoot( &root).unwrap();
    jeeves_assert!( ctx, cask_scene::Build( scene, &AtomicBool::new( true)).is_err());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fascia, CaskSceneViewBoxMeshAdaptation, |ctx| {
    use crate::fascia::{camera::ViewCamera, cask_scene};
    use crate::fenst::{cask::Cask, cask_scene::CaskScene};
    use crate::silo::USeg;
    use std::sync::atomic::AtomicBool;

    let mut branch = Cask::NewWindow( "deep_leaf");
    USeg::FromLen( 4).Traverse( |i| {
        branch = Cask::NewWindow( format!( "node {i}")).WithChild( std::mem::replace( &mut branch, Cask::NewWindow( "unused")));
    });
    let root = Cask::NewWindow( "root")
        .WithChild( branch)
        .WithChild( Cask::NewWindow( "sibling"));

    let full_scene = CaskScene::FromRoot( &root).unwrap();
    let full_asset = cask_scene::Build( full_scene, &AtomicBool::new( false)).unwrap();
    jeeves_assert_eq!( ctx, full_asset.MaxDepth(), 6);

    let camera = ViewCamera::default();
    let vb = camera.ViewBox( [800.0, 600.0], full_asset.Bounds())
        .WithMinPixelThreshold( 20.0);

    let scene_for_vb = CaskScene::FromRoot( &root).unwrap();
    let adapted = cask_scene::BuildWithViewBox( scene_for_vb, &vb, &AtomicBool::new( false)).unwrap();

    jeeves_assert!( ctx, adapted.VertexCount() <= full_asset.VertexCount());
    jeeves_assert!( ctx, adapted.FaceCount() <= full_asset.FaceCount());
    jeeves_assert!( ctx, adapted.MaxDepth() <= full_asset.MaxDepth());
    jeeves_assert_eq!( ctx, adapted.Bounds(), full_asset.Bounds());

    let v_scale = camera.ViewScale( 600.0, full_asset.Bounds());
    jeeves_assert!( ctx, v_scale.is_finite() && v_scale > 0.0);
    jeeves_assert_eq!( ctx, vb.Depth(), 100.0 * vb.MaxViewDimension());

    // 2. Test scale along depth heuristic:
    // If worldview scale along far depth falls below pixel threshold, far depth is uniformly chosen.
    let high_thresh_scale = camera.ViewScaleWithThreshold( 600.0, full_asset.Bounds(), 1000.0);
    jeeves_assert!( ctx, high_thresh_scale >= v_scale,
                    "Scale taking worldview below threshold along far depth must uniformly assume far depth");

    // 3. Test GPU memory threshold capping on 3D Cask mesh generation:
    let capped_vb = camera.ViewBoxWithThreshold( [800.0, 600.0], full_asset.Bounds(), 0.0, 3 * crate::fenst::cask_scene::ViewBox::GPU_BYTES_PER_NODE);
    let scene_capped = CaskScene::FromRoot( &root).unwrap();
    let capped_asset = cask_scene::BuildWithViewBox( scene_capped, &capped_vb, &AtomicBool::new( false)).unwrap();
    jeeves_assert!( ctx, capped_asset.VertexCount() <= 3 * 8,
                    "GPU memory threshold must bound generated mesh vertices");
    jeeves_assert!( ctx, capped_asset.VertexCount() < full_asset.VertexCount());
    return;
});

jeeves_test!( Fascia, PythonConsoleLifecycleAndTabs, |ctx| {
    #[cfg(feature = "python")]
    use crate::fascia::python_console::ConsoleExecutionResult;
    use crate::fascia::python_console::PythonConsoleAction;

    // 1. Test TabManager open_python_console
    let mut tm = TabManager::new();
    let (idx1, is_new1, id1) = tm.open_python_console();
    jeeves_assert!( ctx, is_new1);
    jeeves_assert_eq!( ctx, tm.active_tab().unwrap().kind, TabKind::PythonConsole);
    jeeves_assert_eq!( ctx, tm.active_tab().unwrap().icon, "🐍");

    // Idempotent: opening again selects existing tab
    let (idx2, is_new2, id2) = tm.open_python_console();
    jeeves_assert!( ctx, !is_new2);
    jeeves_assert_eq!( ctx, idx1, idx2);
    jeeves_assert_eq!( ctx, id1, id2);

    // 2. Test AppState message handling
    let mut app = AppState::new();
    let _ = app.update( AppMessage::OpenPythonConsole);
    jeeves_assert_eq!( ctx, app.tab_manager.active_tab().unwrap().kind, TabKind::PythonConsole);
    let tab_id = app.tab_manager.active_tab().unwrap().id;
    jeeves_assert!( ctx, app.Console(tab_id).is_some());

    // Input changed
    let _ = app.update( AppMessage::PythonConsole( tab_id, PythonConsoleAction::InputChanged( "2 + 3".to_string())));
    jeeves_assert_eq!( ctx, app.Console(tab_id).unwrap().input_line, "2 + 3");

    // Submit
    let _ = app.update( AppMessage::PythonConsole( tab_id, PythonConsoleAction::Submit));
    #[cfg(feature = "python")]
    {
        jeeves_assert!( ctx, app.Console(tab_id).unwrap().is_running);

        // Execution result arrives
        let result = ConsoleExecutionResult {
            output: "5\n".to_string(),
            is_error: false,
            needs_more_input: false,
        };
        let _ = app.update( AppMessage::PythonConsoleExecuted( tab_id, result));
        let console = app.Console(tab_id).unwrap();
        jeeves_assert!( ctx, !console.is_running);
        jeeves_assert_eq!( ctx, console.history.len(), 2);
        jeeves_assert_eq!( ctx, console.history[1].input, "2 + 3");
        jeeves_assert_eq!( ctx, console.history[1].output, "5\n");
        jeeves_assert!( ctx, !console.history[1].is_error);
    }
    #[cfg(not(feature = "python"))]
    {
        let console = app.Console(tab_id).unwrap();
        jeeves_assert!( ctx, !console.is_running);
        jeeves_assert_eq!( ctx, console.history.len(), 2);
        jeeves_assert_eq!( ctx, console.history[1].input, "2 + 3");
        jeeves_assert!( ctx, console.history[1].output.contains( "Python support is not enabled"));
        jeeves_assert!( ctx, console.history[1].is_error);
    }

    // Close tab cleans up console state
    let active_idx = app.tab_manager.active_index().unwrap();
    let _ = app.update( AppMessage::CloseTab( active_idx));
    jeeves_assert!( ctx, app.Console(tab_id).is_none());
    return;
});

jeeves_test!( Fascia, Cask2dCanvasRenderer, |ctx| {
    use crate::fenst::cask2d::{Cask2dLayout, Cask2dOptions};
    use crate::fascia::cask_view::view_cask2d;
    use iced::Length;

    let opts = Cask2dOptions::New().WithMaxDepth(2);
    let layout = Cask2dLayout::FromUri("settings://", &opts).expect("settings layout");
    let _element: iced::Element<'_, ()> = view_cask2d(&layout, Length::Fill, Length::Fill);
    jeeves_assert!(ctx, layout.NodeCount() > 0);
});

jeeves_test!( Fascia, Cask2dTabManagerAndAppIntegration, |ctx| {
    use crate::fascia::app::{AppMessage, AppState};
    use crate::fascia::cask_view::{view_cask2d_tab, Cask2dAction};
    use crate::fascia::explorer::ExplorerAction;
    use crate::fascia::tabs::TabKind;
    use crate::fenst::cask2d::Cask2dLayoutMode;
    use std::path::PathBuf;

    let mut app = AppState::new();

    // 1. Open Cask 2D via explorer action
    let _ = app.update(AppMessage::Explorer(ExplorerAction::OpenCask2d(PathBuf::from("settings://"))));
    let tab = app.tab_manager.active_tab().expect("active tab");
    jeeves_assert_eq!(ctx, tab.kind, TabKind::Cask2dViewer);
    jeeves_assert_eq!(ctx, app.status_info.language, "Cask 2D");

    let tab_id = tab.id;
    jeeves_assert!(ctx, app.Cask2d(tab_id).is_some());

    // 2. Interact with Cask 2D viewer state
    let palette = app.palette();
    {
        let viewer = app.Cask2dMut(tab_id).unwrap();
        jeeves_assert!(ctx, viewer.layout.is_some());
        let initial_nodes = viewer.layout.as_ref().unwrap().NodeCount();
        jeeves_assert!(ctx, initial_nodes > 0);

        // Switch layout mode to HorizontalTree
        viewer.Update(Cask2dAction::Mode(Cask2dLayoutMode::HorizontalTree));
        jeeves_assert_eq!(ctx, viewer.mode, Cask2dLayoutMode::HorizontalTree);

        // Switch to VerticalTree
        viewer.Update(Cask2dAction::Mode(Cask2dLayoutMode::VerticalTree));
        jeeves_assert_eq!(ctx, viewer.mode, Cask2dLayoutMode::VerticalTree);

        // Switch to IndentedList
        viewer.Update(Cask2dAction::Mode(Cask2dLayoutMode::IndentedList));
        jeeves_assert_eq!(ctx, viewer.mode, Cask2dLayoutMode::IndentedList);

        // Change max depth
        viewer.Update(Cask2dAction::MaxDepth(2));
        jeeves_assert_eq!(ctx, viewer.max_depth, 2);

        // Test Zoom & Pan
        let initial_zoom = viewer.zoom;
        viewer.Update(Cask2dAction::Zoom(0.2));
        jeeves_assert!(ctx, viewer.zoom > initial_zoom);

        viewer.Update(Cask2dAction::Pan(10.0, -15.0));
        jeeves_assert_eq!(ctx, viewer.pan[0], 50.0);
        jeeves_assert_eq!(ctx, viewer.pan[1], 25.0);

        viewer.Update(Cask2dAction::ResetView);
        jeeves_assert_eq!(ctx, viewer.zoom, 1.0);
        jeeves_assert_eq!(ctx, viewer.pan, [40.0, 40.0]);

        // Fit view
        viewer.Update(Cask2dAction::Resize(1024.0, 768.0));
        viewer.Update(Cask2dAction::FitView);
        jeeves_assert!(ctx, viewer.zoom > 0.0);

        // Node selection
        viewer.Update(Cask2dAction::SelectNode(Some(0)));
        jeeves_assert_eq!(ctx, viewer.selected_idx, Some(0));
        jeeves_assert!(ctx, viewer.SelectedNode().is_some());

        // Pin & net interactive selection
        let pin = crate::fenst::circuit_layout::Cask2dPinNode::New(
            "ALU",
            "A",
            crate::fenst::PortDirection::In,
            crate::fenst::cask::Rect::New(10.0, 10.0, 16.0, 16.0),
            crate::fenst::cask::Point::New(18.0, 18.0),
            "clk_net",
        );
        viewer.Update(Cask2dAction::SelectPin(Some(pin.clone())));
        jeeves_assert!(ctx, viewer.SelectedPin().is_some());
        jeeves_assert_eq!(ctx, viewer.SelectedNet(), Some("clk_net"));
        jeeves_assert_eq!(ctx, viewer.selected_idx, None);

        // Pin & net hover preview
        viewer.Update(Cask2dAction::Hover(Some(pin), Some("clk_net".to_string())));
        jeeves_assert!(ctx, viewer.HoveredPin().is_some());
        jeeves_assert_eq!(ctx, viewer.HoveredNet(), Some("clk_net"));
        viewer.Update(Cask2dAction::Hover(None, None));
        jeeves_assert!(ctx, viewer.HoveredPin().is_none());
        jeeves_assert!(ctx, viewer.HoveredNet().is_none());

        // Net selection
        viewer.Update(Cask2dAction::SelectNet(Some("data_bus".to_string())));
        jeeves_assert!(ctx, viewer.SelectedPin().is_none());
        jeeves_assert_eq!(ctx, viewer.SelectedNet(), Some("data_bus"));

        // Selecting a node clears pin and net selection
        viewer.Update(Cask2dAction::SelectNode(Some(0)));
        jeeves_assert_eq!(ctx, viewer.selected_idx, Some(0));
        jeeves_assert!(ctx, viewer.SelectedPin().is_none());
        jeeves_assert!(ctx, viewer.SelectedNet().is_none());

        // Search query
        viewer.Update(Cask2dAction::Search("theme".to_string()));
        jeeves_assert_eq!(ctx, viewer.search_query, "theme");

        // Theme toggle
        let was_dark = viewer.dark_theme;
        viewer.Update(Cask2dAction::ToggleTheme);
        jeeves_assert_eq!(ctx, viewer.dark_theme, !was_dark);

        // Render tab element
        let _elem: iced::Element<'_, AppMessage> = view_cask2d_tab(
            tab_id,
            viewer,
            palette,
            move |act| AppMessage::Cask2d(tab_id, act),
        );
    }

    // 3. Dispatch Cask2d action through AppState::update
    let _ = app.update(AppMessage::Cask2d(tab_id, Cask2dAction::SelectNode(None)));
    jeeves_assert_eq!(ctx, app.Cask2d(tab_id).unwrap().selected_idx, None);

    // 4. Close tab and verify cleanup
    let active_idx = app.tab_manager.active_index().unwrap();
    let _ = app.update(AppMessage::CloseTab(active_idx));
    jeeves_assert!(ctx, app.Cask2d(tab_id).is_none());
});

//-------------------------------------------------------------------------------------------------
// Theme Menu Static Labels (Zero Allocation)
jeeves_test!( Fascia, ThemeMenuZeroAllocationStaticLabels, |ctx| {
    for &theme in FasciaTheme::ALL {
        let active = theme.menu_label(true);
        let inactive = theme.menu_label(false);
        jeeves_assert!(ctx, active.starts_with("✓ "));
        jeeves_assert!(ctx, inactive.starts_with("   "));
        jeeves_assert!(ctx, active.contains(theme.display_name()));
        jeeves_assert!(ctx, inactive.contains(theme.display_name()));
    }
});

//-------------------------------------------------------------------------------------------------
// Close All Tabs Entry Points Resource Disposal Parity
jeeves_test!( Fascia, CloseAllTabsEntryPointsResourceDisposalParity, |ctx| {
    let mut app_menu = AppState::new();
    let mut app_bar = AppState::new();

    let _ = app_menu.update(AppMessage::NewFile);
    let _ = app_menu.update(AppMessage::NewFile);
    let _ = app_menu.update(AppMessage::OpenPythonConsole);

    let _ = app_bar.update(AppMessage::NewFile);
    let _ = app_bar.update(AppMessage::NewFile);
    let _ = app_bar.update(AppMessage::OpenPythonConsole);

    jeeves_assert_eq!(ctx, app_menu.tab_manager.tabs().Len(), 4);
    jeeves_assert_eq!(ctx, app_bar.tab_manager.tabs().Len(), 4);

    // Close all via Menu on app_menu
    let _ = app_menu.update(AppMessage::Menu(MenuAction::CloseAllTabs));

    // Close all via TabBar on app_bar
    let _ = app_bar.update(AppMessage::TabBar(TabBarAction::CloseAll));

    // Both must dispose all resources identically
    jeeves_assert_eq!(ctx, app_menu.tab_manager.tabs().Len(), 0);
    jeeves_assert_eq!(ctx, app_bar.tab_manager.tabs().Len(), 0);
    jeeves_assert_eq!(ctx, app_menu.EditorCount(), 0);
    jeeves_assert_eq!(ctx, app_bar.EditorCount(), 0);
    jeeves_assert_eq!(ctx, app_menu.ConsoleCount(), 0);
    jeeves_assert_eq!(ctx, app_bar.ConsoleCount(), 0);
    jeeves_assert_eq!(ctx, app_menu.WaveformCount(), 0);
    jeeves_assert_eq!(ctx, app_bar.WaveformCount(), 0);
    jeeves_assert_eq!(ctx, app_menu.Cask2dCount(), 0);
    jeeves_assert_eq!(ctx, app_bar.Cask2dCount(), 0);
    jeeves_assert_eq!(ctx, app_menu.status_info.message, "All tabs closed");
    jeeves_assert_eq!(ctx, app_bar.status_info.message, "All tabs closed");
});

//-------------------------------------------------------------------------------------------------
// Cask2D Hit Test Reverse Draw Order (Nested Child Wins)
jeeves_test!( Fascia, Cask2dHitTestReverseDrawOrderNestedChildWins, |ctx| {
    use crate::fenst::cask::{Point, Rect};
    use crate::fenst::cask2d::{Cask2dLayout, Cask2dNode};

    let parent = Cask2dNode {
        id: "0".to_string(),
        name: "parent_module".to_string(),
        path: "root".to_string(),
        bounds: Rect::New(0.0, 0.0, 200.0, 200.0),
        size: 0,
        depth: 0,
        parent_index: None,
        children_indices: vec![1],
        is_leaf: false,
        extension: String::new(),
    };
    let child = Cask2dNode {
        id: "1".to_string(),
        name: "child_node".to_string(),
        path: "root/child".to_string(),
        bounds: Rect::New(50.0, 50.0, 50.0, 50.0),
        size: 0,
        depth: 1,
        parent_index: Some(0),
        children_indices: vec![],
        is_leaf: true,
        extension: String::new(),
    };
    let layout = Cask2dLayout {
        bounds: Rect::New(0.0, 0.0, 200.0, 200.0),
        nodes: vec![parent, child],
        pins: Vec::new(),
        wires: Vec::new(),
        commands: vec![],
        cask_tree: None,
    };

    // Point inside child (60, 60): hits both parent and child bounds.
    // Reverse draw order must return child (index 1), not parent (index 0).
    let hit_child = layout.FindNodeIndexAt(Point::New(60.0, 60.0));
    jeeves_assert_eq!(ctx, hit_child, Some(1));

    // Point outside child but inside parent (10, 10): hits parent only.
    let hit_parent = layout.FindNodeIndexAt(Point::New(10.0, 10.0));
    jeeves_assert_eq!(ctx, hit_parent, Some(0));

    // Point outside both: hits None.
    let hit_none = layout.FindNodeIndexAt(Point::New(300.0, 300.0));
    jeeves_assert_eq!(ctx, hit_none, None);
});

//-------------------------------------------------------------------------------------------------
// Editor Actual Cursor Position Reporting
jeeves_test!( Fascia, EditorActualCursorPositionReporting, |ctx| {
    use iced::widget::text_editor;

    let mut app = AppState::new();
    let _ = app.update(AppMessage::NewFile);

    // Default cursor is line 1, col 1
    jeeves_assert_eq!(ctx, app.status_info.line, 1);
    jeeves_assert_eq!(ctx, app.status_info.column, 1);

    // Type two characters
    let _ = app.update(AppMessage::EditorAction(text_editor::Action::Edit(text_editor::Edit::Insert('A'))));
    let _ = app.update(AppMessage::EditorAction(text_editor::Action::Edit(text_editor::Edit::Insert('B'))));
    jeeves_assert_eq!(ctx, app.status_info.line, 1);
    jeeves_assert_eq!(ctx, app.status_info.column, 3);

    // Press Enter to go to line 2
    let _ = app.update(AppMessage::EditorAction(text_editor::Action::Edit(text_editor::Edit::Enter)));
    jeeves_assert_eq!(ctx, app.status_info.line, 2);
    jeeves_assert_eq!(ctx, app.status_info.column, 1);
});

//-------------------------------------------------------------------------------------------------
// Cask 2D/3D Theme Palette Adherence
jeeves_test!( Fascia, Cask2dAnd3dThemePaletteAdherence, |ctx| {
    use crate::fascia::app::{AppMessage, AppState};
    use crate::fascia::cask_scene::{self, PaletteNodeColor};
    use crate::fascia::cask_view::{cask2d_scheme_from_palette, Cask2dViewerState};
    use crate::fascia::explorer::ExplorerAction;
    use crate::fascia::theme::FasciaTheme;
    use crate::fenst::cask::Cask;
    use crate::fenst::cask_scene::CaskScene;
    use std::sync::atomic::AtomicBool;

    let light_theme = FasciaTheme::WindowsLight;
    let dark_theme = FasciaTheme::WindowsDark;
    let light_pal = light_theme.palette();
    let dark_pal = dark_theme.palette();

    // 1. Verify 2D color scheme derivation from theme palettes
    let light_scheme = cask2d_scheme_from_palette(&light_pal);
    let dark_scheme = cask2d_scheme_from_palette(&dark_pal);

    // Light scheme must have light backgrounds and dark text
    jeeves_assert!(ctx, light_scheme.background_root.r > 0.8);
    jeeves_assert!(ctx, light_scheme.background_branch.r > 0.8);
    jeeves_assert!(ctx, light_scheme.text_primary.r < 0.3);

    // Dark scheme must have dark backgrounds and light text
    jeeves_assert!(ctx, dark_scheme.background_root.r < 0.3);
    jeeves_assert!(ctx, dark_scheme.background_branch.r < 0.3);
    jeeves_assert!(ctx, dark_scheme.text_primary.r > 0.8);

    // Depth tinting must not produce dark backgrounds in light mode
    let depth2_light = light_scheme.color_for_depth(2, false);
    jeeves_assert!(ctx, depth2_light.r > 0.7);

    // 2. Verify Cask 2D viewer state palette adherence and reactive update
    let mut viewer = Cask2dViewerState::NewWithPalette(std::path::PathBuf::from("settings://"), light_pal);
    jeeves_assert_eq!(ctx, viewer.dark_theme, false);
    jeeves_assert!(ctx, viewer.layout.is_some());

    viewer.SetPalette(dark_pal);
    jeeves_assert_eq!(ctx, viewer.dark_theme, true);
    jeeves_assert!(ctx, viewer.layout.is_some());

    // 3. Verify Cask 3D palette node colors adhere to theme palette
    let root_light_3d = PaletteNodeColor(1, 0, Some(&light_pal));
    let root_dark_3d = PaletteNodeColor(1, 0, Some(&dark_pal));
    jeeves_assert!(ctx, root_light_3d[0] > 0.8);
    jeeves_assert!(ctx, root_dark_3d[0] < 0.3);

    let module_light_3d = PaletteNodeColor(2, 0, Some(&light_pal));
    let module_dark_3d = PaletteNodeColor(2, 0, Some(&dark_pal));
    jeeves_assert!(ctx, module_light_3d[0] > 0.8);
    jeeves_assert!(ctx, module_dark_3d[0] < 0.3);

    // 4. Verify full 3D Cask mesh generation with palette
    let root = Cask::NewWindow("cask_root")
        .WithChild(Cask::NewWindow("module_a").WithChild(Cask::NewWindow("leaf_a")));
    let scene_light = CaskScene::FromRoot(&root).unwrap();
    let asset_light = cask_scene::BuildWithPalette(scene_light, Some(&light_pal), &AtomicBool::new(false)).unwrap();
    jeeves_assert_eq!(ctx, asset_light.VertexCount(), 24); // 3 cuboids * 8 vertices
    jeeves_assert!(ctx, asset_light.Vertices()[0].Color()[0] > 0.7);

    let scene_dark = CaskScene::FromRoot(&root).unwrap();
    let asset_dark = cask_scene::BuildWithPalette(scene_dark, Some(&dark_pal), &AtomicBool::new(false)).unwrap();
    jeeves_assert_eq!(ctx, asset_dark.VertexCount(), 24);
    jeeves_assert!(ctx, asset_dark.Vertices()[0].Color()[0] < 0.4);

    // 5. Verify AppState theme change updates 2D views and triggers 3D Cask reload
    let mut app = AppState::new();
    let _ = app.update(AppMessage::Explorer(ExplorerAction::OpenCask2d(std::path::PathBuf::from("settings://"))));
    let _ = app.update(AppMessage::SelectTheme(FasciaTheme::WindowsLight));
    app.Documents().Traverse(|(_, document)| {
        if let crate::fascia::TabDocument::Cask2d(viewer) = document {
            jeeves_assert_eq!(ctx, viewer.dark_theme, false);
        }
    });
    let _ = app.update(AppMessage::SelectTheme(FasciaTheme::WindowsDark));
    app.Documents().Traverse(|(_, document)| {
        if let crate::fascia::TabDocument::Cask2d(viewer) = document {
            jeeves_assert_eq!(ctx, viewer.dark_theme, true);
        }
    });
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, SketchToolControllerHeadlessFsmAndSnapping, |ctx| {
    use crate::fascia::sketch_tool::{
        KeyModifiers, PointerButton, SketchInputEvent, SketchToolController, SketchToolKind,
    };
    use crate::fenst::sketch::{SketchDocument, SketchPoint};

    let mut doc = SketchDocument::New();
    let mut ctrl = SketchToolController::New();
    ctrl.SetTool(SketchToolKind::Line);

    // 1. Line creation gesture
    ctrl.ProcessEvent(
        &doc,
        SketchInputEvent::PointerDown {
            pos:       SketchPoint::New(0.0, 0.0),
            button:    PointerButton::Left,
            modifiers: KeyModifiers::default(),
        },
    );
    ctrl.ProcessEvent(
        &doc,
        SketchInputEvent::PointerMove {
            pos:       SketchPoint::New(100.0, 0.0),
            modifiers: KeyModifiers::default(),
        },
    );
    jeeves_assert!(ctx, ctrl.DraftGeometry().is_some());

    ctrl.ProcessEvent(
        &doc,
        SketchInputEvent::PointerUp {
            pos:       SketchPoint::New(100.0, 0.0),
            button:    PointerButton::Left,
            modifiers: KeyModifiers::default(),
        },
    );
    let patch = ctrl.TakeCommittedPatch().expect("committed line patch");
    jeeves_assert_eq!(ctx, patch.Added().Len(), 1);

    // 2. Commit to doc and verify snapping hysteresis
    let added_ent = patch.Added().Get(0).unwrap().clone();
    let ent_id = added_ent.Id();
    doc.InsertEntity(added_ent);

    // Acquire snap within 8px
    let snapped = ctrl.EvaluateSnap(&doc, SketchPoint::New(5.0, 0.0));
    jeeves_assert_eq!(ctx, snapped._X, 0.0);
    jeeves_assert!(ctx, ctrl.ActiveSnap().is_some());

    // Retain snap via hysteresis at 10px (< 12px release threshold)
    let snapped2 = ctrl.EvaluateSnap(&doc, SketchPoint::New(10.0, 0.0));
    jeeves_assert_eq!(ctx, snapped2._X, 0.0);

    // Release snap when exceeding 12px
    let snapped3 = ctrl.EvaluateSnap(&doc, SketchPoint::New(15.0, 0.0));
    jeeves_assert_eq!(ctx, snapped3._X, 15.0);
    jeeves_assert!(ctx, ctrl.ActiveSnap().is_none());

    // 3. Selection & Grip reshaping
    ctrl.SetTool(SketchToolKind::Select);
    ctrl.SetSelectedEntityId(ent_id);
    let grips = ctrl.ActiveGrips(&doc);
    jeeves_assert_eq!(ctx, grips.Len(), 3); // Start, End, Midpoint

    // Drag End grip from (100, 0) to (120, 20)
    ctrl.ProcessEvent(
        &doc,
        SketchInputEvent::PointerDown {
            pos:       SketchPoint::New(100.0, 0.0),
            button:    PointerButton::Left,
            modifiers: KeyModifiers::default(),
        },
    );
    ctrl.ProcessEvent(
        &doc,
        SketchInputEvent::PointerMove {
            pos:       SketchPoint::New(120.0, 20.0),
            modifiers: KeyModifiers::default(),
        },
    );
    jeeves_assert!(ctx, ctrl.DraftGeometry().is_some());

    ctrl.ProcessEvent(
        &doc,
        SketchInputEvent::PointerUp {
            pos:       SketchPoint::New(120.0, 20.0),
            button:    PointerButton::Left,
            modifiers: KeyModifiers::default(),
        },
    );
    let reshape_patch = ctrl.TakeCommittedPatch().expect("committed reshape patch");
    jeeves_assert_eq!(ctx, reshape_patch.Replaced().Len(), 1);

    // 4. Cancellation resets draft
    ctrl.SetTool(SketchToolKind::Circle);
    ctrl.ProcessEvent(
        &doc,
        SketchInputEvent::PointerDown {
            pos:       SketchPoint::New(50.0, 50.0),
            button:    PointerButton::Left,
            modifiers: KeyModifiers::default(),
        },
    );
    jeeves_assert!(ctx, ctrl.DraftGeometry().is_some());

    ctrl.ProcessEvent(
        &doc,
        SketchInputEvent::KeyDown {
            key_escape: true,
            key_enter:  false,
            key_delete: false,
            modifiers:  KeyModifiers::default(),
        },
    );
    jeeves_assert!(ctx, ctrl.DraftGeometry().is_none());
    jeeves_assert!(ctx, ctrl.TakeCommittedPatch().is_none());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, NativeSketchDrawingSaveRestoreAndClose, |ctx| {
    use crate::fascia::app::{AppMessage, AppState, SketchCloseDecision};
    use crate::fascia::sketch_canvas::SketchCanvasAction;
    use crate::fascia::sketch_tool::{
        KeyModifiers, PointerButton, SketchInputEvent, SketchToolKind,
    };
    use crate::fascia::sketch_view::{SketchViewAction, SketchViewerState};
    use crate::fenst::sketch::{SketchPoint, SketchPrimitive};
    use crate::fleck::sketch_flux::SketchFlux;
    use crate::silo::{Arr, IArr, USeg};

    fn Click(viewer: &mut SketchViewerState, point: SketchPoint)
    {
        viewer.ProcessAction(SketchViewAction::CanvasAction(SketchCanvasAction::Input(SketchInputEvent::PointerDown { pos: point, button: PointerButton::Left, modifiers: KeyModifiers::default() })));
        viewer.ProcessAction(SketchViewAction::CanvasAction(SketchCanvasAction::Input(SketchInputEvent::PointerUp { pos: point, button: PointerButton::Left, modifiers: KeyModifiers::default() })));
    }
    fn Draw(viewer: &mut SketchViewerState, tool: SketchToolKind, points: Arr<'_, SketchPoint>)
    {
        viewer.ProcessAction(SketchViewAction::SetTool(tool));
        points.Traverse(|point| {
            viewer.ProcessAction(SketchViewAction::CanvasAction(SketchCanvasAction::Input(SketchInputEvent::PointerMove { pos: *point, modifiers: KeyModifiers::default() })));
            Click(viewer, *point);
        });
    }
    let mut app = AppState::new();
    let _ = app.update(AppMessage::NewSketch);
    let id = app.tab_manager.active_tab().unwrap().id;
    let binaryPath =
        std::env::temp_dir().join(format!("trellis-ui-sketch-{}.flx", std::process::id()));
    let textPath = binaryPath.with_extension("tflx");
    {
        let viewer = app.SketchMut(id).unwrap();
        Draw(viewer,
             SketchToolKind::Line,
             crate::Buff![SketchPoint::New(0.0, 0.0), SketchPoint::New(100.0, 0.0)].Arr());
        Draw(viewer,
             SketchToolKind::Circle,
             crate::Buff![SketchPoint::New(300.0, 0.0), SketchPoint::New(350.0, 0.0)].Arr());
        Draw(viewer,
             SketchToolKind::Arc,
             crate::Buff![SketchPoint::New(600.0, 0.0),
                          SketchPoint::New(650.0, 0.0),
                          SketchPoint::New(600.0, 50.0)].Arr());
        Draw(viewer,
             SketchToolKind::Ellipse,
             crate::Buff![SketchPoint::New(800.0, 0.0),
                          SketchPoint::New(850.0, 0.0),
                          SketchPoint::New(800.0, 30.0)].Arr());
        Draw(viewer,
             SketchToolKind::EllipticalArc,
             crate::Buff![SketchPoint::New(1000.0, 0.0),
                          SketchPoint::New(1050.0, 0.0),
                          SketchPoint::New(1000.0, 30.0)].Arr());
        Draw(viewer,
             SketchToolKind::Polyline,
             crate::Buff![SketchPoint::New(0.0, 200.0),
                          SketchPoint::New(100.0, 200.0),
                          SketchPoint::New(100.0, 300.0)].Arr());
        viewer.ProcessAction(SketchViewAction::CanvasAction(SketchCanvasAction::Input(SketchInputEvent::KeyDown { key_escape: false, key_enter: true, key_delete: false, modifiers: KeyModifiers::default() })));
        jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), 6);
        jeeves_assert_eq!(ctx, viewer.doc.PointArena().Size(), 3);
        let ellipseId = viewer.doc.Entities()[4].Id();
        viewer.tool_ctrl.SetSelectedEntityId(ellipseId);
        viewer.ProcessAction(SketchViewAction::ArcStart("20.125".into()));
        viewer.ProcessAction(SketchViewAction::ArcSweep("-135.25".into()));
        viewer.ProcessAction(SketchViewAction::ApplyArc);
        if let SketchPrimitive::Ellipse(ellipse) =
            viewer.doc.FindEntity(ellipseId).unwrap().Primitive()
        {
            jeeves_assert_eq!(ctx, ellipse._SweepAngle, (-135.25f64).to_radians());
        } else {
            panic!("expected elliptical arc");
        }
        viewer.SaveToFile(&binaryPath).unwrap();
        viewer.SaveToFile(&textPath).unwrap();
        jeeves_assert!(ctx, !viewer.IsDirty());
        let palette = crate::fascia::theme::FasciaTheme::WindowsDark.palette();
        USeg::FromLen(2).Traverse(|i| {
                            let path = if i == 0 { &binaryPath } else { &textPath };
                            let mut restored =
                                SketchViewerState::FromFile(path.clone(), palette).unwrap();
                            jeeves_assert_eq!(ctx,
                                              SketchFlux::Payload(&viewer.doc),
                                              SketchFlux::Payload(&restored.doc));
                            jeeves_assert_eq!(ctx, restored.doc.Revision(), viewer.doc.Revision());
                            jeeves_assert!(ctx, !restored.IsDirty());
                            Draw(&mut restored,
                                 SketchToolKind::Line,
                                 crate::Buff![SketchPoint::New(1200.0, 0.0),
                                              SketchPoint::New(1300.0, 0.0)].Arr());
                            jeeves_assert_eq!(ctx, restored.doc.EntityCount(), 7);
                            restored.Undo();
                            jeeves_assert_eq!(ctx, restored.doc.EntityCount(), 6);
                            jeeves_assert!(ctx, !restored.IsDirty());
                        });
        viewer.ProcessAction(SketchViewAction::SetTool(SketchToolKind::Select));
        viewer.ProcessAction(SketchViewAction::DeleteSelected);
        jeeves_assert!(ctx, viewer.IsDirty());

        // Restoring must preserve edits until explicitly confirmed.
        viewer.ProcessAction(SketchViewAction::Restore);
        jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), 5);
        viewer.ProcessAction(SketchViewAction::CancelRestore);
        jeeves_assert!(ctx, viewer.IsDirty());
        viewer.ProcessAction(SketchViewAction::Restore);
        viewer.ProcessAction(SketchViewAction::ConfirmRestore);
        jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), 6);
        jeeves_assert!(ctx, !viewer.IsDirty());

        // Test SaveFormat to binary .flx and text .tflx
        viewer.ProcessAction(SketchViewAction::SaveFormat(crate::flux::FluxFileFormat::Binary));
        jeeves_assert!(ctx, !viewer.IsDirty());
        jeeves_assert!(ctx, binaryPath.exists());

        viewer.ProcessAction(SketchViewAction::SaveFormat(crate::flux::FluxFileFormat::Text));
        jeeves_assert!(ctx, !viewer.IsDirty());
        jeeves_assert!(ctx, textPath.exists());

        // Test grips on Elliptical Arc
        viewer.tool_ctrl.SetSelectedEntityId(ellipseId);
        let ell_grips = viewer.tool_ctrl.ActiveGrips(&viewer.doc);
        jeeves_assert!(ctx, ell_grips.Len() >= 5); // Center, Major, Minor, ArcStart, ArcEnd

        viewer.ProcessAction(SketchViewAction::DeleteSelected);
        jeeves_assert!(ctx, viewer.IsDirty());
    }
    let count = app.tab_manager.tabs().Size();
    let _ = app.update(AppMessage::CloseActiveTab);
    jeeves_assert_eq!(ctx, app.tab_manager.tabs().Size(), count);
    let _ = app.update(AppMessage::ResolveSketchClose(SketchCloseDecision::Cancel));
    jeeves_assert!(ctx, app.Sketch(id).is_some());
    let _ = app.update(AppMessage::CloseActiveTab);
    let _ = app.update(AppMessage::ResolveSketchClose(SketchCloseDecision::Discard));
    jeeves_assert!(ctx, app.Sketch(id).is_none());
    let _ = app.update(AppMessage::OpenFile(binaryPath.clone()));
    jeeves_assert_eq!(ctx, app.SketchCount(), 1);
    let invalidPath =
        binaryPath.with_file_name(format!("trellis-invalid-ui-{}.flx", std::process::id()));
    std::fs::write(&invalidPath, "invalid file").unwrap();
    let _ = app.update(AppMessage::OpenFile(invalidPath.clone()));
    jeeves_assert_eq!(ctx, app.SketchCount(), 1);
    jeeves_assert!(ctx, app.status_info.message.contains("Could not open"));
    std::fs::remove_file(binaryPath).unwrap();
    std::fs::remove_file(textPath).unwrap();
    std::fs::remove_file(invalidPath).unwrap();
});

jeeves_test!(Fascia, SketchViewerTabLifecycleAndSerialization, |ctx| {
    use crate::fascia::app::{AppMessage, AppState};
    use crate::fascia::sketch_tool::SketchToolKind;
    use crate::fascia::sketch_view::{SketchViewAction, SketchViewerState};
    use crate::fascia::tabs::TabKind;
    use crate::fenst::sketch::SketchPoint;
    use std::path::PathBuf;

    // 1. Create a SketchViewerState with entities
    let palette = crate::fascia::theme::FasciaTheme::WindowsDark.palette();
    let mut viewer = SketchViewerState::New(
        Some(PathBuf::from("test_sketch.flx")),
        "TestSketch".to_string(),
        palette,
    );
    jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), 0);
    jeeves_assert!(ctx, !viewer.IsDirty());

    // Add entities directly to doc
    let p1 = SketchPoint::New(0.0, 0.0);
    let p2 = SketchPoint::New(80.0, 0.0);
    let line_id = viewer.doc.AddLine(p1, p2).expect("add line");
    let circ_id = viewer.doc.AddCircle(SketchPoint::New(40.0, 40.0), 20.0).expect("add circle");
    jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), 2);

    // 2. Test FitAll and ResetView
    viewer.ProcessAction(SketchViewAction::FitAll);
    jeeves_assert!(ctx, viewer.zoom > 0.0);
    viewer.ProcessAction(SketchViewAction::ResetView);
    jeeves_assert_eq!(ctx, viewer.zoom, 1.0);

    // 3. Test SVG export
    let svg = viewer.ExportSvgString();
    jeeves_assert!(ctx, svg.contains("<svg"));
    jeeves_assert!(ctx, svg.contains("<line"));
    jeeves_assert!(ctx, svg.contains("<circle"));

    // 4. Test Save to temporary file and reload
    let tmp_dir = std::env::temp_dir();
    let file_path = tmp_dir.join(format!("trellis_test_output_{}.flx", std::process::id()));
    let save_res = viewer.SaveToFile(&file_path);
    jeeves_assert!(ctx, save_res.is_ok());
    jeeves_assert!(ctx, !viewer.IsDirty());

    let loaded = SketchViewerState::FromFile(file_path.clone(), palette).expect("load sketch");
    jeeves_assert_eq!(ctx, loaded.doc.EntityCount(), 2);
    jeeves_assert!(ctx, loaded.doc.FindEntity(line_id).is_some());
    jeeves_assert!(ctx, loaded.doc.FindEntity(circ_id).is_some());

    // Clean up temp file
    let _ = std::fs::remove_file(file_path);

    // 5. Test AppState tab opening & message routing
    let mut app = AppState::new();
    let (idx, _is_new, tab_id) = app.tab_manager.open_sketch(None, "New Sketch".to_string());
    let sketch_viewer = SketchViewerState::New(None, "New Sketch".to_string(), app.palette());
    app.SetDocument(tab_id, crate::fascia::TabDocument::Sketch(Box::new(sketch_viewer))).unwrap();

    jeeves_assert!(ctx, app.tab_manager.active_tab().is_some());
    jeeves_assert_eq!(ctx, app.tab_manager.active_tab().unwrap().kind, TabKind::SketchViewer);

    // Route a tool switch action
    let _ = app.update(AppMessage::Sketch(tab_id, SketchViewAction::SetTool(SketchToolKind::Circle)));
    jeeves_assert_eq!(ctx, app.Sketch(tab_id).unwrap().tool_ctrl.Tool(), SketchToolKind::Circle);

    // Close tab cleanly
    app.close_tab_internal(idx);
    jeeves_assert_eq!(ctx, app.SketchCount(), 0);
});

jeeves_test!(Fascia, SketchInspectorNavigationAndPropertiesEditing, |ctx| {
    use crate::fascia::sketch_inspector::{InspectorPanelTab, SketchInspectorAction};
    use crate::fascia::sketch_view::{SketchViewAction, SketchViewerState};
    use crate::fenst::sketch::{SketchPoint, SketchPrimitive};

    let palette = crate::fascia::theme::FasciaTheme::WindowsDark.palette();
    let mut viewer = SketchViewerState::New(None, "InspectorTest".to_string(), palette);

    // Initial state
    jeeves_assert!(ctx, viewer.inspector.ShowPanel());
    viewer.ProcessAction(SketchViewAction::TogglePanel);
    jeeves_assert!(ctx, !viewer.inspector.ShowPanel());
    viewer.ProcessAction(SketchViewAction::TogglePanel);
    jeeves_assert!(ctx, viewer.inspector.ShowPanel());

    // Navigation and tabs
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::SelectTab(InspectorPanelTab::Inspector)));
    jeeves_assert_eq!(ctx, viewer.inspector.ActiveTab(), InspectorPanelTab::Inspector);
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::SelectTab(InspectorPanelTab::Tree)));
    jeeves_assert_eq!(ctx, viewer.inspector.ActiveTab(), InspectorPanelTab::Tree);

    // Add Line, Circle, Arc, Ellipse
    let line_id = viewer.doc.AddLine(SketchPoint::New(10.0, 20.0), SketchPoint::New(50.0, 60.0)).expect("line");
    let circ_id = viewer.doc.AddCircle(SketchPoint::New(100.0, 100.0), 25.0).expect("circle");
    let arc_id = viewer.doc.AddArc(SketchPoint::New(200.0, 200.0), 30.0, 0.0, std::f64::consts::PI).expect("arc");
    let _ell_id = viewer.doc.AddEllipse(SketchPoint::New(300.0, 300.0), 50.0, 25.0, 0.0).expect("ellipse");

    // Select Line and verify inspector fields
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::SelectEntity(line_id)));
    jeeves_assert_eq!(ctx, viewer.tool_ctrl.SelectedEntityId(), line_id);
    jeeves_assert_eq!(ctx, viewer.inspector.Field(0), "10");
    jeeves_assert_eq!(ctx, viewer.inspector.Field(1), "20");
    jeeves_assert_eq!(ctx, viewer.inspector.Field(2), "50");
    jeeves_assert_eq!(ctx, viewer.inspector.Field(3), "60");

    // Edit Line end points via inspector
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::FieldInput { index: 2, value: "80.0".to_string() }));
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::FieldInput { index: 3, value: "90.0".to_string() }));
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::ApplyFieldEdits));

    if let Some(ent) = viewer.doc.FindEntity(line_id) {
        if let SketchPrimitive::Line(l) = ent.Primitive() {
            jeeves_assert_eq!(ctx, l._End._X, 80.0);
            jeeves_assert_eq!(ctx, l._End._Y, 90.0);
        } else {
            jeeves_assert!(ctx, false);
        }
    }

    // Verify Undo/Redo of inspector edit
    viewer.Undo();
    if let Some(ent) = viewer.doc.FindEntity(line_id)
        && let SketchPrimitive::Line(l) = ent.Primitive()
    {
        jeeves_assert_eq!(ctx, l._End._X, 50.0);
    }
    viewer.Redo();
    if let Some(ent) = viewer.doc.FindEntity(line_id)
        && let SketchPrimitive::Line(l) = ent.Primitive()
    {
        jeeves_assert_eq!(ctx, l._End._X, 80.0);
    }

    // Select Circle and edit radius
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::SelectEntity(circ_id)));
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::FieldInput { index: 2, value: "45.0".to_string() }));
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::ApplyFieldEdits));
    if let Some(ent) = viewer.doc.FindEntity(circ_id)
        && let SketchPrimitive::Circle(c) = ent.Primitive()
    {
        jeeves_assert_eq!(ctx, c._Radius, 45.0);
    }

    // Select Arc and edit sweep
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::SelectEntity(arc_id)));
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::FieldInput { index: 4, value: "90.0".to_string() }));
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::ApplyFieldEdits));
    if let Some(ent) = viewer.doc.FindEntity(arc_id)
        && let SketchPrimitive::Arc(a) = ent.Primitive()
    {
        jeeves_assert!(ctx, (a._SweepAngle - std::f64::consts::FRAC_PI_2).abs() < 1e-4);
    }

    // Construction toggle
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::ToggleEntityConstruction(line_id)));
    jeeves_assert!(ctx, viewer.doc.FindEntity(line_id).unwrap().IsConstruction());
    viewer.Undo();
    jeeves_assert!(ctx, !viewer.doc.FindEntity(line_id).unwrap().IsConstruction());
});

jeeves_test!(Fascia, SketchInspectorDuplicationDeletionAndExplorerTree, |ctx| {
    use crate::fascia::sketch_inspector::SketchInspectorAction;
    use crate::fascia::sketch_view::{view_sketch_tab, SketchViewAction, SketchViewerState};
    use crate::fascia::tabs::TabId;
    use crate::fenst::sketch::{SketchPoint, SketchPrimitive};
    use crate::fenst::sketch_xplr::SketchXplr;
    use crate::silo::IArr;

    let palette = crate::fascia::theme::FasciaTheme::WindowsDark.palette();
    let mut viewer = SketchViewerState::New(None, "XplrTest".to_string(), palette);

    let circ_id = viewer.doc.AddCircle(SketchPoint::New(50.0, 50.0), 20.0).expect("circ");
    let count_before = viewer.doc.EntityCount();

    // Duplicate circle
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::DuplicateEntity(circ_id)));
    jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), count_before + 1);
    let dup_id = viewer.tool_ctrl.SelectedEntityId();
    jeeves_assert!(ctx, dup_id != circ_id);
    if let Some(ent) = viewer.doc.FindEntity(dup_id)
        && let SketchPrimitive::Circle(c) = ent.Primitive()
    {
        jeeves_assert_eq!(ctx, c._Center._X, 60.0);
        jeeves_assert_eq!(ctx, c._Radius, 20.0);
    }

    // Delete duplicated entity
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::DeleteEntity(dup_id)));
    jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), count_before);
    jeeves_assert!(ctx, viewer.doc.FindEntity(dup_id).is_none());

    // Layer toggles
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::ToggleLayerVisible(0)));
    jeeves_assert!(ctx, !viewer.doc.Layers().Arr().Get(0).unwrap().IsVisible());
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::ToggleLayerLocked(0)));
    jeeves_assert!(ctx, viewer.doc.Layers().Arr().Get(0).unwrap().IsLocked());
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::ToggleLayerConstruction(0)));
    jeeves_assert!(ctx, viewer.doc.Layers().Arr().Get(0).unwrap().IsConstruction());

    // Explorer tree projection via SketchXplr
    let xplr = SketchXplr::FromDocument(&viewer.doc, "SketchDoc", "sketch://test");
    jeeves_assert_eq!(ctx, xplr.LayerCount(), 1);
    jeeves_assert_eq!(ctx, xplr.EntityCount(), 1);

    // Verify UI view function renders without panic
    let tab_id = TabId(1);
    let element = view_sketch_tab(tab_id, &viewer, palette, |_| ());
    drop(element);
    viewer.inspector.SetShowPanel(false);
    let element_closed = view_sketch_tab(tab_id, &viewer, palette, |_| ());
    drop(element_closed);
});

//-------------------------------------------------------------------------------------------------
// 3D Workplane, Wireframe Projection, and Solid Extrusion Viewport Integration
jeeves_test!(Fascia, SketchEditsPreserveUnchangedPrecision, |ctx| {
    use crate::fascia::sketch_inspector::SketchInspectorAction;
    use crate::fascia::sketch_view::SketchViewerState;
    use crate::fenst::sketch::{SketchPoint, SketchPrimitive};

    let mut viewer =
        SketchViewerState::New(None, "Precision".into(), FasciaTheme::WindowsDark.palette());
    let start = SketchPoint::New(10.1234567890123, 20.9876543210987);
    let id = viewer.doc
                   .AddLine(start, SketchPoint::New(50.0, 60.0))
                   .unwrap();
    viewer.ProcessInspectorAction(SketchInspectorAction::SelectEntity(id));
    viewer.ProcessInspectorAction(SketchInspectorAction::FieldInput { index: 2,
                                                                      value: "80".into(), });
    viewer.ApplyInspectorEdits();
    if let SketchPrimitive::Line(line) = viewer.doc.FindEntity(id).unwrap().Primitive() {
        jeeves_assert_eq!(ctx, line._Start, start);
        jeeves_assert_eq!(ctx, line._End._X, 80.0);
    }
    let arcId = viewer.doc
                      .AddArc(start, 50.0, 0.1234567890123, -1.234567890123)
                      .unwrap();
    let original = *viewer.doc.FindEntity(arcId).unwrap().Primitive();
    viewer.ProcessInspectorAction(SketchInspectorAction::SelectEntity(arcId));
    let revision = viewer.doc.Revision();
    viewer.ApplyInspectorEdits();
    jeeves_assert_eq!(ctx, viewer.doc.Revision(), revision);
    viewer.ProcessInspectorAction(SketchInspectorAction::FieldInput { index: 2,
                                                                      value: "75".into(), });
    viewer.ApplyInspectorEdits();
    if let (SketchPrimitive::Arc(old), SketchPrimitive::Arc(new)) =
        (original, viewer.doc.FindEntity(arcId).unwrap().Primitive())
    {
        jeeves_assert_eq!(ctx, new._Center, old._Center);
        jeeves_assert_eq!(ctx, new._StartAngle.to_bits(), old._StartAngle.to_bits());
        jeeves_assert_eq!(ctx, new._SweepAngle.to_bits(), old._SweepAngle.to_bits());
        jeeves_assert_eq!(ctx, new._Radius, 75.0);
    }
    viewer.ProcessInspectorAction(SketchInspectorAction::FieldInput { index: 2,
                                                                      value: "NaN".into(), });
    let revision = viewer.doc.Revision();
    viewer.ApplyInspectorEdits();
    jeeves_assert_eq!(ctx, viewer.doc.Revision(), revision);
});

jeeves_test!(Fascia, SketchLayerProtectionAndSelectionHistory, |ctx| {
    use crate::fascia::sketch_canvas::SketchCanvasAction;
    use crate::fascia::sketch_inspector::SketchInspectorAction;
    use crate::fascia::sketch_tool::{
        KeyModifiers, PointerButton, SketchInputEvent, SketchToolKind,
    };
    use crate::fascia::sketch_view::{SketchViewAction, SketchViewerState};
    use crate::fenst::sketch::SketchPoint;

    let mut viewer =
        SketchViewerState::New(None, "Layers".into(), FasciaTheme::WindowsLight.palette());
    let id = viewer.doc
                   .AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(100.0, 0.0))
                   .unwrap();
    viewer.ProcessInspectorAction(SketchInspectorAction::SelectEntity(id));
    viewer.ProcessInspectorAction(SketchInspectorAction::ToggleLayerLocked(0));
    jeeves_assert_eq!(ctx, viewer.tool_ctrl.SelectedEntityId(), u32::MAX);
    viewer.ProcessInspectorAction(SketchInspectorAction::DeleteEntity(id));
    viewer.DuplicateEntity(id);
    jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), 1);
    viewer.tool_ctrl.SetSelectedEntityId(id);
    jeeves_assert!(ctx, viewer.tool_ctrl.ActiveGrips(&viewer.doc).IsEmpty());
    viewer.ProcessAction(SketchViewAction::SetTool(SketchToolKind::Line));
    viewer.ProcessAction(SketchViewAction::CanvasAction(SketchCanvasAction::Input(SketchInputEvent::PointerDown {
        pos: SketchPoint::New(200.0, 0.0), button: PointerButton::Left, modifiers: KeyModifiers::default(),
    })));
    jeeves_assert!(ctx, viewer.tool_ctrl.DraftGeometry().is_none());
    viewer.ProcessInspectorAction(SketchInspectorAction::ToggleLayerLocked(0));
    viewer.ProcessInspectorAction(SketchInspectorAction::ToggleLayerVisible(0));
    jeeves_assert_eq!(ctx,
                      viewer.tool_ctrl
                            .HitTestEntity(&viewer.doc, SketchPoint::New(50.0, 0.0), 8.0),
                      u32::MAX);
    viewer.ProcessInspectorAction(SketchInspectorAction::ToggleLayerVisible(0));
    viewer.DuplicateEntity(id);
    jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), 2);
    viewer.Undo();
    jeeves_assert_eq!(ctx, viewer.tool_ctrl.SelectedEntityId(), u32::MAX);
    jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), 1);
});

jeeves_test!(
             Fascia,
             SketchShiftConstraintMatchesCommittedEndpoint,
             |ctx| {
                 use crate::fascia::sketch_tool::{
                     DraftPrimitive, KeyModifiers, PointerButton, SketchInputEvent,
                     SketchToolController, SketchToolKind,
                 };
                 use crate::fenst::sketch::{SketchDocument, SketchPoint, SketchPrimitive};
                 use crate::silo::IArr;

                 crate::Buff![false, true].Arr().Traverse(|clickClick| {
        let doc = SketchDocument::New();
        let mut ctrl = SketchToolController::New();
        ctrl.SetTool(SketchToolKind::Line);
        let modifiers = KeyModifiers::New(true, false, false);
        let start = SketchPoint::New(0.0, 0.0);
        let end = SketchPoint::New(100.0, 21.0);
        ctrl.ProcessEvent(&doc, SketchInputEvent::PointerDown { pos: start, button: PointerButton::Left, modifiers });
        if *clickClick {
            ctrl.ProcessEvent(&doc, SketchInputEvent::PointerUp { pos: start, button: PointerButton::Left, modifiers });
        }
        ctrl.ProcessEvent(&doc, SketchInputEvent::PointerMove { pos: end, modifiers });
        let previewEnd = match ctrl.DraftGeometry().unwrap().primitive.as_ref().unwrap() {
            DraftPrimitive::Line { end, .. } => *end,
            _ => panic!("line preview expected"),
        };
        let event = if *clickClick {
            SketchInputEvent::PointerDown { pos: end, button: PointerButton::Left, modifiers }
        } else {
            SketchInputEvent::PointerUp { pos: end, button: PointerButton::Left, modifiers }
        };
        ctrl.ProcessEvent(&doc, event);
        let patch = ctrl.TakeCommittedPatch().unwrap();
        if let SketchPrimitive::Line(line) = patch.Added()[0].Primitive() {
            jeeves_assert_eq!(ctx, line._End, previewEnd);
            jeeves_assert!(ctx, (line._End._Y.atan2(line._End._X).to_degrees() - 15.0).abs() < 1e-10);
        }
    });
             }
);

jeeves_test!(Fascia,
             SketchNavigationAndExtrusionRejectInvalidInput,
             |ctx| {
                 use crate::fascia::app::{AppMessage, AppState};
                 use crate::fascia::sketch_inspector::SketchInspectorAction;
                 use crate::fascia::sketch_view::{SketchViewAction, SketchViewerState};
                 use crate::fenst::sketch::SketchPoint;

                 let mut viewer = SketchViewerState::New(None,
                                                         "Navigation".into(),
                                                         FasciaTheme::WindowsDark.palette());
                 let pan = viewer.pan;
                 viewer.ProcessAction(SketchViewAction::Zoom(f32::NAN, None));
                 viewer.ProcessAction(SketchViewAction::Zoom(-1.0, None));
                 viewer.ProcessAction(SketchViewAction::Pan(f32::INFINITY, 0.0));
                 jeeves_assert_eq!(ctx, viewer.zoom, 1.0);
                 jeeves_assert_eq!(ctx, viewer.pan, pan);
                 let focal = iced::Point::new(123.0, 87.0);
                 let model = viewer.ScreenToModel(focal);
                 viewer.ProcessAction(SketchViewAction::Zoom(2.0, Some([focal.x, focal.y])));
                 jeeves_assert_eq!(ctx, viewer.ScreenToModel(focal), model);
                 viewer.ProcessAction(SketchViewAction::ExtrudeDepthInput("invalid".into()));
                 jeeves_assert!(ctx, viewer.ExtrudeDepth().is_err());
                 viewer.ProcessAction(SketchViewAction::ExtrudeDepthInput("NaN".into()));
                 jeeves_assert!(ctx, viewer.ExtrudeDepth().is_err());
                 viewer.ProcessAction(SketchViewAction::ExtrudeDepthInput(" -5 ".into()));
                 jeeves_assert_eq!(ctx, viewer.ExtrudeDepth().unwrap(), -5.0);

                 let mut app = AppState::default();
                 let _ = app.update(AppMessage::NewSketch);
                 let id = app.tab_manager.active_tab().unwrap().id;
                 let viewer = app.SketchMut(id).unwrap();
                 viewer.doc
                       .AddCircle(SketchPoint::New(0.0, 0.0), 20.0)
                       .unwrap();
                 viewer.ProcessAction(SketchViewAction::ExtrudeDepthInput("invalid".into()));
                 let count = app.tab_manager.tabs().Size();
                 let _ = app.update(AppMessage::Sketch(id, SketchViewAction::InspectorAction(SketchInspectorAction::Extrude3d)));
                 jeeves_assert_eq!(ctx, app.tab_manager.tabs().Size(), count);
                 jeeves_assert!(ctx,
                                app.Sketch(id)
                                   .unwrap()
                                   .status_message
                                   .is_some());
             });

jeeves_test!(Fascia, Sketch3dViewportAndExtrusionIntegration, |ctx| {
    use crate::fascia::app::{AppState, AppMessage};
    use crate::fascia::sketch_inspector::SketchInspectorAction;
    use crate::fascia::sketch_view::{view_sketch_tab, SketchViewAction, SketchViewerState};
    use crate::fascia::tabs::TabKind;
    use crate::fenst::sketch::{SketchPlane, SketchPoint};

    let palette = crate::fascia::theme::FasciaTheme::WindowsDark.palette();
    let mut viewer = SketchViewerState::New(None, "Integration3D".to_string(), palette);

    // 1. Workplane Switching
    viewer.ProcessAction(SketchViewAction::SetWorkplane(SketchPlane::Xz()));
    jeeves_assert_eq!(ctx, viewer.doc.Plane().PresetName(), "XZ (Top)");

    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::SetPlane(SketchPlane::Yz())));
    jeeves_assert_eq!(ctx, viewer.doc.Plane().PresetName(), "YZ (Right)");

    // 2. Add closed rectangular sketch profile
    viewer.ProcessAction(SketchViewAction::SetWorkplane(SketchPlane::Xy()));
    viewer.doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(50.0, 0.0)).unwrap();
    viewer.doc.AddLine(SketchPoint::New(50.0, 0.0), SketchPoint::New(50.0, 30.0)).unwrap();
    viewer.doc.AddLine(SketchPoint::New(50.0, 30.0), SketchPoint::New(0.0, 30.0)).unwrap();
    viewer.doc.AddLine(SketchPoint::New(0.0, 30.0), SketchPoint::New(0.0, 0.0)).unwrap();

    // 3. 3D Wireframe Generation
    let wire_res = viewer.Generate3dWireframe();
    jeeves_assert!(ctx, wire_res.is_ok());
    let wire_asset = wire_res.unwrap();
    jeeves_assert!(ctx, wire_asset.VertexCount() > 0);
    jeeves_assert!(ctx, wire_asset.Edges().Size() > 0);

    // 4. 3D Solid Extrude Generation
    let ext_res = viewer.Generate3dExtrude(25.0);
    jeeves_assert!(ctx, ext_res.is_ok());
    let ext_asset = ext_res.unwrap();
    jeeves_assert!(ctx, ext_asset.VertexCount() > 0);
    jeeves_assert!(ctx, ext_asset.Triangles().Size() > 0);
    jeeves_assert!(ctx, ext_asset.FaceCount() > 0);

    // 5. App-level message dispatch and 3D tab integration
    let mut app = AppState::default();
    let (_, _, sketch_tab_id) = app.tab_manager.open_sketch(None, viewer.title.clone());
    app.SetDocument(sketch_tab_id, crate::fascia::TabDocument::Sketch(Box::new(viewer))).unwrap();

    // Trigger 3D Extrude via App message and complete background job
    let _ = app.update(AppMessage::Sketch(sketch_tab_id, SketchViewAction::Extrude3d(25.0)));
    let (job_gen, token, snap) = app.SketchMut(sketch_tab_id).unwrap().BeginJob();
    let ext_outcome = iced::futures::executor::block_on(crate::fascia::sketch_job::ExecuteSketchJob(
        snap,
        crate::fascia::sketch_job::SketchJobKind::Extrude(25.0),
        token.clone(),
    ));
    let _ = app.update(AppMessage::SketchJobCompleted {
        tab_id: sketch_tab_id,
        generation: job_gen,
        cancellation: token,
        snapshot_rev: 0,
        outcome: ext_outcome,
    });
    let active_tab = app.tab_manager.active_tab().expect("active 3D tab");
    jeeves_assert_eq!(ctx, active_tab.kind, TabKind::ObjViewer);
    jeeves_assert!(ctx, active_tab.title.contains("Extrude: Integration3D"));
    jeeves_assert!(ctx, app.Geometry(active_tab.id).is_some());

    // Trigger 3D Wireframe via App message and complete background job
    let _ = app.update(AppMessage::Sketch(sketch_tab_id, SketchViewAction::View3dWireframe));
    let (wire_gen, wire_token, wire_snap) = app.SketchMut(sketch_tab_id).unwrap().BeginJob();
    let wire_outcome = iced::futures::executor::block_on(crate::fascia::sketch_job::ExecuteSketchJob(
        wire_snap,
        crate::fascia::sketch_job::SketchJobKind::Wireframe,
        wire_token.clone(),
    ));
    let _ = app.update(AppMessage::SketchJobCompleted {
        tab_id: sketch_tab_id,
        generation: wire_gen,
        cancellation: wire_token,
        snapshot_rev: 0,
        outcome: wire_outcome,
    });
    let active_wire_tab = app.tab_manager.active_tab().expect("active wireframe tab");
    jeeves_assert_eq!(ctx, active_wire_tab.kind, TabKind::ObjViewer);
    jeeves_assert!(ctx, active_wire_tab.title.contains("3D: Integration3D"));
    jeeves_assert!(ctx, app.Geometry(active_wire_tab.id).is_some());

    // 6. View rendering check
    let v_ref = app.Sketch(sketch_tab_id).unwrap();
    let ui_elem = view_sketch_tab(sketch_tab_id, v_ref, palette, |_| ());
    drop(ui_elem);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, SketchContextMenuAndWorkplaneOffsetIntegration, |ctx| {
    use crate::fascia::sketch_canvas::SketchCanvasAction;
    use crate::fascia::sketch_inspector::SketchInspectorAction;
    use crate::fascia::sketch_view::{view_sketch_tab, SketchViewAction, SketchViewerState};
    use crate::fascia::tabs::TabId;
    use crate::fascia::theme::FasciaTheme;
    use crate::fenst::sketch::{SketchPlane, SketchPoint};

    let palette = FasciaTheme::WindowsDark.palette();
    let mut viewer = SketchViewerState::New(None, "ContextHarmonize".to_string(), palette);

    // 1. Initial context menu is None
    jeeves_assert!(ctx, viewer.context_menu.is_none());

    // 2. OpenContextMenu sets coordinates
    viewer.ProcessAction(SketchViewAction::OpenContextMenu([120.0, 240.0]));
    jeeves_assert_eq!(ctx, viewer.context_menu, Some([120.0, 240.0]));

    // 3. Render with active context menu renders cleanly without panicking
    let elem_with_menu = view_sketch_tab(TabId(99), &viewer, palette, |_| ());
    drop(elem_with_menu);

    // 4. DismissContextMenu clears coordinates
    viewer.ProcessAction(SketchViewAction::DismissContextMenu);
    jeeves_assert!(ctx, viewer.context_menu.is_none());

    // 5. CanvasAction::ContextMenu sets coordinates
    viewer.ProcessAction(SketchViewAction::CanvasAction(SketchCanvasAction::ContextMenu([300.0, 150.0])));
    jeeves_assert_eq!(ctx, viewer.context_menu, Some([300.0, 150.0]));
    viewer.ProcessAction(SketchViewAction::DismissContextMenu);
    jeeves_assert!(ctx, viewer.context_menu.is_none());

    // 6. OffsetWorkplane shifts normal
    let initial_origin = viewer.doc.Plane().Origin();
    jeeves_assert_eq!(ctx, initial_origin[2], 0.0);

    viewer.ProcessAction(SketchViewAction::OffsetWorkplane(10.0));
    jeeves_assert_eq!(ctx, viewer.doc.Plane().Origin()[2], 10.0);
    jeeves_assert!(ctx, viewer.status_message.as_ref().unwrap().contains("offset by +10.0 mm"));

    viewer.ProcessAction(SketchViewAction::OffsetWorkplane(-25.0));
    jeeves_assert_eq!(ctx, viewer.doc.Plane().Origin()[2], -15.0);

    // 7. Context menu operations on entity: duplicate, construction toggle, delete
    let line_id = viewer.doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(40.0, 40.0)).unwrap();
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::SelectEntity(line_id)));
    jeeves_assert_eq!(ctx, viewer.tool_ctrl.SelectedEntityId(), line_id);

    // Toggle construction via inspector
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::ToggleEntityConstruction(line_id)));
    jeeves_assert!(ctx, viewer.doc.FindEntity(line_id).unwrap().IsConstruction());

    // Duplicate entity
    viewer.ProcessAction(SketchViewAction::InspectorAction(SketchInspectorAction::DuplicateEntity(line_id)));
    jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), 2);

    // Delete selected entity
    viewer.ProcessAction(SketchViewAction::DeleteSelected);
    jeeves_assert_eq!(ctx, viewer.doc.EntityCount(), 1);

    // Workplane preset switching
    viewer.ProcessAction(SketchViewAction::SetWorkplane(SketchPlane::Yz()));
    jeeves_assert_eq!(ctx, viewer.doc.Plane().PresetName(), "YZ (Right)");

    // Final render verification
    let final_ui = view_sketch_tab(TabId(100), &viewer, palette, |_| ());
    drop(final_ui);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, SketchToolControllerNumericLockDrivesCommit, |ctx| {
    use crate::fascia::sketch_tool::{
        KeyModifiers, NumericFieldKind, PointerButton, SketchInputEvent, SketchToolController,
        SketchToolKind,
    };
    use crate::fenst::sketch::{SketchDocument, SketchPoint, SketchPrimitive};

    let doc = SketchDocument::New();
    let mut ctrl = SketchToolController::New();
    ctrl.SetTool(SketchToolKind::Line);

    // 1. Click start at (10, 10)
    let mods = KeyModifiers::default();
    ctrl.ProcessEvent(&doc, SketchInputEvent::PointerDown {
        pos:       SketchPoint::New(10.0, 10.0),
        button:    PointerButton::Left,
        modifiers: mods,
    });

    // 2. Type length 150.0 via Numeric event
    ctrl.ProcessEvent(&doc, SketchInputEvent::Numeric {
        field: NumericFieldKind::Primary,
        value: 150.0,
    });
    jeeves_assert_eq!(ctx, ctrl.LengthLock(), Some(150.0));

    // 3. Move pointer to (200, 10) -> direction +X
    ctrl.ProcessEvent(&doc, SketchInputEvent::PointerMove {
        pos:       SketchPoint::New(200.0, 10.0),
        modifiers: mods,
    });

    // 4. Click to commit line
    ctrl.ProcessEvent(&doc, SketchInputEvent::PointerDown {
        pos:       SketchPoint::New(200.0, 10.0),
        button:    PointerButton::Left,
        modifiers: mods,
    });

    let patch = ctrl.TakeCommittedPatch();
    jeeves_assert!(ctx, patch.is_some());
    let patch = patch.unwrap();
    jeeves_assert_eq!(ctx, patch.Added().Len(), 1);
    let entity = patch.Added().Get(0).unwrap();
    if let SketchPrimitive::Line(l) = entity.Primitive() {
        jeeves_assert_eq!(ctx, l._Start._X, 10.0);
        jeeves_assert_eq!(ctx, l._Start._Y, 10.0);
        jeeves_assert_eq!(ctx, l._End._X, 160.0);
        jeeves_assert_eq!(ctx, l._End._Y, 10.0);
        jeeves_assert!(ctx, (l.Length() - 150.0).abs() < 1e-6);
    } else {
        panic!("Expected Line primitive");
    }

    // Test Circle with radius lock
    ctrl.SetTool(SketchToolKind::Circle);
    ctrl.ProcessEvent(&doc, SketchInputEvent::PointerDown {
        pos:       SketchPoint::New(0.0, 0.0),
        button:    PointerButton::Left,
        modifiers: mods,
    });
    ctrl.ProcessEvent(&doc, SketchInputEvent::Numeric {
        field: NumericFieldKind::Primary,
        value: 42.5,
    });
    jeeves_assert_eq!(ctx, ctrl.RadiusLock(), Some(42.5));
    ctrl.ProcessEvent(&doc, SketchInputEvent::PointerMove {
        pos:       SketchPoint::New(100.0, 100.0),
        modifiers: mods,
    });
    ctrl.ProcessEvent(&doc, SketchInputEvent::PointerDown {
        pos:       SketchPoint::New(100.0, 100.0),
        button:    PointerButton::Left,
        modifiers: mods,
    });
    let patch = ctrl.TakeCommittedPatch().unwrap();
    if let SketchPrimitive::Circle(c) = patch.Added().Get(0).unwrap().Primitive() {
        jeeves_assert_eq!(ctx, c._Center._X, 0.0);
        jeeves_assert_eq!(ctx, c._Center._Y, 0.0);
        jeeves_assert_eq!(ctx, c._Radius, 42.5);
    } else {
        panic!("Expected Circle primitive");
    }
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, SketchDimensionSelectionAndDeletion, |ctx| {
    use crate::fascia::sketch_tool::{
        KeyModifiers, PointerButton, SketchInputEvent, SketchToolController, SketchToolKind,
    };
    use crate::fenst::sketch::{SketchDocument, SketchPoint};
    use crate::fenst::sketch_dimension::{DimensionKind, SketchDimension};

    let mut doc = SketchDocument::New();
    let l_id = doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(100.0, 0.0)).unwrap();
    let dim = SketchDimension::New(
        1,
        DimensionKind::LinearAligned,
        SketchPoint::New(0.0, 0.0),
        SketchPoint::New(100.0, 0.0),
        20.0,
        l_id,
    );
    doc.AddDimension(dim);
    jeeves_assert_eq!(ctx, doc.Dimensions().Len(), 1);

    let mut ctrl = SketchToolController::New();
    ctrl.SetTool(SketchToolKind::Select);

    // Dimension line for p1=(0,0), p2=(100,0), offset=20 is along y=20 from x=0 to x=100.
    // Click at (50.0, 20.0)
    let mods = KeyModifiers::default();
    ctrl.ProcessEvent(&doc, SketchInputEvent::PointerDown {
        pos:       SketchPoint::New(50.0, 20.0),
        button:    PointerButton::Left,
        modifiers: mods,
    });

    jeeves_assert_eq!(ctx, ctrl.SelectedDimensionId(), 1);
    jeeves_assert_eq!(ctx, ctrl.SelectedEntityId(), u32::MAX);

    // Press Delete key
    ctrl.ProcessEvent(&doc, SketchInputEvent::KeyDown {
        key_escape: false,
        key_enter:  false,
        key_delete: true,
        modifiers:  mods,
    });

    let patch = ctrl.TakeCommittedPatch();
    jeeves_assert!(ctx, patch.is_some());
    let patch = patch.unwrap();
    jeeves_assert_eq!(ctx, patch.RemovedDimensions().Len(), 1);
    jeeves_assert_eq!(ctx, patch.RemovedDimensions().Get(0).unwrap()._Id, 1);
    jeeves_assert_eq!(ctx, ctrl.SelectedDimensionId(), u32::MAX);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, SketchMultiPickToolPreservesStateOnError, |ctx| {
    use crate::fascia::sketch_tool::{
        KeyModifiers, PointerButton, SketchInputEvent, SketchToolController, SketchToolKind,
    };
    use crate::fenst::sketch::{SketchDocument, SketchPoint};

    let mut doc = SketchDocument::New();
    let l1 = doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(50.0, 0.0)).unwrap();
    let _l2 = doc.AddLine(SketchPoint::New(0.0, 20.0), SketchPoint::New(50.0, 20.0)).unwrap();

    let mut ctrl = SketchToolController::New();
    ctrl.SetTool(SketchToolKind::Fillet);
    ctrl.SetFilletRadius(5.0);

    let mods = KeyModifiers::default();
    // Pick line 1
    ctrl.ProcessEvent(&doc, SketchInputEvent::PointerDown {
        pos:       SketchPoint::New(25.0, 0.0),
        button:    PointerButton::Left,
        modifiers: mods,
    });
    jeeves_assert_eq!(ctx, ctrl.SelectedEntityId(), l1);

    // Pick line 2 (parallel lines - fillet will fail)
    ctrl.ProcessEvent(&doc, SketchInputEvent::PointerDown {
        pos:       SketchPoint::New(25.0, 20.0),
        button:    PointerButton::Left,
        modifiers: mods,
    });

    // Error is set and first picked line is preserved
    jeeves_assert!(ctx, ctrl.LastError().is_some());
    jeeves_assert!(ctx, ctrl.TakeCommittedPatch().is_none());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, SketchBroadPhaseCandidateFiltering, |ctx| {
    use crate::fascia::sketch_tool::{
        SketchToolController, SketchToolKind,
    };
    use crate::fenst::sketch::{SketchDocument, SketchPoint};

    let mut doc = SketchDocument::New();
    // Add 1,000 horizontal lines distributed along X
    for i in 0..1000 {
        let x = (i as f64) * 50.0;
        doc.AddLine(SketchPoint::New(x, 0.0), SketchPoint::New(x + 20.0, 0.0))
            .expect("add line");
    }
    jeeves_assert_eq!(ctx, doc.EntityCount(), 1000);

    let mut ctrl = SketchToolController::New();
    ctrl.SetTool(SketchToolKind::Select);

    // Hit test near entity at index 400 (x = 20000.0)
    let hit_id = ctrl.HitTestEntity(&doc, SketchPoint::New(20010.0, 0.0), 5.0);
    jeeves_assert!(ctx, hit_id != u32::MAX);
    let entity = doc.FindEntity(hit_id).expect("found hit entity");
    jeeves_assert_eq!(ctx, entity.CachedBounds()._Min._X, 20000.0);

    // Hit test far in empty space
    let miss_id = ctrl.HitTestEntity(&doc, SketchPoint::New(20010.0, 100.0), 5.0);
    jeeves_assert_eq!(ctx, miss_id, u32::MAX);

    // Snap test near entity at index 800 (x = 40000.0)
    let snap_pt = ctrl.EvaluateSnap(&doc, SketchPoint::New(40002.0, 1.0));
    jeeves_assert_eq!(ctx, snap_pt._X, 40000.0);
    jeeves_assert_eq!(ctx, snap_pt._Y, 0.0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, SketchModelSpaceTessellationCaching, |ctx| {
    use crate::fascia::sketch_canvas::SketchCanvasState;
    use crate::fenst::sketch::SketchPoint;
    use crate::silo::Stash;

    let mut state = SketchCanvasState::New();
    // Populate curve cache for entity 1 and 2
    let mut s1 = Stash::New();
    s1.Push(SketchPoint::New(0.0, 0.0));
    s1.Push(SketchPoint::New(1.0, 1.0));
    let mut s2 = Stash::New();
    s2.Push(SketchPoint::New(2.0, 2.0));
    s2.Push(SketchPoint::New(3.0, 3.0));
    state.InsertCachedCurve(1, 0, s1.IntoBuff());
    state.InsertCachedCurve(2, 0, s2.IntoBuff());
    jeeves_assert_eq!(ctx, state.CurveCacheCount(), 2);

    // InvalidateAll (pan/zoom redraw) does NOT clear curve cache
    state.InvalidateAll();
    jeeves_assert_eq!(ctx, state.CurveCacheCount(), 2);

    // InvalidateGeometry (geometry changed) clears curve cache
    state.InvalidateGeometry();
    jeeves_assert_eq!(ctx, state.CurveCacheCount(), 0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, SketchAsyncJobBridgeAndStaleGenerationCancellation, |ctx| {
    use crate::fascia::app::{AppMessage, AppState};
    use crate::fascia::sketch_job::{ExecuteSketchJob, SketchJobKind, SketchJobOutcome};
    use crate::fenst::sketch::SketchPoint;
    use crate::fleck::RevolveAxis;
    use std::sync::atomic::Ordering;

    let mut app = AppState::new();
    let _ = app.update(AppMessage::NewSketch);
    let tab_id = app.tab_manager.active_tab().unwrap().id;

    // Draw a rectangle from (10, 0) to (30, 20)
    {
        let viewer = app.SketchMut(tab_id).unwrap();
        viewer.doc.AddLine(SketchPoint::New(10.0, 0.0), SketchPoint::New(30.0, 0.0)).unwrap();
        viewer.doc.AddLine(SketchPoint::New(30.0, 0.0), SketchPoint::New(30.0, 20.0)).unwrap();
        viewer.doc.AddLine(SketchPoint::New(30.0, 20.0), SketchPoint::New(10.0, 20.0)).unwrap();
        viewer.doc.AddLine(SketchPoint::New(10.0, 20.0), SketchPoint::New(10.0, 0.0)).unwrap();
    }

    // 1. Begin job 1 (Revolve)
    let (job_gen1, token1, snap1) = app.SketchMut(tab_id).unwrap().BeginJob();
    let outcome1 = iced::futures::executor::block_on(ExecuteSketchJob(
        snap1,
        SketchJobKind::Revolve { axis: RevolveAxis::Y, angle_deg: 360.0 },
        token1.clone(),
    ));
    jeeves_assert!(ctx, matches!(outcome1, SketchJobOutcome::Revolve { ref asset, .. } if asset.is_ok()));

    // 2. Modify sketch by beginning job 2 before job 1 completes (superseding job 1)
    let (job_gen2, token2, snap2) = app.SketchMut(tab_id).unwrap().BeginJob();
    jeeves_assert_eq!(ctx, job_gen2, 2);
    jeeves_assert!(ctx, token1.load(Ordering::Acquire)); // Token 1 cancelled!

    // 3. Complete job 1 with stale generation and cancelled token
    let tabs_before = app.tab_manager.tabs().Len();
    let _ = app.update(AppMessage::SketchJobCompleted {
        tab_id,
        generation: job_gen1,
        cancellation: token1,
        snapshot_rev: 1,
        outcome: outcome1,
    });
    jeeves_assert_eq!(ctx, app.tab_manager.tabs().Len(), tabs_before); // Stale job discarded!

    // 4. Complete job 2 with valid generation and token
    let outcome2 = iced::futures::executor::block_on(ExecuteSketchJob(
        snap2,
        SketchJobKind::Wireframe,
        token2.clone(),
    ));
    let _ = app.update(AppMessage::SketchJobCompleted {
        tab_id,
        generation: job_gen2,
        cancellation: token2,
        snapshot_rev: 2,
        outcome: outcome2,
    });
    jeeves_assert_eq!(ctx, app.tab_manager.tabs().Len(), tabs_before + 1); // Opened 3D wireframe tab!
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, SketchAsyncOrderedSaveMarksCleanOnlyIfRevisionMatches, |ctx| {
    use crate::fascia::app::{AppMessage, AppState};
    use crate::fascia::sketch_job::{ExecuteSketchJob, SketchJobKind, SketchJobOutcome};
    use crate::fenst::sketch::SketchPoint;

    let mut app = AppState::new();
    let _ = app.update(AppMessage::NewSketch);
    let tab_id = app.tab_manager.active_tab().unwrap().id;
    let save_path = std::env::temp_dir().join(format!("test-async-save-{}.flx", std::process::id()));

    // 1. Add line at revision 1
    {
        let viewer = app.SketchMut(tab_id).unwrap();
        viewer.doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(50.0, 0.0)).unwrap();
        jeeves_assert_eq!(ctx, viewer.doc.Revision(), 1);
    }

    // 2. Begin save for snapshot at revision 1
    let (job_gen, token, snap) = app.SketchMut(tab_id).unwrap().BeginJob();
    let snap_rev = snap.revision;
    jeeves_assert_eq!(ctx, snap_rev, 1);

    let outcome = iced::futures::executor::block_on(ExecuteSketchJob(
        snap,
        SketchJobKind::Save(save_path.clone()),
        token.clone(),
    ));
    jeeves_assert!(ctx, matches!(outcome, SketchJobOutcome::Save { ref metadata, .. } if metadata.is_ok()));

    // 3. User edits sketch WHILE save was executing (revision increases to 2)
    {
        let viewer = app.SketchMut(tab_id).unwrap();
        viewer.doc.AddLine(SketchPoint::New(50.0, 0.0), SketchPoint::New(50.0, 50.0)).unwrap();
        jeeves_assert_eq!(ctx, viewer.doc.Revision(), 2);
    }

    // 4. Complete save for snapshot revision 1: document must remain dirty!
    let _ = app.update(AppMessage::SketchJobCompleted {
        tab_id,
        generation: job_gen,
        cancellation: token,
        snapshot_rev: snap_rev,
        outcome,
    });
    let viewer = app.Sketch(tab_id).unwrap();
    jeeves_assert!(ctx, viewer.IsDirty()); // Remained dirty because rev 2 != snap_rev 1!
    jeeves_assert_eq!(ctx, viewer.path.as_ref().unwrap(), &save_path);

    // 5. Subsequent save at current revision marks document clean
    let (job_gen2, token2, snap2) = app.SketchMut(tab_id).unwrap().BeginJob();
    let snap_rev2 = snap2.revision;
    jeeves_assert_eq!(ctx, snap_rev2, 2);

    let outcome2 = iced::futures::executor::block_on(ExecuteSketchJob(
        snap2,
        SketchJobKind::Save(save_path.clone()),
        token2.clone(),
    ));
    let _ = app.update(AppMessage::SketchJobCompleted {
        tab_id,
        generation: job_gen2,
        cancellation: token2,
        snapshot_rev: snap_rev2,
        outcome: outcome2,
    });
    let viewer2 = app.Sketch(tab_id).unwrap();
    jeeves_assert!(ctx, !viewer2.IsDirty()); // Clean now!

    let _ = std::fs::remove_file(save_path);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, SketchAsyncJobCancelledTokenSkipsExecution, |ctx| {
    use crate::fascia::sketch_job::{ExecuteSketchJob, SketchDocumentSnapshot, SketchJobKind, SketchJobOutcome};
    use crate::fenst::sketch::SketchDocument;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    let doc = SketchDocument::New();
    let snap = SketchDocumentSnapshot::New(&doc, None, None, "Test".into());
    let token = Arc::new(AtomicBool::new(true)); // Already cancelled

    let outcome = iced::futures::executor::block_on(ExecuteSketchJob(
        snap,
        SketchJobKind::Wireframe,
        token,
    ));
    jeeves_assert!(ctx, matches!(outcome, SketchJobOutcome::Cancelled));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fascia, CaskViewerHierarchicalNavigationAndLiveProbing, |ctx| {
    use crate::fascia::cask_view::{Cask2dViewerState, LogicLevel, SchematicProbeState};

    // 1. Verify Probe State and Logic Level Colors
    let mut probe = SchematicProbeState::New();
    jeeves_assert!( ctx, !probe.IsOverlayActive());
    probe.ToggleOverlay();
    jeeves_assert!( ctx, probe.IsOverlayActive());

    probe.SetSignal( "clk", LogicLevel::One);
    probe.SetSignal( "rst_n", LogicLevel::Zero);
    probe.SetSignal( "data", LogicLevel::HighZ);
    probe.SetSignal( "bus[0]", LogicLevel::Unknown);

    jeeves_assert_eq!( ctx, probe.GetSignal( "clk" ), Some( LogicLevel::One));
    jeeves_assert_eq!( ctx, probe.GetSignal( "rst_n" ), Some( LogicLevel::Zero));
    jeeves_assert_eq!( ctx, probe.GetSignal( "data" ), Some( LogicLevel::HighZ));
    jeeves_assert_eq!( ctx, probe.GetSignal( "bus[0]" ), Some( LogicLevel::Unknown));
    jeeves_assert_eq!( ctx, probe.GetSignal( "unconnected" ), None);

    let clkCol = probe.GetSignal( "clk" ).unwrap().Color();
    jeeves_assert!( ctx, clkCol.g > clkCol.r && clkCol.g > clkCol.b, "High signal must be green");

    let rstCol = probe.GetSignal( "rst_n" ).unwrap().Color();
    jeeves_assert!( ctx, rstCol.b > rstCol.r, "Low signal must be blue");

    let undefCol = probe.GetSignal( "bus[0]" ).unwrap().Color();
    jeeves_assert!( ctx, undefCol.r > undefCol.g && undefCol.r > undefCol.b, "Undefined signal must be red");

    // 2. Verify Hierarchical Navigation and Breadcrumb Stack
    let mut viewer = Cask2dViewerState::FromUri( "xplr://settings".to_string());
    jeeves_assert!( ctx, !viewer.CanPop());
    let initialCrumbs = viewer.Breadcrumbs();
    jeeves_assert!( ctx, !initialCrumbs.is_empty());
    jeeves_assert_eq!( ctx, initialCrumbs[0].Name(), "settings");

    viewer.zoom = 1.5;
    viewer.pan = [ 100.0, 200.0 ];

    viewer.PushHierarchy( "appearance");
    jeeves_assert!( ctx, viewer.CanPop());
    jeeves_assert_eq!( ctx, viewer.uri, "xplr://settings/appearance");

    let pushedCrumbs = viewer.Breadcrumbs();
    jeeves_assert_eq!( ctx, pushedCrumbs.len(), 2);
    jeeves_assert_eq!( ctx, pushedCrumbs[0].Name(), "settings");
    jeeves_assert_eq!( ctx, pushedCrumbs[1].Name(), "appearance");

    // Pop hierarchy restores previous zoom/pan
    let popped = viewer.PopHierarchy();
    jeeves_assert!( ctx, popped);
    jeeves_assert_eq!( ctx, viewer.uri, "xplr://settings");
    jeeves_assert_eq!( ctx, viewer.zoom, 1.5);
    jeeves_assert_eq!( ctx, viewer.pan, [ 100.0, 200.0 ]);
    jeeves_assert!( ctx, !viewer.CanPop());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fascia, WaveformViewerBusExpansionAndDualMeasurementCursors, |ctx| {
    use crate::fascia::waveform::{WaveformAction, WaveformState};
    use crate::fenst::waveform::{CursorMarker, WaveformModel, WaveformRadix, WaveformSignal};
    use crate::silo::{Arr, Buff};
    use std::sync::Arc;

    // 1. Verify WaveformRadix Formatting
    let hex = WaveformRadix::Hex;
    let dec = WaveformRadix::Decimal;
    let bin = WaveformRadix::Binary;
    let asc = WaveformRadix::Ascii;

    jeeves_assert_eq!( ctx, hex.Format( "b10111100", 8), "hBC");
    jeeves_assert_eq!( ctx, dec.Format( "b10111100", 8), "188");
    jeeves_assert_eq!( ctx, bin.Format( "b10111100", 8), "b10111100");
    jeeves_assert_eq!( ctx, asc.Format( "b01000001", 8), "'A'");

    // Indeterminate states
    jeeves_assert_eq!( ctx, hex.Format( "bx01", 3), "x");
    jeeves_assert_eq!( ctx, dec.Format( "bz11", 3), "z");

    // Radix cycling
    jeeves_assert_eq!( ctx, hex.Next(), WaveformRadix::Decimal);
    jeeves_assert_eq!( ctx, dec.Next(), WaveformRadix::Binary);
    jeeves_assert_eq!( ctx, bin.Next(), WaveformRadix::Ascii);
    jeeves_assert_eq!( ctx, asc.Next(), WaveformRadix::Hex);

    // 2. Mock Waveform Model for Dual Cursor Testing
    struct MockSignal {
        _Changes: Buff<(u64, String)>,
    }
    impl WaveformSignal for MockSignal {
        fn Scope(&self) -> &str { "top" }
        fn Name(&self) -> &str { "bus[7:0]" }
        fn FullName(&self) -> &str { "top.bus[7:0]" }
        fn Bits(&self) -> u32 { 8 }
        fn Id(&self) -> &str { "b" }
        fn Type(&self) -> &str { "reg" }
        fn Changes(&self) -> Arr<'_, (u64, String)> { self._Changes.Arr() }
        fn ChangesBetween(&self, _s: u64, _e: u64) -> Arr<'_, (u64, String)> { self._Changes.Arr() }
        fn ValueAt(&self, time: u64) -> &str {
            if time >= 50 { "b10101010" } else { "b00000000" }
        }
        fn PreviousChange(&self, _time: u64) -> Option<u64> { Some(0) }
        fn NextChange(&self, _time: u64) -> Option<u64> { Some(50) }
    }

    struct MockModel {
        _Signal: MockSignal,
    }
    impl WaveformModel for MockModel {
        fn TimeMin(&self) -> u64 { 0 }
        fn TimeMax(&self) -> u64 { 1000 }
        fn SignalCount(&self) -> u32 { 1 }
        fn Signal(&self, _index: u32) -> Option<&dyn WaveformSignal> { Some(&self._Signal) }
        fn FindSignal(&self, _name: &str) -> u32 { 0 }
        fn Timescale(&self) -> &str { "1ns" }
    }

    let mut stash = crate::silo::Stash::New();
    stash.Push((0, "b00000000".to_string()));
    stash.Push((50, "b10101010".to_string()));

    let model = Arc::new(MockModel {
        _Signal: MockSignal {
            _Changes: stash.IntoBuff(),
        },
    });

    let mut state = WaveformState::New(model);
    jeeves_assert_eq!( ctx, state.CursorA(), 0);
    jeeves_assert_eq!( ctx, state.CursorB(), 1000);
    jeeves_assert_eq!( ctx, state.DeltaTime(), 1000);
    jeeves_assert_eq!( ctx, state.ActiveMarker(), CursorMarker::MarkerA);

    // Set cursor markers
    state.Update( WaveformAction::SetCursorA( 100));
    state.Update( WaveformAction::SetCursorB( 150));
    jeeves_assert_eq!( ctx, state.CursorA(), 100);
    jeeves_assert_eq!( ctx, state.CursorB(), 150);
    jeeves_assert_eq!( ctx, state.DeltaTime(), 50);

    // Frequency calculation: ΔT = 50ns -> 20 MHz (20,000,000 Hz)
    let freq = state.Frequency().unwrap();
    jeeves_assert!( ctx, (freq - 20_000_000.0).abs() < 1.0, "Frequency should be 20 MHz");

    // Toggle active marker
    state.Update( WaveformAction::ToggleActiveMarker);
    jeeves_assert_eq!( ctx, state.ActiveMarker(), CursorMarker::MarkerB);
    jeeves_assert_eq!( ctx, state.CursorTime(), 150);

    // Cycle radix through state action
    jeeves_assert_eq!( ctx, state.Radix(), WaveformRadix::Hex);
    state.Update( WaveformAction::CycleRadix);
    jeeves_assert_eq!( ctx, state.Radix(), WaveformRadix::Decimal);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, FrescoViewerOwnershipFailureAndViewport, |ctx| {
    use crate::fascia::app::AppState;
    use crate::fascia::math_view::FrescoAction;
    use crate::fenst::{FrescoDocument, FrescoRoot};
    use crate::fresco::ExprRepos;
    use crate::silo::{Stash, USeg};

    let mut repos = ExprRepos::New();
    let x = repos.VarCreate("x".into(), true);
    let mut deep = x;
    USeg::FromLen(70).Traverse(|_| { deep = repos.AddCreate(deep, x); });
    let mut roots = Stash::New();
    roots.Push(FrescoRoot::Scalar("small", x));
    roots.Push(FrescoRoot::Scalar("too deep for layout", deep));
    let doc = FrescoDocument::New(repos, roots.IntoBuff()).unwrap();
    let docId = doc.Id();
    let mut app = AppState::new();
    app.close_all_tabs_internal();
    let id = app.OpenFrescoDocument(doc);
    let viewer = app.FrescoMut(id).unwrap();
    jeeves_assert_eq!(ctx, viewer.Document().Id(), docId);
    viewer.Update(FrescoAction::Resize(320.0, 180.0));
    jeeves_assert_eq!(ctx, viewer.ViewportSize(), [320.0, 180.0]);
    viewer.Update(FrescoAction::FitView);
    let bounds = viewer.Layout().unwrap().Bounds();
    let pan = viewer.Pan();
    let zoom = viewer.Zoom();
    jeeves_assert!(ctx, pan[0] + bounds.x * zoom >= 0.0);
    jeeves_assert!(ctx, pan[1] + bounds.y * zoom >= 0.0);
    jeeves_assert!(ctx, pan[0] + (bounds.x + bounds.width) * zoom <= 320.0);
    jeeves_assert!(ctx, pan[1] + (bounds.y + bounds.height) * zoom <= 180.0);
    let pointer = [150.0, 80.0];
    let before = [(pointer[0] - pan[0]) / zoom, (pointer[1] - pan[1]) / zoom];
    viewer.Update(FrescoAction::Zoom(0.8, Some(pointer)));
    jeeves_assert!(ctx, ((pointer[0] - viewer.Pan()[0]) / viewer.Zoom() - before[0]).abs() < 1e-4);
    jeeves_assert!(ctx, ((pointer[1] - viewer.Pan()[1]) / viewer.Zoom() - before[1]).abs() < 1e-4);
    viewer.Update(FrescoAction::SelectOccurrence(0));
    jeeves_assert_eq!(ctx, viewer.SelectedOccurrence().unwrap()._ExprId, x);
    viewer.Update(FrescoAction::SelectRoot(1));
    jeeves_assert_eq!(ctx, viewer.SelectedRoot(), 1);
    jeeves_assert!(ctx, viewer.Layout().is_none());
    jeeves_assert!(ctx, viewer.LayoutError().is_some());
    jeeves_assert!(ctx, viewer.SelectedOccurrence().is_none());
    jeeves_assert_eq!(ctx, viewer.HitTestDoc(1.0, 1.0), u32::MAX);
    viewer.Update(FrescoAction::SelectRoot(0));
    jeeves_assert!(ctx, viewer.Layout().is_some());
    jeeves_assert!(ctx, viewer.LayoutError().is_none());

    let second = app.OpenFrescoDocument(FrescoDocument::CreateExample());
    let third = app.OpenFrescoDocument(FrescoDocument::CreateExample());
    app.close_tab_internal(1);
    jeeves_assert!(ctx, app.Fresco(id).is_some());
    jeeves_assert!(ctx, app.Fresco(second).is_none());
    jeeves_assert!(ctx, app.Fresco(third).is_some());
    jeeves_assert_eq!(ctx, app.FrescoCount(), 2);
    app.close_all_tabs_internal();
    jeeves_assert_eq!(ctx, app.FrescoCount(), 0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, FrescoTextMetricsMatchCanvasAndTheme, |ctx| {
    use crate::fascia::math_text::CosmicMathFontMetrics;
    use crate::fascia::math_view::MathColorForPalette;
    use crate::fascia::theme::FasciaTheme;
    use crate::fenst::{IMathFontMetrics, MathColor};
    use crate::silo::USeg;
    use iced::advanced::text::Paragraph as _;

    let metrics = CosmicMathFontMetrics::New();
    let names = ["x", "WideVariable", "\u{3b1}\u{3b2}", "\u{2211} + x"];
    USeg::FromLen(names.len() as u32).Traverse(|i| {
        USeg::FromLen(4).Traverse(|style| {
            let bold = style & 1 != 0;
            let italic = style & 2 != 0;
            let base = metrics.MeasureAtom(names[i as usize], 20.0, bold, italic);
            USeg::FromLen(3).Traverse(|scaleIndex| {
                let scale = [0.5, 1.0, 2.0][scaleIndex as usize];
                let atom = CosmicMathFontMetrics::CanvasText(names[i as usize].to_string(), 20.0 * scale, bold, italic);
                let paragraph = CosmicMathFontMetrics::Paragraph(&atom);
                let baseline = paragraph.buffer().layout_runs().next().unwrap().line_y;
                let measured = metrics.MeasureAtom(names[i as usize], 20.0 * scale, bold, italic);
                jeeves_assert!(ctx, (paragraph.min_width() - measured.width).abs() < 1e-4);
                jeeves_assert!(ctx, (baseline - measured.paragraphBaselineOffset).abs() < 1e-4);
                jeeves_assert!(ctx, (baseline - base.paragraphBaselineOffset * scale).abs() < 0.01);
                jeeves_assert_eq!(ctx, atom.font, CosmicMathFontMetrics::Font(bold, italic));
                jeeves_assert_eq!(ctx, atom.shaping, iced::widget::text::Shaping::Advanced);
            });
        });
    });
    [FasciaTheme::WindowsLight, FasciaTheme::WindowsDark, FasciaTheme::LinuxLight, FasciaTheme::LinuxDark]
        .into_iter().for_each(|theme| {
            let palette = theme.palette();
            jeeves_assert_eq!(ctx, MathColorForPalette(MathColor::Text, palette), palette.text_primary);
            jeeves_assert_eq!(ctx, MathColorForPalette(MathColor::Operator, palette), palette.accent);
            jeeves_assert_eq!(ctx, MathColorForPalette(MathColor::Muted, palette), palette.text_secondary);
        });
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, WorkbenchStoreOrdersOwnsAndCancelsPayloads, |ctx| {
    use crate::fascia::document::DocumentStore;
    use crate::fascia::geometry_view::GeometryViewerState;
    use crate::fascia::{PythonConsoleState, SketchViewerState, TabDocument};
    use iced::widget::text_editor;
    use std::sync::atomic::Ordering;

    let sketchToken;
    let geometryToken;
    {
        let mut store = DocumentStore::New();
        store.InsertEditor(TabId(30), text_editor::Content::with_text("last"));
        store.InsertEditor(TabId(10), text_editor::Content::with_text("first"));
        let geometry = GeometryViewerState::default();
        let oldToken = geometry.Cancellation();
        store.InsertGeometry(TabId(20), geometry);
        jeeves_assert_eq!(ctx, store.Entries()[0].0, TabId(10));
        jeeves_assert_eq!(ctx, store.Entries()[1].0, TabId(20));
        jeeves_assert_eq!(ctx, store.Entries()[2].0, TabId(30));
        jeeves_assert!(ctx, store.Sketch(TabId(20)).is_none());

        let replacement = GeometryViewerState::default();
        geometryToken = replacement.Cancellation();
        store.InsertGeometry(TabId(20), replacement);
        jeeves_assert!(ctx, oldToken.load(Ordering::Acquire));
        jeeves_assert!(ctx, !geometryToken.load(Ordering::Acquire));
        jeeves_assert_eq!(ctx, store.Len(), 3);

        let sketch = SketchViewerState::New(None, "job".into(), FasciaTheme::default().palette());
        sketchToken = sketch.job_cancellation.clone();
        store.InsertSketch(TabId(25), sketch);
        let mut console = PythonConsoleState::new();
        console.is_running = true;
        store.InsertConsole(TabId(40), console);
        let removed = store.Remove(TabId(40)).unwrap();
        let TabDocument::PythonConsole(console) = removed else {
            panic!("console expected");
        };
        jeeves_assert!(ctx, !console.is_running);
        let _ = store.Remove(TabId(10));
        jeeves_assert_eq!(ctx, store.Entries()[0].0, TabId(20));
        jeeves_assert_eq!(ctx, store.Entries()[1].0, TabId(25));
        jeeves_assert_eq!(ctx, store.Entries()[2].0, TabId(30));
        jeeves_assert!(ctx, !sketchToken.load(Ordering::Acquire));
    }
    jeeves_assert!(ctx, sketchToken.load(Ordering::Acquire));
    jeeves_assert!(ctx, geometryToken.load(Ordering::Acquire));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, WorkbenchEditorOwnsDirtyState, |ctx| {
    use crate::fascia::IWorkbenchDocument;
    use iced::widget::text_editor;

    let mut app = AppState::new();
    let _ = app.update(AppMessage::NewFile);
    let id = app.tab_manager.active_tab().unwrap().id;
    jeeves_assert_eq!(ctx, app.DocumentCount(), 1);
    jeeves_assert!(ctx, !app.Document(id).unwrap().IsDirty());
    let _ = app.update(AppMessage::EditorAction(text_editor::Action::Edit(text_editor::Edit::Insert('x'))));
    jeeves_assert!(ctx, app.Document(id).unwrap().IsDirty());
    jeeves_assert!(ctx, app.tab_manager.active_tab().unwrap().is_dirty);
    let _ = app.update(AppMessage::SaveCurrentFile);
    jeeves_assert!(ctx, !app.Document(id).unwrap().IsDirty());
    jeeves_assert!(ctx, !app.tab_manager.active_tab().unwrap().is_dirty);
    let _ = app.update(AppMessage::NewSketch);
    let sketchId = app.tab_manager.active_tab().unwrap().id;
    jeeves_assert_eq!(ctx, app.DocumentCount(), 2);
    jeeves_assert!(ctx, app.Editor(sketchId).is_none());
    jeeves_assert!(ctx, app.Sketch(id).is_none());
    jeeves_assert!(ctx, app.Editor(id).unwrap().text().contains('x'));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia,
             WorkbenchRejectsOrphanDuplicateAndMismatchedPayloads,
             |ctx| {
                 use crate::fascia::geometry_view::GeometryViewerState;
                 use crate::fascia::{SketchViewerState, TabDocument};
                 use std::sync::atomic::Ordering;

                 let mut app = AppState::new();
                 let orphan = GeometryViewerState::default();
                 let orphanToken = orphan.Cancellation();
                 jeeves_assert!(ctx,
                                app.SetDocument(TabId(u64::MAX),
                                                TabDocument::Geometry(Box::new(orphan)))
                                   .is_err());
                 jeeves_assert!(ctx, orphanToken.load(Ordering::Acquire));
                 let welcome = app.tab_manager.active_tab().unwrap().id;
                 let wrongKind = GeometryViewerState::default();
                 let wrongToken = wrongKind.Cancellation();
                 jeeves_assert!(ctx,
                                app.SetDocument(welcome,
                                                TabDocument::Geometry(Box::new(wrongKind)))
                                   .is_err());
                 jeeves_assert!(ctx, wrongToken.load(Ordering::Acquire));
                 jeeves_assert_eq!(ctx, app.DocumentCount(), 0);

                 let _ = app.update(AppMessage::NewSketch);
                 let id = app.tab_manager.active_tab().unwrap().id;
                 let originalToken = app.Sketch(id).unwrap().job_cancellation.clone();
                 let candidate = SketchViewerState::New(None, "replacement".into(), app.palette());
                 let candidateToken = candidate.job_cancellation.clone();
                 jeeves_assert!(ctx,
                                app.SetDocument(id, TabDocument::Sketch(Box::new(candidate)))
                                   .is_err());
                 jeeves_assert!(ctx, candidateToken.load(Ordering::Acquire));
                 jeeves_assert!(ctx, !originalToken.load(Ordering::Acquire));
                 jeeves_assert_eq!(ctx, app.DocumentCount(), 1);
             });

//-------------------------------------------------------------------------------------------------

jeeves_test!(
             Fascia,
             WorkbenchCancelledClosePreservesJobsAndLateResultsStayClosed,
             |ctx| {
                 use crate::fascia::TabDocument;
                 use crate::fascia::app::SketchCloseDecision;
                 use crate::fascia::geometry_view::GeometryViewerState;
                 use crate::fascia::sketch_job::SketchJobOutcome;
                 use crate::fleck::SketchPoint;
                 use std::sync::atomic::Ordering;

                 let mut app = AppState::new();
                 let _ = app.update(AppMessage::NewSketch);
                 let sketchId = app.tab_manager.active_tab().unwrap().id;
                 let sketchIndex = app.tab_manager.active_index().unwrap();
                 app.SketchMut(sketchId)
                    .unwrap()
                    .doc
                    .AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(2.0, 1.0))
                    .unwrap();
                 let (generation, sketchToken, snapshot) =
                     app.SketchMut(sketchId).unwrap().BeginJob();
                 let (_, _, geometryId) = app.tab_manager.open_geometry_tab("loading".into());
                 let geometry = GeometryViewerState::default();
                 let geometryToken = geometry.Cancellation();
                 app.SetDocument(geometryId, TabDocument::Geometry(Box::new(geometry)))
                    .unwrap();

                 let _ = app.update(AppMessage::CloseTab(sketchIndex));
                 jeeves_assert!(ctx, !sketchToken.load(Ordering::Acquire));
                 let _ = app.update(AppMessage::ResolveSketchClose(SketchCloseDecision::Cancel));
                 jeeves_assert_eq!(ctx, app.DocumentCount(), 2);
                 jeeves_assert!(ctx, app.Sketch(sketchId).is_some());
                 jeeves_assert!(ctx, !geometryToken.load(Ordering::Acquire));
                 let _ = app.update(AppMessage::CloseTab(sketchIndex));
                 let _ = app.update(AppMessage::ResolveSketchClose(SketchCloseDecision::Discard));
                 jeeves_assert!(ctx, sketchToken.load(Ordering::Acquire));
                 jeeves_assert_eq!(ctx, app.DocumentCount(), 1);
                 let status = app.status_info.message.clone();
                 // Simulate a result already queued before cancellation was observed.
                 sketchToken.store(false, Ordering::Release);
                 let _ = app.update(AppMessage::SketchJobCompleted {
        tab_id: sketchId,
        generation,
        cancellation: sketchToken,
        snapshot_rev: snapshot.revision,
        outcome: SketchJobOutcome::Wireframe(Err("late sketch result".into())),
    });
                 jeeves_assert_eq!(ctx, app.status_info.message, status);
                 jeeves_assert!(ctx, app.Sketch(sketchId).is_none());

                 let geometryIndex =
                     app.tab_manager
                        .tabs()
                        .USeg()
                        .BinarySearch(|at| app.tab_manager.tabs()[at].id.0.cmp(&geometryId.0))
                        .unwrap();
                 let _ = app.update(AppMessage::CloseTab(geometryIndex as usize));
                 jeeves_assert!(ctx, geometryToken.load(Ordering::Acquire));
                 let status = app.status_info.message.clone();
                 geometryToken.store(false, Ordering::Release);
                 let _ =
                     app.update(AppMessage::GeometryPrepared(geometryId,
                                                             geometryToken,
                                                             Err("late geometry result".into())));
                 jeeves_assert_eq!(ctx, app.status_info.message, status);
                 jeeves_assert_eq!(ctx, app.DocumentCount(), 0);

                 let _ = app.update(AppMessage::NewSketch);
                 let sketchId = app.tab_manager.active_tab().unwrap().id;
                 app.SketchMut(sketchId)
                    .unwrap()
                    .doc
                    .AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(1.0, 1.0))
                    .unwrap();
                 let _ = app.update(AppMessage::CloseActiveTab);
                 app.close_all_tabs_internal();
                 let _ = app.update(AppMessage::NewFile);
                 jeeves_assert_eq!(ctx, app.DocumentCount(), 1);
                 jeeves_assert_eq!(ctx, app.EditorCount(), 1);
             }
);

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, FrescoViewerTabAndLifecycle, |ctx| {
    use crate::fascia::app::{AppMessage, AppState};
    use crate::fascia::math_view::FrescoAction;
    use crate::fascia::tabs::TabKind;

    let mut app = AppState::new();
    app.close_all_tabs_internal();

    // 1. Open New Math View
    let _ = app.update(AppMessage::NewMathView);
    jeeves_assert_eq!(ctx, app.tab_manager.tabs().Len(), 1);

    let active = app.tab_manager.active_tab().unwrap();
    jeeves_assert_eq!(ctx, active.kind, TabKind::FrescoViewer);
    jeeves_assert_eq!(ctx, active.title, "Math View");
    jeeves_assert_eq!(ctx, app.status_info.language, "Math View");

    let tab_id = active.id;
    jeeves_assert!(ctx, app.Fresco(tab_id).is_some());

    // 2. State Actions: Next Root, Pan, Zoom
    let viewer = app.FrescoMut(tab_id).unwrap();
    jeeves_assert_eq!(ctx, viewer.SelectedRoot(), 0);
    jeeves_assert!(ctx, viewer.RootCount() >= 6);

    viewer.Update(FrescoAction::NextRoot);
    jeeves_assert_eq!(ctx, viewer.SelectedRoot(), 1);

    let old_pan = viewer.Pan();
    viewer.Update(FrescoAction::Pan(20.0, 30.0));
    jeeves_assert_eq!(ctx, viewer.Pan()[0], old_pan[0] + 20.0);
    jeeves_assert_eq!(ctx, viewer.Pan()[1], old_pan[1] + 30.0);

    let old_zoom = viewer.Zoom();
    viewer.Update(FrescoAction::Zoom(1.2, None));
    jeeves_assert!(ctx, (viewer.Zoom() - old_zoom * 1.2).abs() < 1e-4);

    viewer.Update(FrescoAction::ResetView);
    jeeves_assert_eq!(ctx, viewer.Zoom(), 1.0);
    jeeves_assert_eq!(ctx, viewer.Pan(), [40.0, 60.0]);

    // 3. Selection & Hit Testing
    let occ0 = viewer.SelectedOccurrence();
    jeeves_assert!(ctx, occ0.is_none());

    viewer.Update(FrescoAction::SelectOccurrence(0));
    jeeves_assert_eq!(ctx, viewer.SelectedOccurrenceId(), 0);
    jeeves_assert!(ctx, viewer.SelectedOccurrence().is_some());

    // 4. Teardown via close_tab_internal
    let closed = app.close_tab_internal(0);
    jeeves_assert!(ctx, closed.is_some());
    jeeves_assert_eq!(ctx, app.tab_manager.tabs().Len(), 0);
    jeeves_assert!(ctx, app.Fresco(tab_id).is_none());

    // 5. Open and Close All Tabs
    let _ = app.update(AppMessage::NewMathView);
    let _ = app.update(AppMessage::NewMathView);
    jeeves_assert_eq!(ctx, app.tab_manager.tabs().Len(), 2);
    jeeves_assert_eq!(ctx, app.FrescoCount(), 2);

    app.close_all_tabs_internal();
    jeeves_assert_eq!(ctx, app.tab_manager.tabs().Len(), 0);
    jeeves_assert_eq!(ctx, app.FrescoCount(), 0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, FrescoInspectorAndThemePaletteAdherence, |ctx| {
    use crate::fascia::math_inspector::ViewMathInspector;
    use crate::fascia::theme::FasciaTheme;
    use crate::fenst::fresco_document::FrescoDocument;
    use crate::fenst::fresco_properties::{FrescoProperties, FrescoTermValue};

    let doc = FrescoDocument::CreateExample();

    let layout = crate::fenst::MathLayoutBuilder::New(&doc, &crate::fenst::MockFontMetrics, 0, 20.0).Build().unwrap();
    let matrixLayout = crate::fenst::MathLayoutBuilder::New(&doc, &crate::fenst::MockFontMetrics, 4, 20.0).Build().unwrap();
    [
        FasciaTheme::WindowsDark,
        FasciaTheme::WindowsLight,
        FasciaTheme::LinuxDark,
        FasciaTheme::LinuxLight,
    ].into_iter().for_each(|theme| {
        let palette = theme.palette();

        // Render inspector with no occurrence selected
        let _elem_empty: iced::Element<'_, ()> = ViewMathInspector(&doc, 0, u32::MAX, None, palette);

        // Render inspector with occurrence selected
        let _elem_sel: iced::Element<'_, ()> =
            ViewMathInspector(&doc, 0, 0, Some(&layout), palette);

        // Render inspector for a tensor root (root index 4 is Matrix_A)
        let _elem_tensor: iced::Element<'_, ()> =
            ViewMathInspector(&doc, 4, 0, Some(&matrixLayout), palette);
    });

    jeeves_assert_eq!(ctx, layout.OccurrenceAt(0).unwrap()._ExprId, doc.RootAt(0).unwrap().PrimaryExprId());
    let properties = FrescoProperties::Read(&doc, 4, matrixLayout.OccurrenceAt(0)).unwrap();
    jeeves_assert_eq!(ctx, properties.Name, "Matrix_A");
    jeeves_assert_eq!(ctx, properties.Shape.unwrap().Rank(), 2);
    let term = properties.Term.unwrap();
    jeeves_assert_eq!(ctx, term.Coordinates.unwrap().to_string(), "[0, 0]");
    jeeves_assert!(ctx, matches!(term.Value, Ok(FrescoTermValue::Variable { name: "A[0,0]", .. })));
    let otherRoot = FrescoProperties::Read(&doc, 0, matrixLayout.OccurrenceAt(0)).unwrap();
    jeeves_assert!(ctx, otherRoot.Term.is_none());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fascia, FrescoViewerZoomLimitsInkBoundsAndWheelDelta, |ctx| {
    use crate::fascia::app::AppState;
    use crate::fascia::math_text::CosmicMathFontMetrics;
    use crate::fascia::math_view::{
        FrescoAction, FrescoCanvasInteraction, FrescoCanvasProgram,
    };
    use crate::fenst::{FrescoDocument, FrescoRoot, IMathFontMetrics};
    use crate::fresco::ExprRepos;
    use crate::silo::{Arr, Buff, Stash, USeg};
    use iced::advanced::text::Paragraph as _;
    use iced::mouse;
    use iced::widget::canvas::Program;
    use iced::{Point, Rectangle, Size};

    // 1. Issue 3: Glyph ink bounds vs advance width
    let metrics = CosmicMathFontMetrics::New();
    ["f", "j", "\u{00c1}"].into_iter().for_each(|name| {
        let m = metrics.MeasureAtom(name, 20.0, false, true);
        let atom = CosmicMathFontMetrics::CanvasText(name.to_string(), 20.0, false, true);
        let paragraph = CosmicMathFontMetrics::Paragraph(&atom);
        jeeves_assert!(ctx, (m.width - paragraph.min_width()).abs() < 1e-4);
        let baseline = paragraph.buffer().layout_runs().next().unwrap().line_y;
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        let mut swash = cosmic_text::SwashCache::new();
        paragraph.buffer().layout_runs().for_each(|run| {
            run.glyphs.iter().for_each(|glyph| {
                let physical = glyph.physical((0.0, 0.0), 1.0);
                if let Some(image) = swash.get_image(fonts.raw(), physical.cache_key) {
                    let x = physical.x as f32 + image.placement.left as f32;
                    let y = run.line_y + physical.y as f32 - image.placement.top as f32 - baseline;
                    jeeves_assert!(ctx, x >= m.inkBounds.x - 1e-4);
                    jeeves_assert!(ctx, y >= m.inkBounds.y - 1e-4);
                    jeeves_assert!(ctx, x + image.placement.width as f32 <= m.inkBounds.Right() + 1e-4);
                    jeeves_assert!(ctx, y + image.placement.height as f32 <= m.inkBounds.Bottom() + 1e-4);
                }
            });
        });
    });

    // 2. Issue 2: Zoom Out from small fitted zoom should not jump up to 0.02
    let mut app = AppState::new();
    let mut repos = ExprRepos::New();
    let x = repos.VarCreate("w".repeat(100), true);
    let operands = Buff::FromDispenser(100, |_| x);
    let sum = repos.SumCreate(operands.Arr(), Arr::Empty());
    let mut roots = Stash::New();
    roots.Push(FrescoRoot::Scalar("wide", sum));
    let doc = FrescoDocument::New(repos, roots.IntoBuff()).unwrap();
    let tab_id = app.OpenFrescoDocument(doc);
    let viewer = app.FrescoMut(tab_id).unwrap();

    // Exercise an actual fitted formula, rather than manufacturing a small zoom.
    viewer.Update(FrescoAction::Resize(320.0, 180.0));
    let initial_zoom = viewer.Zoom();
    jeeves_assert!(ctx, initial_zoom <= 0.02, "Initial zoom set to small scale");

    // Zooming out with factor 0.87
    viewer.Update(FrescoAction::Zoom(0.87, None));
    let zoomed_out = viewer.Zoom();
    jeeves_assert!(
        ctx,
        zoomed_out < initial_zoom,
        "Zooming out must decrease zoom"
    );
    jeeves_assert!(
        ctx,
        zoomed_out < 0.01,
        "Zooming out must not clamp up to 0.02"
    );

    // Zooming in with factor 1.15
    viewer.Update(FrescoAction::Zoom(1.15, None));
    let zoomed_in = viewer.Zoom();
    jeeves_assert!(
        ctx,
        zoomed_in > zoomed_out,
        "Zooming in must increase zoom"
    );

    USeg::FromLen(32).Traverse(|_| {
        let before = viewer.Zoom();
        viewer.Update(FrescoAction::Zoom(0.5, Some([100.0, 80.0])));
        jeeves_assert!(ctx, viewer.Zoom() <= before);
        jeeves_assert!(ctx, viewer.Zoom() >= crate::fascia::math_view::MIN_ZOOM);
        jeeves_assert!(ctx, viewer.Pan()[0].is_finite() && viewer.Pan()[1].is_finite());
    });

    // 3. Issue 6: Horizontal scrolling with zero vertical delta must be ignored
    let palette = crate::fascia::theme::FasciaTheme::WindowsDark.palette();
    let program = FrescoCanvasProgram::New(viewer, palette, |action: FrescoAction| action);
    let mut interaction = FrescoCanvasInteraction::default();
    let bounds = Rectangle::new(Point::ORIGIN, Size::new(800.0, 600.0));
    let cursor = mouse::Cursor::Available(Point::new(100.0, 100.0));

    // Horizontal Lines scroll: y = 0.0, x = 1.0
    let horiz_lines = iced::Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Lines { x: 1.0, y: 0.0 },
    });
    let result = program.update(&mut interaction, &horiz_lines, bounds, cursor);
    jeeves_assert!(ctx, result.is_none(), "Horizontal Lines wheel scroll must be ignored");

    // Horizontal Pixels scroll: y = 0.0, x = 30.0
    let horiz_pixels = iced::Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: 30.0, y: 0.0 },
    });
    let result_pix = program.update(&mut interaction, &horiz_pixels, bounds, cursor);
    jeeves_assert!(ctx, result_pix.is_none(), "Horizontal Pixels wheel scroll must be ignored");

    [mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 }, mouse::ScrollDelta::Pixels { x: 0.0, y: -30.0 }]
        .into_iter().for_each(|delta| {
            let event = iced::Event::Mouse(mouse::Event::WheelScrolled { delta });
            let result = program.update(&mut interaction, &event, bounds, cursor).unwrap();
            let Some(FrescoAction::Zoom(factor, Some(anchor))) = result.into_inner().0 else {
                panic!("Vertical scrolling must publish anchored zoom");
            };
            let positive = matches!(delta, mouse::ScrollDelta::Lines { y, .. } if y > 0.0);
            jeeves_assert_eq!(ctx, factor > 1.0, positive);
            jeeves_assert_eq!(ctx, anchor, [100.0, 100.0]);
        });
});

//-------------------------------------------------------------------------------------------------
