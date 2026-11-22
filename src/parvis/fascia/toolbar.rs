// src/fascia/toolbar.rs
use crate::fascia::command::CommandId;
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use iced::widget::{Space, button, container, row, text};
use iced::{Alignment, Element, Length};

/// Actions emitted by the toolbar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolBarAction
{
    NewFile,
    NewSketch,
    OpenFile,
    SaveFile,
    Undo,
    Redo,
    ToggleSidebar,
    ToggleTheme,
    OpenSettings,
}

impl ToolBarAction
{
    /// Maps a legacy ToolBarAction to its corresponding unified CommandId.
    pub fn ToCommand(&self) -> CommandId
    {
        match self {
            Self::NewFile => CommandId::FILE_NEW,
            Self::NewSketch => CommandId::FILE_NEW_SKETCH,
            Self::OpenFile => CommandId::FILE_OPEN,
            Self::SaveFile => CommandId::FILE_SAVE,
            Self::Undo => CommandId::EDIT_UNDO,
            Self::Redo => CommandId::EDIT_REDO,
            Self::ToggleSidebar => CommandId::VIEW_TOGGLE_SIDEBAR,
            Self::ToggleTheme => CommandId::THEME_TOGGLE,
            Self::OpenSettings => CommandId::VIEW_OPEN_SETTINGS,
        }
    }
}
fn	toolbar_btn< 'a, Message: 'static + Clone>( 
    icon: &'static str, label: &'static str, action: Message, palette: ThemePalette,
) -> button::Button< 'a, Message> {
    button( 
        row![
            text( icon).size( 14),
            Space::new().width( 4),
            text( label).size( 12).style( move |_| text::Style {
                color: Some( palette.text_primary),
            }),
        ]
        .align_y( Alignment::Center),
    )
    .padding( [3, 8])
    .style( move |_, status| FasciaStyle::toolbar_button( palette, status))
    .on_press( action)
}
fn	separator< 'a, Message: 'static>( palette: ThemePalette) -> Element< 'a, Message> {
    container( Space::new().width( 1))
        .width( Length::Fixed( 1.0))
        .height( Length::Fixed( 18.0))
        .style( move |_| container::Style {
            background: Some( iced::Background::Color( palette.border_subtle)),
            ..Default::default()
        })
        .into()
}
/// Constructs the Fascia Toolbar.
pub fn view_toolbar<'a, Message: 'static + Clone>(palette: ThemePalette,
                                                  map_action: impl Fn(ToolBarAction) -> Message
                                                  + Copy
                                                  + 'static)
                                                  -> Element<'a, Message>
{
    let  	bar = row![
        toolbar_btn( "📄", "New", map_action( ToolBarAction::NewFile), palette),
        toolbar_btn( "✎", "New Sketch", map_action( ToolBarAction::NewSketch), palette),
        toolbar_btn( "📂", "Open", map_action( ToolBarAction::OpenFile), palette),
        toolbar_btn( "💾", "Save", map_action( ToolBarAction::SaveFile), palette),
        separator( palette),
        toolbar_btn( "↩", "Undo", map_action( ToolBarAction::Undo), palette),
        toolbar_btn( "↪", "Redo", map_action( ToolBarAction::Redo), palette),
        separator( palette),
        toolbar_btn( 
            "🗂",
            "Sidebar",
            map_action( ToolBarAction::ToggleSidebar),
            palette
        ),
        Space::new().width( Length::Fill),
        toolbar_btn( 
            "🎨",
            "Theme",
            map_action( ToolBarAction::ToggleTheme),
            palette
        ),
        toolbar_btn( 
            "⚙",
            "Settings",
            map_action( ToolBarAction::OpenSettings),
            palette
        ),
    ]
    .spacing( 4)
    .padding( [3, 8])
    .align_y( Alignment::Center);
    container( bar)
        .width( Length::Fill)
        .style( move |_| FasciaStyle::toolbar_container( palette))
        .into()
}

/// Constructs the compact Top Bar containing menus, Quick Open search trigger, and pane toggles.
pub fn view_top_bar<'a, Message: 'static + Clone>(
    menubar: Element<'a, Message>,
    palette: ThemePalette,
    show_sidebar: bool,
    show_bottom_tray: bool,
    show_properties: bool,
    on_command: impl Fn(CommandId) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let search_pill = button(
        row![
            text("🔍").size(12),
            Space::new().width(4),
            text("Search / Quick Open").size(11).style(move |_| text::Style {
                color: Some(palette.text_secondary),
            }),
            Space::new().width(12),
            text("Ctrl+P").size(10).style(move |_| text::Style {
                color: Some(palette.text_muted),
            }),
        ]
        .align_y(Alignment::Center),
    )
    .padding([2, 12])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(on_command(CommandId::VIEW_OPEN_SEARCH));

    let sidebar_icon = if show_sidebar { "◧" } else { "◻" };
    let sidebar_btn = button(
        row![text(sidebar_icon).size(13), Space::new().width(2), text("Sidebar").size(11)]
            .align_y(Alignment::Center),
    )
    .padding([2, 6])
    .style(move |_, status| FasciaStyle::tree_row_button(palette, show_sidebar, status))
    .on_press(on_command(CommandId::VIEW_TOGGLE_SIDEBAR));

    let bottom_icon = if show_bottom_tray { "⬓" } else { "◻" };
    let bottom_btn = button(
        row![text(bottom_icon).size(13), Space::new().width(2), text("Panel").size(11)]
            .align_y(Alignment::Center),
    )
    .padding([2, 6])
    .style(move |_, status| FasciaStyle::tree_row_button(palette, show_bottom_tray, status))
    .on_press(on_command(CommandId::VIEW_TOGGLE_BOTTOM_PANEL));

    let prop_icon = if show_properties { "◨" } else { "◻" };
    let prop_btn = button(
        row![text(prop_icon).size(13), Space::new().width(2), text("Properties").size(11)]
            .align_y(Alignment::Center),
    )
    .padding([2, 6])
    .style(move |_, status| FasciaStyle::tree_row_button(palette, show_properties, status))
    .on_press(on_command(CommandId::VIEW_TOGGLE_PROPERTIES));

    let settings_btn = button(text("⚙").size(13))
        .padding([2, 6])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(on_command(CommandId::VIEW_OPEN_SETTINGS));

    let bar = row![
        menubar,
        Space::new().width(Length::Fill),
        search_pill,
        Space::new().width(Length::Fill),
        row![sidebar_btn, bottom_btn, prop_btn, settings_btn]
            .spacing(2)
            .align_y(Alignment::Center),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding([1, 6]);

    return container(bar)
        .width(Length::Fill)
        .style(move |_| FasciaStyle::menubar_container(palette))
        .into();
}
