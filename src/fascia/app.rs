// app.rs -----------------------------------------------------------------------------------------------------------------

use crate::fascia::examples::{ExampleAction, ExampleBrowser};
use crate::fascia::explorer::{default_initial_dir, is_text_file};
use crate::fascia::geometry_view::{GeometryAction, GeometryViewerState, ViewGeometry};
use crate::fascia::python_console::ConsoleExecutionResult;
use crate::fascia::theme::default_code_font;
use crate::fascia::{
    ActivityTab, Cask2dAction, Cask2dViewerState, ExplorerAction, ExplorerState, FasciaStyle,
    FasciaTheme, MenuAction, PythonConsoleAction, PythonConsoleState, StatusBarInfo, TabBarAction,
    TabId, TabItem, TabKind, TabManager, ThemePalette, ToolBarAction, WaveformAction,
    WaveformState, view_activity_bar, view_cask2d_tab, view_menubar, view_python_console,
    view_shell, view_status_bar, view_tab_bar, view_toolbar, view_waveform,
};
use crate::fenst::geometry::GeometryAsset;
use crate::fenst::waveform::ParseWaveform;
use crate::silo::{IArr, Stash, USeg};
use iced::widget::{
    Space, button, column, container, row, scrollable, text, text_editor, text_input,
};
use iced::{Alignment, Element, Length, Size, Task};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

//-----------------------------------------------------------------------------------------------------------------------------

/// Unified message enum for the application.
#[derive(Debug, Clone)]
pub enum AppMessage
{
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
    PythonConsole(TabId, PythonConsoleAction),
    PythonConsoleExecuted(TabId, ConsoleExecutionResult),
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

/// Authoritative owned document model for workspace tabs.
pub enum TabDocument
{
    Editor(text_editor::Content),
    Waveform(WaveformState),
    PythonConsole(PythonConsoleState),
    Geometry(GeometryViewerState),
    Cask2d(Box<Cask2dViewerState>),
    Sketch(Box<crate::fascia::sketch_view::SketchViewerState>),
}

impl TabDocument
{
    pub fn cancel(&mut self)
    {
        match self {
            Self::PythonConsole(console) => console.cancel(),
            Self::Sketch(viewer) => {
                viewer.job_cancellation.store(true, std::sync::atomic::Ordering::Release);
            }
            _ => {}
        }
    }
}

//-------------------------------------------------------------------------------------------------

pub struct AppState
{
    pub theme:               FasciaTheme,
    pub show_sidebar:        bool,
    pub show_status_bar:     bool,
    pub active_activity_tab: Option<ActivityTab>,
    pub explorer:            ExplorerState,
    _Examples:               ExampleBrowser,
    pub tab_manager:         TabManager,
    pub open_editors:        HashMap<TabId, text_editor::Content>,
    pub open_waveforms:      HashMap<TabId, WaveformState>,
    pub open_consoles:       HashMap<TabId, PythonConsoleState>,
    pub open_cask2d_views:   HashMap<TabId, Cask2dViewerState>,
    pub open_sketch_views:   HashMap<TabId, crate::fascia::sketch_view::SketchViewerState>,
    _FrescoViews:            Stash<(TabId, crate::fascia::math_view::FrescoViewerState)>,
    _GeometryViews:          BTreeMap<u64, GeometryViewerState>,
    pub status_info:         StatusBarInfo,
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
               active_activity_tab: Some(ActivityTab::Explorer),
               explorer,
               _Examples: ExampleBrowser::default(),
               tab_manager,
               open_editors: HashMap::new(),
               open_waveforms: HashMap::new(),
               open_consoles: HashMap::new(),
               open_cask2d_views: HashMap::new(),
               open_sketch_views: HashMap::new(),
               _FrescoViews: Stash::New(),
               _GeometryViews: BTreeMap::new(),
               status_info, _CloseQueue: Stash::New(), _ClosePrompt: None, _ExitAfterClose: false }
    }
}

//-----------------------------------------------------------------------------------------------------------------------------

impl AppState
{
    pub fn new() -> Self { Self::default() }
    pub fn palette(&self) -> ThemePalette { self.theme.palette() }
    pub fn Geometry(&self, id: TabId) -> Option<&GeometryViewerState>
    {
        self._GeometryViews.get(&id.0)
    }

