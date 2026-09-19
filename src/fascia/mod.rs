// src/fascia/mod.rs
//! # Fascia
//!
//! A modular native shell and multi-tabbed UI component library built with Iced.
//!
//! Provides ready-to-use desktop IDE components:
//! - Native Menubar (`menubar`)
//! - Action Toolbar (`toolbar`)
//! - Activity Bar (`activity_bar`)
//! - Filesystem Tree Explorer (`explorer`)
//! - VS Code-style Multi-Tabbed Content Area (`tabs`)
//! - Status Bar (`status_bar`)
//! - Complete Shell Layout (`shell`)
//! - Native Look styling for Windows and Linux (`theme`)
pub mod _tests;
pub mod activity_bar;
pub mod app;
pub mod explorer;
pub mod pts_view;
pub mod obj_view;
pub mod menubar;
pub mod shell;
pub mod status_bar;
pub mod tabs;
pub mod theme;
pub mod toolbar;
pub mod waveform;
pub use	activity_bar::{ ActivityTab, view_activity_bar };
pub use	app::run_app;
pub use	explorer::{ ExplorerAction, ExplorerState, FileTreeNode, default_initial_dir, detect_system_roots, is_text_file, view_explorer };
pub use	menubar::{ MenuAction, view_menubar };
pub use	shell::view_shell;
pub use	status_bar::{ StatusBarInfo, view_status_bar };
pub use	tabs::{ TabBarAction, TabId, TabItem, TabKind, TabManager, view_tab_bar };
pub use	theme::{ FasciaStyle, FasciaTheme, ThemePalette, default_code_font, default_system_font };
pub use	toolbar::{ ToolBarAction, view_toolbar };
pub use	waveform::{ WaveformAction, WaveformState, view_waveform };
