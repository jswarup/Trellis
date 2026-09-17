use crate::{jeeves_assert, jeeves_assert_eq, jeeves_assert_ne, jeeves_println, jeeves_test};
// src/fascia/_test/mod.rs
use crate::cove::jeeves::{
};
#[allow( unused_imports)]
use crate::fascia::explorer::{default_initial_dir, detect_system_roots, is_text_file};
#[allow( unused_imports)]
use crate::fascia::{
    ActivityTab, ExplorerAction, ExplorerState, FasciaTheme, FileTreeNode, MenuAction,
    StatusBarInfo, TabBarAction, TabId, TabItem, TabKind, TabManager, ToolBarAction,
    default_code_font, default_system_font,
};
use std::path::PathBuf;

//-------------------------------------------------------------------------------------------------

// TabManager lifecycle and operations
jeeves_test!( Fascia, TabManagerLifecycle, |ctx| {
    let  mut manager = TabManager::new();
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
    let  file_path = PathBuf::from( "src/main.rs");
    let  ( idx, newly_opened) = manager.open_file( file_path.clone());
    jeeves_assert!( ctx, newly_opened);
    jeeves_assert_eq!( ctx, idx, 2);
    jeeves_assert_eq!( ctx, manager.active_index(), Some( 2));
    jeeves_assert_eq!( ctx, manager.active_tab().unwrap().kind, TabKind::FileEditor);
    jeeves_assert_eq!( ctx, manager.active_tab().unwrap().title, "main.rs");
    // Open same file should focus existing
    let  ( idx2, newly_opened2) = manager.open_file( file_path);
    jeeves_assert!( ctx, !newly_opened2);
    jeeves_assert_eq!( ctx, idx2, 2);
    // Mark dirty
    jeeves_assert!( ctx, !manager.active_tab().unwrap().is_dirty);
    manager.mark_active_dirty( true);
    jeeves_assert!( ctx, manager.active_tab().unwrap().is_dirty);
    manager.mark_active_dirty( false);
    jeeves_assert!( ctx, !manager.active_tab().unwrap().is_dirty);
    // Close tab
    let  closed = manager.close_tab( 2);
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
    let  roots = detect_system_roots();
    jeeves_assert!( ctx, !roots.is_empty());
    let  init_dir = default_initial_dir();
    jeeves_assert!( ctx, init_dir.exists());
    let  mut state = ExplorerState::new( init_dir.clone());
    state.refresh();
    jeeves_assert_eq!( ctx, state.root_node.path, init_dir);
    jeeves_assert!( ctx, state.root_node.is_expanded);
    jeeves_assert!( ctx, state.root_node.is_dir);
    // Extension text check
    jeeves_assert!( ctx, is_text_file( &PathBuf::from( "test.rs")));
    jeeves_assert!( ctx, is_text_file( &PathBuf::from( "config.toml")));
    jeeves_assert!( ctx, is_text_file( &PathBuf::from( "README.md")));
    // Node icon checks
    let  rs_node = FileTreeNode::new( PathBuf::from( "lib.rs"), 0);
    jeeves_assert_eq!( ctx, rs_node.icon(), "🦀");
    let  toml_node = FileTreeNode::new( PathBuf::from( "Cargo.toml"), 0);
    jeeves_assert_eq!( ctx, toml_node.icon(), "⚙");
    let  dir_node = FileTreeNode::new( PathBuf::from( "src"), 0);
    jeeves_assert_eq!( ctx, dir_node.icon(), "📁");
});

//-------------------------------------------------------------------------------------------------

// Theme palettes and font selection
jeeves_test!( Fascia, ThemePaletteVariants, |ctx| {
    for &theme in FasciaTheme::ALL
    {
        let  palette = theme.palette();
        if theme.is_dark()
        {
            jeeves_assert!( ctx, palette.text_primary.r > 0.5);
        } else
        {
            jeeves_assert!( ctx, palette.text_primary.r < 0.5);
        }
        let  _sys_font = default_system_font( theme);
        let  _code_font = default_code_font( theme);
    }
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
    for &theme in FasciaTheme::ALL
    {
        jeeves_println!(
            ctx,
            "             Theme: {:<20} (Dark: {})",
            theme.display_name(),
            theme.is_dark()
        );
    }
    let  roots = detect_system_roots();
    jeeves_println!( ctx, "           System Drive Roots  : {:?}", roots);
    let  status = StatusBarInfo::default();
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
    let  mut tab_manager = TabManager::new();
    let  file_path = PathBuf::from( "src/lib.rs");
    tab_manager.open_file( file_path);
    jeeves_assert_eq!( ctx, tab_manager.tabs().len(), 2);
    jeeves_assert_eq!( ctx, tab_manager.active_index(), Some( 1));
    // 2. Setup Explorer
    let  explorer = ExplorerState::new( PathBuf::from( "."));
    jeeves_assert!( ctx, explorer.root_node.is_expanded);
    // 3. Setup Theme and Status
    let  theme = FasciaTheme::WindowsDark;
    let  palette = theme.palette();
    let  status = StatusBarInfo {
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
    let  has_e_flag = std::env::args().any( |a| a == "-e");
    let  is_headless = std::env::var( "SEGUE_HEADLESS").as_deref() == Ok( "1");
    if has_e_flag && !is_headless
    {
        jeeves_println!(
            ctx,
            "         [Launching Fascia Interactive Desktop GUI Shell...]"
        );
        if let  Err( err) = crate::fascia::run_app()
        {
            jeeves_println!( ctx, "         [Error launching Fascia GUI: {:?}]", err);
        }
    }
});
