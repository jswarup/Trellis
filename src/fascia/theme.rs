// src/fascia/theme.rs
use iced::widget::{button, container};
use iced::{Background, Border, Color, Font, Shadow};

//-------------------------------------------------------------------------------------------------
/// Predefined native visual themes supported by Fascia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FasciaTheme
{
    WindowsDark,
    WindowsLight,
    LinuxDark,
    LinuxLight,
    VsCodeDark,
}
impl Default for FasciaTheme
{
    fn default() -> Self { Self::native_default() }
}
impl FasciaTheme
{
    /// Returns the native default theme for the current operating system.
    pub fn native_default() -> Self
    {
        #[cfg(target_os = "windows")]
        {
            Self::WindowsLight
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::LinuxLight
        }
    }
    pub const ALL: &'static [FasciaTheme] = &[FasciaTheme::WindowsDark,
                                              FasciaTheme::WindowsLight,
                                              FasciaTheme::LinuxDark,
                                              FasciaTheme::LinuxLight,
                                              FasciaTheme::VsCodeDark];
    pub fn display_name(&self) -> &'static str
    {
        match self {
            Self::WindowsDark => "Windows 11 Dark (Fluent)",
            Self::WindowsLight => "Windows 11 Light (Fluent)",
            Self::LinuxDark => "Linux Dark (GNOME Adwaita)",
            Self::LinuxLight => "Linux Light (GNOME Adwaita)",
            Self::VsCodeDark => "VS Code Dark",
        }
    }
    pub fn is_dark(&self) -> bool
    {
        match self {
            Self::WindowsDark | Self::LinuxDark | Self::VsCodeDark => true,
            Self::WindowsLight | Self::LinuxLight => false,
        }
    }
    pub fn palette(&self) -> ThemePalette
    {
        match self {
            Self::WindowsDark => {
                ThemePalette { app_bg:                   Color::from_rgb8(32, 32, 32),
                               menubar_bg:               Color::from_rgb8(32, 32, 32),
                               toolbar_bg:               Color::from_rgb8(37, 37, 37),
                               activity_bar_bg:          Color::from_rgb8(26, 26, 26),
                               activity_bar_item_active: Color::from_rgb8(45, 45, 45),
                               sidebar_bg:               Color::from_rgb8(40, 40, 40),
                               content_bg:               Color::from_rgb8(30, 30, 30),
                               tab_bar_bg:               Color::from_rgb8(35, 35, 35),
                               tab_active_bg:            Color::from_rgb8(30, 30, 30),
                               tab_inactive_bg:          Color::from_rgb8(42, 42, 42),
                               status_bar_bg:            Color::from_rgb8(0, 120, 212),
                               border:                   Color::from_rgb8(58, 58, 58),
                               border_subtle:            Color::from_rgb8(48, 48, 48),
                               accent:                   Color::from_rgb8(0, 120, 212),
                               accent_hover:             Color::from_rgb8(96, 205, 255),
                               text_primary:             Color::from_rgb8(245, 245, 245),
                               text_secondary:           Color::from_rgb8(180, 180, 180),
                               text_muted:               Color::from_rgb8(120, 120, 120),
                               hover_bg:                 Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                               selected_bg:              Color::from_rgba(0.0, 0.47, 0.83, 0.28),
                               corner_radius:            5.0, }
            }
            Self::WindowsLight => {
                ThemePalette { app_bg:                   Color::from_rgb8(243, 243, 243),
                               menubar_bg:               Color::from_rgb8(243, 243, 243),
                               toolbar_bg:               Color::from_rgb8(248, 248, 248),
                               activity_bar_bg:          Color::from_rgb8(232, 232, 232),
                               activity_bar_item_active: Color::from_rgb8(255, 255, 255),
                               sidebar_bg:               Color::from_rgb8(248, 248, 248),
                               content_bg:               Color::from_rgb8(255, 255, 255),
                               tab_bar_bg:               Color::from_rgb8(238, 238, 238),
                               tab_active_bg:            Color::from_rgb8(255, 255, 255),
                               tab_inactive_bg:          Color::from_rgb8(235, 235, 235),
                               status_bar_bg:            Color::from_rgb8(0, 95, 184),
                               border:                   Color::from_rgb8(220, 220, 220),
                               border_subtle:            Color::from_rgb8(230, 230, 230),
                               accent:                   Color::from_rgb8(0, 95, 184),
                               accent_hover:             Color::from_rgb8(0, 120, 212),
                               text_primary:             Color::from_rgb8(26, 26, 26),
                               text_secondary:           Color::from_rgb8(80, 80, 80),
                               text_muted:               Color::from_rgb8(140, 140, 140),
                               hover_bg:                 Color::from_rgba(0.0, 0.0, 0.0, 0.05),
                               selected_bg:              Color::from_rgba(0.0, 0.37, 0.72, 0.15),
                               corner_radius:            5.0, }
            }
            Self::LinuxDark => {
                ThemePalette { app_bg:                   Color::from_rgb8(36, 36, 36),
                               menubar_bg:               Color::from_rgb8(36, 36, 36),
                               toolbar_bg:               Color::from_rgb8(42, 42, 42),
                               activity_bar_bg:          Color::from_rgb8(30, 30, 30),
                               activity_bar_item_active: Color::from_rgb8(50, 50, 50),
                               sidebar_bg:               Color::from_rgb8(48, 48, 48),
                               content_bg:               Color::from_rgb8(30, 30, 30),
                               tab_bar_bg:               Color::from_rgb8(38, 38, 38),
                               tab_active_bg:            Color::from_rgb8(30, 30, 30),
                               tab_inactive_bg:          Color::from_rgb8(44, 44, 44),
                               status_bar_bg:            Color::from_rgb8(53, 132, 228),
                               border:                   Color::from_rgb8(60, 60, 60),
                               border_subtle:            Color::from_rgb8(48, 48, 48),
                               accent:                   Color::from_rgb8(53, 132, 228),
                               accent_hover:             Color::from_rgb8(98, 160, 234),
                               text_primary:             Color::from_rgb8(255, 255, 255),
                               text_secondary:           Color::from_rgb8(170, 170, 170),
                               text_muted:               Color::from_rgb8(120, 120, 120),
                               hover_bg:                 Color::from_rgba(1.0, 1.0, 1.0, 0.07),
                               selected_bg:              Color::from_rgba(0.21, 0.52, 0.89, 0.3),
                               corner_radius:            6.0, }
            }
            Self::LinuxLight => {
                ThemePalette { app_bg:                   Color::from_rgb8(250, 250, 250),
                               menubar_bg:               Color::from_rgb8(246, 246, 246),
                               toolbar_bg:               Color::from_rgb8(240, 240, 240),
                               activity_bar_bg:          Color::from_rgb8(235, 235, 235),
                               activity_bar_item_active: Color::from_rgb8(255, 255, 255),
                               sidebar_bg:               Color::from_rgb8(242, 242, 242),
                               content_bg:               Color::from_rgb8(255, 255, 255),
                               tab_bar_bg:               Color::from_rgb8(235, 235, 235),
                               tab_active_bg:            Color::from_rgb8(255, 255, 255),
                               tab_inactive_bg:          Color::from_rgb8(240, 240, 240),
                               status_bar_bg:            Color::from_rgb8(28, 113, 216),
                               border:                   Color::from_rgb8(210, 210, 210),
                               border_subtle:            Color::from_rgb8(225, 225, 225),
                               accent:                   Color::from_rgb8(28, 113, 216),
                               accent_hover:             Color::from_rgb8(53, 132, 228),
                               text_primary:             Color::from_rgb8(30, 30, 30),
                               text_secondary:           Color::from_rgb8(90, 90, 90),
                               text_muted:               Color::from_rgb8(140, 140, 140),
                               hover_bg:                 Color::from_rgba(0.0, 0.0, 0.0, 0.04),
                               selected_bg:              Color::from_rgba(0.11, 0.44, 0.85, 0.15),
                               corner_radius:            6.0, }
            }
            Self::VsCodeDark => {
                ThemePalette { app_bg:                   Color::from_rgb8(30, 30, 30),
                               menubar_bg:               Color::from_rgb8(60, 60, 60),
                               toolbar_bg:               Color::from_rgb8(50, 50, 50),
                               activity_bar_bg:          Color::from_rgb8(51, 51, 51),
                               activity_bar_item_active: Color::from_rgb8(37, 37, 38),
                               sidebar_bg:               Color::from_rgb8(37, 37, 38),
                               content_bg:               Color::from_rgb8(30, 30, 30),
                               tab_bar_bg:               Color::from_rgb8(37, 37, 38),
                               tab_active_bg:            Color::from_rgb8(30, 30, 30),
                               tab_inactive_bg:          Color::from_rgb8(45, 45, 45),
                               status_bar_bg:            Color::from_rgb8(0, 122, 204),
                               border:                   Color::from_rgb8(69, 69, 69),
                               border_subtle:            Color::from_rgb8(55, 55, 55),
                               accent:                   Color::from_rgb8(0, 122, 204),
                               accent_hover:             Color::from_rgb8(28, 151, 234),
                               text_primary:             Color::from_rgb8(204, 204, 204),
                               text_secondary:           Color::from_rgb8(150, 150, 150),
                               text_muted:               Color::from_rgb8(110, 110, 110),
                               hover_bg:                 Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                               selected_bg:              Color::from_rgba(0.0, 0.48, 0.8, 0.25),
                               corner_radius:            0.0, }
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------
/// Color palette properties for styling Fascia components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemePalette
{
    pub app_bg:                   Color,
    pub menubar_bg:               Color,
    pub toolbar_bg:               Color,
    pub activity_bar_bg:          Color,
    pub activity_bar_item_active: Color,
    pub sidebar_bg:               Color,
    pub content_bg:               Color,
    pub tab_bar_bg:               Color,
    pub tab_active_bg:            Color,
    pub tab_inactive_bg:          Color,
    pub status_bar_bg:            Color,
    pub border:                   Color,
    pub border_subtle:            Color,
    pub accent:                   Color,
    pub accent_hover:             Color,
    pub text_primary:             Color,
    pub text_secondary:           Color,
    pub text_muted:               Color,
    pub hover_bg:                 Color,
    pub selected_bg:              Color,
    pub corner_radius:            f32,
}

//-------------------------------------------------------------------------------------------------
/// Styling helpers for Fascia UI elements.
pub struct FasciaStyle;
impl FasciaStyle
{
    pub fn menubar_container(palette: ThemePalette) -> container::Style
    {
        container::Style { background: Some(Background::Color(palette.menubar_bg)),
                           text_color: Some(palette.text_primary),
                           border:     Border { color:  palette.border_subtle,
                                                width:  0.0,
                                                radius: 0.0.into(), },
                           shadow:     Shadow::default(),
                           snap:       true, }
    }
    pub fn toolbar_container(palette: ThemePalette) -> container::Style
    {
        container::Style { background: Some(Background::Color(palette.toolbar_bg)),
                           text_color: Some(palette.text_primary),
                           border:     Border { color:  palette.border_subtle,
                                                width:  1.0,
                                                radius: 0.0.into(), },
                           shadow:     Shadow::default(),
                           snap:       true, }
    }
    pub fn activity_bar_container(palette: ThemePalette) -> container::Style
    {
        container::Style { background: Some(Background::Color(palette.activity_bar_bg)),
                           text_color: Some(palette.text_secondary),
                           border:     Border { color:  palette.border_subtle,
                                                width:  1.0,
                                                radius: 0.0.into(), },
                           shadow:     Shadow::default(),
                           snap:       true, }
    }
    pub fn sidebar_container(palette: ThemePalette) -> container::Style
    {
        container::Style { background: Some(Background::Color(palette.sidebar_bg)),
                           text_color: Some(palette.text_primary),
                           border:     Border { color:  palette.border_subtle,
                                                width:  1.0,
                                                radius: 0.0.into(), },
                           shadow:     Shadow::default(),
                           snap:       true, }
    }
    pub fn tab_bar_container(palette: ThemePalette) -> container::Style
    {
        container::Style { background: Some(Background::Color(palette.tab_bar_bg)),
                           text_color: Some(palette.text_primary),
                           border:     Border { color:  palette.border_subtle,
                                                width:  1.0,
                                                radius: 0.0.into(), },
                           shadow:     Shadow::default(),
                           snap:       true, }
    }
    pub fn content_container(palette: ThemePalette) -> container::Style
    {
        container::Style { background: Some(Background::Color(palette.content_bg)),
                           text_color: Some(palette.text_primary),
                           border:     Border::default(),
                           shadow:     Shadow::default(),
                           snap:       true, }
    }
    pub fn status_bar_container(palette: ThemePalette) -> container::Style
    {
        container::Style { background: Some(Background::Color(palette.status_bar_bg)),
                           text_color: Some(Color::WHITE),
                           border:     Border::default(),
                           shadow:     Shadow::default(),
                           snap:       true, }
    }
    pub fn activity_bar_button(palette: ThemePalette, is_active: bool, status: button::Status)
                               -> button::Style
    {
        let bg = if is_active {
            Some(Background::Color(palette.activity_bar_item_active))
        } else {
            match status {
                button::Status::Hovered => Some(Background::Color(palette.hover_bg)),
                button::Status::Pressed => Some(Background::Color(palette.selected_bg)),
                _ => None,
            }
        };
        button::Style { background: bg,
                        text_color: if is_active {
                            palette.accent
                        } else {
                            palette.text_secondary
                        },
                        border:     Border { color:  if is_active {
                                                 palette.accent
                                             } else {
                                                 Color::TRANSPARENT
                                             },
                                             width:  if is_active { 2.0 } else { 0.0 },
                                             radius: palette.corner_radius.into(), },
                        shadow:     Shadow::default(),
                        snap:       true, }
    }
    pub fn toolbar_button(palette: ThemePalette, status: button::Status) -> button::Style
    {
        let bg = match status {
            button::Status::Hovered => Some(Background::Color(palette.hover_bg)),
            button::Status::Pressed => Some(Background::Color(palette.selected_bg)),
            _ => None,
        };
        button::Style { background: bg,
                        text_color: palette.text_primary,
                        border:     Border { color:  Color::TRANSPARENT,
                                             width:  0.0,
                                             radius: palette.corner_radius.into(), },
                        shadow:     Shadow::default(),
                        snap:       true, }
    }
    pub fn menu_item_button(palette: ThemePalette, status: button::Status) -> button::Style
    {
        let bg = match status {
            button::Status::Hovered => Some(Background::Color(palette.hover_bg)),
            button::Status::Pressed => Some(Background::Color(palette.selected_bg)),
            _ => None,
        };
        button::Style { background: bg,
                        text_color: palette.text_primary,
                        border:     Border { color:  Color::TRANSPARENT,
                                             width:  0.0,
                                             radius: 3.0.into(), },
                        shadow:     Shadow::default(),
                        snap:       true, }
    }
    pub fn tab_button(palette: ThemePalette, is_active: bool, status: button::Status)
                      -> button::Style
    {
        let bg = if is_active {
            palette.tab_active_bg
        } else {
            match status {
                button::Status::Hovered => palette.hover_bg,
                button::Status::Pressed => palette.selected_bg,
                _ => palette.tab_inactive_bg,
            }
        };
        button::Style { background: Some(Background::Color(bg)),
                        text_color: if is_active {
                            palette.text_primary
                        } else {
                            palette.text_secondary
                        },
                        border:     Border { color:  if is_active {
                                                 palette.accent
                                             } else {
                                                 palette.border_subtle
                                             },
                                             width:  if is_active { 1.0 } else { 0.5 },
                                             radius: 3.0.into(), },
                        shadow:     Shadow::default(),
                        snap:       true, }
    }
    pub fn tab_close_button(palette: ThemePalette, status: button::Status) -> button::Style
    {
        let bg = match status {
            button::Status::Hovered => Some(Background::Color(palette.hover_bg)),
            button::Status::Pressed => Some(Background::Color(palette.selected_bg)),
            _ => None,
        };
        button::Style { background: bg,
                        text_color: palette.text_secondary,
                        border:     Border::default(),
                        shadow:     Shadow::default(),
                        snap:       true, }
    }
    pub fn tree_row_button(palette: ThemePalette, is_selected: bool, status: button::Status)
                           -> button::Style
    {
        let bg = if is_selected {
            Some(Background::Color(palette.selected_bg))
        } else {
            match status {
                button::Status::Hovered => Some(Background::Color(palette.hover_bg)),
                button::Status::Pressed => Some(Background::Color(palette.selected_bg)),
                _ => None,
            }
        };
        button::Style { background: bg,
                        text_color: if is_selected {
                            palette.text_primary
                        } else {
                            palette.text_secondary
                        },
                        border:     Border::default(),
                        shadow:     Shadow::default(),
                        snap:       true, }
    }
}

//-------------------------------------------------------------------------------------------------
/// Returns the primary native font family for the current OS.
pub fn default_system_font(theme: FasciaTheme) -> Font
{
    match theme {
        FasciaTheme::WindowsDark | FasciaTheme::WindowsLight => {
            Font { family:  iced::font::Family::Name("Segoe UI"),
                   weight:  iced::font::Weight::Normal,
                   stretch: iced::font::Stretch::Normal,
                   style:   iced::font::Style::Normal, }
        }
        FasciaTheme::LinuxDark | FasciaTheme::LinuxLight => {
            Font { family:  iced::font::Family::Name("Inter"),
                   weight:  iced::font::Weight::Normal,
                   stretch: iced::font::Stretch::Normal,
                   style:   iced::font::Style::Normal, }
        }
        FasciaTheme::VsCodeDark => {
            Font { family:  iced::font::Family::SansSerif,
                   weight:  iced::font::Weight::Normal,
                   stretch: iced::font::Stretch::Normal,
                   style:   iced::font::Style::Normal, }
        }
    }
}
/// Returns a monospace font for code editors and line numbers.
pub fn default_code_font(theme: FasciaTheme) -> Font
{
    match theme {
        FasciaTheme::WindowsDark | FasciaTheme::WindowsLight => {
            Font { family:  iced::font::Family::Name("Cascadia Code"),
                   weight:  iced::font::Weight::Normal,
                   stretch: iced::font::Stretch::Normal,
                   style:   iced::font::Style::Normal, }
        }
        FasciaTheme::LinuxDark | FasciaTheme::LinuxLight => {
            Font { family:  iced::font::Family::Name("Fira Code"),
                   weight:  iced::font::Weight::Normal,
                   stretch: iced::font::Stretch::Normal,
                   style:   iced::font::Style::Normal, }
        }
        FasciaTheme::VsCodeDark => {
            Font { family:  iced::font::Family::Monospace,
                   weight:  iced::font::Weight::Normal,
                   stretch: iced::font::Stretch::Normal,
                   style:   iced::font::Style::Normal, }
        }
    }
}
