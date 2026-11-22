// src/fascia/shell.rs
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use iced::widget::{column, container, row};
use iced::{Element, Length};

/// Builds the complete workbench shell layout containing Top Bar (or Menubar + Toolbar),
/// Activity Bar, Sidebar, Content Area, Bottom Tray, Properties Panel, and Status Bar.
#[allow(clippy::too_many_arguments)]
pub fn view_shell<'a, Message: 'static>(
    menubar: Element<'a, Message>,
    toolbar: Option<Element<'a, Message>>,
    activity_bar: Element<'a, Message>,
    sidebar: Option<Element<'a, Message>>,
    content: Element<'a, Message>,
    bottom_tray: Option<Element<'a, Message>>,
    properties: Option<Element<'a, Message>>,
    status_bar: Option<Element<'a, Message>>,
    palette: ThemePalette,
) -> Element<'a, Message>
{
    let mut center_col = column![content].width(Length::Fill).height(Length::Fill);
    if let Some(tray) = bottom_tray {
        center_col = center_col.push(tray);
    }

    let mut middle_row = row![activity_bar].height(Length::Fill);
    if let Some(sb) = sidebar {
        middle_row = middle_row.push(sb);
    }
    middle_row = middle_row.push(
        container(center_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| FasciaStyle::content_container(palette)),
    );
    if let Some(props) = properties {
        middle_row = middle_row.push(props);
    }

    let mut layout = column![menubar];
    if let Some(tb) = toolbar {
        layout = layout.push(tb);
    }
    layout = layout.push(middle_row);
    if let Some(sb) = status_bar {
        layout = layout.push(sb);
    }
    return container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
}
