// src/fascia/app.rs
use	crate::fascia::explorer::{ default_initial_dir, is_text_file };
use crate::fascia::geometry_view::{ GeometryAction, GeometryViewerState, ViewGeometry };
use	crate::fascia::theme::default_code_font;
use	crate::fascia::{ ActivityTab, ExplorerAction, ExplorerState, FasciaStyle, FasciaTheme, MenuAction, StatusBarInfo, TabBarAction, TabId, TabKind, TabManager, ThemePalette, ToolBarAction, WaveformAction, WaveformState, view_activity_bar, view_explorer, view_menubar, view_shell, view_status_bar, view_tab_bar, view_toolbar, view_waveform };
use	crate::rube::{ ParseVcd, VcdDisplayModel };
use crate::fleck::geometry::GeometryAsset;
use	iced::widget::{ Space, button, column, container, row, scrollable, text, text_editor, text_input };
use	iced::{ Alignment, Element, Length, Size, Task };
use	std::collections::HashMap;
use std::collections::BTreeMap;
use std::sync::Arc;
use	std::path::PathBuf;

//-------------------------------------------------------------------------------------------------
/// Unified message enum for the application.
#[derive( Debug, Clone)]
pub enum AppMessage {
    Menu( MenuAction),
    ToolBar( ToolBarAction),
    SelectActivityTab( ActivityTab),
    Explorer( ExplorerAction),
    TabBar( TabBarAction),
    EditorAction( text_editor::Action),
    Waveform( WaveformAction),
    Geometry( TabId, GeometryAction),
    GeometryLoaded( TabId, Result< Arc< GeometryAsset>, String>),
    OpenFile( PathBuf),
    SaveCurrentFile,
    NewFile,
    CloseActiveTab,
    CloseTab( usize),
    SelectTab( usize),
    ToggleSidebar,
    ToggleStatusBar,
    SelectTheme( FasciaTheme),
    ToggleTheme,
    OpenSettings,
}

//-------------------------------------------------------------------------------------------------

