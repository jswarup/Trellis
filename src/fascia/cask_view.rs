//-- cask_view.rs ------------------------------------------------------------------------------------------------------
//! 2D visual canvas renderer for Cask UI hierarchies using Iced canvas.

use crate::fascia::tabs::TabId;
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::cask::{BuildCaskHierarchyFromPath, Cask, CaskRenderCommand, LayoutAndRenderCask};
use iced::widget::canvas::{Frame, Geometry, Path, Program, Stroke, Text};
use iced::widget::{Space, button, canvas, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Size, mouse};
use std::path::PathBuf;

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CaskRenderer
{
    commands: Vec<CaskRenderCommand>,
}
impl CaskRenderer
{
    pub fn New(commands: Vec<CaskRenderCommand>) -> Self
    {
        Self { commands }
    }

    pub fn FromRoot(root: &Cask, origin_x: f32, origin_y: f32) -> Self
    {
        let commands = LayoutAndRenderCask(root, origin_x, origin_y);
        Self { commands }
    }

    pub fn Commands(&self) -> &[CaskRenderCommand]
    {
        &self.commands
    }
}

impl<Message> Program<Message> for CaskRenderer
{
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry>
    {
        let mut frame = Frame::new(renderer, bounds.size());

        for cmd in &self.commands {
            match cmd {
                CaskRenderCommand::Rectangle { bounds: r, color, corner_radius: _ } => {
                    let rect_color = Color::from_rgba(color.r, color.g, color.b, color.a);
                    let top_left = Point::new(r.x, r.y);
                    let size = Size::new(r.width, r.height);
                    frame.fill_rectangle(top_left, size, rect_color);
                }
                CaskRenderCommand::Border { bounds: r, color, width, corner_radius: _ } => {
                    let border_color = Color::from_rgba(color.r, color.g, color.b, color.a);
                    let stroke = Stroke::default().with_color(border_color).with_width(*width);
                    let top_left = Point::new(r.x, r.y);
                    let size = Size::new(r.width, r.height);
                    frame.stroke(&Path::rectangle(top_left, size), stroke);
                }
                CaskRenderCommand::Text { bounds: r, text, color, font_size } => {
                    let text_color = Color::from_rgba(color.r, color.g, color.b, color.a);
                    frame.fill_text(Text {
                        content: text.clone(),
                        position: Point::new(r.x, r.y),
                        color: text_color,
                        size: (*font_size).into(),
                        ..Default::default()
                    });
                }
            }
        }

        vec![frame.into_geometry()]
    }
}

pub fn view_cask<'a, Message: 'static>(
    commands: Vec<CaskRenderCommand>,
    width: Length,
    height: Length,
) -> Element<'a, Message>
{
    canvas(CaskRenderer::New(commands))
        .width(width)
        .height(height)
        .into()
}

pub fn view_cask_root<'a, Message: 'static>(
    root: &Cask,
    origin_x: f32,
    origin_y: f32,
    width: Length,
    height: Length,
) -> Element<'a, Message>
{
    canvas(CaskRenderer::FromRoot(root, origin_x, origin_y))
        .width(width)
        .height(height)
        .into()
}

//---------------------------------------------------------------------------------------------------------------------------------

/// Actions emitted by the dedicated Cask viewer window toolbar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaskViewerAction {
    Refresh,
    Reset,
}

/// State for an open dedicated Cask graphics window/tab.
#[derive(Debug, Clone)]
pub struct CaskViewerState {
    pub path: PathBuf,
    pub root: Cask,
    pub commands: Vec<CaskRenderCommand>,
}

impl CaskViewerState {
    pub fn new(path: PathBuf) -> Self {
        let root = BuildCaskHierarchyFromPath(&path, 3);
        let commands = LayoutAndRenderCask(&root, 24.0, 24.0);
        Self {
            path,
            root,
            commands,
        }
    }

    pub fn refresh(&mut self) {
        self.root = BuildCaskHierarchyFromPath(&self.path, 3);
        self.commands = LayoutAndRenderCask(&self.root, 24.0, 24.0);
    }
}

/// Constructs the dedicated 2D Cask graphics window content view.
pub fn view_cask_viewer<'a, Message: 'static + Clone>(
    _tab_id: TabId,
    state: &'a CaskViewerState,
    palette: ThemePalette,
    map_action: impl Fn(CaskViewerAction) -> Message + Copy + 'static,
) -> Element<'a, Message> {
    let header_title = text("📦 CASK 2D GRAPHICS WINDOW")
        .size(13)
        .style(move |_| text::Style {
            color: Some(palette.accent),
        });

    let path_str = state.path.display().to_string();
    let path_label = text(format!("Node: {}", path_str))
        .size(12)
        .style(move |_| text::Style {
            color: Some(palette.text_secondary),
        });

    let badge = |lbl: &'static str, color: iced::Color| {
        container(
            text(lbl).size(10).style(move |_| text::Style {
                color: Some(color),
            }),
        )
        .padding([2, 6])
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                color.r, color.g, color.b, 0.15,
            ))),
            border: iced::Border {
                color,
                width: 1.0,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
    };

    let badges = row![
        badge("L0: Root", palette.accent),
        text("➔").size(10).style(move |_| text::Style {
            color: Some(palette.text_muted),
        }),
        badge("L1: Modules", iced::Color::from_rgb8(198, 160, 246)),
        text("➔").size(10).style(move |_| text::Style {
            color: Some(palette.text_muted),
        }),
        badge("L2: Sub-boxes", iced::Color::from_rgb8(137, 220, 235)),
        text("➔").size(10).style(move |_| text::Style {
            color: Some(palette.text_muted),
        }),
        badge("L3: Leaves", iced::Color::from_rgb8(166, 218, 149)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let refresh_btn = button(
        row![
            text("🔄").size(12),
            Space::new().width(Length::Fixed(4.0)),
            text("Refresh").size(11).style(move |_| text::Style {
                color: Some(palette.text_primary),
            }),
        ]
        .align_y(Alignment::Center),
    )
    .padding([3, 8])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(map_action(CaskViewerAction::Refresh));

    let toolbar = row![
        header_title,
        Space::new().width(Length::Fixed(12.0)),
        path_label,
        Space::new().width(Length::Fill),
        badges,
        Space::new().width(Length::Fixed(16.0)),
        refresh_btn,
    ]
    .align_y(Alignment::Center)
    .padding([8, 16]);

    let canvas_widget = canvas(CaskRenderer::New(state.commands.clone()))
        .width(Length::Fixed(960.0))
        .height(Length::Fixed(640.0));

    let canvas_container = container(canvas_widget)
        .padding(16)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center);

    let scrollable_canvas = scrollable(canvas_container)
        .width(Length::Fill)
        .height(Length::Fill);

    let layout = column![toolbar, scrollable_canvas]
        .width(Length::Fill)
        .height(Length::Fill);

    container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| FasciaStyle::content_container(palette))
        .into()
}
