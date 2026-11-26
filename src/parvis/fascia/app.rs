// app.rs -----------------------------------------------------------------------------------------------------------------

pub use crate::fascia::document::TabDocument;
use crate::fascia::document::{DocumentStore, IWorkbenchDocument};
use crate::fascia::examples::{ExampleAction, ExampleBrowser};
use crate::fascia::explorer::{default_initial_dir, is_text_file};
use crate::fascia::geometry_view::{GeometryAction, GeometryViewerState, ViewGeometry};
use crate::fascia::menubar::ViewMenubarWithContext;
use crate::fascia::python_console::ConsoleExecutionResult;
use crate::fascia::theme::default_code_font;
use crate::fascia::{
    ActiveToolContext, ActivityTab, Cask2dAction, Cask2dViewerState, CommandContext, CommandId,
    CommandRegistry, ExplorerAction, ExplorerState, FasciaStyle, FasciaTheme, HeistUiEvent,
    HeistUiRuntime, MenuAction, Plot2dAction, Plot2dState, PopupAction, PopupMode, PopupState,
    PythonConsoleAction, PythonConsoleState, SampleFrescoPlot2d, SampleFrescoSurface3d,
    ShellAction, StatusBarInfo, TabBarAction, TabId, TabItem, TabKind, TabManager, ThemePalette,
    ToastLevel, ToolBarAction, ViewPlot2d, ViewPopupOverlay, WaveformAction, WaveformState,
    WorkbenchLayout, view_activity_bar, view_cask2d_tab, view_contextual_toolbar,
    view_python_console, view_shell_with_layout, view_status_bar, view_tab_bar, view_top_bar,
    view_waveform,
};
use crate::fenst::geometry::GeometryAsset;
use crate::fenst::fresco_resource::{FRESCO_EXAMPLE_TITLE, FRESCO_EXAMPLE_URI, OpenFrescoExample};
use crate::fenst::rube_resource::{DADDA_EXAMPLE_URI, DADDA_STIMULUS, DaddaRunError, RunDadda};
use crate::fenst::waveform::ParseWaveform;
use crate::silo::{Arr, IArr, Stash};
use iced::widget::{
    Space, button, column, container, row, scrollable, text, text_editor, text_input,
};
use iced::{Alignment, Element, Length, Point, Size, Task, mouse};
use std::path::PathBuf;
use std::sync::Arc;

//-----------------------------------------------------------------------------------------------------------------------------

/// Unified message enum for the application.
#[derive(Debug, Clone)]
pub enum AppMessage
{
    Command(CommandId),
    Menu(MenuAction),
    ToolBar(ToolBarAction),
    SelectActivityTab(ActivityTab),
    Explorer(ExplorerAction),
    Example(ExampleAction),
    TabBar(TabBarAction),
    EditorAction(text_editor::Action),
    Waveform(WaveformAction),
    Geometry(TabId, GeometryAction),
    GeometryPrepared(TabId,
                     Arc<std::sync::atomic::AtomicBool>,
                     Result<Arc<GeometryAsset>, String>),
    Cask2d(TabId, Cask2dAction),
    Sketch(TabId, crate::fascia::sketch_view::SketchViewAction),
    SketchJobCompleted
    {
        tab_id:       TabId,
        generation:   u64,
        cancellation: Arc<std::sync::atomic::AtomicBool>,
        snapshot_rev: u64,
        outcome:      crate::fascia::sketch_job::SketchJobOutcome,
    },
    OpenFile(PathBuf),
    SaveCurrentFile,
    NewFile,
    NewSketch,
    NewMathView,
    Fresco(TabId, crate::fascia::math_view::FrescoAction),
    Plot2d(TabId, Plot2dAction),
    RequestExit,
    ResolveSketchClose(SketchCloseDecision),
    CloseActiveTab,
    CloseTab(usize),
    SelectTab(usize),
    ToggleSidebar,
    ToggleStatusBar,
    SelectTheme(FasciaTheme),
    ToggleTheme,
    OpenSettings,
    OpenPythonConsole,
    OpenOutput,
    OpenOutputLink(String),
    RunExample(String),
    JobFinished(crate::fascia::output::JobId, Result<(String, Option<String>, Option<String>), String>),
    DaddaFinished(crate::fascia::output::JobId, Result<String, DaddaRunError>),
    ToggleBottomTray,
    SelectBottomTab(crate::fascia::bottom_tray::BottomTab),
    BottomTray(crate::fascia::bottom_tray::BottomTrayAction),
    ToggleProperties,
    Properties(crate::fascia::properties::PropertiesAction),
    PythonConsole(TabId, PythonConsoleAction),
    PythonConsoleExecuted(TabId, ConsoleExecutionResult),
    TrayPythonConsole(PythonConsoleAction),
    TrayPythonConsoleExecuted(ConsoleExecutionResult),
    HeistEvent(HeistUiEvent),
    CancelJob(crate::fascia::output::JobId),
    SearchQueryChanged(String),
    SearchSubmit,
    Shell(ShellAction),
    Popup(PopupAction),
    CursorMoved(Point),
    MouseButtonReleased,
}

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum SketchCloseDecision
{
    Save,
    Discard,
    Cancel,
}

//-------------------------------------------------------------------------------------------------

pub struct AppState
{
    pub theme:               FasciaTheme,
    pub show_sidebar:        bool,
    pub show_status_bar:     bool,
    pub show_bottom_tray:    bool,
    pub active_bottom_tab:   crate::fascia::bottom_tray::BottomTab,
    pub show_properties:     bool,
    pub properties:          crate::fascia::properties::PropertiesState,
    pub console:             crate::fascia::python_console::PythonConsoleState,
    pub active_activity_tab: Option<ActivityTab>,
    pub explorer:            ExplorerState,
    _Examples:               ExampleBrowser,
    pub tab_manager:         TabManager,
    _Documents:              DocumentStore,
    pub output:              crate::fascia::output::OutputStore,
    pub waveform_results:    std::collections::HashMap<String, String>,
    pub status_info:         StatusBarInfo,
    pub layout:              WorkbenchLayout,
    pub popups:              PopupState,
    pub search_query:        String,
    pub search_hits:         Vec<crate::fascia::WorkspaceSearchHit>,
    pub search_running:      bool,
    _SearchJobId:            Option<crate::fascia::output::JobId>,
    pub cursor_pos:          Point,
    pub heist:               HeistUiRuntime,
    _CloseQueue:             Stash<TabId>,
    _ClosePrompt:            Option<TabId>,
    _ExitAfterClose:         bool,
}

//-----------------------------------------------------------------------------------------------------------------------------

