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
pub mod bottom_tray;
pub mod camera;
mod cask_labels;
pub mod cask_scene;
pub mod cask_view;
pub mod circuit_scene;
pub mod command;
pub mod document;
pub mod examples;
pub mod examples_workspace;
pub mod explorer;
mod geometry_load;
pub mod geometry_view;
pub mod heist_bridge;
pub mod icons;
pub mod math_inspector;
pub mod math_text;
pub mod math_view;
pub mod menubar;
pub mod output;
pub mod plot2d;
pub mod plot3d;
pub mod popups;
pub mod properties;
pub mod python_console;
pub mod shell;
pub mod sketch_canvas;
pub mod sketch_inspector;
pub mod sketch_job;
pub mod sketch_tool;
pub mod sketch_view;
pub mod status_bar;
pub mod tabs;
pub mod theme;
pub mod toolbar;
pub mod waveform;
pub use activity_bar::{ActivityTab, view_activity_bar};
pub use app::{AppMessage, AppState, TabDocument, run_app};
pub use bottom_tray::{BottomTab, BottomTrayAction, view_bottom_tray};
pub use cask_view::{
    Cask2dAction, Cask2dViewerState, CaskRenderer, view_cask, view_cask_root, view_cask2d,
    view_cask2d_tab,
};
pub use command::{
    CommandContext, CommandDef, CommandId, CommandOutcome, CommandRegistry, Shortcut,
};
pub use document::{EditorDocument, IWorkbenchDocument};
pub use explorer::{
    ExplorerAction, ExplorerState, ExplorerTab, FileTreeNode, default_initial_dir,
    detect_system_roots, is_text_file, view_explorer,
};
pub use heist_bridge::{HeistProgressReporter, HeistUiEvent, HeistUiRuntime, WorkspaceSearchHit};
pub use icons::{IconId, ViewIcon};
pub use math_inspector::ViewMathInspector;
pub use math_text::CosmicMathFontMetrics;
pub use math_view::{FrescoAction, FrescoCanvasProgram, FrescoViewerState, ViewMathTab};
pub use menubar::{MenuAction, view_menubar};
pub use output::{
    DiagnosticSeverity, JobId, JobRecord, JobStatus, OutputEntry, OutputStore, view_output,
};
pub use plot2d::{Plot2dAction, Plot2dSeries, Plot2dState, SampleFrescoPlot2d, ViewPlot2d};
pub use plot3d::SampleFrescoSurface3d;
pub use popups::{
    ContextAction, ContextMenuItem, ContextMenuState, PopupAction, PopupMode, PopupState,
    ToastItem, ToastLevel, ViewPopupOverlay,
};
pub use properties::{InspectedTarget, PropertiesAction, PropertiesState, view_properties};
pub use python_console::{PythonConsoleAction, PythonConsoleState, view_python_console};
pub use shell::{PaneId, ShellAction, SplitTarget, WorkbenchLayout, view_shell, view_shell_with_layout};
pub use sketch_canvas::{
    SketchCanvasAction, SketchCanvasInteraction, SketchCanvasProgram, SketchCanvasState,
};
pub use sketch_inspector::{
    InspectorPanelTab, SketchInspectorAction, SketchInspectorState, view_sketch_inspector,
};
pub use sketch_tool::{
    DraftGeometry, DraftPrimitive, GripInfo, GripKind, KeyModifiers, NumericFieldKind,
    PointerButton, SketchInputEvent, SketchToolController, SketchToolKind,
};
pub use sketch_view::{SketchViewAction, SketchViewerState, view_sketch_tab};
pub use status_bar::{StatusBarInfo, view_status_bar};
pub use tabs::{TabBarAction, TabId, TabItem, TabKind, TabManager, view_tab_bar};
pub use theme::{FasciaStyle, FasciaTheme, ThemePalette, default_code_font, default_system_font};
pub use toolbar::{ActiveToolContext, ToolBarAction, view_contextual_toolbar, view_toolbar, view_top_bar};
pub use waveform::{WaveformAction, WaveformState, view_waveform};

//-------------------------------------------------------------------------------------------------
