// src/fascia/toolbar.rs
use crate::fascia::command::CommandId;
use crate::fascia::icons::{IconId, ViewIcon};
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
fn toolbar_icon_btn<'a, Message: 'static + Clone>(
    icon: IconId,
    label: &'static str,
    action: Option<Message>,
    palette: ThemePalette,
) -> button::Button<'a, Message>
{
    let fg = if action.is_some() { palette.text_primary } else { palette.text_muted };
    let mut btn = button(
        row![
            ViewIcon(icon, 13.0, fg),
            Space::new().width(5),
            text(label).size(11).style(move |_| text::Style { color: Some(fg) }),
        ]
        .align_y(Alignment::Center),
    )
    .padding([3, 8])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status));
    if let Some(msg) = action {
        btn = btn.on_press(msg);
    }
    return btn;
}

fn separator<'a, Message: 'static>(palette: ThemePalette) -> Element<'a, Message>
{
    container(Space::new().width(1))
        .width(Length::Fixed(1.0))
        .height(Length::Fixed(18.0))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(palette.border_subtle)),
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
    let bar = row![
        toolbar_icon_btn(IconId::Plus, "New", Some(map_action(ToolBarAction::NewFile)), palette),
        toolbar_icon_btn(IconId::Sketch, "New Sketch", Some(map_action(ToolBarAction::NewSketch)), palette),
        toolbar_icon_btn(IconId::FolderOpen, "Open", Some(map_action(ToolBarAction::OpenFile)), palette),
        toolbar_icon_btn(IconId::Save, "Save", Some(map_action(ToolBarAction::SaveFile)), palette),
        separator(palette),
        toolbar_icon_btn(IconId::Undo, "Undo", Some(map_action(ToolBarAction::Undo)), palette),
        toolbar_icon_btn(IconId::Redo, "Redo", Some(map_action(ToolBarAction::Redo)), palette),
        separator(palette),
        toolbar_icon_btn(
            IconId::SidebarLeft,
            "Sidebar",
            Some(map_action(ToolBarAction::ToggleSidebar)),
            palette,
        ),
        Space::new().width(Length::Fill),
        toolbar_icon_btn(
            IconId::Palette,
            "Theme",
            Some(map_action(ToolBarAction::ToggleTheme)),
            palette,
        ),
        toolbar_icon_btn(
            IconId::Sliders,
            "Settings",
            Some(map_action(ToolBarAction::OpenSettings)),
            palette,
        ),
    ]
    .spacing(4)
    .padding([3, 8])
    .align_y(Alignment::Center);
    return container(bar)
        .width(Length::Fill)
        .style(move |_| FasciaStyle::toolbar_container(palette))
        .into();
}

/// Active document context driving the contextual toolbar strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveToolContext
{
    None,
    Editor { can_save: bool },
    Sketch { can_undo: bool, can_redo: bool, can_save: bool },
    Fresco,
    Other,
}