impl Default for AppState
{
    fn default() -> Self
    {
        let initial_dir = default_initial_dir();
        let explorer = ExplorerState::new(initial_dir.clone());
        let tab_manager = TabManager::default();
        let status_info = StatusBarInfo { message: format!("Ready - {}", initial_dir.display()),
                                          ..Default::default() };
        Self { theme: FasciaTheme::default(),
               show_sidebar: true,
               show_status_bar: true,
               show_bottom_tray: false,
               active_bottom_tab: crate::fascia::bottom_tray::BottomTab::Output,
               show_properties: true,
               properties: crate::fascia::properties::PropertiesState::New(),
               console: crate::fascia::python_console::PythonConsoleState::new(),
               active_activity_tab: Some(ActivityTab::Explorer),
               explorer,
               _Examples: ExampleBrowser::default(),
               tab_manager,
               _Documents: DocumentStore::New(),
               output: crate::fascia::output::OutputStore::New(),
               waveform_results: std::collections::HashMap::new(),
               status_info,
               layout: WorkbenchLayout::default(),
               popups: PopupState::New(),
               search_query: String::new(),
               search_hits: Vec::new(),
               search_running: false,
               _SearchJobId: None,
               cursor_pos: Point::ORIGIN,
               heist: HeistUiRuntime::New(),
               _CloseQueue: Stash::New(), _ClosePrompt: None, _ExitAfterClose: false }
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl AppState
{
    pub fn new() -> Self { Self::default() }
    pub fn palette(&self) -> ThemePalette { self.theme.palette() }
    pub fn Documents(&self) -> Arr<'_, (TabId, TabDocument)> { return self._Documents.Entries(); }

    pub fn Document(&self, id: TabId) -> Option<&TabDocument> { return self._Documents.Get(id); }

    pub fn DocumentCount(&self) -> u32 { return self._Documents.Len(); }

    /// Evaluates current UI and document state for command enablement.
    fn RunnableExampleUri(&self) -> Option<&str>
    {
        let selected = self.explorer.selected_path.as_deref().and_then(|path| path.to_str())
            .filter(|uri| uri.starts_with("example://"));
        let active = self.tab_manager.active_tab()
            .and_then(|tab| tab.path.as_deref()).and_then(|path| path.to_str());
        return selected.or(active).filter(|uri| {
            *uri == DADDA_EXAMPLE_URI || crate::fenst::example::XplrExample::FindByUri(uri).is_some()
        });
    }

    pub fn CommandContext(&self) -> CommandContext
    {
        let active_tab = self.tab_manager.active_tab();
        let has_active_tab = active_tab.is_some();
        let is_active_tab_dirty = active_tab.map(|t| t.is_dirty).unwrap_or(false);
        let tab_count = self.tab_manager.tabs().Size();
        let (can_undo, can_redo) = if let Some(tab) = active_tab {
            if let Some(viewer) = self._Documents.Sketch(tab.id) {
                (viewer.CanUndo(), viewer.CanRedo())
            } else {
                (false, false)
            }
        } else {
            (false, false)
        };
        let can_save = if let Some(tab) = active_tab {
            tab.is_dirty && matches!(tab.kind, TabKind::FileEditor | TabKind::SketchViewer)
        } else {
            false
        };
        return CommandContext {
            has_active_tab,
            is_active_tab_dirty,
            tab_count,
            can_save,
            can_undo,
            can_redo,
            is_sidebar_visible: self.show_sidebar,
            is_status_bar_visible: self.show_status_bar,
            has_visible_roots: self.explorer.has_visible_roots(),
            has_runnable_example: self.RunnableExampleUri().is_some(),
            has_active_job: self.output.StoppableJob().is_some(),
        };
    }

    /// Dispatches a typed command through the centralized workbench dispatcher.
    pub fn ExecuteCommand(&mut self, cmd: CommandId) -> Task<AppMessage>
    {
        let ctx = self.CommandContext();
        if !CommandRegistry::IsAvailable(cmd, &ctx) {
            return Task::none();
        }
        match cmd {
            CommandId::FILE_NEW => return self.update(AppMessage::NewFile),
            CommandId::FILE_NEW_SKETCH => return self.update(AppMessage::NewSketch),
            CommandId::FILE_NEW_MATH => return self.update(AppMessage::NewMathView),
            CommandId::MATH_EXPORT_TEXT => {
                if let Some(tab) = self.tab_manager.active_tab()
                    && let Some(viewer) = self._Documents.FrescoMut(tab.id)
                {
                    viewer.Update(crate::fascia::math_view::FrescoAction::ExportDisplayText);
                    if let Some(Ok(txt)) = viewer.LastExportText() {
                        self.status_info.message = format!("Exported display text ({} bytes)", txt.len());
                    } else if let Some(Err(err)) = viewer.LastExportText() {
                        self.status_info.message = format!("Export text failed: {}", err);
                    }
                }
            }
            CommandId::MATH_EXPORT_SVG => {
                if let Some(tab) = self.tab_manager.active_tab()
                    && let Some(viewer) = self._Documents.FrescoMut(tab.id)
                {
                    viewer.Update(crate::fascia::math_view::FrescoAction::ExportSvg);
                    if let Some(Ok(svg)) = viewer.LastExportSvg() {
                        self.status_info.message = format!("Exported SVG ({} bytes)", svg.len());
                    } else if let Some(Err(err)) = viewer.LastExportSvg() {
                        self.status_info.message = format!("Export SVG failed: {}", err);
                    }
                }
            }
            CommandId::MATH_PLOT_2D => {
                if let Some(tab) = self.tab_manager.active_tab()
                    && let Some(viewer) = self._Documents.Fresco(tab.id)
                {
                    let title = format!("{} · 2D Plot", tab.title);
                    match SampleFrescoPlot2d(viewer.Document(), -5.0, 5.0, 240) {
                        Ok(series) => {
                            let (_, _, plot_id) = self.tab_manager.OpenPlot2d(title.clone());
                            self._Documents.InsertPlot2d(plot_id, Plot2dState::New(title, series, -5.0, 5.0));
                            self.update_status_for_active_tab();
                        }
                        Err(err) => {
                            self.popups.PushToast(ToastLevel::Warning, "2D Plot", Some(err));
                        }
                    }
                }
            }
            CommandId::MATH_PLOT_3D => {
                if let Some(tab) = self.tab_manager.active_tab()
                    && let Some(viewer) = self._Documents.Fresco(tab.id)
                {
                    let root_idx = viewer.SelectedRoot();
                    let title = format!("{} · 3D Surface", tab.title);
                    match SampleFrescoSurface3d(viewer.Document(), root_idx, (-3.0, 3.0), (-3.0, 3.0), 40) {
                        Ok(asset) => {
                            let (_, _, geom_id) = self.tab_manager.open_geometry_tab(title);
                            let mut geom_state = GeometryViewerState::default();
                            geom_state.Complete(Ok(Arc::new(asset)));
                            self._Documents.InsertGeometry(geom_id, geom_state);
                            self.update_status_for_active_tab();
                        }
                        Err(err) => {
                            self.popups.PushToast(ToastLevel::Warning, "3D Plot", Some(err));
                        }
                    }
                }
            }
            CommandId::FILE_OPEN | CommandId::FOLDER_OPEN => {
                if self.explorer.has_visible_roots() {
                    self.active_activity_tab = Some(ActivityTab::Explorer);
                    self.show_sidebar = true;
                }
            }
            CommandId::FILE_SAVE => return self.update(AppMessage::SaveCurrentFile),
            CommandId::TAB_CLOSE => return self.update(AppMessage::CloseActiveTab),
            CommandId::TAB_CLOSE_ALL => {
                return self.BeginClosing(None, false);
            }
            CommandId::APP_EXIT => {
                return self.update(AppMessage::RequestExit);
            }
            CommandId::EDIT_UNDO => {
                if let Some(tab) = self.tab_manager.active_tab()
                    && let Some(viewer) = self._Documents.SketchMut(tab.id)
                {
                    viewer.Undo();
                    let is_dirty = viewer.IsDirty();
                    self.tab_manager.mark_active_dirty(is_dirty);
                }
            }
            CommandId::EDIT_REDO => {
                if let Some(tab) = self.tab_manager.active_tab()
                    && let Some(viewer) = self._Documents.SketchMut(tab.id)
                {
                    viewer.Redo();
                    let is_dirty = viewer.IsDirty();
                    self.tab_manager.mark_active_dirty(is_dirty);
                }
            }
            CommandId::EDIT_CUT
            | CommandId::EDIT_COPY
            | CommandId::EDIT_PASTE
            | CommandId::EDIT_SELECT_ALL => {}
            CommandId::VIEW_TOGGLE_SIDEBAR => return self.update(AppMessage::ToggleSidebar),
            CommandId::VIEW_TOGGLE_STATUS_BAR => return self.update(AppMessage::ToggleStatusBar),
            CommandId::VIEW_TOGGLE_BOTTOM_PANEL => return self.update(AppMessage::ToggleBottomTray),
            CommandId::VIEW_TOGGLE_PROPERTIES => return self.update(AppMessage::ToggleProperties),
            CommandId::VIEW_TOGGLE_EXPLORER_FS => {
                self.explorer.toggle_filesystem();
                if self.explorer.show_filesystem {
                    self.active_activity_tab = Some(ActivityTab::Explorer);
                    self.show_sidebar = true;
                    self.status_info.message =
                        "Filesystem view enabled in Explorer".to_string();
                } else if !self.explorer.has_visible_roots() {
                    self.show_sidebar = false;
                    self.status_info.message =
                        "Explorer collapsed (all views disabled)".to_string();
                }
            }
            CommandId::VIEW_TOGGLE_EXPLORER_SETTINGS => {
                self.explorer.toggle_settings();
                if self.explorer.show_settings {
                    self.active_activity_tab = Some(ActivityTab::Explorer);
                    self.show_sidebar = true;
                    self.status_info.message =
                        "Settings view enabled in Explorer".to_string();
                } else if !self.explorer.has_visible_roots() {
                    self.show_sidebar = false;
                    self.status_info.message =
                        "Explorer collapsed (all views disabled)".to_string();
                }
            }
            CommandId::VIEW_TOGGLE_EXPLORER_EXAMPLES => {
                self.explorer.toggle_examples();
                if self.explorer.show_examples {
                    self.active_activity_tab = Some(ActivityTab::Explorer);
                    self.show_sidebar = true;
                    self.status_info.message =
                        "Examples view enabled in Explorer".to_string();
                } else if !self.explorer.has_visible_roots() {
                    self.show_sidebar = false;
                    self.status_info.message =
                        "Explorer collapsed (all views disabled)".to_string();
                }
            }
            CommandId::VIEW_OPEN_EXPLORER => {
                if self.explorer.has_visible_roots() {
                    self.active_activity_tab = Some(ActivityTab::Explorer);
                    self.show_sidebar = true;
                    self.explorer.ensure_valid_active_tab();
                } else {
                    self.show_sidebar = false;
                    self.status_info.message =
                    "Explorer is disabled: Select Filesystem, Settings, or Examples from View menu".to_string();
                }
            }
            CommandId::VIEW_OPEN_SEARCH => {
                self.active_activity_tab = Some(ActivityTab::Search);
                self.show_sidebar = true;
            }
            CommandId::VIEW_QUICK_OPEN => {
                self.popups.Open(PopupMode::QuickOpen);
            }
            CommandId::VIEW_COMMAND_PALETTE => {
                self.popups.Open(PopupMode::CommandPalette);
            }
            CommandId::VIEW_OPEN_SETTINGS => return self.update(AppMessage::OpenSettings),
            CommandId::VIEW_OPEN_PYTHON_CONSOLE => {
                return self.update(AppMessage::OpenPythonConsole);
            }
            CommandId::THEME_TOGGLE => return self.update(AppMessage::ToggleTheme),
            CommandId::THEME_SELECT_WINDOWS_DARK => {
                return self.update(AppMessage::SelectTheme(FasciaTheme::WindowsDark));
            }
            CommandId::THEME_SELECT_WINDOWS_LIGHT => {
                return self.update(AppMessage::SelectTheme(FasciaTheme::WindowsLight));
            }
            CommandId::THEME_SELECT_LINUX_DARK => {
                return self.update(AppMessage::SelectTheme(FasciaTheme::LinuxDark));
            }
            CommandId::THEME_SELECT_LINUX_LIGHT => {
                return self.update(AppMessage::SelectTheme(FasciaTheme::LinuxLight));
            }
            CommandId::THEME_SELECT_VSCODE_DARK => {
                return self.update(AppMessage::SelectTheme(FasciaTheme::VsCodeDark));
            }
            CommandId::VIEW_OPEN_OUTPUT => {
                return self.update(AppMessage::OpenOutput);
            }
            CommandId::RUN_EXAMPLE => {
                if let Some(uri) = self.RunnableExampleUri() {
                    return self.update(AppMessage::RunExample(uri.to_string()));
                }
            }
            CommandId::RUN_STOP => {
                if let Some(id) = self.output.StoppableJob() {
                    self.output.RequestCancellation(id);
                    self.status_info.message = format!("Stop requested for {id}");
                }
            }
            CommandId::HELP_ABOUT => {
                self.status_info.message = "Fascia UI Shell v0.1.0".to_string();
            }
            CommandId::HELP_DOCUMENTATION => {
                self.status_info.message =
                    "Documentation: https://github.com/iced-rs/iced".to_string();
            }
            _ => {}
        }
        return Task::none();
    }

    /// Attaches one payload to an existing empty tab of the corresponding kind.
    pub fn SetDocument(&mut self, id: TabId, mut document: TabDocument)
                       -> Result<(), &'static str>
    {
        if self._Documents.Get(id).is_some() {
            document.CancelJobs();
            return Err("Document tab already has a payload");
        }
        let mut tab = None;
        let mut index = 0u32;
        self.tab_manager.tabs().Span(|item| {
                                   if item.id == id {
                                       tab = Some(item.kind.clone());
                                       return false;
                                   }
                                   index += 1;
                                   return true;
                               });
        let Some(kind) = tab else {
            document.CancelJobs();
            return Err("Document tab does not exist");
        };
        if !document.MatchesKind(kind) {
            document.CancelJobs();
            return Err("Document does not match tab kind");
        }
        let dirty = document.IsDirty();
        self._Documents.Insert(id, document);
        self.tab_manager.mark_dirty(index as usize, dirty);
        return Ok(());
    }

    pub fn OpenFrescoDocument(&mut self, doc: crate::fenst::fresco_document::FrescoDocument)
                              -> TabId
    {
        let (_, _, id) = self.tab_manager.OpenFresco("Math View".to_string());
        self._Documents
            .InsertFresco(id, crate::fascia::math_view::FrescoViewerState::New(doc));
        self.update_status_for_active_tab();
        return id;
    }

    fn InspectActiveFresco(&mut self)
    {
        if let Some(tab) = self.tab_manager.active_tab()
            && let Some(viewer) = self._Documents.Fresco(tab.id)
            && let Some(props) = crate::fenst::fresco_properties::FrescoProperties::Read(
                viewer.Document(), viewer.SelectedRoot(), viewer.SelectedOccurrence())
        {
            self.properties.InspectFrescoResource(&props, tab.path.as_deref().and_then(|p| p.to_str()));
        }
    }

    pub fn OpenWaveformDocumentWithPath(
        &mut self,
        title: String,
        path: Option<PathBuf>,
        waveform: WaveformState,
    ) -> TabId
    {
        let (_, is_new, id) = self.tab_manager.open_waveform(title, path);
        if is_new {
            self._Documents.InsertWaveform(id, waveform);
        }
        self.update_status_for_active_tab();
        return id;
    }

    pub fn OpenWaveformDocument(&mut self, title: String, waveform: WaveformState) -> TabId
    {
        return self.OpenWaveformDocumentWithPath(title, None, waveform);
    }

    /// Simulates a 4x4 structural Dadda multiplier on explicit stimulus vectors and captures real VCD transitions.
    pub fn SimulateDaddaMult(stimulus: &[(u8, u8)]) -> String
    {
        use crate::nexome::rube::dadda::DaddaMult4x4;
        use crate::nexome::rube::engine::SimEngine;
        use crate::nexome::rube::layout::Layout;

        let mut layout = Layout::New();
        let mult = DaddaMult4x4::New(&mut layout, "Dadda4x4");
        layout.Freeze();
        let mut engine = SimEngine::Create(&layout);

        let mut vcd = String::from(
            "$version Trellis Rube Engine $end\n\
             $timescale 1ns $end\n\
             $scope module Dadda4x4 $end\n\
             $var wire 1 ! clk $end\n\
             $var wire 4 \" a $end\n\
             $var wire 4 # b $end\n\
             $var wire 8 % prod $end\n\
             $upscope $end\n\
             $enddefinitions $end\n"
        );

        let mut time = 0;
        let mut clk = 0;
        for (idx, &(a, b)) in stimulus.iter().enumerate() {
            mult.SetA(&mut engine, a);
            mult.SetB(&mut engine, b);
            engine.Settle(20);
            let prod = mult.GetProduct(&engine);

            if idx == 0 {
                vcd.push_str("$dumpvars\n");
                vcd.push_str(&format!("{clk}!\n"));
                vcd.push_str(&format!("b{a:04b} \"\n"));
                vcd.push_str(&format!("b{b:04b} #\n"));
                vcd.push_str(&format!("b{prod:08b} %\n"));
                vcd.push_str("$end\n");
            } else {
                vcd.push_str(&format!("#{time}\n"));
                vcd.push_str(&format!("{clk}!\n"));
                vcd.push_str(&format!("b{a:04b} \"\n"));
                vcd.push_str(&format!("b{b:04b} #\n"));
                vcd.push_str(&format!("b{prod:08b} %\n"));
            }
            time += 10;
            clk = 1 - clk;
        }

        return vcd;
    }

    pub fn Editor(&self, id: TabId) -> Option<&text_editor::Content>
    {
        return self._Documents.Editor(id);
    }

    pub fn EditorMut(&mut self, id: TabId) -> Option<&mut text_editor::Content>
    {
        return self._Documents.EditorMut(id);
    }

    pub fn EditorCount(&self) -> u32 { return self._Documents.EditorCount(); }

    pub fn Waveform(&self, id: TabId) -> Option<&WaveformState>
    {
        return self._Documents.Waveform(id);
    }

    pub fn WaveformMut(&mut self, id: TabId) -> Option<&mut WaveformState>
    {
        return self._Documents.WaveformMut(id);
    }

    pub fn WaveformCount(&self) -> u32 { return self._Documents.WaveformCount(); }

    pub fn Console(&self, id: TabId) -> Option<&PythonConsoleState>
    {
        if id == TabId(0) {
            return Some(&self.console);
        }
        return self._Documents.Console(id);
    }

    pub fn ConsoleMut(&mut self, id: TabId) -> Option<&mut PythonConsoleState>
    {
        if id == TabId(0) {
            return Some(&mut self.console);
        }
        return self._Documents.ConsoleMut(id);
    }

    pub fn ConsoleCount(&self) -> u32 { return self._Documents.ConsoleCount(); }

    pub fn Geometry(&self, id: TabId) -> Option<&GeometryViewerState>
    {
        return self._Documents.Geometry(id);
    }

    pub fn GeometryMut(&mut self, id: TabId) -> Option<&mut GeometryViewerState>
    {
        return self._Documents.GeometryMut(id);
    }

    pub fn GeometryCount(&self) -> u32 { return self._Documents.GeometryCount(); }

    pub fn Cask2d(&self, id: TabId) -> Option<&Cask2dViewerState>
    {
        return self._Documents.Cask2d(id);
    }

    pub fn Cask2dMut(&mut self, id: TabId) -> Option<&mut Cask2dViewerState>
    {
        return self._Documents.Cask2dMut(id);
    }

    pub fn Cask2dCount(&self) -> u32 { return self._Documents.Cask2dCount(); }

    pub fn Sketch(&self, id: TabId) -> Option<&crate::fascia::sketch_view::SketchViewerState>
    {
        return self._Documents.Sketch(id);
    }

    pub fn SketchMut(&mut self, id: TabId)
                     -> Option<&mut crate::fascia::sketch_view::SketchViewerState>
    {
        return self._Documents.SketchMut(id);
    }

    pub fn SketchCount(&self) -> u32 { return self._Documents.SketchCount(); }

    pub fn Fresco(&self, id: TabId) -> Option<&crate::fascia::math_view::FrescoViewerState>
    {
        return self._Documents.Fresco(id);
    }

    pub fn FrescoMut(&mut self, id: TabId)
                     -> Option<&mut crate::fascia::math_view::FrescoViewerState>
    {
        return self._Documents.FrescoMut(id);
    }

    pub fn FrescoCount(&self) -> u32 { return self._Documents.FrescoCount(); }

    fn update_status_for_active_tab(&mut self)
    {
        if let Some(tab) = self.tab_manager.active_tab() {
            let lang = if tab.kind == TabKind::CaskViewer {
                "Cask 3D".to_string()
            } else if tab.kind == TabKind::Cask2dViewer {
                "Cask 2D".to_string()
            } else if tab.kind == TabKind::SketchViewer {
                "2D Sketch".to_string()
            } else if tab.kind == TabKind::FrescoViewer {
                "Math View".to_string()
            } else if tab.kind == TabKind::PythonConsole {
                "Python 3.10".to_string()
            } else if tab.kind == TabKind::Output {
                "Output".to_string()
            } else if let Some(path) = &tab.path {
                path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("text")
                    .to_uppercase()
            } else {
                "Plain Text".to_string()
            };
            self.status_info.language = lang;
        } else {
            self.status_info.language = "None".to_string();
        }
    }
    pub fn close_tab_internal(&mut self, idx: usize) -> Option<TabItem>
    {
        if let Some(closed) = self.tab_manager.close_tab(idx) {
            let _ = self._Documents.Remove(closed.id);
            self.update_status_for_active_tab();
            self.status_info.message = format!("Closed {}", closed.title);
            Some(closed)
        } else {
            None
        }
    }
    pub fn close_all_tabs_internal(&mut self)
    {
        self._CloseQueue.Clear();
        self._ClosePrompt = None;
        self._ExitAfterClose = false;
        self._Documents.Clear();
        self.tab_manager.close_all();
        self.update_status_for_active_tab();
        self.status_info.message = "All tabs closed".to_string();
    }

    fn HandlePythonConsole(&mut self, tab: Option<TabId>, action: PythonConsoleAction)
                           -> Task<AppMessage>
    {
        let console = match tab {
            Some(id) => self.ConsoleMut(id),
            None => Some(&mut self.console),
        };
        let Some(console) = console else {
            return Task::none();
        };
        match action {
            PythonConsoleAction::InputChanged(text) => console.input_changed(text),
            PythonConsoleAction::Submit => {
                if let Some(_line) = console.submit() {
                    #[cfg(feature = "python")]
                    if let Some(engine) = console.engine.clone() {
                        let job_id = self.output.StartJob("Python Console", None);
                        self.heist.Submit(
                            job_id,
                            crate::heist::scheduler::JobPriority::Normal,
                            "Python Console",
                            move |_token, reporter| {
                                let result = engine.execute_line(&_line);
                                reporter.Emit(HeistUiEvent::PythonCompleted {
                                    tab_id: tab,
                                    result,
                                });
                            },
                        );
                        return Task::none();
                    }
                    console.on_executed(ConsoleExecutionResult {
                        output: console.status_message.clone(),
                        is_error: true,
                        needs_more_input: false,
                    });
                }
            }
            PythonConsoleAction::Cancel => {
                if console.is_running {
                    console.RequestInterrupt();
                    self.status_info.message = "Python interrupt requested".to_string();
                }
            }
            PythonConsoleAction::Clear => console.clear(),
        }
        return Task::none();
    }

    fn CompletePythonConsole(&mut self, tab: Option<TabId>, result: ConsoleExecutionResult)
    {
        let console = match tab {
            Some(id) => self.ConsoleMut(id),
            None => Some(&mut self.console),
        };
        if let Some(console) = console.filter(|console| console.is_running) {
            console.on_executed(result);
            self.status_info.message = if console.is_multiline {
                "Python: waiting for block continuation...".to_string()
            } else {
                "Python: ready".to_string()
            };
        }
    }

    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage>
    {
        if self._ClosePrompt.is_some()
           && !matches!(message,
                        AppMessage::ResolveSketchClose(_)
                        | AppMessage::GeometryPrepared(..)
                        | AppMessage::SketchJobCompleted { .. }
                        | AppMessage::PythonConsoleExecuted(..)
                        | AppMessage::TrayPythonConsoleExecuted(..)
                        | AppMessage::HeistEvent(..)
                        | AppMessage::JobFinished(..)
                        | AppMessage::DaddaFinished(..))
        {
            return Task::none();
        }
        match message {
            AppMessage::HeistEvent(event) => {
                match event {
                    HeistUiEvent::Started { job_id, title } => {
                        self.status_info.message = format!("Running: {}", title);
                        let _ = job_id;
                    }
                    HeistUiEvent::Progress { job_id, progress, stage } => {
                        self.output.UpdateJobProgress(job_id, progress, stage);
                    }
                    HeistUiEvent::Log { job_id: _, severity, channel, message } => {
                        self.output.Log(severity, channel, message);
                    }
                    HeistUiEvent::Finished { job_id } => {
                        self.output.CompleteJob(job_id, "Done", None);
                        self.popups.PruneExpiredToasts();
                    }
                    HeistUiEvent::SketchCompleted {
                        tab_id,
                        generation,
                        snapshot_rev,
                        cancellation,
                        outcome,
                    } => {
                        return self.update(AppMessage::SketchJobCompleted {
                            tab_id,
                            generation,
                            cancellation,
                            snapshot_rev,
                            outcome,
                        });
                    }
                    HeistUiEvent::PythonCompleted { tab_id, result } => {
                        self.CompletePythonConsole(tab_id, result);
                    }
                    HeistUiEvent::SearchCompleted { query, hits, cancelled } => {
                        if !cancelled && query == self.search_query {
                            self.search_hits = hits;
                        }
                        self.search_running = false;
                    }
                }
            }
            AppMessage::CancelJob(job_id) => {
                let _ = self.heist.Cancel(job_id);
                self.output.CancelJob(job_id);
            }
            AppMessage::SearchQueryChanged(q) => {
                self.search_query = q;
                if let Some(prev) = self._SearchJobId.take() {
                    let _ = self.heist.Cancel(prev);
                    self.output.CancelJob(prev);
                }
                if self.search_query.trim().is_empty() {
                    self.search_hits.clear();
                    self.search_running = false;
                }
            }
            AppMessage::SearchSubmit => {
                let query = self.search_query.trim().to_string();
                if query.is_empty() {
                    self.search_hits.clear();
                    self.search_running = false;
                    return Task::none();
                }
                if let Some(prev) = self._SearchJobId.take() {
                    let _ = self.heist.Cancel(prev);
                    self.output.CancelJob(prev);
                }
                let root = self.explorer.root_node.path.clone();
                let job_id = self.output.StartJob(format!("Search '{}'", query), None);
                self._SearchJobId = Some(job_id);
                self.search_running = true;
                self.heist.Submit(
                    job_id,
                    crate::heist::scheduler::JobPriority::Background,
                    format!("Search '{}'", query),
                    move |token, reporter| {
                        let hits = run_workspace_search(&root, &query, token);
                        reporter.Emit(HeistUiEvent::SearchCompleted {
                            query,
                            hits,
                            cancelled: token.IsCancelled(),
                        });
                    },
                );
            }
            AppMessage::Popup(action) => {
                match action {
                    PopupAction::OpenQuickOpen => self.popups.Open(PopupMode::QuickOpen),
                    PopupAction::OpenCommandPalette => self.popups.Open(PopupMode::CommandPalette),
                    PopupAction::CloseModal => self.popups.CloseModal(),
                    PopupAction::QueryChanged(q) => {
                        self.popups.query = q;
                        self.popups.selected_index = 0;
                    }
                    PopupAction::MoveSelection(delta) => {
                        let count = crate::fascia::popups::CollectPaletteEntries(
                            self.popups.mode,
                            &self.popups.query,
                            self.tab_manager.tabs(),
                            &self.CommandContext(),
                        )
                        .len()
                        .min(12);
                        if count > 0 {
                            let cur = self.popups.selected_index as isize;
                            let next = (cur + delta as isize).rem_euclid(count as isize) as usize;
                            self.popups.selected_index = next;
                        }
                    }
                    PopupAction::SubmitSelection => {
                        let entries = crate::fascia::popups::CollectPaletteEntries(
                            self.popups.mode,
                            &self.popups.query,
                            self.tab_manager.tabs(),
                            &self.CommandContext(),
                        );
                        let idx = self.popups.selected_index.min(entries.len().saturating_sub(1));
                        if let Some(entry) = entries.into_iter().nth(idx) {
                            self.popups.CloseModal();
                            match entry {
                                crate::fascia::popups::QuickOpenEntry::Tab(tab_idx, _) => {
                                    return self.update(AppMessage::SelectTab(tab_idx));
                                }
                                crate::fascia::popups::QuickOpenEntry::Command(cmd, _, _, _, enabled) => {
                                    if enabled {
                                        return self.update(AppMessage::Command(cmd));
                                    }
                                }
                            }
                        } else {
                            self.popups.CloseModal();
                        }
                    }
                    PopupAction::ExecuteCommand(cmd) => {
                        self.popups.CloseModal();
                        return self.update(AppMessage::Command(cmd));
                    }
                    PopupAction::SelectTab(tab_idx) => {
                        self.popups.CloseModal();
                        return self.update(AppMessage::SelectTab(tab_idx));
                    }
                    PopupAction::OpenFile(path) => {
                        self.popups.CloseModal();
                        return self.update(AppMessage::OpenFile(path));
                    }
                    PopupAction::DismissContextMenu => {
                        self.popups.DismissContextMenu();
                    }
                    PopupAction::TriggerContextAction(ctx_act) => {
                        self.popups.DismissContextMenu();
                        use crate::fascia::popups::ContextAction;
                        match ctx_act {
                            ContextAction::OpenFile(path) => return self.update(AppMessage::OpenFile(path)),
                            ContextAction::InspectPath(path) => {
                                return self.update(AppMessage::Explorer(ExplorerAction::SelectNode(path)));
                            }
                            ContextAction::RunExample(uri) => return self.update(AppMessage::RunExample(uri)),
                            ContextAction::CopyText(txt) => {
                                self.popups.PushToast(ToastLevel::Info, "Copied", Some(txt.clone()));
                                return iced::clipboard::write(txt);
                            }
                            ContextAction::SelectTab(idx) => return self.update(AppMessage::SelectTab(idx)),
                            ContextAction::CloseTab(idx) => return self.update(AppMessage::CloseTab(idx)),
                            ContextAction::CloseOtherTabs(idx) => {
                                if let Some(keep_id) = self.tab_manager.tabs().Get(idx as u32).map(|t| t.id) {
                                    let mut to_close = Vec::new();
                                    self.tab_manager.tabs().Traverse(|t| {
                                        if t.id != keep_id {
                                            to_close.push(t.id);
                                        }
                                    });
                                    for id in to_close {
                                        let tabs = self.tab_manager.tabs();
                                        let mut found = None;
                                        for i in 0..tabs.Len() {
                                            if tabs.Get(i).is_some_and(|t| t.id == id) {
                                                found = Some(i as usize);
                                                break;
                                            }
                                        }
                                        if let Some(pos) = found {
                                            self.close_tab_internal(pos);
                                        }
                                    }
                                }
                            }
                            ContextAction::CloseAllTabs => return self.update(AppMessage::Command(CommandId::TAB_CLOSE_ALL)),
                            ContextAction::CancelJob(job_id) => {
                                let _ = self.heist.Cancel(job_id);
                                self.output.CancelJob(job_id);
                            }
                            ContextAction::ExecuteCommand(cmd) => return self.update(AppMessage::Command(cmd)),
                        }
                    }
                    PopupAction::DismissToast(id) => {
                        self.popups.DismissToast(id);
                    }
                }
            }
            AppMessage::Shell(action) => {
                match action {
                    ShellAction::SplitterPressed(target) => {
                        self.layout.BeginDrag(target, self.cursor_pos);
                    }
                    ShellAction::SplitterDoubleClicked(target) => {
                        self.layout.ResetSplit(target);
                    }
                    ShellAction::SplitterHovered(hovered) => {
                        self.layout.hovered_splitter = hovered;
                    }
                    ShellAction::FocusPane(pane) => {
                        self.layout.focused_pane = pane;
                    }
                }
            }
            AppMessage::CursorMoved(pos) => {
                self.cursor_pos = pos;
                if self.layout.IsDragging() {
                    self.layout.UpdateDrag(pos);
                }
            }
            AppMessage::MouseButtonReleased => {
                self.layout.EndDrag();
            }
            AppMessage::NewSketch => {
                let title = "Untitled Sketch".to_string();
                let (_, _, id) = self.tab_manager.open_sketch(None, title.clone());
                self._Documents.InsertSketch(id, crate::fascia::sketch_view::SketchViewerState::New(None, title, self.palette()));
                self.update_status_for_active_tab();
                self.status_info.message = "Choose a drawing tool; set a .flx or .tflx path to save".into();
            }
            AppMessage::NewMathView => {
                let doc = crate::fenst::fresco_document::FrescoDocument::CreateExample();
                self.OpenFrescoDocument(doc);
                self.status_info.message = "Opened Math View with built-in examples".into();
            }
            AppMessage::Plot2d(tab_id, action) => {
                if let Some(plot) = self._Documents.Plot2dMut(tab_id) {
                    plot.ProcessAction(action);
                }
            }
            AppMessage::Fresco(tab_id, action) => {
                use crate::fascia::math_view::FrescoAction;
                let changesSelection = matches!(&action,
                    FrescoAction::SelectRoot(_) | FrescoAction::SelectOccurrence(_)
                    | FrescoAction::NextOccurrence | FrescoAction::PrevOccurrence
                    | FrescoAction::CommitScalarEdit
                    | FrescoAction::NextRoot | FrescoAction::PrevRoot);
                if let Some(viewer) = self._Documents.FrescoMut(tab_id) {
                    viewer.Update(action);
                }
                if changesSelection && self.tab_manager.active_tab().is_some_and(|tab| tab.id == tab_id) {
                    self.InspectActiveFresco();
                }
            }
            AppMessage::SketchJobCompleted {
                tab_id,
                generation,
                cancellation,
                snapshot_rev,
                outcome,
            } => {
                if cancellation.load(std::sync::atomic::Ordering::Acquire) {
                    return Task::none();
                }
                let Some(viewer) = self._Documents.SketchMut(tab_id) else {
                    return Task::none();
                };
                if viewer.job_generation != generation
                    || !Arc::ptr_eq(&cancellation, &viewer.job_cancellation)
                {
                    return Task::none();
                }
                self.handle_sketch_job_outcome(tab_id, snapshot_rev, outcome);
                return Task::none();
            }
            AppMessage::RequestExit => { return self.BeginClosing(None, true); }
            AppMessage::ResolveSketchClose(decision) => {
                let Some(id) = self._ClosePrompt.take() else { return Task::none(); };
                if matches!(decision, SketchCloseDecision::Cancel) {
                    self._CloseQueue.Clear(); self._ExitAfterClose = false; return Task::none();
                }
                if matches!(decision, SketchCloseDecision::Save)
                    && let Some(viewer) = self._Documents.SketchMut(id) {
                        viewer.ProcessAction(crate::fascia::sketch_view::SketchViewAction::Save);
                        if viewer.IsDirty() {
                            self._CloseQueue.Clear(); self._ExitAfterClose = false;
                            self.status_info.message = "Sketch remains open: finish saving or resolve the file error".into();
                            return Task::none();
                        }
                }
                let mut index = None;
                let mut i = 0;
                self.tab_manager.tabs().Traverse(|tab| { if tab.id == id { index = Some(i); } i += 1; });
                if let Some(index) = index { self.close_tab_internal(index); }
                self._CloseQueue.Pop();
                return self.ContinueClosing();
            }
            AppMessage::Command(cmd) => {
                return self.ExecuteCommand(cmd);
            }
            AppMessage::Menu(action) => {
                if let Some(cmd) = action.ToCommand() {
                    return self.ExecuteCommand(cmd);
                }
                match action {
                    MenuAction::SelectTheme(t) => return self.update(AppMessage::SelectTheme(t)),
                    _ => return Task::none(),
                }
            }
            AppMessage::ToolBar(action) => {
                return self.ExecuteCommand(action.ToCommand());
            }
            AppMessage::SelectActivityTab(tab) => {
                if tab == ActivityTab::Settings {
                    self.tab_manager.open_settings();
                } else if tab == ActivityTab::Explorer {
                    if self.active_activity_tab == Some(ActivityTab::Explorer) && self.show_sidebar
                    {
                        self.show_sidebar = false;
                    } else if self.explorer.has_visible_roots() {
                        self.active_activity_tab = Some(ActivityTab::Explorer);
                        self.show_sidebar = true;
                        self.explorer.ensure_valid_active_tab();
                    } else {
                        self.show_sidebar = false;
                        self.status_info.message =
                            "Explorer is disabled: Select Filesystem, Settings, or Examples from View menu".to_string();
                    }
                } else if self.active_activity_tab == Some(tab) && self.show_sidebar {
                    self.show_sidebar = false;
                } else {
                    self.active_activity_tab = Some(tab);
                    self.show_sidebar = true;
                }
            }
            AppMessage::Example(action) => {
                return self._Examples.Update(action).map(AppMessage::Example);
            }
            AppMessage::Explorer(action) => {
                match action {
                    ExplorerAction::SelectTab(tab) => {
                        self.explorer.select_tab(tab);
                        self.status_info.message =
                            format!("Explorer: {} tab selected", tab.title());
                    }
                    ExplorerAction::SelectNode(path) => {
                        self.explorer.selected_path = Some(path.clone());
                        self.properties.InspectPath(&path);
                        self.status_info.message = format!("Selected {}", path.display());
                    }
                    ExplorerAction::ToggleFolder(path) => {
                        self.explorer.selected_path = Some(path.clone());
                        self.properties.InspectPath(&path);
                        self.explorer.toggle_path(&path);
                    }
                    ExplorerAction::OpenFile(path) => {
                        self.explorer.selected_path = Some(path.clone());
                        self.properties.InspectPath(&path);
                        return self.update(AppMessage::OpenFile(path));
                    }
                    ExplorerAction::SelectDrive(root) => {
                        self.explorer.set_root(root.clone());
                        self.status_info.message = format!("Root: {}", root.display());
                    }
                    ExplorerAction::Refresh => {
                        self.explorer.refresh();
                        self.status_info.message = "Explorer refreshed".to_string();
                    }
                    ExplorerAction::NodeContextMenu(path) => {
                        self.explorer.open_context_menu(path);
                    }
                    ExplorerAction::DismissContextMenu => {
                        self.explorer.dismiss_context_menu();
                    }
                    ExplorerAction::ToggleFilesystem => {
                        self.explorer.toggle_filesystem();
                        if !self.explorer.has_visible_roots() {
                            self.show_sidebar = false;
                            self.status_info.message =
                                "Explorer collapsed (all roots hidden)".to_string();
                        } else {
                            self.status_info.message = if self.explorer.show_filesystem {
                                "Filesystem shown in Explorer".to_string()
                            } else {
                                "Filesystem hidden in Explorer".to_string()
                            };
                        }
                    }
                    ExplorerAction::ToggleExamples => {
                        self.explorer.toggle_examples();
                        if !self.explorer.has_visible_roots() {
                            self.show_sidebar = false;
                            self.status_info.message =
                                "Explorer collapsed (all roots hidden)".to_string();
                        } else {
                            self.status_info.message = if self.explorer.show_examples {
                                "Examples shown in Explorer".to_string()
                            } else {
                                "Examples hidden in Explorer".to_string()
                            };
                        }
                    }
                    ExplorerAction::ToggleSettings => {
                        self.explorer.toggle_settings();
                        if !self.explorer.has_visible_roots() {
                            self.show_sidebar = false;
                            self.status_info.message =
                                "Explorer collapsed (all roots hidden)".to_string();
                        } else {
                            self.status_info.message = if self.explorer.show_settings {
                                "Settings shown in Explorer".to_string()
                            } else {
                                "Settings hidden in Explorer".to_string()
                            };
                        }
                    }
                    ExplorerAction::OpenCask(path) => {
                        self.explorer.dismiss_context_menu();
                        let (_idx, is_new, id) = self.tab_manager.open_cask(path.clone());
                        if is_new {
                            let view = GeometryViewerState::default();
                            let cancelled = view.Cancellation();
                            let replyToken = cancelled.clone();
                            self._Documents.InsertGeometry(id, view);
                            self.update_status_for_active_tab();
                            let palette = self.palette();
                            return Task::perform(
                            crate::fascia::geometry_load::LoadCaskWithPalette( path, cancelled, Some(palette)),
                            move |result| AppMessage::GeometryPrepared( id, replyToken.clone(), result),
                        );
                        }
                        self.update_status_for_active_tab();
                        self.status_info.message =
                            format!("Opened Cask 3D view for {}", path.display());
                    }
                    ExplorerAction::OpenCask2d(path) => {
                        self.explorer.dismiss_context_menu();
                        self.explorer.selected_path = Some(path.clone());
                        let (_idx, is_new, id) = self.tab_manager.open_cask2d(path.clone());
                        if is_new {
                            let viewer =
                                Cask2dViewerState::NewWithPalette(path.clone(), self.palette());
                            self._Documents.InsertCask2d(id, viewer);
                        }
                        self.update_status_for_active_tab();
                        self.status_info.message =
                            format!("Opened Cask 2D view for {}", path.display());
                    }
                }
            }
            AppMessage::TabBar(action) => {
                match action {
                    TabBarAction::SelectTab(idx) => return self.update(AppMessage::SelectTab(idx)),
                    TabBarAction::CloseTab(idx) => return self.update(AppMessage::CloseTab(idx)),
                    TabBarAction::NewTab => return self.update(AppMessage::NewFile),
                    TabBarAction::CloseAll => {
                        return self.BeginClosing(None, false);
                    }
                }
            }
            AppMessage::EditorAction(action) => {
                let is_edit = action.is_edit();
                if let Some(tab) = self.tab_manager.active_tab() {
                    let tab_id = tab.id;
                    if let Some(editor) = self._Documents.EditorMut(tab_id) {
                        editor.perform(action);
                        let pos = editor.cursor().position;
                        self.status_info.line = pos.line + 1;
                        self.status_info.column = pos.column + 1;
                    }
                    if is_edit {
                        self._Documents.SetEditorDirty(tab_id, true);
                        self.tab_manager.mark_active_dirty(true);
                    }
                }
            }
            AppMessage::Waveform(action) => {
                if let Some(tab) = self.tab_manager.active_tab()
                   && let Some(waveform) = self._Documents.WaveformMut(tab.id)
                {
                    waveform.Update(action);
                }
            }
            AppMessage::Geometry(id, action) => {
                if matches!(action, GeometryAction::Refresh) {
                    let mut request = None;
                    self.tab_manager.tabs().Traverse(|tab| {
                                               if tab.id == id {
                                                   request = tab.path.clone().map(|path| {
                                                                                 (path,
                                                                            tab.kind
                                                                            == TabKind::CaskViewer)
                                                                             });
                                               }
                                           });
                    if let Some((path, isCask)) = request
                       && let Some(view) = self._Documents.GeometryMut(id)
                    {
                        let viewbox = if isCask { view.CurrentViewBox() } else { None };
                        let cancelled = view.BeginReload();
                        let replyToken = cancelled.clone();
                        let palette = if isCask { Some(self.palette()) } else { None };
                        return Task::perform(
                            crate::fascia::geometry_load::ReloadWithViewBoxAndPalette( path, cancelled, isCask, viewbox, palette),
                            move |result| AppMessage::GeometryPrepared( id, replyToken.clone(), result),
                        );
                    }
                } else if let Some(view) = self._Documents.GeometryMut(id) {
                    view.Update(action);
                }
            }
            AppMessage::GeometryPrepared(id, cancelled, result) => {
                if !cancelled.load(std::sync::atomic::Ordering::Acquire)
                   && let Some(view) = self._Documents.GeometryMut(id)
                   && Arc::ptr_eq(&cancelled, &view.Cancellation())
                {
                    view.Complete(result);
                    if self.tab_manager
                           .active_tab()
                           .is_some_and(|tab| tab.id == id)
                    {
                        self.status_info.message = view.Error()
                                                       .or_else(|| view.ReloadError())
                                                       .unwrap_or("Geometry ready")
                                                       .to_string();
                    }
                }
            }
            AppMessage::Cask2d(id, action) => {
                match action {
                    Cask2dAction::OpenFile(path) => {
                        return self.update(AppMessage::OpenFile(path));
                    }
                    Cask2dAction::OpenSelectedIn2d(path) => {
                        return self.update(AppMessage::Explorer(ExplorerAction::OpenCask2d(path)));
                    }
                    Cask2dAction::OpenIn3d(path) => {
                        return self.update(AppMessage::Explorer(ExplorerAction::OpenCask(path)));
                    }
                    Cask2dAction::ToggleTheme => {
                        return self.update(AppMessage::ToggleTheme);
                    }
                    _ => {
                        if let Some(viewer) = self._Documents.Cask2dMut(id) {
                            viewer.Update(action);
                        }
                        if !self.properties.IsPinned()
                            && let Some(viewer) = self._Documents.Cask2d(id)
                        {
                            if let Some(pin) = viewer.SelectedPin() {
                                let dir = match pin.Direction() {
                                    crate::fenst::PortDirection::In => "InPort (West)",
                                    crate::fenst::PortDirection::Out => "OutPort (East)",
                                    crate::fenst::PortDirection::InOut => "InOutPort",
                                };
                                self.properties.InspectCustom(
                                    "Circuit Pin",
                                    format!("{}::{}", pin.ModuleName(), pin.PortName()),
                                    vec![
                                        ("Module".to_string(), pin.ModuleName().to_string()),
                                        ("Port".to_string(), pin.PortName().to_string()),
                                        ("Direction".to_string(), dir.to_string()),
                                        ("Net".to_string(), pin.NetName().to_string()),
                                    ],
                                );
                            } else if let Some(net) = viewer.SelectedNet() {
                                self.properties.InspectCustom(
                                    "Circuit Net",
                                    format!("net://{}", net),
                                    vec![
                                        ("Net Name".to_string(), net.to_string()),
                                        ("Kind".to_string(), "Interconnect Wire".to_string()),
                                    ],
                                );
                            } else if let Some(node) = viewer.SelectedNode() {
                                self.properties.InspectCircuit(
                                    node.Name(),
                                    node.Path(),
                                    0,
                                    node.ChildrenIndices().len() as u32,
                                );
                            }
                        }
                    }
                }
            }
            AppMessage::Sketch(id, action) => {
                if matches!(action, crate::fascia::sketch_view::SketchViewAction::OpenPath)
                    && let Some(viewer) = self._Documents.Sketch(id) {
                        return self.update(AppMessage::OpenFile(viewer.RequestedPath()));
                }
                if (matches!(action, crate::fascia::sketch_view::SketchViewAction::View3dWireframe)
                    || matches!(action, crate::fascia::sketch_view::SketchViewAction::InspectorAction(
                        crate::fascia::sketch_inspector::SketchInspectorAction::View3dWireframe
                    )))
                {
                    self.launch_sketch_job_on_heist(id, crate::fascia::sketch_job::SketchJobKind::Wireframe);
                    return Task::none();
                }

                let extrude_req = match &action {
                    crate::fascia::sketch_view::SketchViewAction::Extrude3d(d) => Some(*d),
                    crate::fascia::sketch_view::SketchViewAction::InspectorAction(
                        crate::fascia::sketch_inspector::SketchInspectorAction::Extrude3d
                    ) => {
                        if let Some(viewer) = self._Documents.SketchMut(id) {
                            match viewer.ExtrudeDepth() {
                                Ok(depth) => Some(depth),
                                Err(error) => {
                                    viewer.status_message = Some(error.clone());
                                    self.status_info.message = error;
                                    return Task::none();
                                }
                            }
                        } else { None }
                    }
                    _ => None,
                };

                if let Some(depth) = extrude_req {
                    self.launch_sketch_job_on_heist(id, crate::fascia::sketch_job::SketchJobKind::Extrude(depth));
                    return Task::none();
                }

                if let crate::fascia::sketch_view::SketchViewAction::Revolve3d { axis, angle_deg } = action {
                    self.launch_sketch_job_on_heist(id, crate::fascia::sketch_job::SketchJobKind::Revolve { axis, angle_deg });
                    return Task::none();
                }

                if matches!(action, crate::fascia::sketch_view::SketchViewAction::ExportSvg) {
                    let svg_path = self._Documents.Sketch(id).and_then(|v| v.path.as_ref().map(|p| p.with_extension("svg")));
                    if let Some(svg_path) = svg_path {
                        self.launch_sketch_job_on_heist(id, crate::fascia::sketch_job::SketchJobKind::ExportSvg(svg_path));
                    } else if let Some(viewer) = self._Documents.SketchMut(id) {
                        viewer.status_message = Some("Save the sketch before exporting SVG".into());
                    }
                    return Task::none();
                }

                if let Some(viewer) = self._Documents.SketchMut(id) {
                    viewer.ProcessAction(action);
                    if let Some(save_path) = viewer.TakePendingSave() {
                        self.launch_sketch_job_on_heist(id, crate::fascia::sketch_job::SketchJobKind::Save(save_path));
                        return Task::none();
                    }
                    let is_dirty = viewer.IsDirty();
                    let mut found_idx = None;
                    let mut idx = 0u32;
                    self.tab_manager.tabs().Traverse(|tab| {
                        if tab.id == id {
                            found_idx = Some(idx);
                        }
                        idx += 1;
                    });
                    if let Some(i) = found_idx {
                        self.tab_manager.mark_dirty(i as usize, is_dirty);
                        if let Some(tab) = self.tab_manager.active_tab_mut() && tab.id == id {
                            tab.path = viewer.path.clone(); tab.title = viewer.title.clone();
                        }
                    }
                }
            }
            AppMessage::OpenFile(path) => {
                let path_str = path.to_string_lossy();
                if path_str == FRESCO_EXAMPLE_URI || path_str.starts_with("example://Fresco/") {
                    let title = if path_str == FRESCO_EXAMPLE_URI {
                        FRESCO_EXAMPLE_TITLE.to_string()
                    } else {
                        path_str.trim_start_matches("example://Fresco/").to_string()
                    };
                    if let Some(doc) = OpenFrescoExample(&path_str) {
                        let (_, isNew, id) = self.tab_manager.OpenFrescoResource(path, title);
                        if isNew {
                            self._Documents.InsertFresco(id,
                                crate::fascia::math_view::FrescoViewerState::New(doc));
                        }
                        self.update_status_for_active_tab();
                        self.InspectActiveFresco();
                        return Task::none();
                    }
                }
                if let Some(theme_name) = path_str.strip_prefix("settings://theme/") {
                    let theme = match theme_name.to_lowercase().as_str() {
                        "windowsdark" => Some(FasciaTheme::WindowsDark),
                        "windowslight" => Some(FasciaTheme::WindowsLight),
                        "linuxdark" => Some(FasciaTheme::LinuxDark),
                        "linuxlight" => Some(FasciaTheme::LinuxLight),
                        "vscodedark" => Some(FasciaTheme::VsCodeDark),
                        _ => None,
                    };
                    if let Some(t) = theme {
                        return self.update(AppMessage::SelectTheme(t));
                    }
                } else if path_str == "settings://layout/Sidebar" {
                    return self.update(AppMessage::ToggleSidebar);
                } else if path_str == "settings://layout/StatusBar" {
                    return self.update(AppMessage::ToggleStatusBar);
                }
                if path_str.starts_with("example://") {
                    return self.update(AppMessage::Explorer(ExplorerAction::OpenCask2d(path)));
                }
                if path_str.contains("://")
                    && let Ok((_, branch)) = crate::fenst::XplrRegistry::New().OpenRoot(&path_str)
                    && branch.Circuit().is_some()
                {
                    return self.update(AppMessage::Explorer(ExplorerAction::OpenCask2d(path)));
                }
                let (idx, is_new) = self.tab_manager.open_file(path.clone());
                if is_new {
                    let tab = &self.tab_manager.tabs()[idx as u32];
                    if tab.kind == TabKind::VcdViewer {
                        let waveform = match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                let ext =
                                    path.extension().and_then(|s| s.to_str()).unwrap_or("vcd");
                                ParseWaveform(ext, &content)
                                    .map(|model| WaveformState::New(model.into()))
                                    .unwrap_or_else(WaveformState::FromError)
                            }
                            Err(error) => WaveformState::FromError(error.to_string()),
                        };
                        self._Documents.InsertWaveform(tab.id, waveform);
                    } else if tab.kind == TabKind::SketchViewer {
                        let viewer = crate::fascia::sketch_view::SketchViewerState::FromFile(
                            path.clone(),
                            self.palette(),
                        );
                        let id = tab.id;
                        match viewer {
                            Ok(viewer) => { self._Documents.InsertSketch(id, viewer); }
                            Err(error) => {
                                self.close_tab_internal(idx);
                                self.status_info.message = format!("Could not open {}: {error}", path.display());
                                return Task::none();
                            }
                        }
                    } else if matches!(tab.kind, TabKind::PtsViewer | TabKind::ObjViewer) {
                        let id = tab.id;
                        let view = GeometryViewerState::default();
                        let cancelled = view.Cancellation();
                        self._Documents.InsertGeometry(id, view);
                        self.update_status_for_active_tab();
                        self.status_info.message = format!("Loading {}", path.display());
                        return Task::perform(
                            crate::fascia::geometry_load::Load( path, cancelled.clone()),
                            move |result| AppMessage::GeometryPrepared( id, cancelled.clone(), result),
                        );
                    } else {
                        let content_str =
                            if path_str.contains("://") {
                                format!("// Virtual Xplr node: {}\n", path_str)
                            } else if is_text_file(&path) {
                                std::fs::read_to_string(&path)
                                .unwrap_or_else(|e| format!("// Error reading file: {}\n", e))
                            } else {
                                format!("// Binary or unsupported file: {}\n", path.display())
                            };
                        let editor_content = text_editor::Content::with_text(&content_str);
                        self._Documents.InsertEditor(tab.id, editor_content);
                    }
                }
                self.update_status_for_active_tab();
                self.status_info.message = format!("Opened {}", path.display());
            }
            AppMessage::SaveCurrentFile => {
                if let Some(tab) = self.tab_manager.active_tab() {
                    let tab_id = tab.id;
                    if tab.kind == TabKind::SketchViewer {
                        return self.update(AppMessage::Sketch(tab_id, crate::fascia::sketch_view::SketchViewAction::Save));
                    }
                    if let Some(path) = tab.path.clone() {
                        if let Some(editor) = self._Documents.Editor(tab_id) {
                            let text = editor.text();
                            if let Err(e) = std::fs::write(&path, text) {
                                self.status_info.message = format!("Error saving: {}", e);
                            } else {
                                self.tab_manager.mark_active_dirty(false);
                                self._Documents.SetEditorDirty(tab_id, false);
                                self.status_info.message = format!("Saved {}", path.display());
                            }
                        } else if let Some(sketch) = self._Documents.SketchMut(tab_id) {
                            if let Err(e) = sketch.SaveToFile(&path) {
                                self.status_info.message = format!("Error saving sketch: {}", e);
                            } else {
                                self.tab_manager.mark_active_dirty(false);
                                self._Documents.SetEditorDirty(tab_id, false);
                                self.status_info.message = format!("Saved {}", path.display());
                            }
                        }
                    } else {
                        self.tab_manager.mark_active_dirty(false);
                        self._Documents.SetEditorDirty(tab_id, false);
                        self.status_info.message = "Saved draft".to_string();
                    }
                }
            }
            AppMessage::NewFile => {
                self.tab_manager.open_new_file();
                if let Some(tab) = self.tab_manager.active_tab() {
                    let editor_content = text_editor::Content::with_text("// New file\n");
                    self._Documents.InsertEditor(tab.id, editor_content);
                }
                self.update_status_for_active_tab();
                self.status_info.message = "Created new file".to_string();
            }
            AppMessage::CloseActiveTab => {
                if let Some(idx) = self.tab_manager.active_index() {
                    return self.update(AppMessage::CloseTab(idx));
                }
            }
            AppMessage::CloseTab(idx) => {
                if let Some(tab) = self.tab_manager.tabs().Get(idx as u32) {
                    return self.BeginClosing(Some(tab.id), false);
                }
            }
            AppMessage::SelectTab(idx) => {
                if let Some(tab) = self.tab_manager.active_tab() && let Some(sketch) = self._Documents.SketchMut(tab.id) { sketch.tool_ctrl.ResetDraft(); }
                self.tab_manager.select_tab(idx);
                self.update_status_for_active_tab();
                self.InspectActiveFresco();
            }
            AppMessage::ToggleSidebar => {
                if self.show_sidebar {
                    self.show_sidebar = false;
                } else if self.active_activity_tab == Some(ActivityTab::Explorer)
                          && !self.explorer.has_visible_roots()
                {
                    self.show_sidebar = false;
                    self.status_info.message =
                        "Explorer is disabled: Select Filesystem, Settings, or Examples from View menu".to_string();
                } else {
                    self.show_sidebar = true;
                }
            }
            AppMessage::ToggleStatusBar => {
                self.show_status_bar = !self.show_status_bar;
            }
            AppMessage::SelectTheme(t) => {
                self.theme = t;
                let palette = self.palette();
                self.status_info.message = format!("Theme: {}", t.display_name());
                self._Documents.TraverseMut(|document| match document {
                    TabDocument::Cask2d(viewer) => viewer.SetPalette(palette),
                    TabDocument::Sketch(viewer) => {
                        viewer.palette = Some(palette);
                        viewer.dark_theme = palette.is_dark();
                        viewer.canvas_state.InvalidateAll();
                    }
                    _ => {}
                });
                let mut reload_tasks = Vec::new();
                self.tab_manager.tabs().Traverse(|tab| {
                    if tab.kind == TabKind::CaskViewer
                        && let Some(ref path) = tab.path
                        && let Some(view) = self._Documents.GeometryMut(tab.id)
                    {
                        let viewbox = view.CurrentViewBox();
                        let cancelled = view.BeginReload();
                        let replyToken = cancelled.clone();
                        let id = tab.id;
                        let path_clone = path.clone();
                        reload_tasks.push(Task::perform(
                            crate::fascia::geometry_load::ReloadWithViewBoxAndPalette(
                                path_clone, cancelled, true, viewbox, Some(palette),
                            ),
                            move |result| AppMessage::GeometryPrepared(id, replyToken.clone(), result),
                        ));
                    }
                });
                if !reload_tasks.is_empty() {
                    return Task::batch(reload_tasks);
                }
            }
            AppMessage::ToggleTheme => {
                let next_theme = match self.theme {
                    FasciaTheme::WindowsDark => FasciaTheme::WindowsLight,
                    FasciaTheme::WindowsLight => FasciaTheme::LinuxDark,
                    FasciaTheme::LinuxDark => FasciaTheme::LinuxLight,
                    FasciaTheme::LinuxLight => FasciaTheme::VsCodeDark,
                    FasciaTheme::VsCodeDark => FasciaTheme::WindowsDark,
                };
                return self.update(AppMessage::SelectTheme(next_theme));
            }
            AppMessage::OpenSettings => {
                self.tab_manager.open_settings();
                self.update_status_for_active_tab();
            }
            AppMessage::OpenPythonConsole => {
                return self.update(AppMessage::SelectBottomTab(
                    crate::fascia::bottom_tray::BottomTab::PythonConsole));
            }
            AppMessage::PythonConsole(id, action) => {
                return self.HandlePythonConsole(Some(id), action);
            }
            AppMessage::PythonConsoleExecuted(id, result) => {
                self.CompletePythonConsole(Some(id), result);
                return Task::none();
            }
            AppMessage::TrayPythonConsole(action) => {
                return self.HandlePythonConsole(None, action);
            }
            AppMessage::TrayPythonConsoleExecuted(result) => {
                self.CompletePythonConsole(None, result);
                return Task::none();
            }
            AppMessage::OpenOutput => {
                return self.update(AppMessage::SelectBottomTab(
                    crate::fascia::bottom_tray::BottomTab::Output));
            }
            AppMessage::OpenOutputLink(uri) => {
                let path = PathBuf::from(&uri);
                if uri.starts_with("waveform://") {
                    let vcd = uri.strip_prefix("waveform://job/")
                        .and_then(|id| id.parse().ok())
                        .and_then(|id| self.output.GetJob(crate::fascia::output::JobId(id)))
                        .and_then(|job| job.waveform.as_deref())
                        .or_else(|| self.waveform_results.get(&uri).map(String::as_str));
                    let Some(vcd) = vcd else {
                        self.status_info.message = "Waveform result is unavailable".into();
                        return Task::none();
                    };
                    let (_, isNew, tab) = self.tab_manager.open_waveform(
                        format!("Waveform · {}", uri.trim_start_matches("waveform://")), Some(path));
                    if isNew {
                        let waveform = ParseWaveform("vcd", vcd)
                            .map(|model| WaveformState::New(model.into()))
                            .unwrap_or_else(WaveformState::FromError);
                        self._Documents.InsertWaveform(tab, waveform);
                    }
                    self.update_status_for_active_tab();
                    return Task::none();
                } else if uri == FRESCO_EXAMPLE_URI || uri.starts_with("example://Fresco/") {
                    return self.update(AppMessage::OpenFile(path));
                } else if uri.starts_with("example://") {
                    return self.update(AppMessage::Explorer(ExplorerAction::OpenCask2d(path)));
                } else {
                    return self.update(AppMessage::OpenFile(path));
                }
            }
            AppMessage::RunExample(uri) => {
                if uri.starts_with("example://Fresco/") {
                    let id = self.output.StartJob(format!("Run {}", uri), Some(uri.clone()));
                    let uri_clone = uri.clone();
                    self.heist.Submit(
                        id,
                        crate::heist::scheduler::JobPriority::Normal,
                        format!("Run {}", uri),
                        move |_token, reporter| {
                            if uri_clone == FRESCO_EXAMPLE_URI {
                                reporter.Log(
                                    crate::fascia::output::DiagnosticSeverity::Info,
                                    "Fresco",
                                    format!("Ready: {}", uri_clone),
                                );
                            } else if let Some(ex) = crate::fenst::example::XplrExample::FindByUri(&uri_clone) {
                                match ex.Run() {
                                    Ok(_) => {
                                        reporter.Log(
                                            crate::fascia::output::DiagnosticSeverity::Info,
                                            "Fresco",
                                            format!("Verified {}::{}", ex.suite, ex.name),
                                        );
                                    }
                                    Err(err) => {
                                        reporter.Log(
                                            crate::fascia::output::DiagnosticSeverity::Error,
                                            "Fresco",
                                            format!("{} failed: {}", uri_clone, err),
                                        );
                                    }
                                }
                            }
                        },
                    );
                    return self.update(AppMessage::OpenFile(PathBuf::from(uri)));
                }
                if uri == DADDA_EXAMPLE_URI {
                    let id = self.output.StartJob("Simulate Dadda multiplier", Some(uri));
                    self.output.UpdateJobProgress(id, 0.0, "Recording simulation");
                    let token = self.output.JobCancellation(id).expect("New job has a token");
                    return Task::perform(
                        async move { RunDadda(DADDA_STIMULUS.as_slice().into(),
                                             || token.load(std::sync::atomic::Ordering::Acquire)) },
                        move |result| AppMessage::DaddaFinished(id, result),
                    );
                }
                let Some(ex) = crate::fenst::example::XplrExample::FindByUri(&uri) else {
                    self.status_info.message = format!("Runnable example not found: {uri}");
                    self.output.Log(crate::fascia::output::DiagnosticSeverity::Error, "Runner",
                                    self.status_info.message.clone());
                    return Task::none();
                };
                let job_id = self.output.StartJob(format!("Run {}", uri), Some(uri.clone()));
                self.output.UpdateJobProgress(job_id, 0.0, "Starting...");
                self.status_info.message = format!("Running example: {}", uri);
                let cancellation = self.output.JobCancellation(job_id).expect("Job cancellation token exists");
                return Task::perform(
                    async move {
                        if cancellation.load(std::sync::atomic::Ordering::Acquire) {
                            return Err("Cancelled before start".to_string());
                        }
                        let (suite, name) = (ex.suite, ex.name);
                        if cancellation.load(std::sync::atomic::Ordering::Acquire) {
                            return Err("Cancelled before execution".to_string());
                        }
                        match ex.Run() {
                            Ok(root) => {
                                if cancellation.load(std::sync::atomic::Ordering::Acquire) {
                                    return Err("Cancelled after execution".to_string());
                                }
                                let summary = format!("Example {}::{} passed (root: {})", suite, name, root.Name());
                                return Ok((summary, Some(uri), None));
                            }
                            Err(err) => {
                                return Err(format!("Example {}::{} failed: {}", suite, name, err));
                            }
                        }
                    },
                    move |res| AppMessage::JobFinished(job_id, res),
                );
            }
            AppMessage::JobFinished(job_id, result) => {
                if self.output.GetJob(job_id).is_none_or(|job| job.status.IsTerminal()) {
                    return Task::none();
                }
                match result {
                    Ok((summary, result_uri, waveform)) => {
                        if let (Some(uri), Some(waveform)) = (result_uri.as_ref(), waveform) {
                            self.waveform_results.insert(uri.clone(), waveform);
                        }
                        self.output.CompleteJob(job_id, &summary, result_uri);
                        self.status_info.message = format!("Job completed: {}", summary);
                    }
                    Err(error) => {
                        if error.starts_with("Cancelled") {
                            self.output.CancelJob(job_id);
                            self.status_info.message = format!("Job cancelled: {}", job_id);
                        } else {
                            self.output.FailJob(job_id, &error);
                            self.status_info.message = format!("Job failed: {}", error);
                        }
                    }
                }
                return Task::none();
            }
            AppMessage::DaddaFinished(id, result) => {
                let Some(job) = self.output.GetJob(id).filter(|job| !job.status.IsTerminal()) else {
                    return Task::none();
                };
                if job.cancellation.load(std::sync::atomic::Ordering::Acquire) {
                    self.output.CancelJob(id);
                } else {
                    match result {
                        Ok(waveform) => self.output.CompleteWaveform(id, waveform),
                        Err(DaddaRunError::Cancelled) => self.output.CancelJob(id),
                        Err(DaddaRunError::Failed(error)) => self.output.FailJob(id, error),
                    }
                }
                return Task::none();
            }
            AppMessage::ToggleBottomTray => {
                self.show_bottom_tray = !self.show_bottom_tray;
                self.status_info.message = if self.show_bottom_tray {
                    "Bottom panel shown".to_string()
                } else {
                    "Bottom panel hidden".to_string()
                };
                return Task::none();
            }
            AppMessage::SelectBottomTab(tab) => {
                self.active_bottom_tab = tab;
                self.show_bottom_tray = true;
                return Task::none();
            }
            AppMessage::BottomTray(action) => {
                match action {
                    crate::fascia::bottom_tray::BottomTrayAction::SelectTab(tab) => {
                        self.active_bottom_tab = tab;
                    }
                    crate::fascia::bottom_tray::BottomTrayAction::Clear => {
                        match self.active_bottom_tab {
                            crate::fascia::bottom_tray::BottomTab::Output => self.output.Clear(),
                            crate::fascia::bottom_tray::BottomTab::PythonConsole => self.console.clear(),
                        }
                    }
                    crate::fascia::bottom_tray::BottomTrayAction::Close => {
                        self.show_bottom_tray = false;
                    }
                }
                return Task::none();
            }
            AppMessage::ToggleProperties => {
                self.show_properties = !self.show_properties;
                self.status_info.message = if self.show_properties {
                    "Properties panel shown".to_string()
                } else {
                    "Properties panel hidden".to_string()
                };
                return Task::none();
            }
            AppMessage::Properties(action) => {
                match action {
                    crate::fascia::properties::PropertiesAction::TogglePin => {
                        self.properties.TogglePinned();
                    }
                    crate::fascia::properties::PropertiesAction::Close => {
                        self.show_properties = false;
                    }
                    crate::fascia::properties::PropertiesAction::OpenTarget(uri) => {
                        return self.update(AppMessage::OpenOutputLink(uri));
                    }
                    crate::fascia::properties::PropertiesAction::RunTarget(uri) => {
                        return self.update(AppMessage::RunExample(uri));
                    }
                    crate::fascia::properties::PropertiesAction::UseInPython(snippet) => {
                        self.console.input_changed(snippet);
                        return self.update(AppMessage::OpenPythonConsole);
                    }
                }
                return Task::none();
            }
        }
        Task::none()
    }
    pub fn view(&self) -> Element<'_, AppMessage>
    {
        let palette = self.palette();
        // 1. Top Bar (Menubar + Quick Open + Pane Toggles)
        let menubar = ViewMenubarWithContext(palette,
                                   self.theme,
                                   (self.explorer.show_filesystem,
                                    self.explorer.show_settings,
                                    self.explorer.show_examples),
                                   Some(&self.CommandContext()),
                                   AppMessage::Menu);
        let top_bar = view_top_bar(
            menubar,
            palette,
            self.show_sidebar,
            self.show_bottom_tray,
            self.show_properties,
            AppMessage::Command,
        );
        // 3. Activity Bar (Leftmost vertical strip)
        let activity_bar = view_activity_bar(self.active_activity_tab,
                                             palette,
                                             AppMessage::SelectActivityTab);
        // 4. Sidebar (Collapsible)
        let sidebar = if self.show_sidebar {
            match self.active_activity_tab {
                Some(ActivityTab::Explorer) => {
                    if self.explorer.has_visible_roots() {
                        Some(crate::fascia::explorer::view_explorer(&self.explorer,
                                                                    palette,
                                                                    AppMessage::Explorer))
                    } else {
                        None
                    }
                }
                Some(ActivityTab::Search) => Some(view_search_sidebar(
                    &self.search_query,
                    &self.search_hits,
                    self.search_running,
                    palette,
                )),
                Some(ActivityTab::Settings) => {
                    if self.explorer.has_visible_roots() {
                        Some(crate::fascia::explorer::view_explorer(&self.explorer,
                                                                    palette,
                                                                    AppMessage::Explorer))
                    } else {
                        None
                    }
                }
                None => None,
            }
        } else {
            None
        };
        // 5. Content Area (VS Code-style Multi-Tabs)
        let tab_bar = view_tab_bar(self.tab_manager.tabs(),
                                   self.tab_manager.active_index(),
                                   palette,
                                   AppMessage::TabBar);
        let active_tab_content: Element<'_, AppMessage> =
            if let Some(active_tab) = self.tab_manager.active_tab() {
                match active_tab.kind {
                    TabKind::Welcome => view_welcome(palette),
                    TabKind::Settings => {
                        view_settings(self.theme, palette, self.show_sidebar, self.show_status_bar)
                    }
                    TabKind::FileEditor => {
                        if let Some(editor) = self._Documents.Editor(active_tab.id) {
                            view_editor(editor, self.theme, palette)
                        } else {
                            container(text("Opening buffer...").size(14)).padding(20)
                                                                         .into()
                        }
                    }
                    TabKind::VcdViewer => {
                        if let Some(waveform) = self._Documents.Waveform(active_tab.id) {
                            view_waveform(waveform, palette, AppMessage::Waveform)
                        } else {
                            container(text("Opening waveform...").size(14)).padding(20)
                                                                           .into()
                        }
                    }
                    TabKind::PtsViewer | TabKind::ObjViewer | TabKind::CaskViewer => {
                        if let Some(view) = self._Documents.Geometry(active_tab.id) {
                            let id = active_tab.id;
                            ViewGeometry(id.0, view, palette, move |action| {
                                AppMessage::Geometry(id, action)
                            })
                        } else {
                            container(text("Opening geometry...").size(14)).padding(20)
                                                                           .into()
                        }
                    }
                    TabKind::Cask2dViewer => {
                        if let Some(viewer) = self._Documents.Cask2d(active_tab.id) {
                            let id = active_tab.id;
                            view_cask2d_tab(id, viewer, palette, move |action| {
                                AppMessage::Cask2d(id, action)
                            })
                        } else {
                            container(text("Opening Cask 2D...").size(14)).padding(20)
                                                                          .into()
                        }
                    }
                    TabKind::SketchViewer => {
                        if let Some(viewer) = self._Documents.Sketch(active_tab.id) {
                            let id = active_tab.id;
                            crate::fascia::sketch_view::view_sketch_tab(id, viewer, palette, move |action| {
                                AppMessage::Sketch(id, action)
                            })
                        } else {
                            container(text("Opening sketch...").size(14)).padding(20)
                                                                         .into()
                        }
                    }
                    TabKind::FrescoViewer => {
                        if let Some(viewer) = self.Fresco(active_tab.id) {
                            let id = active_tab.id;
                            crate::fascia::math_view::ViewMathTab(viewer, move |action| {
                                AppMessage::Fresco(id, action)
                            }, palette)
                        } else {
                            container(text("Opening Math View...").size(14)).padding(20)
                                                                         .into()
                        }
                    }
                    TabKind::Plot2d => {
                        if let Some(plot) = self._Documents.Plot2d(active_tab.id) {
                            let id = active_tab.id;
                            ViewPlot2d(plot, palette, move |action| {
                                AppMessage::Plot2d(id, action)
                            })
                        } else {
                            container(text("Opening 2D Plot...").size(14)).padding(20)
                                                                       .into()
                        }
                    }
                    TabKind::PythonConsole => {
                        if let Some(console) = self._Documents.Console(active_tab.id) {
                            let id = active_tab.id;
                            view_python_console(console, self.theme, palette, move |action| {
                                AppMessage::PythonConsole(id, action)
                            })
                        } else {
                            container(text("Opening Python console...").size(14)).padding(20)
                                                                                 .into()
                        }
                    }
                    TabKind::Output => {
                        crate::fascia::output::view_output_with_cancel(
                            &self.output,
                            palette,
                            AppMessage::OpenOutputLink,
                            Some(AppMessage::CancelJob),
                        )
                    }
                }
            } else {
                container(
                          column![
                    text("No Open Editors").size(18).style(move |_| {
                        text::Style { color: Some(palette.text_muted), }
                    }),
                    text("Select a file from the explorer or press New File (Ctrl+N)").size(13)
                                                                                      .style(
                        move |_| text::Style { color: Some(palette.text_secondary), }
                    ),
                ].spacing(8)
                          .align_x(Alignment::Center),
                ).width(Length::Fill)
                 .height(Length::Fill)
                 .align_x(Alignment::Center)
                 .align_y(Alignment::Center)
                 .into()
            };
        let content_column =
            column![tab_bar,
                    container(active_tab_content).width(Length::Fill)
                                                 .height(Length::Fill),].width(Length::Fill)
                                                                        .height(Length::Fill);
        // 6. Bottom Tray
        let bottom_tray = if self.show_bottom_tray {
            let output_widget = crate::fascia::output::view_output_with_cancel(
                &self.output,
                palette,
                AppMessage::OpenOutputLink,
                Some(AppMessage::CancelJob),
            );
            let console_widget = view_python_console(
                &self.console,
                self.theme,
                palette,
                AppMessage::TrayPythonConsole,
            );
            Some(crate::fascia::bottom_tray::view_bottom_tray(
                self.active_bottom_tab,
                output_widget,
                Some(console_widget),
                self.output.ErrorCount(),
                self.output.WarningCount(),
                palette,
                AppMessage::BottomTray,
            ))
        } else {
            None
        };

        // 7. Properties Panel
        let properties = if self.show_properties {
            Some(crate::fascia::properties::view_properties(
                &self.properties,
                palette,
                AppMessage::Properties,
            ))
        } else {
            None
        };

        // 8. Status Bar
        let status_bar = if self.show_status_bar {
            Some(view_status_bar(&self.status_info, self.theme, palette))
        } else {
            None
        };
        let ctx = self.CommandContext();
        let tool_ctx = match self.tab_manager.active_tab() {
            None => ActiveToolContext::None,
            Some(tab) => match tab.kind {
                TabKind::FileEditor => ActiveToolContext::Editor { can_save: ctx.can_save },
                TabKind::SketchViewer => ActiveToolContext::Sketch {
                    can_undo: ctx.can_undo,
                    can_redo: ctx.can_redo,
                    can_save: ctx.can_save,
                },
                TabKind::FrescoViewer => ActiveToolContext::Fresco,
                _ => ActiveToolContext::Other,
            },
        };
        let contextual_toolbar = view_contextual_toolbar(
            tool_ctx,
            self.output.ActiveJobCount() as u32,
            ctx.has_runnable_example,
            palette,
            AppMessage::Command,
        );

        // Complete Shell Assembly
        let shell = view_shell_with_layout(
            top_bar,
            Some(contextual_toolbar),
            activity_bar,
            sidebar,
            content_column.into(),
            bottom_tray,
            properties,
            status_bar,
            &self.layout,
            palette,
            AppMessage::Shell,
        );
        let with_popups = ViewPopupOverlay(
            shell,
            &self.popups,
            self.tab_manager.tabs(),
            &ctx,
            palette,
            AppMessage::Popup,
        );
        if self._ClosePrompt.is_some() {
            let prompt = container(column![
                text("Save this sketch before closing?").size(18).style(move |_| text::Style {
                    color: Some(palette.text_primary),
                }),
                text("Unsaved changes will be lost if you discard them.").size(13).style(move |_| text::Style {
                    color: Some(palette.text_secondary),
                }),
                row![
                    button("Save and close").padding([6, 12]).on_press(AppMessage::ResolveSketchClose(SketchCloseDecision::Save)),
                    button("Discard").padding([6, 12]).on_press(AppMessage::ResolveSketchClose(SketchCloseDecision::Discard)),
                    button("Cancel").padding([6, 12]).on_press(AppMessage::ResolveSketchClose(SketchCloseDecision::Cancel)),
                ].spacing(10),
            ].spacing(14).padding(22)).style(move |_| FasciaStyle::overlay_container(palette));
            return iced::widget::stack![with_popups, iced::widget::opaque(container(prompt).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).center_y(Length::Fill))].into();
        }
        return with_popups;
    }
    pub fn theme(&self) -> iced::Theme
    {
        match self.theme {
            FasciaTheme::WindowsDark | FasciaTheme::LinuxDark | FasciaTheme::VsCodeDark => {
                iced::Theme::Dark
            }
            FasciaTheme::WindowsLight | FasciaTheme::LinuxLight => iced::Theme::Light,
        }
    }

    fn BeginClosing(&mut self, only: Option<TabId>, exit: bool) -> Task<AppMessage>
    {
        self._CloseQueue.Clear();
        self._ExitAfterClose = exit;
        if let Some(id) = only {
            self._CloseQueue.Push(id);
        } else {
            self.tab_manager
                .tabs()
                .Traverse(|tab| self._CloseQueue.Push(tab.id));
        }
        return self.ContinueClosing();
    }

    fn ContinueClosing(&mut self) -> Task<AppMessage>
    {
        while !self._CloseQueue.IsEmpty() {
            let id = self._CloseQueue[self._CloseQueue.Size() - 1];
            let mut index = None;
            let mut i = 0;
            self.tab_manager.tabs().Traverse(|tab| {
                                       if tab.id == id {
                                           index = Some(i);
                                       }
                                       i += 1;
                                   });
            if let Some(index) = index {
                if self._Documents.Get(id)
                       .is_some_and(IWorkbenchDocument::RequiresCloseConfirmation)
                {
                    self.tab_manager.select_tab(index);
                    self._ClosePrompt = Some(id);
                    return Task::none();
                }
                self.close_tab_internal(index);
            }
            self._CloseQueue.Pop();
        }
        if self._CloseQueue.IsEmpty() && self.tab_manager.tabs().IsEmpty() {
            self.status_info.message = "All tabs closed".to_string();
        }
        if self._ExitAfterClose {
            self._ExitAfterClose = false;
            if self.console.is_running {
                self.console.RequestInterrupt();
            }
            return iced::exit();
        }
        return Task::none();
    }

    fn launch_sketch_job_on_heist(
        &mut self,
        id: TabId,
        kind: crate::fascia::sketch_job::SketchJobKind,
    )
    {
        if let Some(viewer) = self._Documents.SketchMut(id) {
            let (job_gen, token, snap) = viewer.BeginJob();
            let job_id = self.output.StartJob(format!("Sketch {:?}", kind), None);
            crate::fascia::sketch_job::RunSketchJobOnHeist(
                &self.heist,
                job_id,
                id,
                job_gen,
                snap,
                kind,
                token,
            );
        }
    }

    fn handle_sketch_job_outcome(&mut self, tab_id: TabId, snapshot_rev: u64,
                                 outcome: crate::fascia::sketch_job::SketchJobOutcome)
    {
        match outcome {
            crate::fascia::sketch_job::SketchJobOutcome::Wireframe(res) => {
                let title = self._Documents.Sketch(tab_id).map(|v| v.title.clone()).unwrap_or_default();
                self.open_sketch_wireframe_tab(tab_id, title, res);
            }
            crate::fascia::sketch_job::SketchJobOutcome::Extrude { depth, asset } => {
                let title = self._Documents.Sketch(tab_id).map(|v| v.title.clone()).unwrap_or_default();
                self.open_sketch_extrude_tab(tab_id, title, depth, asset);
            }
            crate::fascia::sketch_job::SketchJobOutcome::Revolve { angle_deg, asset } => {
                let title = self._Documents.Sketch(tab_id).map(|v| v.title.clone()).unwrap_or_default();
                self.open_sketch_revolve_tab(tab_id, title, angle_deg, asset);
            }
            crate::fascia::sketch_job::SketchJobOutcome::ExportSvg(res) => {
                self.handle_sketch_svg_outcome(tab_id, res);
            }
            crate::fascia::sketch_job::SketchJobOutcome::Save { path, metadata } => {
                self.handle_sketch_save_outcome(tab_id, snapshot_rev, path, metadata);
            }
            crate::fascia::sketch_job::SketchJobOutcome::Cancelled => {}
        }
    }

    fn open_sketch_wireframe_tab(&mut self, tab_id: TabId, title: String,
                                 res: Result<Arc<GeometryAsset>, String>)
    {
        match res {
            Ok(asset) => {
                let tab_title = format!("🌐 3D: {}", title);
                let (_, _, new_id) = self.tab_manager.open_geometry_tab(tab_title);
                let mut view = GeometryViewerState::default();
                view.Complete(Ok(asset));
                self._Documents.InsertGeometry(new_id, view);
                self.update_status_for_active_tab();
                self.status_info.message = format!("Generated 3D wireframe for {}", title);
            }
            Err(e) => {
                if let Some(v) = self._Documents.SketchMut(tab_id) {
                    v.status_message = Some(format!("3D wireframe error: {}", e));
                }
                self.status_info.message = format!("3D wireframe error: {}", e);
            }
        }
    }

    fn open_sketch_extrude_tab(&mut self, tab_id: TabId, title: String, depth: f64,
                               asset: Result<Arc<GeometryAsset>, String>)
    {
        match asset {
            Ok(asset) => {
                let tab_title = format!("🧊 Extrude: {}", title);
                let (_, _, new_id) = self.tab_manager.open_geometry_tab(tab_title);
                let mut view = GeometryViewerState::default();
                view.Complete(Ok(asset));
                self._Documents.InsertGeometry(new_id, view);
                self.update_status_for_active_tab();
                self.status_info.message = format!("Extruded 3D solid ({:.1} mm) for {}", depth, title);
            }
            Err(e) => {
                if let Some(v) = self._Documents.SketchMut(tab_id) {
                    v.status_message = Some(format!("3D extrude error: {}", e));
                }
                self.status_info.message = format!("3D extrude error: {}", e);
            }
        }
    }

    fn open_sketch_revolve_tab(&mut self, tab_id: TabId, title: String, angle_deg: f64,
                               asset: Result<Arc<GeometryAsset>, String>)
    {
        match asset {
            Ok(asset) => {
                let tab_title = format!("🔄 Revolve: {}", title);
                let (_, _, new_id) = self.tab_manager.open_geometry_tab(tab_title);
                let mut view = GeometryViewerState::default();
                view.Complete(Ok(asset));
                self._Documents.InsertGeometry(new_id, view);
                self.update_status_for_active_tab();
                self.status_info.message = format!("Revolved 3D lathe ({:.0}°) for {}", angle_deg, title);
            }
            Err(e) => {
                if let Some(v) = self._Documents.SketchMut(tab_id) {
                    v.status_message = Some(format!("3D revolve error: {}", e));
                }
                self.status_info.message = format!("3D revolve error: {}", e);
            }
        }
    }

    fn handle_sketch_svg_outcome(&mut self, tab_id: TabId, res: Result<PathBuf, String>)
    {
        match res {
            Ok(path) => {
                let msg = format!("Exported SVG: {}", path.display());
                if let Some(v) = self._Documents.SketchMut(tab_id) {
                    v.status_message = Some(msg.clone());
                }
                self.status_info.message = msg;
            }
            Err(e) => {
                let msg = format!("SVG export failed: {}", e);
                if let Some(v) = self._Documents.SketchMut(tab_id) {
                    v.status_message = Some(msg.clone());
                }
                self.status_info.message = msg;
            }
        }
    }

    fn handle_sketch_save_outcome(&mut self, tab_id: TabId, snapshot_rev: u64, path: PathBuf,
                                  metadata: Result<crate::flux::FluxValue, String>)
    {
        let Some(viewer) = self._Documents.SketchMut(tab_id) else {
            return;
        };
        match metadata {
            Ok(meta) => {
                viewer.ApplySavedOutcome(path.clone(), snapshot_rev, meta);
                let is_dirty = viewer.IsDirty();
                let mut found_idx = None;
                let mut idx = 0u32;
                self.tab_manager.tabs().Traverse(|tab| {
                    if tab.id == tab_id {
                        found_idx = Some(idx);
                    }
                    idx += 1;
                });
                if let Some(i) = found_idx {
                    self.tab_manager.mark_dirty(i as usize, is_dirty);
                    if let Some(tab) = self.tab_manager.active_tab_mut() && tab.id == tab_id {
                        tab.path = viewer.path.clone();
                        tab.title = viewer.title.clone();
                    }
                }
                self.status_info.message = format!("Saved {}", path.display());
            }
            Err(e) => {
                let msg = format!("Error saving sketch: {}", e);
                viewer.status_message = Some(msg.clone());
                self.status_info.message = msg;
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

fn view_welcome<'a>(palette: ThemePalette) -> Element<'a, AppMessage>
{
    let title = text("Fascia").size(36)
                              .style(move |_| text::Style { color: Some(palette.accent), });
    let subtitle = text("Native Multi-Tabbed Workspace Shell for Rust & Iced").size(16)
                                                                              .style(move |_| {
                                                                                  text::Style {
            color: Some(palette.text_secondary),
        }
                                                                              });
    let action_card =
        |icon: &'static str, label: &'static str, desc: &'static str, msg: AppMessage| {
            button(column![
                    row![
                        text(icon).size(20),
                        Space::new().width(Length::Fixed(8.0)),
                        text(label).size(14).style(move |_| text::Style {
                            color: Some(palette.text_primary),
                        }),
                    ]
                    .align_y(Alignment::Center),
                    Space::new().height(Length::Fixed(4.0)),
                    text(desc).size(12).style(move |_| text::Style {
                        color: Some(palette.text_muted),
                    }),
                ].padding([12, 16])).width(Length::Fixed(260.0))
                                    .style(move |_, status| {
                                        FasciaStyle::tree_row_button(palette, false, status)
                                    })
                                    .on_press(msg)
        };
    let actions = row![action_card("📄",
                                   "New File",
                                   "Create an untitled draft buffer",
                                   AppMessage::NewFile),
                       action_card("📁",
                                   "Explore Files",
                                   "Browse folders and drives",
                                   AppMessage::SelectActivityTab(ActivityTab::Explorer)),
                       action_card("🐍",
                                   "Python Console",
                                   "In-process Python REPL with Trellis API",
                                   AppMessage::OpenPythonConsole),].spacing(12);
    let actions_row_2 = row![
        action_card(
            "🎨",
            "Switch Native Theme",
            "Cycle Windows Fluent & Linux Adwaita",
            AppMessage::ToggleTheme
        ),
        action_card(
            "⚙",
            "Configure Settings",
            "Customize layout and preferences",
            AppMessage::OpenSettings
        ),
        action_card(
            "🗺",
            "Cask 2D Visualizer",
            "Explore workspace tree in 2D",
            AppMessage::Explorer(ExplorerAction::OpenCask2d(crate::fascia::explorer::default_initial_dir())),
        ),
    ]
    .spacing(12);
    let content = column![title,
                          subtitle,
                          Space::new().height(Length::Fixed(24.0)),
                          actions,
                          Space::new().height(Length::Fixed(12.0)),
                          actions_row_2,].padding([32, 48])
                                         .max_width(700.0);
    scrollable(container(content).width(Length::Fill)
                                 .align_x(Alignment::Center)).into()
}

//-----------------------------------------------------------------------------------------------------------------------------

fn view_editor<'a>(content: &'a text_editor::Content, theme: FasciaTheme, palette: ThemePalette)
                   -> Element<'a, AppMessage>
{
    let font = default_code_font(theme);
    let editor = text_editor(content).font(font)
                                     .padding(12)
                                     .on_action(AppMessage::EditorAction);
    container(editor).width(Length::Fill)
                     .height(Length::Fill)
                     .style(move |_| FasciaStyle::content_container(palette))
                     .into()
}
fn run_workspace_search(
    root: &std::path::Path,
    query: &str,
    token: &crate::heist::token::CancellationToken,
) -> Vec<crate::fascia::WorkspaceSearchHit>
{
    let mut hits = Vec::new();
    let needle = query.to_lowercase();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if token.IsCancelled() || hits.len() >= 100 {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else { continue; };
        for entry in entries.flatten() {
            if token.IsCancelled() || hits.len() >= 100 {
                break;
            }
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.starts_with('.') && name != "target" {
                    stack.push(path);
                }
            } else if is_text_file(&path)
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                for (idx, line) in text.lines().enumerate() {
                    if line.to_lowercase().contains(&needle) {
                        hits.push(crate::fascia::WorkspaceSearchHit {
                            path:    path.clone(),
                            line:    idx + 1,
                            preview: line.trim().chars().take(80).collect(),
                        });
                        if hits.len() >= 100 {
                            break;
                        }
                    }
                }
            }
        }
    }
    return hits;
}

