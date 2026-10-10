// src/fascia/explorer.rs
use	crate::fascia::theme::{ FasciaStyle, ThemePalette };
use	crate::fenst::{ Xplr, XplrRegistry };
use	crate::silo::IArr;
use	iced::widget::{ Space, button, column, container, row, scrollable, text };
use	iced::{ Alignment, Element, Length };
use	std::path::{ Path, PathBuf };

//-------------------------------------------------------------------------------------------------
/// Returns a list of available drive roots (on Windows) or root directories.
pub fn	detect_system_roots() -> Vec< PathBuf>
{
    #[cfg( target_os = "windows")] {
        let  	mut drives = Vec::new();
        for letter in b'A'..=b'Z' {
            let  	path_str = format!( "{}:\\", letter as char);
            let  	path = PathBuf::from( &path_str);
            if path.exists() {
                drives.push( path);
            }
        }
        if drives.is_empty() {
            drives.push( PathBuf::from( "C:\\"));
        }
        drives
    }
    #[cfg( not( target_os = "windows"))] {
        vec![PathBuf::from( "/")]
    }
}
/// Returns default initial directory for the explorer (e.g. current working directory or C:\).
pub fn	default_initial_dir() -> PathBuf
{
    if let  	Ok( cwd) = std::env::current_dir() {
        return cwd;
    }
    #[cfg( target_os = "windows")] {
        PathBuf::from( "C:\\")
    }
    #[cfg( not( target_os = "windows"))] {
        PathBuf::from( "/")
    }
}
/// Checks if a file is likely a readable text file by extension or content header.
pub fn	is_text_file( path: &Path) -> bool
{
    let  	ext = path
        .extension()
        .and_then( |e| e.to_str())
        .unwrap_or( "")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "rs" | "toml"
            | "json"
            | "yaml"
            | "yml"
            | "ini"
            | "cfg"
            | "md"
            | "txt"
            | "log"
            | "html"
            | "css"
            | "js"
            | "ts"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "py"
            | "sh"
            | "bat"
            | "cmd"
            | "ps1"
            | "xml"
            | "sql"
            | "gitignore"
            | "lock"
            | ""
    )
}