    fn FrescoIndex(&self, id: TabId) -> u32
    {
        return USeg::FromLen(self._FrescoViews.Size())
            .BinarySearch(|index| self._FrescoViews[index].0.0.cmp(&id.0)).unwrap_or(u32::MAX);
    }

    pub fn Fresco(&self, id: TabId) -> Option<&crate::fascia::math_view::FrescoViewerState>
    {
        let index = self.FrescoIndex(id);
        if index == u32::MAX { return None; }
        return Some(&self._FrescoViews[index].1);
    }

    pub fn FrescoMut(&mut self, id: TabId) -> Option<&mut crate::fascia::math_view::FrescoViewerState>
    {
        let index = self.FrescoIndex(id);
        if index == u32::MAX { return None; }
        return Some(&mut self._FrescoViews[index].1);
    }

    pub fn FrescoCount(&self) -> u32 { return self._FrescoViews.Size(); }

    pub fn OpenFrescoDocument(&mut self, doc: crate::fenst::fresco_document::FrescoDocument) -> TabId
    {
        let (_, _, id) = self.tab_manager.OpenFresco("Math View".to_string());
        self._FrescoViews.Push((id, crate::fascia::math_view::FrescoViewerState::New(doc)));
        self.update_status_for_active_tab();
        return id;
    }