fn view_search_sidebar<'a>(
    query: &'a str,
    hits: &'a [crate::fascia::WorkspaceSearchHit],
    is_running: bool,
    palette: ThemePalette,
) -> Element<'a, AppMessage>
{
    let header = row![
        text("SEARCH").size(11).style(move |_| text::Style { color: Some(palette.text_muted) }),
        Space::new().width(Length::Fill),
        if is_running {
            text("Searching...").size(10).style(move |_| text::Style { color: Some(palette.accent) })
        } else {
            text(format!("{} hits", hits.len())).size(10).style(move |_| text::Style { color: Some(palette.text_muted) })
        },
    ]
    .align_y(Alignment::Center);

    let search_input = text_input("Search in files (Enter)...", query)
        .on_input(AppMessage::SearchQueryChanged)
        .on_submit(AppMessage::SearchSubmit)
        .padding([6, 10])
        .size(12);

    let mut results_col = column![].spacing(3).width(Length::Fill);
    if hits.is_empty() {
        let msg = if query.trim().is_empty() {
            "Type a query and press Enter to search via Heist."
        } else if is_running {
            "Scanning workspace..."
        } else {
            "No matches found."
        };
        results_col = results_col.push(
            text(msg).size(12).style(move |_| text::Style { color: Some(palette.text_muted) }),
        );
    } else {
        for hit in hits {
            let name = hit
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();
            let path = hit.path.clone();
            let row_btn = button(
                column![
                    text(format!("{}:{}", name, hit.line)).size(11).style(move |_| text::Style {
                        color: Some(palette.accent),
                    }),
                    text(&hit.preview).size(11).style(move |_| text::Style {
                        color: Some(palette.text_secondary),
                    }),
                ]
                .spacing(1),
            )
            .width(Length::Fill)
            .padding([4, 6])
            .style(move |_, status| FasciaStyle::tree_row_button(palette, false, status))
            .on_press(AppMessage::OpenFile(path));
            results_col = results_col.push(row_btn);
        }
    }

    let layout = column![
        header,
        Space::new().height(Length::Fixed(8.0)),
        search_input,
        Space::new().height(Length::Fixed(10.0)),
        scrollable(results_col).width(Length::Fill).height(Length::Fill),
    ]
    .padding([10, 12])
    .width(Length::Fill)
    .height(Length::Fill);
    return container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| FasciaStyle::sidebar_container(palette))
        .into();
}