pub struct AppState
{
    pub theme: FasciaTheme,
    pub show_sidebar: bool,
    pub show_status_bar: bool,
    pub active_activity_tab: Option< ActivityTab>,
    pub explorer: ExplorerState,
    pub tab_manager: TabManager,
    pub open_editors: HashMap< TabId, text_editor::Content>,
    pub open_waveforms: HashMap< TabId, WaveformState>,
    _GeometryViews: BTreeMap< u64, GeometryViewerState>,
    pub status_info: StatusBarInfo,
}
impl Default for AppState {
    fn	default() -> Self
    {
        let  	initial_dir = default_initial_dir();
        let  	explorer = ExplorerState::new( initial_dir.clone());
        let  	tab_manager = TabManager::default();
        let  	status_info = StatusBarInfo {
            message: format!( "Ready - {}", initial_dir.display()),
            ..Default::default()
        };
        Self {
            theme: FasciaTheme::default(),
            show_sidebar: true,
            show_status_bar: true,
            active_activity_tab: Some( ActivityTab::Explorer),
            explorer,
            tab_manager,
            open_editors: HashMap::new(),
            open_waveforms: HashMap::new(),
            _GeometryViews: BTreeMap::new(),
            status_info,
        }
    }
}
impl AppState
{
    pub fn	new() -> Self
    {
        Self::default()
    }
    pub fn	palette( &self) -> ThemePalette
    {
        self.theme.palette()
    }
    pub fn Geometry( &self, id: TabId) -> Option< &GeometryViewerState>
    {
        self._GeometryViews.get( &id.0)
    }
    fn	update_status_for_active_tab( &mut self)
    {
        if let  	Some( tab) = self.tab_manager.active_tab() {
            let  	lang = if let  	Some( path) = &tab.path {
                path.extension()
                    .and_then( |e| e.to_str())
                    .unwrap_or( "text")
                    .to_uppercase()
            } else {
                "Plain Text".to_string()
            };
            self.status_info.language = lang;
        } else {
            self.status_info.language = "None".to_string();
        }
    }
    pub fn	update( &mut self, message: AppMessage) -> Task< AppMessage>
    {
        match message {
            AppMessage::Menu( action) => match action {
                MenuAction::NewFile => return self.update( AppMessage::NewFile),
                MenuAction::OpenFile | MenuAction::OpenFolder => {
                    self.active_activity_tab = Some( ActivityTab::Explorer);
                    self.show_sidebar = true;
                }
                MenuAction::SaveFile => return self.update( AppMessage::SaveCurrentFile),
                MenuAction::CloseTab => return self.update( AppMessage::CloseActiveTab),
                MenuAction::CloseAllTabs => {
                    self.tab_manager.close_all();
                    self.open_editors.clear();
                    self.open_waveforms.clear();
                    self._GeometryViews.clear();
                    self.status_info.message = "All tabs closed".to_string();
                }
                MenuAction::Exit => {
                    std::process::exit( 0);
                }
                MenuAction::Undo
                | MenuAction::Redo
                | MenuAction::Cut
                | MenuAction::Copy
                | MenuAction::Paste
                | MenuAction::SelectAll => {}
                MenuAction::ToggleSidebar => return self.update( AppMessage::ToggleSidebar),
                MenuAction::ToggleStatusBar => return self.update( AppMessage::ToggleStatusBar),
                MenuAction::OpenExplorer => {
                    self.active_activity_tab = Some( ActivityTab::Explorer);
                    self.show_sidebar = true;
                }
                MenuAction::OpenSearch => {
                    self.active_activity_tab = Some( ActivityTab::Search);
                    self.show_sidebar = true;
                }
                MenuAction::OpenSettings => return self.update( AppMessage::OpenSettings),
                MenuAction::SelectTheme( t) => return self.update( AppMessage::SelectTheme( t)),
                MenuAction::About => {
                    self.status_info.message = "Fascia UI Shell v0.1.0".to_string();
                }
                MenuAction::Documentation => {
                    self.status_info.message =
                        "Documentation: https://github.com/iced-rs/iced".to_string();
                }
            },
            AppMessage::ToolBar( action) => match action {
                ToolBarAction::NewFile => return self.update( AppMessage::NewFile),
                ToolBarAction::OpenFile => {
                    self.active_activity_tab = Some( ActivityTab::Explorer);
                    self.show_sidebar = true;
                }
                ToolBarAction::SaveFile => return self.update( AppMessage::SaveCurrentFile),
                ToolBarAction::Undo | ToolBarAction::Redo => {}
                ToolBarAction::ToggleSidebar => return self.update( AppMessage::ToggleSidebar),
                ToolBarAction::ToggleTheme => return self.update( AppMessage::ToggleTheme),
                ToolBarAction::OpenSettings => return self.update( AppMessage::OpenSettings),
            },
            AppMessage::SelectActivityTab( tab) => {
                if self.active_activity_tab == Some( tab) && self.show_sidebar {
                    self.show_sidebar = false;
                } else {
                    self.active_activity_tab = Some( tab);
                    self.show_sidebar = true;
                }
            }
            AppMessage::Explorer( action) => match action {
                ExplorerAction::ToggleFolder( path) => {
                    self.explorer.toggle_path( &path);
                }
                ExplorerAction::OpenFile( path) => {
                    self.explorer.selected_path = Some( path.clone());
                    return self.update( AppMessage::OpenFile( path));
                }
                ExplorerAction::SelectDrive( root) => {
                    self.explorer.set_root( root.clone());
                    self.status_info.message = format!( "Root: {}", root.display());
                }
                ExplorerAction::Refresh => {
                    self.explorer.refresh();
                    self.status_info.message = "Explorer refreshed".to_string();
                }
            },
            AppMessage::TabBar( action) => match action {
                TabBarAction::SelectTab( idx) => return self.update( AppMessage::SelectTab( idx)),
                TabBarAction::CloseTab( idx) => return self.update( AppMessage::CloseTab( idx)),
                TabBarAction::NewTab => return self.update( AppMessage::NewFile),
                TabBarAction::CloseAll => {
                    self.tab_manager.close_all();
                    self.open_editors.clear();
                    self.open_waveforms.clear();
                    self._GeometryViews.clear();
                }
            },
            AppMessage::EditorAction( action) => {
                let  	is_edit = action.is_edit();
                if let  	Some( tab) = self.tab_manager.active_tab() {
                    let  	tab_id = tab.id;
                    if let  	Some( editor) = self.open_editors.get_mut( &tab_id) {
                        editor.perform( action);
                        let  	( line, col) = ( 0, 0);
                        self.status_info.line = line + 1;
                        self.status_info.column = col + 1;
                    }
                    if is_edit {
                        self.tab_manager.mark_active_dirty( true);
                    }
                }
            }
            AppMessage::Waveform( action) => {
                if let  	Some( tab) = self.tab_manager.active_tab()
                    && let  	Some( waveform) = self.open_waveforms.get_mut( &tab.id)
                {
                    waveform.Update( action);
                }
            }
            AppMessage::Geometry( id, action) => {
                if let  Some( view) = self._GeometryViews.get_mut( &id.0)
                {
                    view.Update( action);
                }
            }
            AppMessage::GeometryLoaded( id, result) => {
                if let  Some( view) = self._GeometryViews.get_mut( &id.0)
                {
                    view.Complete( result);
                    if self.tab_manager.active_tab().is_some_and( |tab| tab.id == id)
                    {
                        self.status_info.message = view.Error().unwrap_or( "Geometry ready").to_string();
                    }
                }
            }
            AppMessage::OpenFile( path) => {
                let  	( idx, is_new) = self.tab_manager.open_file( path.clone());
                if is_new {
                    let  	tab = &self.tab_manager.tabs()[idx as u32];
                    if tab.kind == TabKind::VcdViewer {
                        let  	waveform = match std::fs::read_to_string( &path) {
                            Ok( content) => ParseVcd( &content)
                                .map( |model| {
                                    WaveformState::New( VcdDisplayModel::FromVcdModel( &model))
                                })
                                .unwrap_or_else( WaveformState::FromError),
                            Err( error) => WaveformState::FromError( error.to_string()),
                        };
                        self.open_waveforms.insert( tab.id, waveform);
                    } else if matches!( tab.kind, TabKind::PtsViewer | TabKind::ObjViewer) {
                        let     id = tab.id;
                        let     view = GeometryViewerState::default();
                        let     cancelled = view.Cancellation();
                        self._GeometryViews.insert( id.0, view);
                        self.update_status_for_active_tab();
                        self.status_info.message = format!( "Loading {}", path.display());
                        return Task::perform(
                            crate::fascia::geometry_load::Load( path, cancelled),
                            move |result| AppMessage::GeometryLoaded( id, result),
                        );
                    } else {
                        let  	content_str = if is_text_file( &path) {
                            std::fs::read_to_string( &path)
                                .unwrap_or_else( |e| format!( "// Error reading file: {}\n", e))
                        } else {
                            format!( "// Binary or unsupported file: {}\n", path.display())
                        };
                        let  	editor_content = text_editor::Content::with_text( &content_str);
                        self.open_editors.insert( tab.id, editor_content);
                    }
                }
                self.update_status_for_active_tab();
                self.status_info.message = format!( "Opened {}", path.display());
            }
            AppMessage::SaveCurrentFile => {
                if let  	Some( tab) = self.tab_manager.active_tab() {
                    let  	tab_id = tab.id;
                    if let  	Some( path) = tab.path.clone() {
                        if let  	Some( editor) = self.open_editors.get( &tab_id) {
                            let  	text = editor.text();
                            if let  	Err( e) = std::fs::write( &path, text) {
                                self.status_info.message = format!( "Error saving: {}", e);
                            } else {
                                self.tab_manager.mark_active_dirty( false);
                                self.status_info.message = format!( "Saved {}", path.display());
                            }
                        }
                    } else {
                        self.tab_manager.mark_active_dirty( false);
                        self.status_info.message = "Saved draft".to_string();
                    }
                }
            }
            AppMessage::NewFile => {
                self.tab_manager.open_new_file();
                if let  	Some( tab) = self.tab_manager.active_tab() {
                    let  	editor_content = text_editor::Content::with_text( "// New file\n");
                    self.open_editors.insert( tab.id, editor_content);
                }
                self.update_status_for_active_tab();
                self.status_info.message = "Created new file".to_string();
            }
            AppMessage::CloseActiveTab => {
                if let  	Some( idx) = self.tab_manager.active_index() {
                    return self.update( AppMessage::CloseTab( idx));
                }
            }
            AppMessage::CloseTab( idx) => {
                if let  	Some( closed) = self.tab_manager.close_tab( idx) {
                    self.open_editors.remove( &closed.id);
                    self.open_waveforms.remove( &closed.id);
                    self._GeometryViews.remove( &closed.id.0);
                    self.update_status_for_active_tab();
                    self.status_info.message = format!( "Closed {}", closed.title);
                }
            }
            AppMessage::SelectTab( idx) => {
                self.tab_manager.select_tab( idx);
                self.update_status_for_active_tab();
            }
            AppMessage::ToggleSidebar => {
                self.show_sidebar = !self.show_sidebar;
            }
            AppMessage::ToggleStatusBar => {
                self.show_status_bar = !self.show_status_bar;
            }
            AppMessage::SelectTheme( t) => {
                self.theme = t;
                self.status_info.message = format!( "Theme: {}", t.display_name());
            }
            AppMessage::ToggleTheme => {
                let  	next_theme = match self.theme {
                    FasciaTheme::WindowsDark => FasciaTheme::WindowsLight,
                    FasciaTheme::WindowsLight => FasciaTheme::LinuxDark,
                    FasciaTheme::LinuxDark => FasciaTheme::LinuxLight,
                    FasciaTheme::LinuxLight => FasciaTheme::VsCodeDark,
                    FasciaTheme::VsCodeDark => FasciaTheme::WindowsDark,
                };
                return self.update( AppMessage::SelectTheme( next_theme));
            }
            AppMessage::OpenSettings => {
                self.tab_manager.open_settings();
                self.update_status_for_active_tab();
            }
        }
        Task::none()
    }
    pub fn	view( &self) -> Element< '_, AppMessage> {
        let  	palette = self.palette();
        // 1. Top Menubar
        let  	menubar = view_menubar( palette, self.theme, AppMessage::Menu);
        // 2. Toolbar
        let  	toolbar = view_toolbar( palette, AppMessage::ToolBar);
        // 3. Activity Bar (Leftmost vertical strip)
        let  	activity_bar = view_activity_bar( 
            self.active_activity_tab,
            palette,
            AppMessage::SelectActivityTab,
        );
        // 4. Sidebar (Collapsible)
        let  	sidebar = if self.show_sidebar {
            match self.active_activity_tab {
                Some( ActivityTab::Explorer) => {
                    Some( view_explorer( &self.explorer, palette, AppMessage::Explorer))
                }
                Some( ActivityTab::Search) => Some( view_search_sidebar( palette)),
                Some( ActivityTab::Settings) => None,
                None => None,
            }
        } else {
            None
        };
        // 5. Content Area (VS Code-style Multi-Tabs)
        let  	tab_bar = view_tab_bar( 
            self.tab_manager.tabs(),
            self.tab_manager.active_index(),
            palette,
            AppMessage::TabBar,
        );
        let  	active_tab_content: Element< '_, AppMessage> =
            if let  	Some( active_tab) = self.tab_manager.active_tab() {
                match active_tab.kind {
                    TabKind::Welcome => view_welcome( palette),
                    TabKind::Settings => {
                        view_settings( self.theme, palette, self.show_sidebar, self.show_status_bar)
                    }
                    TabKind::FileEditor => {
                        if let  	Some( editor) = self.open_editors.get( &active_tab.id) {
                            view_editor( editor, self.theme, palette)
                        } else {
                            container( text( "Opening buffer...").size( 14))
                                .padding( 20)
                                .into()
                        }
                    }
                    TabKind::VcdViewer => {
                        if let  	Some( waveform) = self.open_waveforms.get( &active_tab.id) {
                            view_waveform( waveform, palette, AppMessage::Waveform)
                        } else {
                            container( text( "Opening waveform...").size( 14))
                                .padding( 20)
                                .into()
                        }
                    }
                    TabKind::PtsViewer | TabKind::ObjViewer => {
                        if let  Some( view) = self._GeometryViews.get( &active_tab.id.0)
                        {
                            let     id = active_tab.id;
                            ViewGeometry( id.0, view, palette, move |action| {
                                AppMessage::Geometry( id, action)
                            })
                        } else {
                            container( text( "Opening geometry...").size( 14))
                                .padding( 20)
                                .into()
                        }
                    }
                }
            } else {
                container( 
                    column![
                        text( "No Open Editors")
                            .size( 18)
                            .style( move |_| text::Style {
                                color: Some( palette.text_muted),
                            }),
                        text( "Select a file from the explorer or press New File (Ctrl+N)")
                            .size( 13)
                            .style( move |_| text::Style {
                                color: Some( palette.text_secondary),
                            }),
                    ]
                    .spacing( 8)
                    .align_x( Alignment::Center),
                )
                .width( Length::Fill)
                .height( Length::Fill)
                .align_x( Alignment::Center)
                .align_y( Alignment::Center)
                .into()
            };
        let  	content_column = column![
            tab_bar,
            container( active_tab_content)
                .width( Length::Fill)
                .height( Length::Fill),
        ]
        .width( Length::Fill)
        .height( Length::Fill);
        // 6. Status Bar
        let  	status_bar = if self.show_status_bar {
            Some( view_status_bar( &self.status_info, self.theme, palette))
        } else {
            None
        };
        // Complete Shell Assembly
        view_shell( 
            menubar,
            Some( toolbar),
            activity_bar,
            sidebar,
            content_column.into(),
            status_bar,
            palette,
        )
    }
    pub fn	theme( &self) -> iced::Theme
    {
        match self.theme {
            FasciaTheme::WindowsDark | FasciaTheme::LinuxDark | FasciaTheme::VsCodeDark => {
                iced::Theme::Dark
            }
            FasciaTheme::WindowsLight | FasciaTheme::LinuxLight => iced::Theme::Light,
        }
    }
}

