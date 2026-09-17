// src/fascia/_test/mod.rs
use crate::cove::jeeves::{
    segue_assert, segue_assert_eq, segue_console_test, segue_example_test, segue_println,
    segue_test,
};
#[allow(unused_imports)]
use crate::fascia::explorer::{default_initial_dir, detect_system_roots, is_text_file};
#[allow(unused_imports)]
use crate::fascia::{
    ActivityTab, ExplorerAction, ExplorerState, FasciaTheme, FileTreeNode, MenuAction,
    StatusBarInfo, TabBarAction, TabId, TabItem, TabKind, TabManager, ToolBarAction,
    default_code_font, default_system_font,
};
use std::path::PathBuf;

//-------------------------------------------------------------------------------------------------

// TabManager lifecycle and operations
segue_test!(Fascia, TabManagerLifecycle, |ctx| {
    let mut manager = TabManager::new();
    // Initial state should have Welcome tab opened
    segue_assert_eq!(ctx, manager.tabs().len(), 1);
    segue_assert_eq!(ctx, manager.active_index(), Some(0));
    segue_assert_eq!(ctx, manager.active_tab().unwrap().kind, TabKind::Welcome);
    segue_assert_eq!(ctx, manager.active_tab().unwrap().title, "Welcome");
    // Open settings tab
    manager.open_settings();
    segue_assert_eq!(ctx, manager.tabs().len(), 2);
    segue_assert_eq!(ctx, manager.active_index(), Some(1));
    segue_assert_eq!(ctx, manager.active_tab().unwrap().kind, TabKind::Settings);
    // Opening settings again should not duplicate, but focus it
    manager.select_tab(0);
    segue_assert_eq!(ctx, manager.active_index(), Some(0));
    manager.open_settings();
    segue_assert_eq!(ctx, manager.tabs().len(), 2);
    segue_assert_eq!(ctx, manager.active_index(), Some(1));
    // Open a file
    let file_path = PathBuf::from("src/main.rs");
    let (idx, newly_opened) = manager.open_file(file_path.clone());
    segue_assert!(ctx, newly_opened);
    segue_assert_eq!(ctx, idx, 2);
    segue_assert_eq!(ctx, manager.active_index(), Some(2));
    segue_assert_eq!(ctx, manager.active_tab().unwrap().kind, TabKind::FileEditor);
    segue_assert_eq!(ctx, manager.active_tab().unwrap().title, "main.rs");
    // Open same file should focus existing
    let (idx2, newly_opened2) = manager.open_file(file_path);
    segue_assert!(ctx, !newly_opened2);
    segue_assert_eq!(ctx, idx2, 2);
    // Mark dirty
    segue_assert!(ctx, !manager.active_tab().unwrap().is_dirty);
    manager.mark_active_dirty(true);
    segue_assert!(ctx, manager.active_tab().unwrap().is_dirty);
    manager.mark_active_dirty(false);
    segue_assert!(ctx, !manager.active_tab().unwrap().is_dirty);
    // Close tab
    let closed = manager.close_tab(2);
    segue_assert!(ctx, closed.is_some());
    segue_assert_eq!(ctx, closed.unwrap().title, "main.rs");
    segue_assert_eq!(ctx, manager.tabs().len(), 2);
    segue_assert_eq!(ctx, manager.active_index(), Some(1));
    // Close all
    manager.close_all();
    segue_assert_eq!(ctx, manager.tabs().len(), 0);
    segue_assert_eq!(ctx, manager.active_index(), None);
    segue_assert!(ctx, manager.active_tab().is_none());
});

//-------------------------------------------------------------------------------------------------

