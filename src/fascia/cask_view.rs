//-- cask_view.rs ------------------------------------------------------------------------------------------------------
//! 2D visual canvas renderer for Cask UI hierarchies using Iced canvas.

use crate::fascia::tabs::TabId;
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::cask::{Cask, CaskRenderCommand, LayoutAndRenderCask};
use crate::fenst::cask2d::{Cask2dColorScheme, Cask2dLayout, Cask2dLayoutMode, Cask2dNode, Cask2dOptions};
use iced::widget::canvas::{Frame, Geometry, Path, Program, Stroke, Text};
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Size, mouse};
use std::path::PathBuf;

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CaskRenderer
{
    commands: Vec<CaskRenderCommand>,
    opacity: f32,
}
impl CaskRenderer
{
    pub fn New(commands: Vec<CaskRenderCommand>) -> Self
    {
        Self { commands, opacity: 1.0 }
    }

    pub fn FromRoot(root: &Cask, origin_x: f32, origin_y: f32) -> Self
    {
        let commands = LayoutAndRenderCask(root, origin_x, origin_y);
        Self { commands, opacity: 1.0 }
    }

    pub fn FromCask2d(layout: &Cask2dLayout) -> Self
    {
        Self {
            commands: layout.Commands().to_vec(),
            opacity: 1.0,
        }
    }

    pub fn Commands(&self) -> &[CaskRenderCommand]
    {
        &self.commands
    }