    fn RemoveFresco(&mut self, id: TabId)
    {
        let index = self.FrescoIndex(id);
        if index == u32::MAX { return; }
        USeg::WithLen(index, self._FrescoViews.Size() - index - 1)
            .Traverse(|i| self._FrescoViews.MutArr().Swap(i, i + 1));
        self._FrescoViews.Pop();
    }
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
            if let Some(mut console) = self.open_consoles.remove(&closed.id) {
                console.cancel();
            }
            self.open_editors.remove(&closed.id);
            self.open_waveforms.remove(&closed.id);
            self.open_cask2d_views.remove(&closed.id);
            if let Some(viewer) = self.open_sketch_views.remove(&closed.id) {
                viewer.job_cancellation.store(true, std::sync::atomic::Ordering::Release);
            }
            self.RemoveFresco(closed.id);
            self._GeometryViews.remove(&closed.id.0);
            self.update_status_for_active_tab();
            self.status_info.message = format!("Closed {}", closed.title);
            Some(closed)
        } else {
            None
        }
    }
    pub fn close_all_tabs_internal(&mut self)
    {
        for (_, mut console) in self.open_consoles.drain() {
            console.cancel();
        }
        self.open_editors.clear();
        self.open_waveforms.clear();
        self.open_cask2d_views.clear();
        for (_, viewer) in self.open_sketch_views.drain() {
            viewer.job_cancellation.store(true, std::sync::atomic::Ordering::Release);
        }
        self._FrescoViews.Clear();
        self._GeometryViews.clear();
        self.tab_manager.close_all();
        self.update_status_for_active_tab();
        self.status_info.message = "All tabs closed".to_string();
    }
    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage>
    {
        if self._ClosePrompt.is_some()
           && !matches!(message,
                        AppMessage::ResolveSketchClose(_)
                        | AppMessage::GeometryPrepared(..)
                        | AppMessage::SketchJobCompleted { .. }
                        | AppMessage::PythonConsoleExecuted(..))
        {
            return Task::none();
        }
        match message {
            AppMessage::NewSketch => {
                let title = "Untitled Sketch".to_string();
                let (_, _, id) = self.tab_manager.open_sketch(None, title.clone());
                self.open_sketch_views.insert(id, crate::fascia::sketch_view::SketchViewerState::New(None, title, self.palette()));
                self.update_status_for_active_tab();
                self.status_info.message = "Choose a drawing tool; set a .flx or .tflx path to save".into();
            }
            AppMessage::NewMathView => {
                let doc = crate::fenst::fresco_document::FrescoDocument::CreateExample();
                self.OpenFrescoDocument(doc);
                self.status_info.message = "Opened Math View with built-in examples".into();
            }
            AppMessage::Fresco(tab_id, action) => {
                if let Some(viewer) = self.FrescoMut(tab_id) {
                    viewer.Update(action);
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
                let Some(viewer) = self.open_sketch_views.get_mut(&tab_id) else {
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
                    && let Some(viewer) = self.open_sketch_views.get_mut(&id) {
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
            AppMessage::Menu(action) => {
                match action {
                    MenuAction::NewFile => return self.update(AppMessage::NewFile),
                    MenuAction::NewSketch => return self.update(AppMessage::NewSketch),
                    MenuAction::NewMathView => return self.update(AppMessage::NewMathView),
                    MenuAction::OpenFile | MenuAction::OpenFolder => {
                        if self.explorer.has_visible_roots() {
                            self.active_activity_tab = Some(ActivityTab::Explorer);
                            self.show_sidebar = true;
                        }
                    }
                    MenuAction::SaveFile => return self.update(AppMessage::SaveCurrentFile),
                    MenuAction::CloseTab => return self.update(AppMessage::CloseActiveTab),
                    MenuAction::CloseAllTabs => {
                        return self.BeginClosing(None, false);
                    }
                    MenuAction::Exit => {
                        return self.update(AppMessage::RequestExit);
                    }
                    MenuAction::Undo => {
                        if let Some(tab) = self.tab_manager.active_tab()
                            && let Some(viewer) = self.open_sketch_views.get_mut(&tab.id)
                        {
                            viewer.Undo();
                            let is_dirty = viewer.IsDirty();
                            self.tab_manager.mark_active_dirty(is_dirty);
                        }
                    }
                    MenuAction::Redo => {
                        if let Some(tab) = self.tab_manager.active_tab()
                            && let Some(viewer) = self.open_sketch_views.get_mut(&tab.id)
                        {
                            viewer.Redo();
                            let is_dirty = viewer.IsDirty();
                            self.tab_manager.mark_active_dirty(is_dirty);
                        }
                    }
                    MenuAction::Cut
                    | MenuAction::Copy
                    | MenuAction::Paste
                    | MenuAction::SelectAll => {}
                    MenuAction::ToggleSidebar => return self.update(AppMessage::ToggleSidebar),
                    MenuAction::ToggleStatusBar => return self.update(AppMessage::ToggleStatusBar),
                    MenuAction::ToggleExplorerFilesystem => {
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
                    MenuAction::ToggleExplorerSettings => {
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
                    MenuAction::ToggleExplorerExamples => {
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
                    MenuAction::OpenExplorer => {
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
                    MenuAction::OpenSearch => {
                        self.active_activity_tab = Some(ActivityTab::Search);
                        self.show_sidebar = true;
                    }
                    MenuAction::OpenSettings => return self.update(AppMessage::OpenSettings),
                    MenuAction::OpenPythonConsole => {
                        return self.update(AppMessage::OpenPythonConsole);
                    }
                    MenuAction::SelectTheme(t) => return self.update(AppMessage::SelectTheme(t)),
                    MenuAction::About => {
                        self.status_info.message = "Fascia UI Shell v0.1.0".to_string();
                    }
                    MenuAction::Documentation => {
                        self.status_info.message =
                            "Documentation: https://github.com/iced-rs/iced".to_string();
                    }
                }
            }
            AppMessage::ToolBar(action) => {
                match action {
                    ToolBarAction::NewFile => return self.update(AppMessage::NewFile),
                    ToolBarAction::NewSketch => return self.update(AppMessage::NewSketch),
                    ToolBarAction::OpenFile => {
                        if self.explorer.has_visible_roots() {
                            self.active_activity_tab = Some(ActivityTab::Explorer);
                            self.show_sidebar = true;
                        }
                    }
                    ToolBarAction::SaveFile => return self.update(AppMessage::SaveCurrentFile),
                    ToolBarAction::Undo => return self.update(AppMessage::Menu(MenuAction::Undo)),
                    ToolBarAction::Redo => return self.update(AppMessage::Menu(MenuAction::Redo)),
                    ToolBarAction::ToggleSidebar => return self.update(AppMessage::ToggleSidebar),
                    ToolBarAction::ToggleTheme => return self.update(AppMessage::ToggleTheme),
                    ToolBarAction::OpenSettings => return self.update(AppMessage::OpenSettings),
                }
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
                    ExplorerAction::ToggleFolder(path) => {
                        self.explorer.selected_path = Some(path.clone());
                        self.explorer.toggle_path(&path);
                        let path_str = path.to_string_lossy();
                        if path_str.starts_with("example://")
                            && let Ok((_, branch)) =
                                crate::fenst::XplrRegistry::New().OpenRoot(&path_str)
                            && branch.Circuit().is_some()
                        {
                            let (_idx, is_new, id) =
                                self.tab_manager.open_cask2d(path.clone());
                            if is_new {
                                let viewer =
                                    Cask2dViewerState::NewWithPalette(path.clone(),
                                                                      self.palette());
                                self.open_cask2d_views.insert(id, viewer);
                            }
                            self.update_status_for_active_tab();
                        }
                    }
                    ExplorerAction::OpenFile(path) => {
                        self.explorer.selected_path = Some(path.clone());
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
                            self._GeometryViews.insert(id.0, view);
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
                            self.open_cask2d_views.insert(id, viewer);
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
                    if let Some(editor) = self.open_editors.get_mut(&tab_id) {
                        editor.perform(action);
                        let pos = editor.cursor().position;
                        self.status_info.line = pos.line + 1;
                        self.status_info.column = pos.column + 1;
                    }
                    if is_edit {
                        self.tab_manager.mark_active_dirty(true);
                    }
                }
            }
            AppMessage::Waveform(action) => {
                if let Some(tab) = self.tab_manager.active_tab()
                   && let Some(waveform) = self.open_waveforms.get_mut(&tab.id)
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
                       && let Some(view) = self._GeometryViews.get_mut(&id.0)
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
                } else if let Some(view) = self._GeometryViews.get_mut(&id.0) {
                    view.Update(action);
                }
            }
            AppMessage::GeometryPrepared(id, cancelled, result) => {
                if !cancelled.load(std::sync::atomic::Ordering::Acquire)
                   && let Some(view) = self._GeometryViews.get_mut(&id.0)
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
                        if let Some(viewer) = self.open_cask2d_views.get_mut(&id) {
                            viewer.Update(action);
                        }
                    }
                }
            }
            AppMessage::Sketch(id, action) => {
                if matches!(action, crate::fascia::sketch_view::SketchViewAction::OpenPath)
                    && let Some(viewer) = self.open_sketch_views.get(&id) {
                        return self.update(AppMessage::OpenFile(viewer.RequestedPath()));
                }
                if (matches!(action, crate::fascia::sketch_view::SketchViewAction::View3dWireframe)
                    || matches!(action, crate::fascia::sketch_view::SketchViewAction::InspectorAction(
                        crate::fascia::sketch_inspector::SketchInspectorAction::View3dWireframe
                    )))
                {
                    if let Some(viewer) = self.open_sketch_views.get_mut(&id) {
                        let (job_gen, token, snap) = viewer.BeginJob();
                        let rev = snap.revision;
                        return Task::perform(
                            crate::fascia::sketch_job::ExecuteSketchJob(
                                snap,
                                crate::fascia::sketch_job::SketchJobKind::Wireframe,
                                token.clone(),
                            ),
                            move |outcome| AppMessage::SketchJobCompleted {
                                tab_id: id,
                                generation: job_gen,
                                cancellation: token,
                                snapshot_rev: rev,
                                outcome,
                            },
                        );
                    }
                    return Task::none();
                }

                let extrude_req = match &action {
                    crate::fascia::sketch_view::SketchViewAction::Extrude3d(d) => Some(*d),
                    crate::fascia::sketch_view::SketchViewAction::InspectorAction(
                        crate::fascia::sketch_inspector::SketchInspectorAction::Extrude3d
                    ) => {
                        if let Some(viewer) = self.open_sketch_views.get_mut(&id) {
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
                    if let Some(viewer) = self.open_sketch_views.get_mut(&id) {
                        let (job_gen, token, snap) = viewer.BeginJob();
                        let rev = snap.revision;
                        return Task::perform(
                            crate::fascia::sketch_job::ExecuteSketchJob(
                                snap,
                                crate::fascia::sketch_job::SketchJobKind::Extrude(depth),
                                token.clone(),
                            ),
                            move |outcome| AppMessage::SketchJobCompleted {
                                tab_id: id,
                                generation: job_gen,
                                cancellation: token,
                                snapshot_rev: rev,
                                outcome,
                            },
                        );
                    }
                    return Task::none();
                }

                if let crate::fascia::sketch_view::SketchViewAction::Revolve3d { axis, angle_deg } = action {
                    if let Some(viewer) = self.open_sketch_views.get_mut(&id) {
                        let (job_gen, token, snap) = viewer.BeginJob();
                        let rev = snap.revision;
                        return Task::perform(
                            crate::fascia::sketch_job::ExecuteSketchJob(
                                snap,
                                crate::fascia::sketch_job::SketchJobKind::Revolve { axis, angle_deg },
                                token.clone(),
                            ),
                            move |outcome| AppMessage::SketchJobCompleted {
                                tab_id: id,
                                generation: job_gen,
                                cancellation: token,
                                snapshot_rev: rev,
                                outcome,
                            },
                        );
                    }
                    return Task::none();
                }

                if matches!(action, crate::fascia::sketch_view::SketchViewAction::ExportSvg) {
                    if let Some(viewer) = self.open_sketch_views.get_mut(&id) {
                        if let Some(path) = &viewer.path {
                            let svg_path = path.with_extension("svg");
                            let (job_gen, token, snap) = viewer.BeginJob();
                            let rev = snap.revision;
                            return Task::perform(
                                crate::fascia::sketch_job::ExecuteSketchJob(
                                    snap,
                                    crate::fascia::sketch_job::SketchJobKind::ExportSvg(svg_path),
                                    token.clone(),
                                ),
                                move |outcome| AppMessage::SketchJobCompleted {
                                    tab_id: id,
                                    generation: job_gen,
                                    cancellation: token,
                                    snapshot_rev: rev,
                                    outcome,
                                },
                            );
                        } else {
                            viewer.status_message = Some("Save the sketch before exporting SVG".into());
                        }
                    }
                    return Task::none();
                }

                if let Some(viewer) = self.open_sketch_views.get_mut(&id) {
                    viewer.ProcessAction(action);
                    if let Some(save_path) = viewer.TakePendingSave() {
                        let (job_gen, token, snap) = viewer.BeginJob();
                        let rev = snap.revision;
                        return Task::perform(
                            crate::fascia::sketch_job::ExecuteSketchJob(
                                snap,
                                crate::fascia::sketch_job::SketchJobKind::Save(save_path),
                                token.clone(),
                            ),
                            move |outcome| AppMessage::SketchJobCompleted {
                                tab_id: id,
                                generation: job_gen,
                                cancellation: token,
                                snapshot_rev: rev,
                                outcome,
                            },
                        );
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
                        self.open_waveforms.insert(tab.id, waveform);
                    } else if tab.kind == TabKind::SketchViewer {
                        let viewer = crate::fascia::sketch_view::SketchViewerState::FromFile(
                            path.clone(),
                            self.palette(),
                        );
                        let id = tab.id;
                        match viewer {
                            Ok(viewer) => { self.open_sketch_views.insert(id, viewer); }
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
                        self._GeometryViews.insert(id.0, view);
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
                        self.open_editors.insert(tab.id, editor_content);
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
                        if let Some(editor) = self.open_editors.get(&tab_id) {
                            let text = editor.text();
                            if let Err(e) = std::fs::write(&path, text) {
                                self.status_info.message = format!("Error saving: {}", e);
                            } else {
                                self.tab_manager.mark_active_dirty(false);
                                self.status_info.message = format!("Saved {}", path.display());
                            }
                        } else if let Some(sketch) = self.open_sketch_views.get_mut(&tab_id) {
                            if let Err(e) = sketch.SaveToFile(&path) {
                                self.status_info.message = format!("Error saving sketch: {}", e);
                            } else {
                                self.tab_manager.mark_active_dirty(false);
                                self.status_info.message = format!("Saved {}", path.display());
                            }
                        }
                    } else {
                        self.tab_manager.mark_active_dirty(false);
                        self.status_info.message = "Saved draft".to_string();
                    }
                }
            }
            AppMessage::NewFile => {
                self.tab_manager.open_new_file();
                if let Some(tab) = self.tab_manager.active_tab() {
                    let editor_content = text_editor::Content::with_text("// New file\n");
                    self.open_editors.insert(tab.id, editor_content);
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
                if let Some(tab) = self.tab_manager.active_tab() && let Some(sketch) = self.open_sketch_views.get_mut(&tab.id) { sketch.tool_ctrl.ResetDraft(); }
                self.tab_manager.select_tab(idx);
                self.update_status_for_active_tab();
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
                for viewer in self.open_cask2d_views.values_mut() {
                    viewer.SetPalette(palette);
                }
                for viewer in self.open_sketch_views.values_mut() {
                    viewer.palette = Some(palette);
                    viewer.dark_theme = palette.is_dark();
                    viewer.canvas_state.InvalidateAll();
                }
                let mut reload_tasks = Vec::new();
                self.tab_manager.tabs().Traverse(|tab| {
                    if tab.kind == TabKind::CaskViewer
                        && let Some(ref path) = tab.path
                        && let Some(view) = self._GeometryViews.get_mut(&tab.id.0)
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
                let (_idx, is_new, id) = self.tab_manager.open_python_console();
                if is_new {
                    self.open_consoles.insert(id, PythonConsoleState::new());
                }
                self.update_status_for_active_tab();
                self.status_info.message = "Opened Python Console".to_string();
                return Task::none();
            }
            AppMessage::PythonConsole(id, action) => {
                if let Some(console) = self.open_consoles.get_mut(&id) {
                    match action {
                        PythonConsoleAction::InputChanged(text) => {
                            console.input_changed(text);
                            return Task::none();
                        }
                        PythonConsoleAction::Submit => {
                            if let Some(line) = console.submit() {
                                #[cfg(feature = "python")]
                                if let Some(engine) = console.engine.clone() {
                                    return Task::perform(async move { engine.execute_line(&line) },
                                                         move |res| {
                                                             AppMessage::PythonConsoleExecuted(id,
                                                                                               res)
                                                         });
                                }
                                #[cfg(not(feature = "python"))]
                                {
                                    let _ = &line;
                                    let res = ConsoleExecutionResult {
                                        output: "Python support is not enabled in this build. Rebuild with '--features python'.".to_string(),
                                        is_error: true,
                                        needs_more_input: false,
                                    };
                                    console.on_executed(res);
                                    return Task::none();
                                }
                            }
                            return Task::none();
                        }
                        PythonConsoleAction::Cancel => {
                            console.cancel();
                            self.status_info.message = "Execution cancelled".to_string();
                            return Task::none();
                        }
                        PythonConsoleAction::Clear => {
                            console.clear();
                            return Task::none();
                        }
                    }
                }
                return Task::none();
            }
            AppMessage::PythonConsoleExecuted(id, result) => {
                if let Some(console) = self.open_consoles.get_mut(&id) {
                    console.on_executed(result);
                    self.status_info.message = if console.is_multiline {
                        "Python: waiting for block continuation...".to_string()
                    } else {
                        "Python: ready".to_string()
                    };
                }
                return Task::none();
            }
        }
        Task::none()
    }
    pub fn view(&self) -> Element<'_, AppMessage>
    {
        let palette = self.palette();
        // 1. Top Menubar
        let menubar = view_menubar(palette,
                                   self.theme,
                                   (self.explorer.show_filesystem,
                                    self.explorer.show_settings,
                                    self.explorer.show_examples),
                                   AppMessage::Menu);
        // 2. Toolbar
        let toolbar = view_toolbar(palette, AppMessage::ToolBar);
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
                Some(ActivityTab::Search) => Some(view_search_sidebar(palette)),
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
                        if let Some(editor) = self.open_editors.get(&active_tab.id) {
                            view_editor(editor, self.theme, palette)
                        } else {
                            container(text("Opening buffer...").size(14)).padding(20)
                                                                         .into()
                        }
                    }
                    TabKind::VcdViewer => {
                        if let Some(waveform) = self.open_waveforms.get(&active_tab.id) {
                            view_waveform(waveform, palette, AppMessage::Waveform)
                        } else {
                            container(text("Opening waveform...").size(14)).padding(20)
                                                                           .into()
                        }
                    }
                    TabKind::PtsViewer | TabKind::ObjViewer | TabKind::CaskViewer => {
                        if let Some(view) = self._GeometryViews.get(&active_tab.id.0) {
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
                        if let Some(viewer) = self.open_cask2d_views.get(&active_tab.id) {
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
                        if let Some(viewer) = self.open_sketch_views.get(&active_tab.id) {
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
                    TabKind::PythonConsole => {
                        if let Some(console) = self.open_consoles.get(&active_tab.id) {
                            let id = active_tab.id;
                            view_python_console(console, self.theme, palette, move |action| {
                                AppMessage::PythonConsole(id, action)
                            })
                        } else {
                            container(text("Opening Python console...").size(14)).padding(20)
                                                                                 .into()
                        }
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
        // 6. Status Bar
        let status_bar = if self.show_status_bar {
            Some(view_status_bar(&self.status_info, self.theme, palette))
        } else {
            None
        };
        // Complete Shell Assembly
        let shell = view_shell(menubar,
                   Some(toolbar),
                   activity_bar,
                   sidebar,
                   content_column.into(),
                   status_bar,
                   palette);
        if self._ClosePrompt.is_some() {
            let prompt = container(column![
                text("Save this sketch before closing?").size(22),
                text("Unsaved changes will be lost if you discard them."),
                row![
                    button("Save and close").on_press(AppMessage::ResolveSketchClose(SketchCloseDecision::Save)),
                    button("Discard").on_press(AppMessage::ResolveSketchClose(SketchCloseDecision::Discard)),
                    button("Cancel").on_press(AppMessage::ResolveSketchClose(SketchCloseDecision::Cancel)),
                ].spacing(12),
            ].spacing(18).padding(28)).style(move |_| FasciaStyle::menubar_container(palette));
            return iced::widget::stack![shell, iced::widget::opaque(container(prompt).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).center_y(Length::Fill))].into();
        }
        return shell;
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
                if self.open_sketch_views
                       .get(&id)
                       .is_some_and(|viewer| viewer.IsDirty())
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
            return iced::exit();
        }
        return Task::none();
    }

    fn handle_sketch_job_outcome(
        &mut self,
        tab_id: TabId,
        snapshot_rev: u64,
        outcome: crate::fascia::sketch_job::SketchJobOutcome,
    )
    {
        match outcome {
            crate::fascia::sketch_job::SketchJobOutcome::Wireframe(res) => {
                let title = self.open_sketch_views.get(&tab_id).map(|v| v.title.clone()).unwrap_or_default();
                self.open_sketch_wireframe_tab(tab_id, title, res);
            }
            crate::fascia::sketch_job::SketchJobOutcome::Extrude { depth, asset } => {
                let title = self.open_sketch_views.get(&tab_id).map(|v| v.title.clone()).unwrap_or_default();
                self.open_sketch_extrude_tab(tab_id, title, depth, asset);
            }
            crate::fascia::sketch_job::SketchJobOutcome::Revolve { angle_deg, asset } => {
                let title = self.open_sketch_views.get(&tab_id).map(|v| v.title.clone()).unwrap_or_default();
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

    fn open_sketch_wireframe_tab(
        &mut self,
        tab_id: TabId,
        title: String,
        res: Result<Arc<GeometryAsset>, String>,
    )
    {
        match res {
            Ok(asset) => {
                let tab_title = format!("🌐 3D: {}", title);
                let (_, _, new_id) = self.tab_manager.open_geometry_tab(tab_title);
                let mut view = GeometryViewerState::default();
                view.Complete(Ok(asset));
                self._GeometryViews.insert(new_id.0, view);
                self.update_status_for_active_tab();
                self.status_info.message = format!("Generated 3D wireframe for {}", title);
            }
            Err(e) => {
                if let Some(v) = self.open_sketch_views.get_mut(&tab_id) {
                    v.status_message = Some(format!("3D wireframe error: {}", e));
                }
                self.status_info.message = format!("3D wireframe error: {}", e);
            }
        }
    }

    fn open_sketch_extrude_tab(
        &mut self,
        tab_id: TabId,
        title: String,
        depth: f64,
        asset: Result<Arc<GeometryAsset>, String>,
    )
    {
        match asset {
            Ok(asset) => {
                let tab_title = format!("🧊 Extrude: {}", title);
                let (_, _, new_id) = self.tab_manager.open_geometry_tab(tab_title);
                let mut view = GeometryViewerState::default();
                view.Complete(Ok(asset));
                self._GeometryViews.insert(new_id.0, view);
                self.update_status_for_active_tab();
                self.status_info.message = format!("Extruded 3D solid ({:.1} mm) for {}", depth, title);
            }
            Err(e) => {
                if let Some(v) = self.open_sketch_views.get_mut(&tab_id) {
                    v.status_message = Some(format!("3D extrude error: {}", e));
                }
                self.status_info.message = format!("3D extrude error: {}", e);
            }
        }
    }

    fn open_sketch_revolve_tab(
        &mut self,
        tab_id: TabId,
        title: String,
        angle_deg: f64,
        asset: Result<Arc<GeometryAsset>, String>,
    )
    {
        match asset {
            Ok(asset) => {
                let tab_title = format!("🔄 Revolve: {}", title);
                let (_, _, new_id) = self.tab_manager.open_geometry_tab(tab_title);
                let mut view = GeometryViewerState::default();
                view.Complete(Ok(asset));
                self._GeometryViews.insert(new_id.0, view);
                self.update_status_for_active_tab();
                self.status_info.message = format!("Revolved 3D lathe ({:.0}°) for {}", angle_deg, title);
            }
            Err(e) => {
                if let Some(v) = self.open_sketch_views.get_mut(&tab_id) {
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
                if let Some(v) = self.open_sketch_views.get_mut(&tab_id) {
                    v.status_message = Some(msg.clone());
                }
                self.status_info.message = msg;
            }
            Err(e) => {
                let msg = format!("SVG export failed: {}", e);
                if let Some(v) = self.open_sketch_views.get_mut(&tab_id) {
                    v.status_message = Some(msg.clone());
                }
                self.status_info.message = msg;
            }
        }
    }

    fn handle_sketch_save_outcome(
        &mut self,
        tab_id: TabId,
        snapshot_rev: u64,
        path: PathBuf,
        metadata: Result<crate::flux::FluxValue, String>,
    )
    {
        let Some(viewer) = self.open_sketch_views.get_mut(&tab_id) else { return; };
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
fn view_search_sidebar<'a>(palette: ThemePalette) -> Element<'a, AppMessage>
{
    let header = text("SEARCH").size(11)
                               .style(move |_| text::Style { color: Some(palette.text_muted), });
    let search_input = text_input("Search in files...", "").padding([6, 10]);
    let placeholder = text("Type a query to search across the workspace...").size(12)
                                                                            .style(move |_| {
                                                                                text::Style {
            color: Some(palette.text_muted),
        }
                                                                            });
    let layout = column![header,
                         Space::new().height(Length::Fixed(8.0)),
                         search_input,
                         Space::new().height(Length::Fixed(16.0)),
                         placeholder,].padding([10, 12])
                                      .width(Length::Fill);
    container(layout).width(Length::Fixed(260.0))
                     .height(Length::Fill)
                     .style(move |_| FasciaStyle::sidebar_container(palette))
                     .into()
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
    .subscription(|_| iced::event::listen_with(|event, status, _| {
        if matches!(event, iced::Event::Window(iced::window::Event::CloseRequested)) { return Some(AppMessage::RequestExit); }
        if status == iced::event::Status::Ignored && let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key: iced::keyboard::Key::Character(character), modifiers, .. }) = event && modifiers.control() {
            return match character.to_ascii_lowercase().as_str() {
                "n" if modifiers.shift() => Some(AppMessage::NewSketch),
                "s" => Some(AppMessage::SaveCurrentFile),
                "z" if modifiers.shift() => Some(AppMessage::Menu(MenuAction::Redo)),
                "z" => Some(AppMessage::Menu(MenuAction::Undo)),
                "y" => Some(AppMessage::Menu(MenuAction::Redo)),
                _ => None,
            };
        }
        return None;
    }))
    .window_size(Size::new(1280.0, 800.0))
    .centered()
    .run()
}

//-----------------------------------------------------------------------------------------------------------------------------
