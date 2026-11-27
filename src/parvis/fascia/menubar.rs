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
    OpenExplorer,
    OpenExamplesWorkspace,
    OpenSearch,
    QuickOpen,
    CommandPalette,
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
            Self::OpenExplorer => Some(CommandId::VIEW_OPEN_EXPLORER),
            Self::OpenExamplesWorkspace => Some(CommandId::VIEW_OPEN_EXAMPLES),
            Self::OpenSearch => Some(CommandId::VIEW_OPEN_SEARCH),
            Self::QuickOpen => Some(CommandId::VIEW_QUICK_OPEN),
            Self::CommandPalette => Some(CommandId::VIEW_COMMAND_PALETTE),
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
                                                    context: Option<&CommandContext>,
                                                    map_action: impl Fn(MenuAction) -> Message + Copy + 'static,
                                                    palette: ThemePalette)
                                                    -> button::Button<'a, Message>
{
    let cmd = action.ToCommand();
    let shortcut = cmd.and_then(CommandRegistry::ShortcutFor);
    let enabled = match (cmd, context) {
        (Some(id), Some(ctx)) => CommandRegistry::IsAvailable(id, ctx),
        _ => true,
    };
    return menu_button(label, shortcut, enabled.then(|| map_action(action)), palette);
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
                                                  _active_theme: FasciaTheme,
                                                  _explorer_views: (bool, bool, bool),
                                                  map_action: impl Fn(MenuAction) -> Message
                                                  + Copy
                                                  + 'static)
                                                  -> Element<'a, Message>
{
    return ViewMenubarWithContext(palette, None, map_action);
}

pub(super) fn ViewMenubarWithContext<'a, Message: 'static + Clone>(
    palette: ThemePalette,
    context: Option<&CommandContext>, map_action: impl Fn(MenuAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let menu = |title, actions: &[(&'static str, MenuAction)]| {
        let items = actions.iter().map(|(label, action)| {
            Item::new(menu_action_button(label, action.clone(), context, map_action, palette))
        }).collect();
        Item::with_menu(root_button(title, palette), Menu::new(items).max_width(220.0).offset(2.0))
    };
    let roots = vec![
        menu("File", &[
            ("New file", MenuAction::NewFile),
            ("New sketch", MenuAction::NewSketch),
            ("New math view", MenuAction::NewMathView),
            ("Browse files", MenuAction::OpenExplorer),
            ("Save", MenuAction::SaveFile),
            ("Close tab", MenuAction::CloseTab),
            ("Exit", MenuAction::Exit),
        ]),
        menu("View", &[
            ("Command palette", MenuAction::CommandPalette),
            ("Examples", MenuAction::OpenExamplesWorkspace),
            ("Python console", MenuAction::OpenPythonConsole),
            ("Output", MenuAction::OpenOutput),
            ("Settings", MenuAction::OpenSettings),
        ]),
    ];
    let menu_bar = MenuBar::new(roots);
    container(menu_bar).padding([2, 6])
                       .width(Length::Fill)
                       .style(move |_| FasciaStyle::menubar_container(palette))
                       .into()
}