//-------------------------------------------------------------------------------------------------
/// Represents a single entry (file or folder) in the filesystem explorer.
#[derive( Debug, Clone)]
pub struct FileTreeNode
{
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub children: Option< Vec< FileTreeNode>>,
    pub depth: usize,
}
impl FileTreeNode
{
    pub fn	new( path: PathBuf, depth: usize) -> Self
    {
        let  	name = path
            .file_name()
            .and_then( |n| n.to_str())
            .unwrap_or_else( || path.to_str().unwrap_or( "Root"))
            .to_string();
        let  	is_dir = path.is_dir();
        Self {
            path,
            name,
            is_dir,
            is_expanded: false,
            children: None,
            depth,
        }
    }
    fn	from_xplr( node: &dyn Xplr, depth: usize) -> Self
    {
        Self {
            path: PathBuf::from( node.Path()),
            name: node.Name().to_string(),
            is_dir: !node.IsLeaf(),
            is_expanded: false,
            children: None,
            depth,
        }
    }
    /// Returns an appropriate icon for the file or folder.
    pub fn	icon( &self) -> &'static str {
        let  	path_str = self.path.to_string_lossy();
        if path_str.starts_with( "settings://") {
            return if self.is_dir { "⚙" } else { "🔧" };
        }
        if path_str.starts_with( "example://") {
            return if self.is_dir { "🧪" } else { "🔬" };
        }
        if path_str == "file://" || self.name == "Filesystem" {
            return "💻";
        }
        if self.is_dir {
            if (path_str.len() <= 3 && path_str.contains( ':')) || self.name.ends_with( ':') {
                return "💾";
            }
            if self.is_expanded { "📂" } else { "📁" }
        } else {
            let  	ext = self
                .path
                .extension()
                .and_then( |e| e.to_str())
                .unwrap_or( "")
                .to_lowercase();
            match ext.as_str() {
                "rs" => "🦀",
                "toml" | "json" | "yaml" | "yml" | "ini" | "cfg" => "⚙",
                "md" | "txt" | "log" | "doc" | "docx" => "📝",
                "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" => "🖼",
                "exe" | "bat" | "cmd" | "ps1" | "sh" => "⚡",
                "lock" => "🔒",
                "zip" | "tar" | "gz" | "7z" | "rar" => "📦",
                "html" | "htm" | "css" | "js" | "ts" => "🌐",
                "vcd" => "VCD",
                "c" | "cpp" | "h" | "hpp" => "🇨",
                "py" => "🐍",
                _ => "📄",
            }
        }
    }
    /// Toggles expansion of this node or recursively searches children.
    pub fn	toggle_path( &mut self, target: &Path) -> bool
    {
        let  	path_str = self.path.to_string_lossy();
        let  	target_str = target.to_string_lossy();
        if self.path == target || path_str == target_str {
            if self.is_dir {
                self.is_expanded = !self.is_expanded;
                if self.is_expanded && self.children.is_none() {
                    self.load_children();
                }
            }
            return true;
        }
        if let  	Some( children) = &mut self.children {
            for child in children {
                let  	child_str = child.path.to_string_lossy();
                let  	matches_prefix = target.starts_with( &child.path)
                    || ( child_str.contains( "://") && target_str.starts_with( &*child_str));
                if ( matches_prefix || child.path == target || child_str == target_str)
                    && child.toggle_path( target)
                {
                    return true;
                }
            }
        }
        false
    }
    /// Loads directory entries from the filesystem into `children`.
    pub fn	load_children( &mut self)
    {
        if !self.is_dir {
            return;
        }
        let  	mut dirs = Vec::new();
        let  	mut files = Vec::new();
        let  	registry = XplrRegistry::New();
        let  	path_str = self.path.to_string_lossy();
        let  	uri = if path_str.contains( "://") {
            path_str.to_string()
        } else {
            format!( "file://{}", self.path.display())
        };
        if let  	Ok( ( _, branch)) = registry.OpenRoot( &uri)
            && let  	Ok( children) = branch.Children()
        {
            children.Arr().Traverse( |child| {
                let  	node = FileTreeNode::from_xplr( child.as_ref(), self.depth + 1);
                if node.is_dir {
                    dirs.push( node);
                } else {
                    files.push( node);
                }
            });
        }
        dirs.sort_by_key( |node| node.name.to_lowercase());
        files.sort_by_key( |node| node.name.to_lowercase());
        dirs.extend( files);
        self.children = Some( dirs);
    }
    /// Recursively expands all directories along the ancestor path to `target`.
    pub fn	expand_path_ancestors( &mut self, target: &Path) -> bool
    {
        if target.starts_with( &self.path) || self.path == target {
            if self.is_dir {
                self.is_expanded = true;
                if self.children.is_none() {
                    self.load_children();
                }
                if let  	Some( children) = &mut self.children {
                    for child in children.iter_mut() {
                        if target.starts_with( &child.path) || child.path == target {
                            child.expand_path_ancestors( target);
                        }
                    }
                }
            }
            return true;
        }
        false
    }
    /// Creates an expanded Filesystem root node with system drives as first-level children.
    pub fn	root_filesystem() -> Self
    {
        let  	registry = XplrRegistry::New();
        let  	mut root = registry
            .OpenRoot( "file://")
            .map( |( _, branch)| FileTreeNode::from_xplr( branch.as_ref(), 0))
            .unwrap_or_else( |_| FileTreeNode::new( PathBuf::from( "file://"), 0));
        root.name = "Filesystem".to_string();
        root.is_expanded = true;
        root.load_children();
        root
    }
    /// Creates an expanded root directory through the Fenst filesystem provider.
    fn	root( path: PathBuf) -> Self
    {
        let  	path_str = path.to_string_lossy();
        if path_str.is_empty() || path_str == "file://" {
            return Self::root_filesystem();
        }
        let  	registry = XplrRegistry::New();
        let  	uri = if path_str.contains( "://") {
            path_str.to_string()
        } else {
            format!( "file://{}", path.display())
        };
        let  	mut root = registry
            .OpenRoot( &uri)
            .map( |( _, branch)| FileTreeNode::from_xplr( branch.as_ref(), 0))
            .unwrap_or_else( |_| FileTreeNode::new( path, 0));
        root.is_expanded = true;
        root.load_children();
        root
    }
    /// Reloads this folder if already expanded.
    pub fn	refresh( &mut self)
    {
        if self.is_dir && self.is_expanded {
            self.load_children();
            if let  	Some( children) = &mut self.children {
                for child in children {
                    if child.is_dir && child.is_expanded {
                        child.refresh();
                    }
                }
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
/// Active tab in the Explorer sidebar.
#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerTab {
    Filesystem,
    Settings,
    Examples,
}
impl ExplorerTab {
    pub fn	title( &self) -> &'static str {
        match self {
            Self::Filesystem => "Filesystem",
            Self::Settings => "Settings",
            Self::Examples => "Examples",
        }
    }
    pub fn	icon( &self) -> &'static str {
        match self {
            Self::Filesystem => "📁",
            Self::Settings => "⚙",
            Self::Examples => "🧪",
        }
    }
}

