// plot2d.rs --------------------------------------------------------------------------------------
//! Interactive 2D function plotter for Fresco scalar expressions.

use crate::fascia::icons::{IconId, ViewIcon};
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::fresco_document::{FrescoDocument, FrescoRootKind};
use crate::fresco::{CompileLimits, CompiledProgram, ExprRepos};
use crate::silo::Arr;
use iced::widget::canvas::{self, Frame, Geometry, Path, Program, Stroke, Text as CanvasText};
use iced::widget::{Space, button, column, container, row, sensor, text, text_input};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Size, mouse};

//-------------------------------------------------------------------------------------------------

/// A single sampled 2D curve $y = f(x)$.
#[derive(Debug, Clone)]
pub struct Plot2dSeries
{
    pub name:   String,
    pub points: Vec<[f64; 2]>,
    pub color:  Color,
}

/// Actions emitted by the 2D plotter UI.
#[derive(Debug, Clone)]
pub enum Plot2dAction
{
    SetXMin(String),
    SetXMax(String),
    ResetView,
    ToggleGrid,
    Hover(Option<Point>),
    Resize(f32, f32),
}

/// Interactive state for a 2D function plot tab.
#[derive(Debug, Clone)]
pub struct Plot2dState
{
    pub title:       String,
    pub series:      Vec<Plot2dSeries>,
    pub x_min:       f64,
    pub x_max:       f64,
    pub y_min:       f64,
    pub y_max:       f64,
    pub x_min_input: String,
    pub x_max_input: String,
    pub show_grid:   bool,
    pub hover_point: Option<[f64; 2]>,
    pub canvas_size: Size,
    pub status:      String,
}

impl Plot2dState
{
    pub fn New(title: impl Into<String>, series: Vec<Plot2dSeries>, x_min: f64, x_max: f64) -> Self
    {
        let (y_min, y_max) = compute_y_bounds(&series);
        return Self {
            title:       title.into(),
            series,
            x_min,
            x_max,
            y_min,
            y_max,
            x_min_input: format!("{:.2}", x_min),
            x_max_input: format!("{:.2}", x_max),
            show_grid:   true,
            hover_point: None,
            canvas_size: Size::new(640.0, 400.0),
            status:      "Hover over the plot to inspect coordinates".to_string(),
        };
    }

    pub fn ProcessAction(&mut self, action: Plot2dAction)
    {
        match action {
            Plot2dAction::SetXMin(s) => {
                self.x_min_input = s.clone();
                if let Ok(v) = s.trim().parse::<f64>()
                    && v.is_finite()
                    && v < self.x_max
                {
                    self.x_min = v;
                }
            }
            Plot2dAction::SetXMax(s) => {
                self.x_max_input = s.clone();
                if let Ok(v) = s.trim().parse::<f64>()
                    && v.is_finite()
                    && v > self.x_min
                {
                    self.x_max = v;
                }
            }
            Plot2dAction::ResetView => {
                let (ymin, ymax) = compute_y_bounds(&self.series);
                self.y_min = ymin;
                self.y_max = ymax;
                if let (Some(first), Some(last)) = (
                    self.series.first().and_then(|s| s.points.first()),
                    self.series.first().and_then(|s| s.points.last()),
                ) {
                    self.x_min = first[0];
                    self.x_max = last[0];
                    self.x_min_input = format!("{:.2}", self.x_min);
                    self.x_max_input = format!("{:.2}", self.x_max);
                }
            }
            Plot2dAction::ToggleGrid => {
                self.show_grid = !self.show_grid;
            }
            Plot2dAction::Hover(pt) => {
                self.hover_point = pt.and_then(|p| screen_to_world(p, self.canvas_size, self.x_min, self.x_max, self.y_min, self.y_max));
            }
            Plot2dAction::Resize(w, h) => {
                if w > 1.0 && h > 1.0 {
                    self.canvas_size = Size::new(w, h);
                }
            }
        }
    }
}