// Filesystem explorer operations and tree nodes
segue_test!(Fascia, ExplorerTreeOps, |ctx| {
    let roots = detect_system_roots();
    segue_assert!(ctx, !roots.is_empty());
    let init_dir = default_initial_dir();
    segue_assert!(ctx, init_dir.exists());
    let mut state = ExplorerState::new(init_dir.clone());
    state.refresh();
    segue_assert_eq!(ctx, state.root_node.path, init_dir);
    segue_assert!(ctx, state.root_node.is_expanded);
    segue_assert!(ctx, state.root_node.is_dir);
    // Extension text check
    segue_assert!(ctx, is_text_file(&PathBuf::from("test.rs")));
    segue_assert!(ctx, is_text_file(&PathBuf::from("config.toml")));
    segue_assert!(ctx, is_text_file(&PathBuf::from("README.md")));
    // Node icon checks
    let rs_node = FileTreeNode::new(PathBuf::from("lib.rs"), 0);
    segue_assert_eq!(ctx, rs_node.icon(), "🦀");
    let toml_node = FileTreeNode::new(PathBuf::from("Cargo.toml"), 0);
    segue_assert_eq!(ctx, toml_node.icon(), "⚙");
    let dir_node = FileTreeNode::new(PathBuf::from("src"), 0);
    segue_assert_eq!(ctx, dir_node.icon(), "📁");
});

//-------------------------------------------------------------------------------------------------

// Theme palettes and font selection
segue_test!(Fascia, ThemePaletteVariants, |ctx| {
    for &theme in FasciaTheme::ALL {
        let palette = theme.palette();
        if theme.is_dark() {
            segue_assert!(ctx, palette.text_primary.r > 0.5);
        } else {
            segue_assert!(ctx, palette.text_primary.r < 0.5);
        }
        let _sys_font = default_system_font(theme);
        let _code_font = default_code_font(theme);
    }
});

//-------------------------------------------------------------------------------------------------

// Console test
segue_console_test!(Fascia, Console, |ctx| {
    segue_println!(
        ctx,
        "         [Fascia Console Test: Modular UI Shell Active]"
    );
    segue_println!(
        ctx,
        "           Supported Themes    : {}",
        FasciaTheme::ALL.len()
    );
    for &theme in FasciaTheme::ALL {
        segue_println!(
            ctx,
            "             Theme: {:<20} (Dark: {})",
            theme.display_name(),
            theme.is_dark()
        );
    }
    let roots = detect_system_roots();
    segue_println!(ctx, "           System Drive Roots  : {:?}", roots);
    let status = StatusBarInfo::default();
    segue_println!(
        ctx,
        "           Default Status      : Branch='{}', Encoding='{}', Indent='{}'",
        status.branch,
        status.encoding,
        status.indentation
    );
});

//-------------------------------------------------------------------------------------------------

// Example test
segue_example_test!(Fascia, Example, |ctx| {
    // 1. Initialize Tab Manager with Welcome and a file
    let mut tab_manager = TabManager::new();
    let file_path = PathBuf::from("src/lib.rs");
    tab_manager.open_file(file_path);
    segue_assert_eq!(ctx, tab_manager.tabs().len(), 2);
    segue_assert_eq!(ctx, tab_manager.active_index(), Some(1));
    // 2. Setup Explorer
    let explorer = ExplorerState::new(PathBuf::from("."));
    segue_assert!(ctx, explorer.root_node.is_expanded);
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
    segue_assert_eq!(ctx, theme.is_dark(), true);
    segue_assert_eq!(ctx, status.line, 42);
    segue_assert_eq!(ctx, status.column, 10);
    segue_assert_eq!(ctx, palette.corner_radius, 5.0);
    // If invoked specifically via -e (example mode), launch the interactive GUI app
    let has_e_flag = std::env::args().any(|a| a == "-e");
    let is_headless = std::env::var("SEGUE_HEADLESS").as_deref() == Ok("1");
    if has_e_flag && !is_headless {
        segue_println!(
            ctx,
            "         [Launching Fascia Interactive Desktop GUI Shell...]"
        );
        if let Err(err) = crate::fascia::run_app() {
            segue_println!(ctx, "         [Error launching Fascia GUI: {:?}]", err);
        }
    }
});