/// Constructs the contextual workbench toolbar adapted to the active tab and Heist job state.
pub fn view_contextual_toolbar<'a, Message: 'static + Clone>(
    ctx: ActiveToolContext,
    active_jobs: u32,
    can_run_example: bool,
    palette: ThemePalette,
    on_command: impl Fn(CommandId) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let mut bar = row![
        toolbar_icon_btn(IconId::Plus, "New File", Some(on_command(CommandId::FILE_NEW)), palette),
        toolbar_icon_btn(IconId::Sketch, "New Sketch", Some(on_command(CommandId::FILE_NEW_SKETCH)), palette),
        toolbar_icon_btn(IconId::MathFx, "New Math", Some(on_command(CommandId::FILE_NEW_MATH)), palette),
        separator(palette),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    match ctx {
        ActiveToolContext::Editor { can_save } => {
            bar = bar
                .push(toolbar_icon_btn(
                    IconId::Save,
                    "Save",
                    can_save.then(|| on_command(CommandId::FILE_SAVE)),
                    palette,
                ))
                .push(toolbar_icon_btn(
                    IconId::Close,
                    "Close",
                    Some(on_command(CommandId::TAB_CLOSE)),
                    palette,
                ));
        }
        ActiveToolContext::Sketch { can_undo, can_redo, can_save } => {
            bar = bar
                .push(toolbar_icon_btn(
                    IconId::Undo,
                    "Undo",
                    can_undo.then(|| on_command(CommandId::EDIT_UNDO)),
                    palette,
                ))
                .push(toolbar_icon_btn(
                    IconId::Redo,
                    "Redo",
                    can_redo.then(|| on_command(CommandId::EDIT_REDO)),
                    palette,
                ))
                .push(toolbar_icon_btn(
                    IconId::Save,
                    "Save",
                    can_save.then(|| on_command(CommandId::FILE_SAVE)),
                    palette,
                ));
        }
        ActiveToolContext::Fresco => {
            bar = bar
                .push(toolbar_icon_btn(
                    IconId::MathFx,
                    "Plot 2D",
                    Some(on_command(CommandId::MATH_PLOT_2D)),
                    palette,
                ))
                .push(toolbar_icon_btn(
                    IconId::Mesh3d,
                    "Plot 3D",
                    Some(on_command(CommandId::MATH_PLOT_3D)),
                    palette,
                ))
                .push(toolbar_icon_btn(
                    IconId::FileText,
                    "Export Text",
                    Some(on_command(CommandId::MATH_EXPORT_TEXT)),
                    palette,
                ))
                .push(toolbar_icon_btn(
                    IconId::Copy,
                    "Export SVG",
                    Some(on_command(CommandId::MATH_EXPORT_SVG)),
                    palette,
                ));
        }
        ActiveToolContext::None | ActiveToolContext::Other => {
            bar = bar.push(toolbar_icon_btn(
                IconId::FolderOpen,
                "Open File",
                Some(on_command(CommandId::FILE_OPEN)),
                palette,
            ));
        }
    }

    bar = bar.push(Space::new().width(Length::Fill));

    if active_jobs > 0 {
        let stop_btn = button(
            row![
                ViewIcon(IconId::Stop, 11.0, palette.status_error()),
                Space::new().width(5),
                text(format!("Stop ({} active)", active_jobs)).size(11).style(move |_| text::Style {
                    color: Some(palette.status_error()),
                }),
            ]
            .align_y(Alignment::Center),
        )
        .padding([3, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(on_command(CommandId::RUN_STOP));
        bar = bar.push(stop_btn);
    }

    bar = bar
        .push(toolbar_icon_btn(
            IconId::Play,
            "Run Example",
            can_run_example.then(|| on_command(CommandId::RUN_EXAMPLE)),
            palette,
        ))
        .push(toolbar_icon_btn(
            IconId::Terminal,
            "Python",
            Some(on_command(CommandId::VIEW_OPEN_PYTHON_CONSOLE)),
            palette,
        ))
        .push(toolbar_icon_btn(
            IconId::OutputLog,
            "Output",
            Some(on_command(CommandId::VIEW_OPEN_OUTPUT)),
            palette,
        ));

    return container(bar)
        .padding([3, 8])
        .width(Length::Fill)
        .style(move |_| FasciaStyle::toolbar_container(palette))
        .into();
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
            ViewIcon(IconId::Search, 12.0, palette.text_secondary),
            Space::new().width(6),
            text("Quick Open / Commands").size(11).style(move |_| text::Style {
                color: Some(palette.text_secondary),
            }),
            Space::new().width(12),
            text("Ctrl+P").size(10).style(move |_| text::Style {
                color: Some(palette.text_muted),
            }),
        ]
        .align_y(Alignment::Center),
    )
    .padding([3, 12])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(on_command(CommandId::VIEW_QUICK_OPEN));

    let sidebar_btn = button(
        row![
            ViewIcon(IconId::SidebarLeft, 13.0, if show_sidebar { palette.accent } else { palette.text_muted }),
            Space::new().width(4),
            text("Sidebar").size(11).style(move |_| text::Style {
                color: Some(if show_sidebar { palette.text_primary } else { palette.text_secondary }),
            }),
        ]
        .align_y(Alignment::Center),
    )
    .padding([3, 7])
    .style(move |_, status| FasciaStyle::tree_row_button(palette, show_sidebar, status))
    .on_press(on_command(CommandId::VIEW_TOGGLE_SIDEBAR));

    let bottom_btn = button(
        row![
            ViewIcon(IconId::PanelBottom, 13.0, if show_bottom_tray { palette.accent } else { palette.text_muted }),
            Space::new().width(4),
            text("Panel").size(11).style(move |_| text::Style {
                color: Some(if show_bottom_tray { palette.text_primary } else { palette.text_secondary }),
            }),
        ]
        .align_y(Alignment::Center),
    )
    .padding([3, 7])
    .style(move |_, status| FasciaStyle::tree_row_button(palette, show_bottom_tray, status))
    .on_press(on_command(CommandId::VIEW_TOGGLE_BOTTOM_PANEL));

    let prop_btn = button(
        row![
            ViewIcon(IconId::PanelRight, 13.0, if show_properties { palette.accent } else { palette.text_muted }),
            Space::new().width(4),
            text("Properties").size(11).style(move |_| text::Style {
                color: Some(if show_properties { palette.text_primary } else { palette.text_secondary }),
            }),
        ]
        .align_y(Alignment::Center),
    )
    .padding([3, 7])
    .style(move |_, status| FasciaStyle::tree_row_button(palette, show_properties, status))
    .on_press(on_command(CommandId::VIEW_TOGGLE_PROPERTIES));

    let settings_btn = button(ViewIcon(IconId::Sliders, 13.0, palette.text_secondary))
        .padding([4, 7])
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
    .padding([2, 6]);

    return container(bar)
        .width(Length::Fill)
        .style(move |_| FasciaStyle::menubar_container(palette))
        .into();
}