fn compute_y_bounds(series: &[Plot2dSeries]) -> (f64, f64)
{
    let mut ymin = f64::INFINITY;
    let mut ymax = f64::NEG_INFINITY;
    for s in series {
        for pt in &s.points {
            if pt[1].is_finite() {
                ymin = ymin.min(pt[1]);
                ymax = ymax.max(pt[1]);
            }
        }
    }
    if !ymin.is_finite() || !ymax.is_finite() {
        return (-1.0, 1.0);
    }
    if (ymax - ymin).abs() < 1e-9 {
        return (ymin - 1.0, ymax + 1.0);
    }
    let pad = (ymax - ymin) * 0.10;
    return (ymin - pad, ymax + pad);
}

fn screen_to_world(
    pt: Point,
    size: Size,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> Option<[f64; 2]>
{
    let pad = 36.0f32;
    let w = (size.width - pad * 2.0).max(1.0);
    let h = (size.height - pad * 2.0).max(1.0);
    let nx = ((pt.x - pad) / w) as f64;
    let ny = (1.0 - (pt.y - pad) / h) as f64;
    if !(0.0..=1.0).contains(&nx) || !(0.0..=1.0).contains(&ny) {
        return None;
    }
    return Some([
        x_min + nx * (x_max - x_min),
        y_min + ny * (y_max - y_min),
    ]);
}

//-------------------------------------------------------------------------------------------------

const SERIES_PALETTE: &[Color] = &[
    Color::from_rgb(0.22, 0.56, 0.96),
    Color::from_rgb(0.92, 0.42, 0.28),
    Color::from_rgb(0.24, 0.76, 0.52),
    Color::from_rgb(0.76, 0.44, 0.92),
    Color::from_rgb(0.95, 0.72, 0.18),
];

/// Evaluates all scalar roots of `doc` over `[x_min, x_max]` into `Plot2dSeries`.
pub fn SampleFrescoPlot2d(
    doc: &FrescoDocument,
    x_min: f64,
    x_max: f64,
    samples: usize,
) -> Result<Vec<Plot2dSeries>, String>
{
    if !(x_min.is_finite() && x_max.is_finite() && x_max > x_min) {
        return Err("Invalid 2D plot domain: x_max must exceed x_min".into());
    }
    let n = samples.max(16);
    let repos = doc.Repos();
    let mut out = Vec::new();
    let mut color_idx = 0usize;

    for r in 0..doc.RootCount() {
        let Some(root) = doc.RootAt(r) else { continue; };
        let FrescoRootKind::Scalar { _ExprId } = root.Kind() else { continue; };
        let pts = sample_scalar_1d(repos, *_ExprId, x_min, x_max, n);
        if !pts.is_empty() {
            out.push(Plot2dSeries {
                name:   root.Name().to_string(),
                points: pts,
                color:  SERIES_PALETTE[color_idx % SERIES_PALETTE.len()],
            });
            color_idx += 1;
        }
    }
    if out.is_empty() {
        return Err("No scalar roots available to plot in 2D".into());
    }
    return Ok(out);
}

fn sample_scalar_1d(repos: &ExprRepos, expr_id: u32, x_min: f64, x_max: f64, n: usize) -> Vec<[f64; 2]>
{
    let mut points = Vec::with_capacity(n);
    if let Ok(prog) = CompiledProgram::Compile(repos, Arr::from(&[expr_id]), &CompileLimits::default()) {
        let mut ws = prog.AllocWorkspace();
        let var_count = prog.VarCount() as usize;
        let mut inputs = vec![1.0; var_count];
        let mut x_slot = None;
        for v in 0..prog.VarCount() {
            if prog.VarNameAt(v) == Some("x") {
                x_slot = Some(v as usize);
                break;
            }
        }
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            let x = x_min + t * (x_max - x_min);
            if let Some(slot) = x_slot {
                inputs[slot] = x;
            }
            if ws.SetInputs(Arr::from(inputs.as_slice())).is_ok()
                && let Ok(vals) = ws.Evaluate(&prog)
                && vals[0].is_finite()
            {
                points.push([x, vals[0]]);
            }
        }
        return points;
    }

    for i in 0..n {
        let t = i as f64 / (n - 1) as f64;
        let x = x_min + t * (x_max - x_min);
        if let Ok(y) = repos.EvaluateNamed(expr_id, Arr::from(&[("x", x), ("y", 1.0), ("theta", x)]))
            && y.is_finite()
        {
            points.push([x, y]);
        }
    }
    return points;
}