//-------------------------------------------------------------------------------------------------

fn	view_welcome< 'a>(palette: ThemePalette) -> Element<'a, AppMessage>
{
    let  	title = text( "Fascia").size( 36).style( move |_| text::Style {
        color: Some( palette.accent),
    });
    let  	subtitle = text( "Native Multi-Tabbed Workspace Shell for Rust & Iced")
        .size( 16)
        .style( move |_| text::Style {
            color: Some( palette.text_secondary),
        });
    let  	action_card =
        |icon: &'static str, label: &'static str, desc: &'static str, msg: AppMessage| {
            button( 
                column![
                    row![
                        text( icon).size( 20),
                        Space::new().width( Length::Fixed( 8.0)),
                        text( label).size( 14).style( move |_| text::Style {
                            color: Some( palette.text_primary),
                        }),
                    ]
                    .align_y( Alignment::Center),
                    Space::new().height( Length::Fixed( 4.0)),
                    text( desc).size( 12).style( move |_| text::Style {
                        color: Some( palette.text_muted),
                    }),
                ]
                .padding( [12, 16]),
            )
            .width( Length::Fixed( 260.0))
            .style( move |_, status| FasciaStyle::tree_row_button( palette, false, status))
            .on_press( msg)
        };
    let  	actions = row![
        action_card( 
            "📄",
            "New File",
            "Create an untitled draft buffer",
            AppMessage::NewFile
        ),
        action_card( 
            "📁",
            "Explore Files",
            "Browse folders and drives",
            AppMessage::SelectActivityTab( ActivityTab::Explorer)
        ),
    ]
    .spacing( 12);
    let  	actions_row_2 = row![
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
    ]
    .spacing( 12);
    let  	content = column![
        title,
        subtitle,
        Space::new().height( Length::Fixed( 24.0)),
        actions,
        Space::new().height( Length::Fixed( 12.0)),
        actions_row_2,
    ]
    .padding( [32, 48])
    .max_width( 700.0);
    scrollable( 
        container( content)
            .width( Length::Fill)
            .align_x( Alignment::Center),
    )
    .into()
}
fn	view_editor< 'a>(
    content: &'a text_editor::Content, theme: FasciaTheme, palette: ThemePalette,
) -> Element< 'a, AppMessage> {
    let  	font = default_code_font( theme);
    let  	editor = text_editor( content)
        .font( font)
        .padding( 12)
        .on_action( AppMessage::EditorAction);
    container( editor)
        .width( Length::Fill)
        .height( Length::Fill)
        .style( move |_| FasciaStyle::content_container( palette))
        .into()
}
fn	view_search_sidebar< 'a>(palette: ThemePalette) -> Element<'a, AppMessage>
{
    let  	header = text( "SEARCH").size( 11).style( move |_| text::Style {
        color: Some( palette.text_muted),
    });
    let  	search_input = text_input( "Search in files...", "").padding( [6, 10]);
    let  	placeholder = text( "Type a query to search across the workspace...")
        .size( 12)
        .style( move |_| text::Style {
            color: Some( palette.text_muted),
        });
    let  	layout = column![
        header,
        Space::new().height( Length::Fixed( 8.0)),
        search_input,
        Space::new().height( Length::Fixed( 16.0)),
        placeholder,
    ]
    .padding( [10, 12])
    .width( Length::Fill);
    container( layout)
        .width( Length::Fixed( 260.0))
        .height( Length::Fill)
        .style( move |_| FasciaStyle::sidebar_container( palette))
        .into()
}
fn	view_settings< 'a>(
    current_theme: FasciaTheme, palette: ThemePalette, show_sidebar: bool, show_status_bar: bool,
) -> Element< 'a, AppMessage> {
    let  	header = text( "Settings").size( 28).style( move |_| text::Style {
        color: Some( palette.text_primary),
    });
    let  	theme_header = text( "Appearance & Native Styling")
        .size( 16)
        .style( move |_| text::Style {
            color: Some( palette.text_primary),
        });
    let  	mut theme_buttons = column![].spacing( 6);
    for &theme_choice in FasciaTheme::ALL {
        let  	is_selected = current_theme == theme_choice;
        let  	mark = if is_selected { "● " } else { "○ " };
        let  	btn = button( 
            row![
                text( mark).size( 14).style( move |_| text::Style {
                    color: if is_selected {
                        Some( palette.accent)
                    } else {
                        Some( palette.text_muted)
                    },
                }),
                Space::new().width( Length::Fixed( 6.0)),
                text( theme_choice.display_name())
                    .size( 13)
                    .style( move |_| text::Style {
                        color: Some( palette.text_primary),
                    }),
            ]
            .align_y( Alignment::Center),
        )
        .padding( [8, 14])
        .width( Length::Fixed( 320.0))
        .style( move |_, status| FasciaStyle::tree_row_button( palette, is_selected, status))
        .on_press( AppMessage::SelectTheme( theme_choice));
        theme_buttons = theme_buttons.push( btn);
    }
    let  	layout_header = text( "Layout Options").size( 16).style( move |_| text::Style {
        color: Some( palette.text_primary),
    });
    let  	toggle_sidebar_btn = button( 
        row![
            text( if show_sidebar {
                "✓ Sidebar Visible"
            } else {
                "✕ Sidebar Hidden"
            })
            .size( 13),
        ]
        .align_y( Alignment::Center),
    )
    .padding( [8, 14])
    .style( move |_, status| FasciaStyle::toolbar_button( palette, status))
    .on_press( AppMessage::ToggleSidebar);
    let  	toggle_statusbar_btn = button( 
        row![
            text( if show_status_bar {
                "✓ Status Bar Visible"
            } else {
                "✕ Status Bar Hidden"
            })
            .size( 13),
        ]
        .align_y( Alignment::Center),
    )
    .padding( [8, 14])
    .style( move |_, status| FasciaStyle::toolbar_button( palette, status))
    .on_press( AppMessage::ToggleStatusBar);
    let  	content = column![
        header,
        Space::new().height( Length::Fixed( 20.0)),
        theme_header,
        Space::new().height( Length::Fixed( 8.0)),
        theme_buttons,
        Space::new().height( Length::Fixed( 24.0)),
        layout_header,
        Space::new().height( Length::Fixed( 8.0)),
        row![toggle_sidebar_btn, toggle_statusbar_btn].spacing( 10),
    ]
    .padding( [32, 48])
    .max_width( 600.0);
    scrollable( 
        container( content)
            .width( Length::Fill)
            .align_x( Alignment::Center),
    )
    .into()
}

//-------------------------------------------------------------------------------------------------
/// Launches the interactive Fascia desktop GUI application.
pub fn	run_app() -> iced::Result
{
    iced::application( AppState::new, AppState::update, AppState::view)
        .title( |state: &AppState| {
            if let  	Some( tab) = state.tab_manager.active_tab() {
                format!( "{} - Fascia", tab.title)
            } else {
                "Fascia - Native Workspace Shell".to_string()
            }
        })
        .theme( AppState::theme)
        .window_size( Size::new( 1280.0, 800.0))
        .centered()
        .run()
}
