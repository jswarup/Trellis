// src/fascia/menubar.rs
use crate::fascia::command::{CommandContext, CommandId, CommandRegistry};
use crate::fascia::theme::{FasciaStyle, FasciaTheme, ThemePalette};
use iced::widget::{Space, button, container, row, text};
use iced::{Alignment, Element, Length};
use iced_aw::menu::{Item, Menu, MenuBar};

/// Actions emitted by the menubar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction
{
    NewFile,
    NewSketch,
    NewMathView,
    OpenFile,
    OpenFolder,
    SaveFile,
    CloseTab,
    CloseAllTabs,
    Exit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    ToggleSidebar,
    ToggleStatusBar,
    ToggleBottomPanel,
    ToggleProperties,
    ToggleExplorerFilesystem,
    ToggleExplorerSettings,
    ToggleExplorerExamples,
    OpenExplorer,
    OpenSearch,
    OpenSettings,
    OpenPythonConsole,
    OpenOutput,
    RunExample,
    StopJob,
    SelectTheme(FasciaTheme),
    About,
    Documentation,
}

impl MenuAction
{
    /// Maps a legacy MenuAction to its corresponding unified CommandId where applicable.
    pub fn ToCommand(&self) -> Option<CommandId>
    {
        match self {
            Self::NewFile => Some(CommandId::FILE_NEW),
            Self::NewSketch => Some(CommandId::FILE_NEW_SKETCH),
            Self::NewMathView => Some(CommandId::FILE_NEW_MATH),
            Self::OpenFile => Some(CommandId::FILE_OPEN),
            Self::OpenFolder => Some(CommandId::FOLDER_OPEN),
            Self::SaveFile => Some(CommandId::FILE_SAVE),
            Self::CloseTab => Some(CommandId::TAB_CLOSE),
            Self::CloseAllTabs => Some(CommandId::TAB_CLOSE_ALL),
            Self::Exit => Some(CommandId::APP_EXIT),
            Self::Undo => Some(CommandId::EDIT_UNDO),
            Self::Redo => Some(CommandId::EDIT_REDO),
            Self::Cut => Some(CommandId::EDIT_CUT),
            Self::Copy => Some(CommandId::EDIT_COPY),
            Self::Paste => Some(CommandId::EDIT_PASTE),
            Self::SelectAll => Some(CommandId::EDIT_SELECT_ALL),
            Self::ToggleSidebar => Some(CommandId::VIEW_TOGGLE_SIDEBAR),
            Self::ToggleStatusBar => Some(CommandId::VIEW_TOGGLE_STATUS_BAR),
            Self::ToggleBottomPanel => Some(CommandId::VIEW_TOGGLE_BOTTOM_PANEL),
            Self::ToggleProperties => Some(CommandId::VIEW_TOGGLE_PROPERTIES),
            Self::ToggleExplorerFilesystem => Some(CommandId::VIEW_TOGGLE_EXPLORER_FS),
            Self::ToggleExplorerSettings => Some(CommandId::VIEW_TOGGLE_EXPLORER_SETTINGS),
            Self::ToggleExplorerExamples => Some(CommandId::VIEW_TOGGLE_EXPLORER_EXAMPLES),
            Self::OpenExplorer => Some(CommandId::VIEW_OPEN_EXPLORER),
            Self::OpenSearch => Some(CommandId::VIEW_OPEN_SEARCH),
            Self::OpenSettings => Some(CommandId::VIEW_OPEN_SETTINGS),
            Self::OpenPythonConsole => Some(CommandId::VIEW_OPEN_PYTHON_CONSOLE),
            Self::OpenOutput => Some(CommandId::VIEW_OPEN_OUTPUT),
            Self::RunExample => Some(CommandId::RUN_EXAMPLE),
            Self::StopJob => Some(CommandId::RUN_STOP),
            Self::SelectTheme(FasciaTheme::WindowsDark) => Some(CommandId::THEME_SELECT_WINDOWS_DARK),
            Self::SelectTheme(FasciaTheme::WindowsLight) => Some(CommandId::THEME_SELECT_WINDOWS_LIGHT),
            Self::SelectTheme(FasciaTheme::LinuxDark) => Some(CommandId::THEME_SELECT_LINUX_DARK),
            Self::SelectTheme(FasciaTheme::LinuxLight) => Some(CommandId::THEME_SELECT_LINUX_LIGHT),
            Self::SelectTheme(FasciaTheme::VsCodeDark) => Some(CommandId::THEME_SELECT_VSCODE_DARK),
            Self::About => Some(CommandId::HELP_ABOUT),
            Self::Documentation => Some(CommandId::HELP_DOCUMENTATION),
        }
    }
}
/// Helper to render an item button with label and shortcut.
fn menu_button<'a, Message: 'static + Clone>(label: &'static str, shortcut: Option<&'static str>,
                                             action: Option<Message>, palette: ThemePalette)
                                             -> button::Button<'a, Message>
{
    let label_color = if action.is_some() { palette.text_primary } else { palette.text_muted };
    let mut content = row![text( label).size( 13).style( move |_| text::Style {
        color: Some(label_color),
    })].align_y(Alignment::Center)
                      .width(Length::Fill);
    if let Some(sc) = shortcut {
        content = content.push(Space::new().width(Length::Fill))
                         .push(text(sc).size(11).style(move |_| {
                                                    text::Style { color: Some(palette.text_muted), }
                                                }));
    }
    let mut btn =
        button(content).padding([4, 10])
                       .width(Length::Fill)
                       .style(move |_, status| FasciaStyle::menu_item_button(palette, status));
    if let Some(msg) = action {
        btn = btn.on_press(msg);
    }
    btn
}