//-------------------------------------------------------------------------------------------------

struct Plot2dProgram<'a>
{
    _State:   &'a Plot2dState,
    _Palette: ThemePalette,
}

impl<'a> Program<Plot2dAction> for Plot2dProgram<'a>
{
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Plot2dAction>>
    {
        if let iced::Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            return Some(canvas::Action::publish(Plot2dAction::Hover(cursor.position_in(bounds))));
        }
        return None;
    }

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
        let pad = 36.0f32;
        let plot_w = (bounds.width - pad * 2.0).max(1.0);
        let plot_h = (bounds.height - pad * 2.0).max(1.0);
        let x_span = (self._State.x_max - self._State.x_min).max(1e-9);
        let y_span = (self._State.y_max - self._State.y_min).max(1e-9);

        let to_screen = |wx: f64, wy: f64| -> Point {
            let sx = pad + ((wx - self._State.x_min) / x_span) as f32 * plot_w;
            let sy = pad + (1.0 - ((wy - self._State.y_min) / y_span) as f32) * plot_h;
            Point::new(sx, sy)
        };

        if self._State.show_grid {
            let grid_stroke = Stroke::default()
                .with_color(self._Palette.border_subtle)
                .with_width(1.0);
            for step in 0..=4 {
                let frac = step as f32 / 4.0;
                let gx = pad + frac * plot_w;
                let gy = pad + frac * plot_h;
                frame.stroke(&Path::line(Point::new(gx, pad), Point::new(gx, pad + plot_h)), grid_stroke);
                frame.stroke(&Path::line(Point::new(pad, gy), Point::new(pad + plot_w, gy)), grid_stroke);

                let xv = self._State.x_min + (frac as f64) * x_span;
                let yv = self._State.y_max - (frac as f64) * y_span;
                frame.fill_text(CanvasText {
                    content: format!("{:.2}", xv),
                    position: Point::new(gx - 14.0, pad + plot_h + 6.0),
                    color: self._Palette.text_muted,
                    size: 10.0.into(),
                    ..Default::default()
                });
                frame.fill_text(CanvasText {
                    content: format!("{:.2}", yv),
                    position: Point::new(4.0, gy - 6.0),
                    color: self._Palette.text_muted,
                    size: 10.0.into(),
                    ..Default::default()
                });
            }
        }

        let axis_stroke = Stroke::default().with_color(self._Palette.border).with_width(1.25);
        if self._State.x_min <= 0.0 && self._State.x_max >= 0.0 {
            let x0 = to_screen(0.0, self._State.y_min).x;
            frame.stroke(&Path::line(Point::new(x0, pad), Point::new(x0, pad + plot_h)), axis_stroke);
        }
        if self._State.y_min <= 0.0 && self._State.y_max >= 0.0 {
            let y0 = to_screen(self._State.x_min, 0.0).y;
            frame.stroke(&Path::line(Point::new(pad, y0), Point::new(pad + plot_w, y0)), axis_stroke);
        }

        frame.stroke(
            &Path::rectangle(Point::new(pad, pad), Size::new(plot_w, plot_h)),
            axis_stroke,
        );

        for series in &self._State.series {
            if series.points.len() < 2 {
                continue;
            }
            let path = Path::new(|b| {
                let first = to_screen(series.points[0][0], series.points[0][1]);
                b.move_to(first);
                for pt in &series.points[1..] {
                    b.line_to(to_screen(pt[0], pt[1]));
                }
            });
            frame.stroke(&path, Stroke::default().with_color(series.color).with_width(2.0));
        }

        if let Some([hx, hy]) = self._State.hover_point {
            let sp = to_screen(hx, hy);
            let ch_stroke = Stroke::default().with_color(self._Palette.accent).with_width(1.0);
            frame.stroke(&Path::line(Point::new(sp.x, pad), Point::new(sp.x, pad + plot_h)), ch_stroke);
            frame.stroke(&Path::line(Point::new(pad, sp.y), Point::new(pad + plot_w, sp.y)), ch_stroke);
        }

        return vec![frame.into_geometry()];
    }
}