/// Actions emitted by the explorer view.
#[derive( Debug, Clone, PartialEq, Eq)]
pub enum ExplorerAction {
    ToggleFolder( PathBuf),
    OpenFile( PathBuf),
    SelectDrive( PathBuf),
    Refresh,
    NodeContextMenu( PathBuf),
    OpenCask( PathBuf),
    DismissContextMenu,
    SelectTab( ExplorerTab),
    ToggleFilesystem,
    ToggleExamples,
    ToggleSettings,
}
/// State for the filesystem explorer.
#[derive( Debug, Clone)]
pub struct ExplorerState
{
    pub root_node: FileTreeNode,
    pub selected_path: Option< PathBuf>,
    pub available_roots: Vec< PathBuf>,
    pub context_menu: Option< PathBuf>,
    pub active_tab: ExplorerTab,
    pub show_filesystem: bool,
    pub show_examples: bool,
    pub show_settings: bool,
    pub examples_root: Option< FileTreeNode>,
    pub settings_root: Option< FileTreeNode>,
}
impl ExplorerState
{
    pub fn	new( initial_path: PathBuf) -> Self
    {
        let  	roots = detect_system_roots();
        let  	mut root_node = FileTreeNode::root_filesystem();
        let  	initial_str = initial_path.to_string_lossy();
        let  	selected_path = if !initial_str.is_empty() && initial_str != "file://" {
            if let  	Some( children) = &mut root_node.children {
                for child in children.iter_mut() {
                    if child.expand_path_ancestors( &initial_path) {
                        break;
                    }
                }
            }
            Some( initial_path)
        } else {
            None
        };
        Self {
            root_node,
            selected_path,
            available_roots: roots,
            context_menu: None,
            active_tab: ExplorerTab::Filesystem,
            show_filesystem: true,
            show_examples: false,
            show_settings: false,
            examples_root: None,
            settings_root: None,
        }
    }
    pub fn	has_visible_roots( &self) -> bool
    {
        self.show_filesystem || self.show_examples || self.show_settings
    }
    pub fn	is_tab_visible( &self, tab: ExplorerTab) -> bool
    {
        match tab {
            ExplorerTab::Filesystem => self.show_filesystem,
            ExplorerTab::Settings => self.show_settings,
            ExplorerTab::Examples => self.show_examples,
        }
    }
    pub fn	ensure_valid_active_tab( &mut self)
    {
        if self.is_tab_visible( self.active_tab) {
            return;
        }
        if self.show_filesystem {
            self.active_tab = ExplorerTab::Filesystem;
        } else if self.show_settings {
            self.active_tab = ExplorerTab::Settings;
            self.ensure_settings();
        } else if self.show_examples {
            self.active_tab = ExplorerTab::Examples;
            self.ensure_examples();
        }
    }
    pub fn	select_tab( &mut self, tab: ExplorerTab)
    {
        if self.is_tab_visible( tab) {
            self.active_tab = tab;
            match tab {
                ExplorerTab::Filesystem => {}
                ExplorerTab::Settings => self.ensure_settings(),
                ExplorerTab::Examples => self.ensure_examples(),
            }
        }
    }
    pub fn	ensure_examples( &mut self)
    {
        if self.examples_root.is_none() {
            let  	registry = XplrRegistry::New();
            if let  	Ok( ( _, branch)) = registry.OpenRoot( "example://") {
                let  	mut node = FileTreeNode::from_xplr( branch.as_ref(), 0);
                node.is_expanded = true;
                node.load_children();
                self.examples_root = Some( node);
            }
        }
    }
    pub fn	ensure_settings( &mut self)
    {
        if self.settings_root.is_none() {
            let  	registry = XplrRegistry::New();
            if let  	Ok( ( _, branch)) = registry.OpenRoot( "settings://") {
                let  	mut node = FileTreeNode::from_xplr( branch.as_ref(), 0);
                node.is_expanded = true;
                node.load_children();
                self.settings_root = Some( node);
            }
        }
    }
    pub fn	toggle_filesystem( &mut self)
    {
        self.show_filesystem = !self.show_filesystem;
        if self.show_filesystem {
            self.active_tab = ExplorerTab::Filesystem;
        } else {
            self.ensure_valid_active_tab();
        }
    }
    pub fn	toggle_examples( &mut self)
    {
        self.show_examples = !self.show_examples;
        if self.show_examples {
            self.ensure_examples();
            self.active_tab = ExplorerTab::Examples;
        } else {
            self.ensure_valid_active_tab();
        }
    }
    pub fn	toggle_settings( &mut self)
    {
        self.show_settings = !self.show_settings;
        if self.show_settings {
            self.ensure_settings();
            self.active_tab = ExplorerTab::Settings;
        } else {
            self.ensure_valid_active_tab();
        }
    }
    pub fn	set_root( &mut self, new_root: PathBuf)
    {
        let  	root_node = FileTreeNode::root( new_root);
        self.root_node = root_node;
        self.selected_path = None;
        self.context_menu = None;
    }
    pub fn	toggle_path( &mut self, path: &Path)
    {
        self.selected_path = Some( path.to_path_buf());
        self.context_menu = None;
        match self.active_tab {
            ExplorerTab::Filesystem => {
                if self.show_filesystem {
                    self.root_node.toggle_path( path);
                }
            }
            ExplorerTab::Settings => {
                if self.show_settings
                    && let  	Some( settings) = &mut self.settings_root {
                    settings.toggle_path( path);
                }
            }
            ExplorerTab::Examples => {
                if self.show_examples
                    && let  	Some( examples) = &mut self.examples_root {
                    examples.toggle_path( path);
                }
            }
        }
    }
    pub fn	open_context_menu( &mut self, path: PathBuf)
    {
        self.context_menu = Some( path);
    }
    pub fn	dismiss_context_menu( &mut self)
    {
        self.context_menu = None;
    }
    pub fn	refresh( &mut self)
    {
        match self.active_tab {
            ExplorerTab::Filesystem => {
                if self.show_filesystem {
                    self.root_node.refresh();
                }
            }
            ExplorerTab::Settings => {
                if let  	Some( settings) = &mut self.settings_root {
                    settings.refresh();
                }
            }
            ExplorerTab::Examples => {
                if let  	Some( examples) = &mut self.examples_root {
                    examples.refresh();
                }
            }
        }
    }
}
/// Renders a single row in the explorer tree.
fn	render_tree_node< 'a, Message: 'static + Clone>(
    node: &'a FileTreeNode, selected_path: Option<&'a PathBuf>,
    context_menu: Option<&'a PathBuf>, palette: ThemePalette,
    map_action: impl Fn( ExplorerAction) -> Message + Copy + 'static,
    items: &mut Vec< Element< 'a, Message>>,
)
{
    let  	is_selected = selected_path == Some( &node.path);
    let  	is_menu_open = context_menu == Some( &node.path);
    let  	indent = node.depth as f32 * 12.0;
    let  	arrow = if node.is_dir {
        if node.is_expanded { "▼" } else { "▶" }
    } else {
        " "
    };
    let  	content = row![
        Space::new().width( Length::Fixed( indent)),
        text( arrow).size( 10).style( move |_| text::Style {
            color: Some( palette.text_muted),
        }),
        Space::new().width( Length::Fixed( 4.0)),
        text( node.icon()).size( 14),
        Space::new().width( Length::Fixed( 6.0)),
        text( &node.name).size( 13).style( move |_| text::Style {
            color: Some( palette.text_primary),
        }),
    ]
    .align_y( Alignment::Center);
    let  	path_clone = node.path.clone();
    let  	action_msg = if node.is_dir {
        map_action( ExplorerAction::ToggleFolder( path_clone.clone()))
    } else {
        map_action( ExplorerAction::OpenFile( path_clone.clone()))
    };
    let  	btn = button( content)
        .width( Length::Fill)
        .padding( [3, 6])
        .style( move |_, status| FasciaStyle::tree_row_button( palette, is_selected, status))
        .on_press( action_msg);
    let  	right_press_msg = map_action( ExplorerAction::NodeContextMenu( path_clone.clone()));
    let  	row_element = iced::widget::mouse_area( btn)
        .on_right_press( right_press_msg);
    items.push( row_element.into());

    if is_menu_open {
        let  	cask_path = node.path.clone();
        let  	open_path = node.path.clone();
        let  	is_dir = node.is_dir;

        let  	cask_btn = button(
            row![
                text( "📦").size( 13),
                Space::new().width( Length::Fixed( 6.0)),
                text( "Cask (3D)").size( 12).style( move |_| text::Style {
                    color: Some( palette.accent),
                }),
            ]
            .align_y( Alignment::Center),
        )
        .width( Length::Fill)
        .padding( [4, 8])
        .style( move |_, status| FasciaStyle::menu_item_button( palette, status))
        .on_press( map_action( ExplorerAction::OpenCask( cask_path)));

        let  	open_label = if is_dir { "📂 Open Folder" } else { "📄 Open File" };
        let  	open_action = if is_dir {
            ExplorerAction::ToggleFolder( open_path)
        } else {
            ExplorerAction::OpenFile( open_path)
        };
        let  	open_btn = button(
            row![
                text( if is_dir { "📂" } else { "📄" }).size( 12),
                Space::new().width( Length::Fixed( 6.0)),
                text( open_label).size( 12).style( move |_| text::Style {
                    color: Some( palette.text_primary),
                }),
            ]
            .align_y( Alignment::Center),
        )
        .width( Length::Fill)
        .padding( [4, 8])
        .style( move |_, status| FasciaStyle::menu_item_button( palette, status))
        .on_press( map_action( open_action));

        let  	dismiss_btn = button(
            row![
                text( "✕").size( 11),
                Space::new().width( Length::Fixed( 6.0)),
                text( "Dismiss").size( 11).style( move |_| text::Style {
                    color: Some( palette.text_muted),
                }),
            ]
            .align_y( Alignment::Center),
        )
        .width( Length::Fill)
        .padding( [2, 8])
        .style( move |_, status| FasciaStyle::menu_item_button( palette, status))
        .on_press( map_action( ExplorerAction::DismissContextMenu));

        let  	menu_card = container(
            column![cask_btn, open_btn, dismiss_btn]
                .spacing( 2)
                .width( Length::Fill),
        )
        .padding( 4)
        .width( Length::Fill)
        .style( move |_| container::Style {
            background: Some( iced::Background::Color( palette.sidebar_bg)),
            border: iced::Border {
                color: palette.accent,
                width: 1.0,
                radius: 4.0.into(),
            },
            shadow: iced::Shadow {
                color: iced::Color::from_rgba( 0.0, 0.0, 0.0, 0.35),
                offset: iced::Vector::new( 0.0, 2.0),
                blur_radius: 6.0,
            },
            snap: true,
            ..Default::default()
        });

        let  	menu_row = row![
            Space::new().width( Length::Fixed( indent + 12.0)),
            menu_card,
        ]
        .width( Length::Fill)
        .padding( [2, 4]);

        items.push( menu_row.into());
    }

    if node.is_dir && node.is_expanded {
        for child in node.children.iter().flatten() {
            render_tree_node(
                child,
                selected_path,
                context_menu,
                palette,
                map_action,
                items,
            );
        }
    }
}
/// Constructs the Explorer sidebar panel view.
pub fn view_explorer<'a, Message: 'static + Clone>( state: &'a ExplorerState,
                                                   palette: ThemePalette,
                                                   map_action: impl Fn( ExplorerAction) -> Message
                                                   + Copy
                                                   + 'static)
                                                   -> Element<'a, Message>
{
    return ViewWithExamples( state, palette, map_action, None);
}

pub fn ViewWithExamples<'a, Message: 'static + Clone>(
    state: &'a ExplorerState, palette: ThemePalette,
    map_action: impl Fn( ExplorerAction) -> Message + Copy + 'static,
    examples: Option<Element<'a, Message>>,
) -> Element<'a, Message>
{
    let  	mut tab_buttons = Vec::new();

    if state.show_filesystem {
        let  	is_active = state.active_tab == ExplorerTab::Filesystem;
        let  	btn = button(
            row![
                text( "📁").size( 12),
                Space::new().width( Length::Fixed( 4.0)),
                text( "Filesystem").size( 11).style( move |_| text::Style {
                    color: if is_active {
                        Some( palette.accent)
                    } else {
                        Some( palette.text_secondary)
                    },
                }),
            ]
            .align_y( Alignment::Center),
        )
        .padding( [3, 8])
        .style( move |_, status| FasciaStyle::tab_button( palette, is_active, status))
        .on_press( map_action( ExplorerAction::SelectTab( ExplorerTab::Filesystem)));
        tab_buttons.push( btn.into());
    }

    if state.show_settings {
        let  	is_active = state.active_tab == ExplorerTab::Settings;
        let  	btn = button(
            row![
                text( "⚙").size( 12),
                Space::new().width( Length::Fixed( 4.0)),
                text( "Settings").size( 11).style( move |_| text::Style {
                    color: if is_active {
                        Some( palette.accent)
                    } else {
                        Some( palette.text_secondary)
                    },
                }),
            ]
            .align_y( Alignment::Center),
        )
        .padding( [3, 8])
        .style( move |_, status| FasciaStyle::tab_button( palette, is_active, status))
        .on_press( map_action( ExplorerAction::SelectTab( ExplorerTab::Settings)));
        tab_buttons.push( btn.into());
    }

    if state.show_examples {
        let  	is_active = state.active_tab == ExplorerTab::Examples;
        let  	btn = button(
            row![
                text( "🧪").size( 12),
                Space::new().width( Length::Fixed( 4.0)),
                text( "Examples").size( 11).style( move |_| text::Style {
                    color: if is_active {
                        Some( palette.accent)
                    } else {
                        Some( palette.text_secondary)
                    },
                }),
            ]
            .align_y( Alignment::Center),
        )
        .padding( [3, 8])
        .style( move |_, status| FasciaStyle::tab_button( palette, is_active, status))
        .on_press( map_action( ExplorerAction::SelectTab( ExplorerTab::Examples)));
        tab_buttons.push( btn.into());
    }

    let  	refresh_btn = button( text( "🔄").size( 12).style( move |_| text::Style {
        color: Some( palette.text_secondary),
    }))
    .padding( [2, 5])
    .style( move |_, status| FasciaStyle::toolbar_button( palette, status))
    .on_press( map_action( ExplorerAction::Refresh));

    let  	header_row = row![
        row( tab_buttons).spacing( 2).align_y( Alignment::Center),
        Space::new().width( Length::Fill),
        refresh_btn,
    ]
    .spacing( 4)
    .align_y( Alignment::Center)
    .padding( [4, 6]);

    let  	mut tree_elements = Vec::new();
    match state.active_tab {
        ExplorerTab::Filesystem => {
            if state.show_filesystem {
                render_tree_node(
                    &state.root_node,
                    state.selected_path.as_ref(),
                    state.context_menu.as_ref(),
                    palette,
                    map_action,
                    &mut tree_elements,
                );
            }
        }
        ExplorerTab::Settings => {
            if state.show_settings && let  	Some( settings_root) = &state.settings_root {
                render_tree_node(
                    settings_root,
                    state.selected_path.as_ref(),
                    state.context_menu.as_ref(),
                    palette,
                    map_action,
                    &mut tree_elements,
                );
            }
        }
        ExplorerTab::Examples => {
            if state.show_examples {
                if let  	Some( examples_root) = &state.examples_root {
                    render_tree_node(
                        examples_root,
                        state.selected_path.as_ref(),
                        state.context_menu.as_ref(),
                        palette,
                        map_action,
                        &mut tree_elements,
                    );
                } else if let  	Some( legacy_examples) = examples {
                    tree_elements.insert( 0, legacy_examples);
                }
            }
        }
    }

    let  	tree_column = column( tree_elements).spacing( 1).width( Length::Fill);
    let  	scroll = scrollable( tree_column).height( Length::Fill);
    let  	explorer_layout = column![header_row, scroll]
        .spacing( 2)
        .width( Length::Fill)
        .height( Length::Fill);
    container( explorer_layout)
        .width( Length::Fixed( 260.0))
        .height( Length::Fill)
        .style( move |_| FasciaStyle::sidebar_container( palette))
        .into()
}