fn menu_action_button<'a, Message: 'static + Clone>(label: &'static str, action: MenuAction,
                                                    map_action: impl Fn(MenuAction) -> Message + Copy + 'static,
                                                    palette: ThemePalette)
                                                    -> button::Button<'a, Message>
{
    let shortcut = action.ToCommand().and_then(CommandRegistry::ShortcutFor);
    return menu_button(label, shortcut, Some(map_action(action)), palette);
}
fn root_button<'a, Message: 'static>(title: &'static str, palette: ThemePalette)
                                     -> button::Button<'a, Message>
{
    button( text( title).size( 13).style( move |_| text::Style {
        color: Some( palette.text_primary),
    }))
    .padding( [4, 8])
    .style( move |_, status| FasciaStyle::toolbar_button( palette, status))
}
/// Constructs the full Fascia Menubar.
pub fn view_menubar<'a, Message: 'static + Clone>(palette: ThemePalette,
                                                  active_theme: FasciaTheme,
                                                  explorer_views: (bool, bool, bool),
                                                  map_action: impl Fn(MenuAction) -> Message
                                                  + Copy
                                                  + 'static)
                                                  -> Element<'a, Message>
{
    return ViewMenubarWithContext(palette, active_theme, explorer_views, None, map_action);
}

pub(super) fn ViewMenubarWithContext<'a, Message: 'static + Clone>(
    palette: ThemePalette, active_theme: FasciaTheme, explorer_views: (bool, bool, bool),
    context: Option<&CommandContext>, map_action: impl Fn(MenuAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let sub_template = |items: Vec<_>| Menu::new(items).max_width(220.0).offset(2.0);
    // --- File Menu ---
    let file_items = vec![Item::new(menu_action_button("New File", MenuAction::NewFile, map_action, palette)),
                          Item::new(menu_action_button("New Sketch", MenuAction::NewSketch, map_action, palette)),
                          Item::new(menu_action_button("New Math View", MenuAction::NewMathView, map_action, palette)),
                          Item::new(menu_action_button("Open File...", MenuAction::OpenFile, map_action, palette)),
                          Item::new(menu_action_button("Open Folder...", MenuAction::OpenFolder, map_action, palette)),
                          Item::new(menu_action_button("Save", MenuAction::SaveFile, map_action, palette)),
                          Item::new(menu_action_button("Close Tab", MenuAction::CloseTab, map_action, palette)),
                          Item::new(menu_action_button("Close All Tabs", MenuAction::CloseAllTabs, map_action, palette)),
                          Item::new(menu_action_button("Exit", MenuAction::Exit, map_action, palette)),];
    let file_root = Item::with_menu(root_button("File", palette), sub_template(file_items));
    // --- Edit Menu ---
    let edit_items = vec![Item::new(menu_action_button("Undo", MenuAction::Undo, map_action, palette)),
                          Item::new(menu_action_button("Redo", MenuAction::Redo, map_action, palette)),
                          Item::new(menu_action_button("Cut", MenuAction::Cut, map_action, palette)),
                          Item::new(menu_action_button("Copy", MenuAction::Copy, map_action, palette)),
                          Item::new(menu_action_button("Paste", MenuAction::Paste, map_action, palette)),
                          Item::new(menu_action_button("Select All", MenuAction::SelectAll, map_action, palette)),];
    let edit_root = Item::with_menu(root_button("Edit", palette), sub_template(edit_items));
    // --- View Menu ---
    let (show_fs, show_settings, show_examples) = explorer_views;
    let fs_label = if show_fs { "✓ Filesystem" } else { "   Filesystem" };
    let settings_label = if show_settings { "✓ Settings" } else { "   Settings" };
    let examples_label = if show_examples { "✓ Examples" } else { "   Examples" };

    let view_items = vec![Item::new(menu_action_button("Toggle Primary Sidebar", MenuAction::ToggleSidebar, map_action, palette)),
                          Item::new(menu_action_button("Toggle Status Bar", MenuAction::ToggleStatusBar, map_action, palette)),
                          Item::new(menu_action_button("Toggle Bottom Panel", MenuAction::ToggleBottomPanel, map_action, palette)),
                          Item::new(menu_action_button("Toggle Properties Panel", MenuAction::ToggleProperties, map_action, palette)),
                          Item::new(menu_action_button(fs_label, MenuAction::ToggleExplorerFilesystem, map_action, palette)),
                          Item::new(menu_action_button(settings_label, MenuAction::ToggleExplorerSettings, map_action, palette)),
                          Item::new(menu_action_button(examples_label, MenuAction::ToggleExplorerExamples, map_action, palette)),
                          Item::new(menu_action_button("Explorer View", MenuAction::OpenExplorer, map_action, palette)),
                          Item::new(menu_action_button("Search View", MenuAction::OpenSearch, map_action, palette)),
                          Item::new(menu_action_button("Settings Tab", MenuAction::OpenSettings, map_action, palette)),
                          Item::new(menu_action_button("Python Console", MenuAction::OpenPythonConsole, map_action, palette)),
                          Item::new(menu_action_button("Output", MenuAction::OpenOutput, map_action, palette)),];
    let view_root = Item::with_menu(root_button("View", palette), sub_template(view_items));
    // --- Run Menu ---
    let run_items = vec![
        Item::new(menu_button("Run Example", CommandRegistry::ShortcutFor(CommandId::RUN_EXAMPLE),
            context.is_none_or(|ctx| CommandRegistry::IsAvailable(CommandId::RUN_EXAMPLE, ctx))
                .then(|| map_action(MenuAction::RunExample)), palette)),
        Item::new(menu_button("Stop Job", CommandRegistry::ShortcutFor(CommandId::RUN_STOP),
            context.is_none_or(|ctx| CommandRegistry::IsAvailable(CommandId::RUN_STOP, ctx))
                .then(|| map_action(MenuAction::StopJob)), palette)),
    ];
    let run_root = Item::with_menu(root_button("Run", palette), sub_template(run_items));
    // --- Theme Menu ---
    let mut theme_items = Vec::new();
    for &theme_variant in FasciaTheme::ALL {
        let label = theme_variant.menu_label(theme_variant == active_theme);
        theme_items.push( Item::new( menu_button(
            label,
            None,
            Some( map_action( MenuAction::SelectTheme( theme_variant))),
            palette,
        )));
    }
    let theme_root = Item::with_menu(root_button("Theme", palette), sub_template(theme_items));
    // --- Help Menu ---
    let help_items = vec![Item::new(menu_action_button("About Fascia", MenuAction::About, map_action, palette)),
                          Item::new(menu_action_button("Documentation", MenuAction::Documentation, map_action, palette)),];
    let help_root = Item::with_menu(root_button("Help", palette), sub_template(help_items));
    let roots = vec![file_root, edit_root, view_root, run_root, theme_root, help_root];
    let menu_bar = MenuBar::new(roots);
    container(menu_bar).padding([2, 6])
                       .width(Length::Fill)
                       .style(move |_| FasciaStyle::menubar_container(palette))
                       .into()
}
