// src/fascia/toolbar.rs
use	crate::fascia::theme::{ FasciaStyle, ThemePalette };
use	iced::widget::{ Space, button, container, row, text };
use	iced::{ Alignment, Element, Length };
/// Actions emitted by the toolbar.
#[derive( Debug, Clone, PartialEq, Eq)]
pub enum ToolBarAction {
    NewFile,
    OpenFile,
    SaveFile,
    Undo,
    Redo,
    ToggleSidebar,
    ToggleTheme,
    OpenSettings,
}
fn	toolbar_btn< 'a, Message: 'static + Clone>( 
    icon: &'static str,
    label: &'static str,
    action: Message,
    palette: ThemePalette,
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
pub fn	view_toolbar< 'a, Message: 'static + Clone>( 
    palette: ThemePalette,
    map_action: impl Fn( ToolBarAction) -> Message + Copy + 'static,
) -> Element< 'a, Message> {
    let  	bar = row![
        toolbar_btn( "📄", "New", map_action( ToolBarAction::NewFile), palette),
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