//-----------------------------------------------------------------------------------------------------------------------------

fn view_settings<'a>(current_theme: FasciaTheme, palette: ThemePalette, show_sidebar: bool,
                     show_status_bar: bool)
                     -> Element<'a, AppMessage>
{
    let header =
        text("Settings").size(28)
                        .style(move |_| text::Style { color: Some(palette.text_primary), });
    let theme_header =
        text("Appearance & Native Styling").size(16)
                                           .style(move |_| {
                                               text::Style { color: Some(palette.text_primary), }
                                           });
    let mut theme_buttons = column![].spacing(6);
    for &theme_choice in FasciaTheme::ALL {
        let is_selected = current_theme == theme_choice;
        let mark = if is_selected { "● " } else { "○ " };
        let btn = button(row![
            text(mark).size(14).style(move |_| {
                text::Style { color: if is_selected {
                                  Some(palette.accent)
                              } else {
                                  Some(palette.text_muted)
                              }, }
            }),
            Space::new().width(Length::Fixed(6.0)),
            text(theme_choice.display_name()).size(13).style(move |_| {
                text::Style { color: Some(palette.text_primary), }
            }),
        ].align_y(Alignment::Center)).padding([8, 14])
                                     .width(Length::Fixed(320.0))
                                     .style(move |_, status| {
                                         FasciaStyle::tree_row_button(palette, is_selected, status)
                                     })
                                     .on_press(AppMessage::SelectTheme(theme_choice));
        theme_buttons = theme_buttons.push(btn);
    }
    let layout_header =
        text("Layout Options").size(16)
                              .style(move |_| text::Style { color: Some(palette.text_primary), });
    let toggle_sidebar_btn = button(
        row![
            text(if show_sidebar {
                "✓ Sidebar Visible"
            } else {
                "✕ Sidebar Hidden"
            })
            .size(13),
        ]
        .align_y(Alignment::Center),
    )
    .padding([8, 14])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(AppMessage::ToggleSidebar);
    let toggle_statusbar_btn = button(
        row![
            text(if show_status_bar {
                "✓ Status Bar Visible"
            } else {
                "✕ Status Bar Hidden"
            })
            .size(13),
        ]
        .align_y(Alignment::Center),
    )
    .padding([8, 14])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(AppMessage::ToggleStatusBar);
    let content =
        column![header,
                Space::new().height(Length::Fixed(20.0)),
                theme_header,
                Space::new().height(Length::Fixed(8.0)),
                theme_buttons,
                Space::new().height(Length::Fixed(24.0)),
                layout_header,
                Space::new().height(Length::Fixed(8.0)),
                row![toggle_sidebar_btn, toggle_statusbar_btn].spacing(10),].padding([32, 48])
                                                                            .max_width(600.0);
    scrollable(container(content).width(Length::Fill)
                                 .align_x(Alignment::Center)).into()
}

