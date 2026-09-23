//-- cask_view.rs ------------------------------------------------------------------------------------------------------
//! 2D visual canvas renderer for Cask UI hierarchies using Iced canvas.

use crate::fenst::cask::{Cask, CaskRenderCommand, LayoutAndRenderCask};
use iced::widget::canvas::{Frame, Geometry, Path, Program, Stroke, Text};
use iced::widget::canvas;
use iced::{Color, Element, Length, Point, Rectangle, Size, mouse};

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
