// bottom_tray.rs ----------------------------------------------------------------------------------
//! Collapsible bottom tray housing Output (diagnostics log & job status) and the Python Console.

use crate::fascia::theme::{FasciaStyle, ThemePalette};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length};

//-------------------------------------------------------------------------------------------------

/// Active tab in the workbench bottom tray.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab
{
    Output,
    PythonConsole,
}

impl BottomTab
{
    pub fn Title(&self) -> &'static str
    {
        match self {
            Self::Output => "📋 Output",
            Self::PythonConsole => "🐍 Python Console",
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Actions emitted by the bottom tray controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BottomTrayAction
{
    SelectTab(BottomTab),
    Clear,
    Close,
}

//-------------------------------------------------------------------------------------------------

/// Renders the bottom tray containing Output and Python Console tabs.
pub fn view_bottom_tray<'a, Message: 'static + Clone>(
    active_tab: BottomTab,
    output_element: Element<'a, Message>,
    console_element: Option<Element<'a, Message>>,
    error_count: u32,
    warning_count: u32,
    palette: ThemePalette,
    on_action: impl Fn(BottomTrayAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let output_is_active = active_tab == BottomTab::Output;
    let console_is_active = active_tab == BottomTab::PythonConsole;

    let mut output_label = "📋 Output".to_string();
    if error_count > 0 || warning_count > 0 {
        output_label = format!("📋 Output ({}E, {}W)", error_count, warning_count);
    }

    let output_btn = button(
        text(output_label)
            .size(11)
            .style(move |_| text::Style {
                color: Some(if output_is_active {
                    palette.accent
                } else if error_count > 0 {
                    iced::Color::from_rgb(0.9, 0.3, 0.3)
                } else {
                    palette.text_secondary
                }),
            }),
    )
    .padding([3, 8])
    .style(move |_, status| FasciaStyle::tree_row_button(palette, output_is_active, status))
    .on_press(on_action(BottomTrayAction::SelectTab(BottomTab::Output)));

    let console_btn = button(
        text("🐍 Python Console")
            .size(11)
            .style(move |_| text::Style {
                color: Some(if console_is_active {
                    palette.accent
                } else {
                    palette.text_secondary
                }),
            }),
    )
    .padding([3, 8])
    .style(move |_, status| FasciaStyle::tree_row_button(palette, console_is_active, status))
    .on_press(on_action(BottomTrayAction::SelectTab(BottomTab::PythonConsole)));

    let clear_btn = button(
        text("🗑 Clear")
            .size(11)
            .style(move |_| text::Style { color: Some(palette.text_muted) }),
    )
    .padding([2, 6])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(on_action(BottomTrayAction::Clear));

    let close_btn = button(
        text("✕")
            .size(11)
            .style(move |_| text::Style { color: Some(palette.text_muted) }),
    )
    .padding([2, 6])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(on_action(BottomTrayAction::Close));

    let header = row![
        output_btn,
        console_btn,
        Space::new().width(Length::Fill),
        clear_btn,
        close_btn,
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .padding([2, 6]);

    let content_area: Element<'a, Message> = match active_tab {
        BottomTab::Output => output_element,
        BottomTab::PythonConsole => {
            if let Some(c) = console_element {
                c
            } else {
                container(text("Python console not available").size(12))
                    .padding(8)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }
        }
    };

    let tray = column![
        container(header)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(palette.menubar_bg)),
                border: iced::Border {
                    color: palette.border_subtle,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            }),
        container(content_area)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| FasciaStyle::content_container(palette)),
    ]
    .width(Length::Fill)
    .height(Length::Fixed(200.0));

    return container(tray)
        .width(Length::Fill)
        .height(Length::Fixed(200.0))
        .into();
}
