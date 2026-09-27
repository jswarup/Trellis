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
        if self.is_dir {
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
        if self.path == target {
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
                if ( target.starts_with( &child.path) || child.path == target)
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
        let  	uri = format!( "file://{}", self.path.display());
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
    /// Creates an expanded root directory through the Fenst filesystem provider.
    fn	root( path: PathBuf) -> Self
    {
        let  	registry = XplrRegistry::New();
        let  	uri = format!( "file://{}", path.display());
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
}
/// State for the filesystem explorer.
#[derive( Debug, Clone)]
pub struct ExplorerState
{
    pub root_node: FileTreeNode,
    pub selected_path: Option< PathBuf>,
    pub available_roots: Vec< PathBuf>,
    pub context_menu: Option< PathBuf>,
}
impl ExplorerState
{
    pub fn	new( initial_path: PathBuf) -> Self
    {
        let  	roots = detect_system_roots();
        let  	root_node = FileTreeNode::root( initial_path);
        Self {
            root_node,
            selected_path: None,
            available_roots: roots,
            context_menu: None,
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
        self.root_node.toggle_path( path);
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
        self.root_node.refresh();
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
pub fn	view_explorer< 'a, Message: 'static + Clone>(
    state: &'a ExplorerState, palette: ThemePalette,
    map_action: impl Fn( ExplorerAction) -> Message + Copy + 'static,
) -> Element< 'a, Message> {
    let  	header_title = text( "EXPLORER").size( 11).style( move |_| text::Style {
        color: Some( palette.text_muted),
    });
    let  	refresh_btn = button( text( "🔄").size( 12).style( move |_| text::Style {
        color: Some( palette.text_secondary),
    }))
    .padding( [2, 5])
    .style( move |_, status| FasciaStyle::toolbar_button( palette, status))
    .on_press( map_action( ExplorerAction::Refresh));
    let  	header_row = row![header_title, Space::new().width( Length::Fill), refresh_btn,]
        .align_y( Alignment::Center)
        .padding( [6, 10]);
    let  	mut drive_buttons = row![].spacing( 4).padding( [2, 10]);
    for root in &state.available_roots {
        let  	is_current = state.root_node.path.starts_with( root);
        let  	root_str = root.to_str().unwrap_or( "Drive").to_string();
        let  	target_root = root.clone();
        let  	btn = button( text( root_str).size( 11).style( move |_| text::Style {
            color: if is_current {
                Some( palette.accent)
            } else {
                Some( palette.text_secondary)
            },
        }))
        .padding( [2, 6])
        .style( move |_, status| FasciaStyle::toolbar_button( palette, status))
        .on_press( map_action( ExplorerAction::SelectDrive( target_root)));
        drive_buttons = drive_buttons.push( btn);
    }
    let  	folder_name = state
        .root_node
        .path
        .file_name()
        .and_then( |n| n.to_str())
        .unwrap_or_else( || state.root_node.path.to_str().unwrap_or( "Workspace"));
    let  	folder_banner = container(
        row![
            text( "📂").size( 13),
            Space::new().width( Length::Fixed( 6.0)),
            text( folder_name).size( 12).style( move |_| text::Style {
                color: Some( palette.text_primary),
            }),
        ]
        .align_y( Alignment::Center),
    )
    .padding( [4, 10])
    .width( Length::Fill);
    let  	mut tree_elements = Vec::new();
    if let  	Some( children) = &state.root_node.children {
        for child in children {
            render_tree_node(
                child,
                state.selected_path.as_ref(),
                state.context_menu.as_ref(),
                palette,
                map_action,
                &mut tree_elements,
            );
        }
    } else {
        tree_elements.push(
            text( "Loading...")
                .size( 12)
                .style( move |_| text::Style {
                    color: Some( palette.text_muted),
                })
                .into(),
        );
    }
    let  	tree_column = column( tree_elements).spacing( 1).width( Length::Fill);
    let  	scroll = scrollable( tree_column).height( Length::Fill);
    let  	explorer_layout = column![header_row, drive_buttons, folder_banner, scroll,]
        .spacing( 2)
        .width( Length::Fill)
        .height( Length::Fill);
    container( explorer_layout)
        .width( Length::Fixed( 260.0))
        .height( Length::Fill)
        .style( move |_| FasciaStyle::sidebar_container( palette))
        .into()
}