    pub fn WithOpacity(mut self, opacity: f32) -> Self
    {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
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
                    let rect_color = Color::from_rgba(color.r, color.g, color.b, color.a * self.opacity);
                    let top_left = Point::new(r.x, r.y);
                    let size = Size::new(r.width, r.height);
                    frame.fill_rectangle(top_left, size, rect_color);
                }
                CaskRenderCommand::Border { bounds: r, color, width, corner_radius: _ } => {
                    let border_color = Color::from_rgba(color.r, color.g, color.b, color.a * self.opacity);
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
                CaskRenderCommand::Line { start, end, color, width } => {
                    let stroke_color = Color::from_rgba(color.r, color.g, color.b, color.a * self.opacity);
                    let stroke = Stroke::default().with_color(stroke_color).with_width(*width);
                    let line_path = Path::line(Point::new(start.x, start.y), Point::new(end.x, end.y));
                    frame.stroke(&line_path, stroke);
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
    iced::widget::canvas(CaskRenderer::New(commands))
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
    iced::widget::canvas(CaskRenderer::FromRoot(root, origin_x, origin_y))
        .width(width)
        .height(height)
        .into()
}

pub fn view_cask2d<'a, Message: 'static>(
    layout: &Cask2dLayout,
    width: Length,
    height: Length,
) -> Element<'a, Message>
{
    iced::widget::canvas(CaskRenderer::FromCask2d(layout))
        .width(width)
        .height(height)
        .into()
}

//---------------------------------------------------------------------------------------------------------------------------------
// Interactive Cask 2D Viewer State & Actions

/// User interactions and events within the 2D Cask viewer tab.
#[derive(Debug, Clone, PartialEq)]
pub enum Cask2dAction
{
    Mode(Cask2dLayoutMode),
    MaxDepth(u32),
    Zoom(f32),
    Pan(f32, f32),
    ResetView,
    FitView,
    SelectNode(Option<usize>),
    OpenFile(PathBuf),
    OpenSelectedIn2d(PathBuf),
    OpenIn3d(PathBuf),
    Search(String),
    ToggleTheme,
    Refresh,
    Resize(f32, f32),
}

/// Document state and view parameters for an interactive 2D Cask visualization tab.
#[derive(Debug, Clone)]
pub struct Cask2dViewerState
{
    pub path: PathBuf,
    pub uri: String,
    pub mode: Cask2dLayoutMode,
    pub max_depth: u32,
    pub dark_theme: bool,
    pub layout: Option<Cask2dLayout>,
    pub error: Option<String>,
    pub zoom: f32,
    pub pan: [f32; 2],
    pub selected_idx: Option<usize>,
    pub search_query: String,
    pub size: [f32; 2],
}

impl Cask2dViewerState
{
    pub fn New(path: PathBuf) -> Self
    {
        let path_str = path.to_string_lossy();
        let uri = if path_str.contains("://") {
            path_str.to_string()
        } else if path_str.is_empty() {
            "file://".to_string()
        } else {
            format!("file://{}", path.display())
        };
        let mut state = Self {
            path,
            uri,
            mode: Cask2dLayoutMode::NestedBoxes,
            max_depth: 4,
            dark_theme: true,
            layout: None,
            error: None,
            zoom: 1.0,
            pan: [40.0, 40.0],
            selected_idx: None,
            search_query: String::new(),
            size: [800.0, 600.0],
        };
        state.Recompute();
        state
    }

    pub fn FromUri(uri: String) -> Self
    {
        let path = PathBuf::from(&uri);
        let mut state = Self {
            path,
            uri,
            mode: Cask2dLayoutMode::NestedBoxes,
            max_depth: 4,
            dark_theme: true,
            layout: None,
            error: None,
            zoom: 1.0,
            pan: [40.0, 40.0],
            selected_idx: None,
            search_query: String::new(),
            size: [800.0, 600.0],
        };
        state.Recompute();
        state
    }

    pub fn Recompute(&mut self)
    {
        let scheme = if self.dark_theme {
            Cask2dColorScheme::DefaultDark()
        } else {
            Cask2dColorScheme::DefaultLight()
        };
        let opts = Cask2dOptions::New()
            .WithMode(self.mode)
            .WithMaxDepth(self.max_depth)
            .WithColorScheme(scheme);

        match Cask2dLayout::FromUri(&self.uri, &opts) {
            Ok(layout) => {
                if let Some(idx) = self.selected_idx
                    && idx >= layout.NodeCount()
                {
                    self.selected_idx = None;
                }
                self.layout = Some(layout);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }

    pub fn SelectedNode(&self) -> Option<&Cask2dNode>
    {
        let idx = self.selected_idx?;
        self.layout.as_ref()?.Nodes().get(idx)
    }

    pub fn Update(&mut self, action: Cask2dAction)
    {
        match action {
            Cask2dAction::Mode(mode) => {
                self.mode = mode;
                self.Recompute();
            }
            Cask2dAction::MaxDepth(depth) => {
                self.max_depth = depth.clamp(1, 12);
                self.Recompute();
            }
            Cask2dAction::Zoom(factor) => {
                self.zoom = (self.zoom * (1.0 + factor)).clamp(0.08, 8.0);
            }
            Cask2dAction::Pan(dx, dy) => {
                self.pan[0] += dx;
                self.pan[1] += dy;
            }
            Cask2dAction::ResetView => {
                self.zoom = 1.0;
                self.pan = [40.0, 40.0];
            }
            Cask2dAction::FitView => {
                if let Some(layout) = &self.layout {
                    let b = layout.Bounds();
                    if b.width > 0.0 && b.height > 0.0 && self.size[0] > 0.0 && self.size[1] > 0.0 {
                        let margin = 50.0;
                        let available_w = (self.size[0] - margin * 2.0).max(100.0);
                        let available_h = (self.size[1] - margin * 2.0).max(100.0);
                        let zx = available_w / b.width;
                        let zy = available_h / b.height;
                        let new_zoom = zx.min(zy).clamp(0.08, 3.0);
                        self.zoom = new_zoom;
                        self.pan[0] = margin / new_zoom - b.x;
                        self.pan[1] = margin / new_zoom - b.y;
                    }
                }
            }
            Cask2dAction::SelectNode(idx) => {
                self.selected_idx = idx;
            }
            Cask2dAction::Search(query) => {
                self.search_query = query;
            }
            Cask2dAction::ToggleTheme => {
                self.dark_theme = !self.dark_theme;
                self.Recompute();
            }
            Cask2dAction::Refresh => {
                self.Recompute();
            }
            Cask2dAction::Resize(w, h) => {
                self.size = [w, h];
            }
            Cask2dAction::OpenFile(_)
            | Cask2dAction::OpenSelectedIn2d(_)
            | Cask2dAction::OpenIn3d(_) => {}
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
// Interactive Canvas Program

#[derive(Debug, Clone, Default)]
pub struct Cask2dCanvasInteraction
{
    pub drag: Option<(mouse::Button, Point)>,
    pub click_origin: Option<Point>,
}

pub struct Cask2dCanvasProgram<'a, Message, F>
{
    pub state: &'a Cask2dViewerState,
    pub palette: ThemePalette,
    pub map: F,
    pub _phantom: std::marker::PhantomData<Message>,
}

impl<'a, Message: 'static + Clone, F: Fn(Cask2dAction) -> Message + Copy + 'static> Program<Message>
    for Cask2dCanvasProgram<'a, Message, F>
{
    type State = Cask2dCanvasInteraction;

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
        let bg_color = self.palette.content_bg;
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), bg_color);

        let Some(layout) = &self.state.layout else {
            return vec![frame.into_geometry()];
        };

        let zoom = self.state.zoom;
        let pan = self.state.pan;

        // Subtle infinite canvas dot grid
        let grid_spacing = 40.0 * zoom;
        if (12.0..=200.0).contains(&grid_spacing) {
            let offset_x = (pan[0] * zoom).rem_euclid(grid_spacing);
            let offset_y = (pan[1] * zoom).rem_euclid(grid_spacing);
            let dot_color = Color::from_rgba(self.palette.border.r, self.palette.border.g, self.palette.border.b, 0.22);
            let mut x = offset_x;
            while x < bounds.width {
                let mut y = offset_y;
                while y < bounds.height {
                    frame.fill_rectangle(Point::new(x, y), Size::new(1.5, 1.5), dot_color);
                    y += grid_spacing;
                }
                x += grid_spacing;
            }
        }

        // Render layout commands
        for cmd in layout.Commands() {
            match cmd {
                CaskRenderCommand::Rectangle { bounds: r, color, corner_radius: _ } => {
                    let sx = (r.x + pan[0]) * zoom;
                    let sy = (r.y + pan[1]) * zoom;
                    let sw = r.width * zoom;
                    let sh = r.height * zoom;
                    if sx + sw >= 0.0 && sx <= bounds.width && sy + sh >= 0.0 && sy <= bounds.height {
                        let rect_color = Color::from_rgba(color.r, color.g, color.b, color.a);
                        frame.fill_rectangle(Point::new(sx, sy), Size::new(sw, sh), rect_color);
                    }
                }
                CaskRenderCommand::Border { bounds: r, color, width, corner_radius: _ } => {
                    let sx = (r.x + pan[0]) * zoom;
                    let sy = (r.y + pan[1]) * zoom;
                    let sw = r.width * zoom;
                    let sh = r.height * zoom;
                    if sx + sw >= 0.0 && sx <= bounds.width && sy + sh >= 0.0 && sy <= bounds.height {
                        let border_color = Color::from_rgba(color.r, color.g, color.b, color.a);
                        let stroke = Stroke::default().with_color(border_color).with_width((*width * zoom).max(1.0));
                        frame.stroke(&Path::rectangle(Point::new(sx, sy), Size::new(sw, sh)), stroke);
                    }
                }
                CaskRenderCommand::Text { bounds: r, text, color, font_size } => {
                    let sx = (r.x + pan[0]) * zoom;
                    let sy = (r.y + pan[1]) * zoom;
                    let scaled_font = *font_size * zoom;
                    if scaled_font >= 4.5 && sx >= -250.0 && sx <= bounds.width + 50.0 && sy >= -50.0 && sy <= bounds.height + 50.0 {
                        let text_color = Color::from_rgba(color.r, color.g, color.b, color.a);
                        frame.fill_text(Text {
                            content: text.clone(),
                            position: Point::new(sx, sy),
                            color: text_color,
                            size: scaled_font.into(),
                            ..Default::default()
                        });
                    }
                }
                CaskRenderCommand::Line { start, end, color, width } => {
                    let p1 = Point::new((start.x + pan[0]) * zoom, (start.y + pan[1]) * zoom);
                    let p2 = Point::new((end.x + pan[0]) * zoom, (end.y + pan[1]) * zoom);
                    let stroke_color = Color::from_rgba(color.r, color.g, color.b, color.a);
                    let stroke = Stroke::default().with_color(stroke_color).with_width((*width * zoom).max(1.0));
                    frame.stroke(&Path::line(p1, p2), stroke);
                }
            }
        }

        // Highlight nodes matching search filter
        let query = self.state.search_query.trim().to_lowercase();
        if !query.is_empty() {
            let search_color = Color::from_rgba(1.0, 0.8, 0.2, 0.95);
            let stroke = Stroke::default().with_color(search_color).with_width(2.0);
            for node in layout.Nodes() {
                if node.Name().to_lowercase().contains(&query) {
                    let nb = node.Bounds();
                    let sx = (nb.x + pan[0]) * zoom;
                    let sy = (nb.y + pan[1]) * zoom;
                    let sw = nb.width * zoom;
                    let sh = nb.height * zoom;
                    frame.stroke(&Path::rectangle(Point::new(sx - 1.0, sy - 1.0), Size::new(sw + 2.0, sh + 2.0)), stroke);
                }
            }
        }

        // Highlight selected node
        if let Some(node) = self.state.SelectedNode() {
            let nb = node.Bounds();
            let sx = (nb.x + pan[0]) * zoom;
            let sy = (nb.y + pan[1]) * zoom;
            let sw = nb.width * zoom;
            let sh = nb.height * zoom;
            let sel_stroke = Stroke::default().with_color(self.palette.accent).with_width(2.5);
            frame.stroke(&Path::rectangle(Point::new(sx - 2.0, sy - 2.0), Size::new(sw + 4.0, sh + 4.0)), sel_stroke);
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<iced::widget::Action<Message>>
    {
        let is_over = cursor.is_over(bounds);
        let publish = |action| Some(iced::widget::Action::publish((self.map)(action)).and_capture());

        match event {
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) if is_over => {
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 25.0,
                };
                let zoom_delta = (lines * 0.12).clamp(-0.4, 0.4);
                return publish(Cask2dAction::Zoom(zoom_delta));
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(button)) if is_over => {
                if let Some(pos) = cursor.position_in(bounds) {
                    state.drag = Some((*button, pos));
                    state.click_origin = Some(pos);
                    return Some(iced::widget::Action::capture());
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(button)) => {
                let had_drag = state.drag.take();
                let click_orig = state.click_origin.take();

                if *button == mouse::Button::Left
                    && let (Some((mouse::Button::Left, _)), Some(orig), Some(pos)) =
                        (had_drag, click_orig, cursor.position_in(bounds))
                {
                    let dist = ((pos.x - orig.x).powi(2) + (pos.y - orig.y).powi(2)).sqrt();
                    if dist < 6.0 {
                        let world_x = pos.x / self.state.zoom - self.state.pan[0];
                        let world_y = pos.y / self.state.zoom - self.state.pan[1];
                        let hit = self
                            .state
                            .layout
                            .as_ref()
                            .and_then(|l| l.Nodes().iter().position(|n| n.ContainsPoint(world_x, world_y)));
                        return publish(Cask2dAction::SelectNode(hit));
                    }
                }
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some((btn, prev)) = state.drag
                    && let Some(pos) = cursor.position_in(bounds)
                {
                    state.drag = Some((btn, pos));
                    let dx = pos.x - prev.x;
                    let dy = pos.y - prev.y;
                    if btn == mouse::Button::Middle
                        || btn == mouse::Button::Right
                        || (btn == mouse::Button::Left && (dx.abs() + dy.abs()) > 1.0)
                    {
                        let world_dx = dx / self.state.zoom;
                        let world_dy = dy / self.state.zoom;
                        return publish(Cask2dAction::Pan(world_dx, world_dy));
                    }
                }
            }
            iced::Event::Window(iced::window::Event::RedrawRequested(_))
                if self.state.size != [bounds.width, bounds.height]
                    && bounds.width > 0.0
                    && bounds.height > 0.0 =>
            {
                return publish(Cask2dAction::Resize(bounds.width, bounds.height));
            }
            _ => {}
        }
        None
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
// Full Interactive Tab View

pub fn view_cask2d_tab<'a, Message: 'static + Clone>(
    _id: TabId,
    state: &'a Cask2dViewerState,
    palette: ThemePalette,
    map_action: impl Fn(Cask2dAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    if let Some(error) = &state.error {
        return container(
            column![
                text("Unable to layout Cask 2D tree").size(18).style(move |_| text::Style {
                    color: Some(palette.accent),
                }),
                Space::new().height(Length::Fixed(8.0)),
                text(error).size(13).style(move |_| text::Style {
                    color: Some(palette.text_secondary),
                }),
                Space::new().height(Length::Fixed(12.0)),
                button("Retry").padding([6, 12]).on_press(map_action(Cask2dAction::Refresh))
            ]
            .padding(24)
            .spacing(6),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| FasciaStyle::content_container(palette))
        .into();
    }

    // Top Controls Bar
    let mode_btn = |mode: Cask2dLayoutMode, label: &'static str| {
        let is_active = state.mode == mode;
        button(text(label).size(12).style(move |_| text::Style {
            color: if is_active {
                Some(palette.accent)
            } else {
                Some(palette.text_primary)
            },
        }))
        .padding([4, 8])
        .style(move |_, status| {
            let mut style = FasciaStyle::toolbar_button(palette, status);
            if is_active {
                style.border.color = palette.accent;
                style.border.width = 1.0;
            }
            style
        })
        .on_press(map_action(Cask2dAction::Mode(mode)))
    };

    let modes_group = row![
        mode_btn(Cask2dLayoutMode::NestedBoxes, "🗃 Nested"),
        mode_btn(Cask2dLayoutMode::HorizontalTree, "🌳 Horiz"),
        mode_btn(Cask2dLayoutMode::VerticalTree, "🌲 Vert"),
        mode_btn(Cask2dLayoutMode::IndentedList, "📑 Outline"),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let depth_stepper = row![
        button(text("➖").size(11))
            .padding([3, 6])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(Cask2dAction::MaxDepth(state.max_depth.saturating_sub(1)))),
        text(format!("Depth: {}", state.max_depth)).size(12).style(move |_| text::Style {
            color: Some(palette.text_primary),
        }),
        button(text("➕").size(11))
            .padding([3, 6])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(Cask2dAction::MaxDepth(state.max_depth + 1))),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let zoom_group = row![
        button(text("➖").size(11))
            .padding([3, 6])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(Cask2dAction::Zoom(-0.2))),
        text(format!("{:.0}%", state.zoom * 100.0)).size(12).style(move |_| text::Style {
            color: Some(palette.text_secondary),
        }),
        button(text("➕").size(11))
            .padding([3, 6])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(Cask2dAction::Zoom(0.2))),
        button(text("🎯 Fit").size(11))
            .padding([3, 6])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(Cask2dAction::FitView)),
        button(text("1:1").size(11))
            .padding([3, 6])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(Cask2dAction::ResetView)),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let theme_btn = button(text(if state.dark_theme { "🌙 Dark" } else { "☀ Light" }).size(11))
        .padding([3, 6])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(Cask2dAction::ToggleTheme));

    let refresh_btn = button(text("🔄").size(12))
        .padding([3, 6])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(Cask2dAction::Refresh));

    let search_box = text_input("Filter nodes...", &state.search_query)
        .size(12)
        .padding([3, 6])
        .width(Length::Fixed(150.0))
        .on_input(move |q| map_action(Cask2dAction::Search(q)));

    let toolbar = row![
        text("CASK 2D").size(12).style(move |_| text::Style {
            color: Some(palette.accent),
        }),
        Space::new().width(Length::Fixed(8.0)),
        modes_group,
        Space::new().width(Length::Fixed(12.0)),
        depth_stepper,
        Space::new().width(Length::Fixed(12.0)),
        zoom_group,
        Space::new().width(Length::Fixed(12.0)),
        theme_btn,
        refresh_btn,
        Space::new().width(Length::Fill),
        search_box,
    ]
    .padding([5, 10])
    .spacing(6)
    .align_y(Alignment::Center);

    let toolbar_container = container(toolbar)
        .width(Length::Fill)
        .style(move |_| FasciaStyle::menubar_container(palette));

    // Canvas element
    let canvas_element = iced::widget::canvas(Cask2dCanvasProgram {
        state,
        palette,
        map: map_action,
        _phantom: std::marker::PhantomData,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    // Bottom Inspector / Status Strip
    let bottom_content: Element<'_, Message> = if let Some(node) = state.SelectedNode() {
        let is_leaf = node.IsLeaf();
        let path = PathBuf::from(node.Path());
        let path_clone1 = path.clone();
        let path_clone2 = path.clone();
        let path_clone3 = path.clone();

        let icon = if is_leaf { "📄" } else { "📁" };
        let mut row_elems = row![
            text(icon).size(14),
            Space::new().width(Length::Fixed(4.0)),
            text(node.Name()).size(13).style(move |_| text::Style {
                color: Some(palette.accent),
            }),
            Space::new().width(Length::Fixed(10.0)),
            text(format!("Depth {} • {:.0}×{:.0}", node.Depth(), node.Bounds().width, node.Bounds().height))
                .size(11)
                .style(move |_| text::Style {
                    color: Some(palette.text_secondary),
                }),
            Space::new().width(Length::Fixed(10.0)),
            text(node.Path()).size(11).style(move |_| text::Style {
                color: Some(palette.text_muted),
            }),
            Space::new().width(Length::Fill),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        if is_leaf {
            row_elems = row_elems.push(
                button(text("📄 Open File").size(11))
                    .padding([2, 8])
                    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                    .on_press(map_action(Cask2dAction::OpenFile(path_clone1))),
            );
        } else {
            row_elems = row_elems
                .push(
                    button(text("🗺 Focus in 2D").size(11))
                        .padding([2, 8])
                        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                        .on_press(map_action(Cask2dAction::OpenSelectedIn2d(path_clone2))),
                )
                .push(
                    button(text("📦 Open in 3D").size(11))
                        .padding([2, 8])
                        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                        .on_press(map_action(Cask2dAction::OpenIn3d(path_clone3))),
                );
        }

        row_elems = row_elems.push(
            button(text("✕").size(11))
                .padding([2, 6])
                .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                .on_press(map_action(Cask2dAction::SelectNode(None))),
        );

        row_elems.into()
    } else if let Some(layout) = &state.layout {
        row![
            text(format!(
                "{} nodes ({} branches, {} leaves) • Max depth: {} • Canvas: {:.0}×{:.0}",
                layout.NodeCount(),
                layout.BranchCount(),
                layout.LeafCount(),
                layout.MaxDepth(),
                layout.Bounds().width,
                layout.Bounds().height
            ))
            .size(11)
            .style(move |_| text::Style {
                color: Some(palette.text_muted),
            }),
            Space::new().width(Length::Fill),
            text("💡 Click node to inspect • Drag or right-click to pan • Wheel to zoom")
                .size(11)
                .style(move |_| text::Style {
                    color: Some(palette.text_muted),
                }),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    } else {
        text("Ready").size(11).into()
    };

    let bottom_bar = container(bottom_content)
        .width(Length::Fill)
        .padding([4, 10])
        .style(move |_| FasciaStyle::status_bar_container(palette));

    column![toolbar_container, canvas_element, bottom_bar]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

//---------------------------------------------------------------------------------------------------------------------------------