//-----------------------------------------------------------------------------------------------------------------------------

impl AppState
{
    pub fn subscription(&self) -> iced::Subscription<AppMessage>
    {
        let events = iced::event::listen_with(|event, status, _| {
            if matches!(event, iced::Event::Window(iced::window::Event::CloseRequested)) {
                return Some(AppMessage::RequestExit);
            }
            if let iced::Event::Mouse(mouse::Event::CursorMoved { position }) = event {
                return Some(AppMessage::CursorMoved(position));
            }
            if matches!(event, iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))) {
                return Some(AppMessage::MouseButtonReleased);
            }
            if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key,
                modifiers,
                ..
            }) = &event
            {
                if matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)) {
                    return Some(AppMessage::Popup(PopupAction::CloseModal));
                }
                if matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown)) {
                    return Some(AppMessage::Popup(PopupAction::MoveSelection(1)));
                }
                if matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp)) {
                    return Some(AppMessage::Popup(PopupAction::MoveSelection(-1)));
                }
                if status == iced::event::Status::Ignored
                    && let iced::keyboard::Key::Character(character) = key
                {
                    let key_str = character.to_ascii_lowercase();
                    if let Some(cmd) = CommandRegistry::FindByShortcut(&key_str, *modifiers) {
                        return Some(AppMessage::Command(cmd));
                    }
                }
            }
            return None;
        });
        let heist = self.heist.Subscription().map(AppMessage::HeistEvent);
        return iced::Subscription::batch([events, heist]);
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

/// Launches the interactive Fascia desktop GUI application.
pub fn run_app() -> iced::Result
{
    iced::application(AppState::new, AppState::update, AppState::view).title(|state: &AppState| {
        if let Some(tab) = state.tab_manager.active_tab() {
            format!("{} - Fascia", tab.title)
        } else {
            "Fascia - Native Workspace Shell".to_string()
        }
    })
    .theme(AppState::theme)
    .exit_on_close_request(false)
    .subscription(AppState::subscription)
    .window_size(Size::new(1280.0, 800.0))
    .centered()
    .run()
}

//-----------------------------------------------------------------------------------------------------------------------------
