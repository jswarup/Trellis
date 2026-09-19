// src/fascia/activity_bar.rs
use	crate::fascia::theme::{ FasciaStyle, ThemePalette };
use	iced::widget::{ Space, button, column, container, text };
use	iced::{ Alignment, Element, Length };
/// Side panel tabs in the activity bar.
#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityTab {
    Explorer,
    Search,
    Settings,
}
impl ActivityTab
{
    pub fn	icon( &self) -> &'static str {
        match self {
            Self::Explorer => "📁",
            Self::Search => "🔍",
            Self::Settings => "⚙",
        }
    }
    pub fn	tooltip( &self) -> &'static str {
        match self {
            Self::Explorer => "Explorer (Ctrl+Shift+E)",
            Self::Search => "Search (Ctrl+Shift+F)",
            Self::Settings => "Settings (Ctrl+,)",
        }
    }
}
fn	tab_button< 'a, Message: 'static + Clone>( 
    tab: ActivityTab,
    is_active: bool,
    action: Message,
    palette: ThemePalette,
) -> button::Button< 'a, Message> {
    button( 
        container( text( tab.icon()).size( 18))
            .width( Length::Fill)
            .align_x( Alignment::Center),
    )
    .width( Length::Fixed( 48.0))
    .height( Length::Fixed( 46.0))
    .style( move |_, status| FasciaStyle::activity_bar_button( palette, is_active, status))
    .on_press( action)
}
/// Constructs the vertical Activity Bar widget.
pub fn	view_activity_bar< 'a, Message: 'static + Clone>( 
    active_tab: Option< ActivityTab>,
    palette: ThemePalette,
    on_select_tab: impl Fn( ActivityTab) -> Message + Copy + 'static,
) -> Element< 'a, Message> {
    let  	top_tabs = column![
        tab_button( 
            ActivityTab::Explorer,
            active_tab == Some( ActivityTab::Explorer),
            on_select_tab( ActivityTab::Explorer),
            palette,
        ),
        tab_button( 
            ActivityTab::Search,
            active_tab == Some( ActivityTab::Search),
            on_select_tab( ActivityTab::Search),
            palette,
        ),
    ]
    .spacing( 2);
    let  	bottom_tabs = column![tab_button( 
        ActivityTab::Settings,
        active_tab == Some( ActivityTab::Settings),
        on_select_tab( ActivityTab::Settings),
        palette,
    )];
    let  	bar = column![top_tabs, Space::new().height( Length::Fill), bottom_tabs]
        .width( Length::Fixed( 48.0))
        .height( Length::Fill)
        .align_x( Alignment::Center);
    container( bar)
        .width( Length::Fixed( 48.0))
        .height( Length::Fill)
        .style( move |_| FasciaStyle::activity_bar_container( palette))
        .into()
}