/// Renders the interactive 2D function plotter tab.
pub fn ViewPlot2d<'a, Message: 'static + Clone>(
    state: &'a Plot2dState,
    palette: ThemePalette,
    on_action: impl Fn(Plot2dAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let mut legend = row![].spacing(10).align_y(Alignment::Center);
    for s in &state.series {
        let dot_color = s.color;
        legend = legend.push(
            row![
                container(Space::new().width(8).height(8)).style(move |_| container::Style {
                    background: Some(iced::Background::Color(dot_color)),
                    border:     iced::border::rounded(4),
                    ..Default::default()
                }),
                text(&s.name).size(11).style(move |_| text::Style {
                    color: Some(palette.text_primary),
                }),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        );
    }

    let hover_label = if let Some([hx, hy]) = state.hover_point {
        format!("x: {:.4}, y: {:.4}", hx, hy)
    } else {
        state.status.clone()
    };

    let controls = container(
        row![
            ViewIcon(IconId::MathFx, 13.0, palette.accent),
            text(&state.title).size(12).style(move |_| text::Style {
                color: Some(palette.text_primary),
            }),
            Space::new().width(10),
            text("x:").size(11).style(move |_| text::Style { color: Some(palette.text_muted) }),
            text_input("min", &state.x_min_input)
                .on_input(move |s| on_action(Plot2dAction::SetXMin(s)))
                .width(Length::Fixed(58.0))
                .padding([3, 6])
                .size(11),
            text("..").size(11).style(move |_| text::Style { color: Some(palette.text_muted) }),
            text_input("max", &state.x_max_input)
                .on_input(move |s| on_action(Plot2dAction::SetXMax(s)))
                .width(Length::Fixed(58.0))
                .padding([3, 6])
                .size(11),
            button(text("Reset").size(11))
                .padding([3, 8])
                .style(move |_, st| FasciaStyle::toolbar_button(palette, st))
                .on_press(on_action(Plot2dAction::ResetView)),
            button(text(if state.show_grid { "Grid: On" } else { "Grid: Off" }).size(11))
                .padding([3, 8])
                .style(move |_, st| FasciaStyle::toolbar_button(palette, st))
                .on_press(on_action(Plot2dAction::ToggleGrid)),
            Space::new().width(Length::Fill),
            legend,
            Space::new().width(12),
            text(hover_label).size(11).style(move |_| text::Style {
                color: Some(palette.text_secondary),
            }),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([6, 10])
    .width(Length::Fill)
    .style(move |_| FasciaStyle::toolbar_container(palette));

    let plot_canvas: Element<'a, Plot2dAction> = iced::widget::canvas(Plot2dProgram {
        _State:   state,
        _Palette: palette,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into();
    let plot_mapped = plot_canvas.map(on_action);

    let measured = sensor(plot_mapped)
        .on_resize(move |size| on_action(Plot2dAction::Resize(size.width, size.height)));

    return column![controls, container(measured).width(Length::Fill).height(Length::Fill)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
}
