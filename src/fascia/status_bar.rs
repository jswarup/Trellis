// src/fascia/status_bar.rs
use crate::fascia::theme::{FasciaStyle, FasciaTheme, ThemePalette};
use iced::widget::{Space, container, row, text};
use iced::{Alignment, Element, Length};
/// Information displayed in the status bar.
#[derive(Debug, Clone)]
pub struct StatusBarInfo {
    pub branch: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub encoding: &'static str,
    pub indentation: &'static str,
    pub language: String,
}
impl Default for StatusBarInfo {
    fn default() -> Self {
        Self {
            branch: "main".to_string(),
            message: "Ready".to_string(),
            line: 1,
            column: 1,
            encoding: "UTF-8",
            indentation: "Spaces: 4",
            language: "Plain Text".to_string(),
        }
    }
}
/// Constructs the bottom Status Bar.
pub fn view_status_bar<'a, Message: 'static>(
    info: &'a StatusBarInfo,
    theme: FasciaTheme,
    palette: ThemePalette,
) -> Element<'a, Message> {
    let os_badge = if cfg!(target_os = "windows") {
        "🪟 Windows"
    } else if cfg!(target_os = "linux") {
        "🐧 Linux"
    } else {
        "💻 Desktop"
    };
    let left = row![
        text(format!("⎇ {}", info.branch)).size(11),
        Space::new().width(Length::Fixed(12.0)),
        text(&info.message).size(11),
    ]
    .align_y(Alignment::Center);
    let right = row![
        text(format!("Ln {}, Col {}", info.line, info.column)).size(11),
        Space::new().width(Length::Fixed(12.0)),
        text(info.indentation).size(11),
        Space::new().width(Length::Fixed(12.0)),
        text(info.encoding).size(11),
        Space::new().width(Length::Fixed(12.0)),
        text(&info.language).size(11),
        Space::new().width(Length::Fixed(12.0)),
        text(os_badge).size(11),
        Space::new().width(Length::Fixed(8.0)),
        text(format!("🎨 {}", theme.display_name())).size(11),
    ]
    .align_y(Alignment::Center);
    let bar = row![left, Space::new().width(Length::Fill), right]
        .align_y(Alignment::Center)
        .padding([2, 10]);
    container(bar)
        .width(Length::Fill)
        .style(move |_| FasciaStyle::status_bar_container(palette))
        .into()
}
