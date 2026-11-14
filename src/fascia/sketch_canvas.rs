//-- sketch_canvas.rs -----------------------------------------------------------------------------------------------
use crate::fascia::sketch_tool::{
    DraftPrimitive, GripInfo, KeyModifiers, PointerButton, SketchInputEvent,
    SketchToolController,
};
use crate::fascia::theme::ThemePalette;
use crate::fenst::sketch::{
    Circle, CircularArc, Ellipse, LineSegment, PolylineHeader, SketchDocument,
    SketchPoint, SketchPrimitive,
};
use crate::fleck::sketch_query::SnapKind;
use crate::fleck::sketch_tessellate::SketchTessellate;
use crate::silo::IArr;
use iced::mouse;
use iced::widget::canvas::{Cache, Frame, Geometry, Path, Program, Stroke, Text};
use iced::{Color, Point, Rectangle, Size};

//-------------------------------------------------------------------------------------------------

/// State managing Iced canvas cache invalidation layers and interaction state.
#[derive(Debug, Default)]
pub struct SketchCanvasState
{
    pub grid_cache:     Cache,
    pub geometry_cache: Cache,
    pub last_revision:  u64,
    pub last_zoom:      f32,
    pub last_pan:       [f32; 2],
    pub last_size:      [f32; 2],
}

impl SketchCanvasState
{
    pub fn New() -> Self
    {
        Self {
            grid_cache:     Cache::default(),
            geometry_cache: Cache::default(),
            last_revision:  u64::MAX,
            last_zoom:      0.0,
            last_pan:       [0.0, 0.0],
            last_size:      [0.0, 0.0],
        }
    }

    pub fn InvalidateGrid(&mut self)
    {
        self.grid_cache.clear();
    }

    pub fn InvalidateGeometry(&mut self)
    {
        self.geometry_cache.clear();
    }

    pub fn InvalidateAll(&mut self)
    {
        self.grid_cache.clear();
        self.geometry_cache.clear();
    }
}

//-------------------------------------------------------------------------------------------------

/// Canvas interaction state tracking drag gestures and mouse state.
#[derive(Debug, Default)]
pub struct SketchCanvasInteraction
{
    pub is_panning:   bool,
    pub pan_start:    Option<Point>,
    pub click_origin: Option<Point>,
    pub is_dragging:  bool,
}

//-------------------------------------------------------------------------------------------------

/// Action emitted by the sketch canvas widget.
#[derive(Clone, Debug, PartialEq)]
pub enum SketchCanvasAction
{
    Pan(f32, f32),
    Zoom(f32, Option<[f32; 2]>),
    Input(SketchInputEvent),
    Resize(f32, f32),
}

//-------------------------------------------------------------------------------------------------

/// 3-Layer cached canvas program implementing Iced `canvas::Program`.
pub struct SketchCanvasProgram<'a, Message, F>
{
    pub doc:          &'a SketchDocument,
    pub ctrl:         &'a SketchToolController,
    pub canvas_state: &'a SketchCanvasState,
    pub zoom:         f32,
    pub pan:          [f32; 2],
    pub palette:      ThemePalette,
    pub map_action:   F,
    pub _phantom:     std::marker::PhantomData<Message>,
}

impl<'a, Message: 'static + Clone, F: Fn(SketchCanvasAction) -> Message + Copy + 'static>
    Program<Message> for SketchCanvasProgram<'a, Message, F>
{
    type State = SketchCanvasInteraction;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry>
    {
        let zoom = self.zoom;
        let pan = self.pan;
        let palette = self.palette;
        let doc = self.doc;
        let ctrl = self.ctrl;

        // Layer 1: Background & Adaptive Grid Cache
        let grid_geom = self.canvas_state.grid_cache.draw(renderer, bounds.size(), |frame| {
            // Background fill
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), palette.content_bg);

            // Origin crosshair & axes
            let origin_x = pan[0];
            let origin_y = pan[1];

            // Draw X and Y axes
            let axis_color = Color::from_rgba(palette.border.r, palette.border.g, palette.border.b, 0.45);
            let axis_stroke = Stroke::default().with_color(axis_color).with_width(1.0);

            if origin_y >= 0.0 && origin_y <= bounds.height {
                frame.stroke(
                    &Path::line(Point::new(0.0, origin_y), Point::new(bounds.width, origin_y)),
                    axis_stroke,
                );
            }
            if origin_x >= 0.0 && origin_x <= bounds.width {
                frame.stroke(
                    &Path::line(Point::new(origin_x, 0.0), Point::new(origin_x, bounds.height)),
                    axis_stroke,
                );
            }

            // Adaptive grid dots / lines
            let base_spacing = 50.0;
            let mut grid_step = base_spacing * zoom;
            while grid_step < 20.0 {
                grid_step *= 5.0;
            }
            while grid_step > 150.0 {
                grid_step /= 5.0;
            }

            if grid_step >= 10.0 {
                let dot_color = Color::from_rgba(palette.border.r, palette.border.g, palette.border.b, 0.25);
                let start_x = pan[0].rem_euclid(grid_step);
                let start_y = pan[1].rem_euclid(grid_step);

                let mut x = start_x;
                while x < bounds.width {
                    let mut y = start_y;
                    while y < bounds.height {
                        frame.fill_rectangle(Point::new(x, y), Size::new(1.5, 1.5), dot_color);
                        y += grid_step;
                    }
                    x += grid_step;
                }
            }

            // Origin circle
            let origin_marker_stroke = Stroke::default().with_color(palette.accent).with_width(1.5);
            frame.stroke(
                &Path::circle(Point::new(origin_x, origin_y), 4.0),
                origin_marker_stroke,
            );
        });

        // Layer 2: Committed Geometry Cache
        let geom = self.canvas_state.geometry_cache.draw(renderer, bounds.size(), |frame| {
            let arena = doc.PointArena();
            let zoom_f64 = zoom as f64;

            doc.Entities().Traverse(|entity| {
                let stroke_color = if entity.IsConstruction() {
                    Color::from_rgba(0.6, 0.6, 0.6, 0.5)
                } else {
                    palette.text_primary
                };
                let stroke_width = (1.5 * zoom).clamp(1.0, 4.0);
                let stroke = Stroke::default().with_color(stroke_color).with_width(stroke_width);

                match entity.Primitive() {
                    SketchPrimitive::Line(LineSegment { _Start, _End }) => {
                        let p1 = Point::new(
                            (_Start._X * zoom_f64 + pan[0] as f64) as f32,
                            (_Start._Y * zoom_f64 + pan[1] as f64) as f32,
                        );
                        let p2 = Point::new(
                            (_End._X * zoom_f64 + pan[0] as f64) as f32,
                            (_End._Y * zoom_f64 + pan[1] as f64) as f32,
                        );
                        frame.stroke(&Path::line(p1, p2), stroke);
                    }
                    SketchPrimitive::Circle(circle) => {
                        let pts = SketchTessellate::TessellateCircle(circle, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new(
                                    (p0._X * zoom_f64 + pan[0] as f64) as f32,
                                    (p0._Y * zoom_f64 + pan[1] as f64) as f32,
                                ));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new(
                                        (pt._X * zoom_f64 + pan[0] as f64) as f32,
                                        (pt._Y * zoom_f64 + pan[1] as f64) as f32,
                                    ));
                                    i += 1;
                                }
                                builder.close();
                            });
                            frame.stroke(&path, stroke);
                        }
                    }
                    SketchPrimitive::Arc(arc) => {
                        let pts = SketchTessellate::TessellateArc(arc, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new(
                                    (p0._X * zoom_f64 + pan[0] as f64) as f32,
                                    (p0._Y * zoom_f64 + pan[1] as f64) as f32,
                                ));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new(
                                        (pt._X * zoom_f64 + pan[0] as f64) as f32,
                                        (pt._Y * zoom_f64 + pan[1] as f64) as f32,
                                    ));
                                    i += 1;
                                }
                            });
                            frame.stroke(&path, stroke);
                        }
                    }
                    SketchPrimitive::Ellipse(ellipse) => {
                        let pts = SketchTessellate::TessellateEllipse(ellipse, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new(
                                    (p0._X * zoom_f64 + pan[0] as f64) as f32,
                                    (p0._Y * zoom_f64 + pan[1] as f64) as f32,
                                ));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new(
                                        (pt._X * zoom_f64 + pan[0] as f64) as f32,
                                        (pt._Y * zoom_f64 + pan[1] as f64) as f32,
                                    ));
                                    i += 1;
                                }
                                builder.close();
                            });
                            frame.stroke(&path, stroke);
                        }
                    }
                    SketchPrimitive::Polyline(PolylineHeader {
                        _PointOffset,
                        _PointCount,
                        _IsClosed,
                    }) => {
                        if *_PointCount >= 2 {
                            let path = Path::new(|builder| {
                                if let Some(p0) = arena.Get(*_PointOffset) {
                                    builder.move_to(Point::new(
                                        (p0._X * zoom_f64 + pan[0] as f64) as f32,
                                        (p0._Y * zoom_f64 + pan[1] as f64) as f32,
                                    ));
                                    let mut i = 1u32;
                                    while i < *_PointCount {
                                        if let Some(pt) = arena.Get(*_PointOffset + i) {
                                            builder.line_to(Point::new(
                                                (pt._X * zoom_f64 + pan[0] as f64) as f32,
                                                (pt._Y * zoom_f64 + pan[1] as f64) as f32,
                                            ));
                                        }
                                        i += 1;
                                    }
                                    if *_IsClosed {
                                        builder.close();
                                    }
                                }
                            });
                            frame.stroke(&path, stroke);
                        }
                    }
                }
            });
        });

        // Layer 3: Dynamic Interactive Overlay (Zero-Cache, Instant Previews & Grips)
        let mut overlay_frame = Frame::new(renderer, bounds.size());
        let zoom_f64 = zoom as f64;

        // Selection highlight halo
        let sel_id = ctrl.SelectedEntityId();
        if sel_id != u32::MAX {
            if let Some(sel_ent) = doc.FindEntity(sel_id) {
                let sel_color = palette.accent;
                let sel_stroke = Stroke::default().with_color(sel_color).with_width(3.0);
                match sel_ent.Primitive() {
                    SketchPrimitive::Line(LineSegment { _Start, _End }) => {
                        let p1 = Point::new(
                            (_Start._X * zoom_f64 + pan[0] as f64) as f32,
                            (_Start._Y * zoom_f64 + pan[1] as f64) as f32,
                        );
                        let p2 = Point::new(
                            (_End._X * zoom_f64 + pan[0] as f64) as f32,
                            (_End._Y * zoom_f64 + pan[1] as f64) as f32,
                        );
                        overlay_frame.stroke(&Path::line(p1, p2), sel_stroke);
                    }
                    SketchPrimitive::Circle(circle) => {
                        let pts = SketchTessellate::TessellateCircle(circle, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new(
                                    (p0._X * zoom_f64 + pan[0] as f64) as f32,
                                    (p0._Y * zoom_f64 + pan[1] as f64) as f32,
                                ));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new(
                                        (pt._X * zoom_f64 + pan[0] as f64) as f32,
                                        (pt._Y * zoom_f64 + pan[1] as f64) as f32,
                                    ));
                                    i += 1;
                                }
                                builder.close();
                            });
                            overlay_frame.stroke(&path, sel_stroke);
                        }
                    }
                    _ => {}
                }
            }

            // Draw active grip handles
            let grips = ctrl.ActiveGrips(doc);
            grips.Arr().Traverse(|grip: &GripInfo| {
                let sx = (grip.pos._X * zoom_f64 + pan[0] as f64) as f32;
                let sy = (grip.pos._Y * zoom_f64 + pan[1] as f64) as f32;
                let grip_size = 7.0;
                let rect = Rectangle {
                    x:      sx - grip_size * 0.5,
                    y:      sy - grip_size * 0.5,
                    width:  grip_size,
                    height: grip_size,
                };
                overlay_frame.fill_rectangle(
                    Point::new(rect.x, rect.y),
                    Size::new(rect.width, rect.height),
                    palette.content_bg,
                );
                overlay_frame.stroke(
                    &Path::rectangle(Point::new(rect.x, rect.y), Size::new(rect.width, rect.height)),
                    Stroke::default().with_color(palette.accent).with_width(1.5),
                );
            });
        }

        // Draft geometry preview
        if let Some(draft) = ctrl.DraftGeometry() {
            let draft_stroke = Stroke::default().with_color(palette.accent).with_width(2.0);

            if let Some(prim) = &draft.primitive {
                match prim {
                    DraftPrimitive::Line { start, end } => {
                        let p1 = Point::new(
                            (start._X * zoom_f64 + pan[0] as f64) as f32,
                            (start._Y * zoom_f64 + pan[1] as f64) as f32,
                        );
                        let p2 = Point::new(
                            (end._X * zoom_f64 + pan[0] as f64) as f32,
                            (end._Y * zoom_f64 + pan[1] as f64) as f32,
                        );
                        overlay_frame.stroke(&Path::line(p1, p2), draft_stroke);
                    }
                    DraftPrimitive::Circle { center, radius } => {
                        let c = Circle::New(*center, *radius);
                        let pts = SketchTessellate::TessellateCircle(&c, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new(
                                    (p0._X * zoom_f64 + pan[0] as f64) as f32,
                                    (p0._Y * zoom_f64 + pan[1] as f64) as f32,
                                ));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new(
                                        (pt._X * zoom_f64 + pan[0] as f64) as f32,
                                        (pt._Y * zoom_f64 + pan[1] as f64) as f32,
                                    ));
                                    i += 1;
                                }
                                builder.close();
                            });
                            overlay_frame.stroke(&path, draft_stroke);
                        }
                    }
                    DraftPrimitive::Arc {
                        center,
                        radius,
                        start_angle,
                        sweep_angle,
                    } => {
                        let arc = CircularArc::New(*center, *radius, *start_angle, *sweep_angle);
                        let pts = SketchTessellate::TessellateArc(&arc, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new(
                                    (p0._X * zoom_f64 + pan[0] as f64) as f32,
                                    (p0._Y * zoom_f64 + pan[1] as f64) as f32,
                                ));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new(
                                        (pt._X * zoom_f64 + pan[0] as f64) as f32,
                                        (pt._Y * zoom_f64 + pan[1] as f64) as f32,
                                    ));
                                    i += 1;
                                }
                            });
                            overlay_frame.stroke(&path, draft_stroke);
                        }
                    }
                    DraftPrimitive::Ellipse {
                        center,
                        semi_major,
                        semi_minor,
                        rotation,
                    } => {
                        let ell = Ellipse::New(*center, *semi_major, *semi_minor, *rotation);
                        let pts = SketchTessellate::TessellateEllipse(&ell, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new(
                                    (p0._X * zoom_f64 + pan[0] as f64) as f32,
                                    (p0._Y * zoom_f64 + pan[1] as f64) as f32,
                                ));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new(
                                        (pt._X * zoom_f64 + pan[0] as f64) as f32,
                                        (pt._Y * zoom_f64 + pan[1] as f64) as f32,
                                    ));
                                    i += 1;
                                }
                                builder.close();
                            });
                            overlay_frame.stroke(&path, draft_stroke);
                        }
                    }
                    DraftPrimitive::Polyline { points, closed } => {
                        if points.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = points.Arr().Get(0).unwrap();
                                builder.move_to(Point::new(
                                    (p0._X * zoom_f64 + pan[0] as f64) as f32,
                                    (p0._Y * zoom_f64 + pan[1] as f64) as f32,
                                ));
                                let mut i = 1u32;
                                while i < points.Len() {
                                    let pt = points.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new(
                                        (pt._X * zoom_f64 + pan[0] as f64) as f32,
                                        (pt._Y * zoom_f64 + pan[1] as f64) as f32,
                                    ));
                                    i += 1;
                                }
                                if *closed {
                                    builder.close();
                                }
                            });
                            overlay_frame.stroke(&path, draft_stroke);
                        }
                    }
                }
            }

            // Marquee selection box
            if let Some((start, end)) = draft.marquee {
                let sx = (start._X * zoom_f64 + pan[0] as f64) as f32;
                let sy = (start._Y * zoom_f64 + pan[1] as f64) as f32;
                let ex = (end._X * zoom_f64 + pan[0] as f64) as f32;
                let ey = (end._Y * zoom_f64 + pan[1] as f64) as f32;

                let min_x = sx.min(ex);
                let min_y = sy.min(ey);
                let w = (sx - ex).abs();
                let h = (sy - ey).abs();

                let marquee_fill = Color::from_rgba(palette.accent.r, palette.accent.g, palette.accent.b, 0.15);
                let marquee_stroke = Stroke::default().with_color(palette.accent).with_width(1.0);

                overlay_frame.fill_rectangle(Point::new(min_x, min_y), Size::new(w, h), marquee_fill);
                overlay_frame.stroke(
                    &Path::rectangle(Point::new(min_x, min_y), Size::new(w, h)),
                    marquee_stroke,
                );
            }

            // Floating dimension badge
            if let (Some(text), Some(pos)) = (&draft.dimension_badge, &draft.dimension_pos) {
                let sx = (pos._X * zoom_f64 + pan[0] as f64) as f32 + 15.0;
                let sy = (pos._Y * zoom_f64 + pan[1] as f64) as f32 - 20.0;

                let badge_bg = Color::from_rgba(palette.toolbar_bg.r, palette.toolbar_bg.g, palette.toolbar_bg.b, 0.9);
                overlay_frame.fill_rectangle(
                    Point::new(sx - 4.0, sy - 2.0),
                    Size::new(120.0, 20.0),
                    badge_bg,
                );
                overlay_frame.stroke(
                    &Path::rectangle(Point::new(sx - 4.0, sy - 2.0), Size::new(120.0, 20.0)),
                    Stroke::default().with_color(palette.border).with_width(1.0),
                );
                overlay_frame.fill_text(Text {
                    content:  text.clone(),
                    position: Point::new(sx, sy),
                    color:    palette.text_primary,
                    size:     11.0.into(),
                    ..Default::default()
                });
            }
        }

        // Active snap glyph indicator
        if let Some(snap) = ctrl.ActiveSnap() {
            let sx = (snap._Point._X * zoom_f64 + pan[0] as f64) as f32;
            let sy = (snap._Point._Y * zoom_f64 + pan[1] as f64) as f32;
            let glyph_color = Color::from_rgba(0.2, 0.9, 0.4, 0.95);
            let snap_stroke = Stroke::default().with_color(glyph_color).with_width(2.0);

            match snap._Kind {
                SnapKind::Endpoint => {
                    overlay_frame.stroke(
                        &Path::rectangle(Point::new(sx - 4.0, sy - 4.0), Size::new(8.0, 8.0)),
                        snap_stroke,
                    );
                }
                SnapKind::Center => {
                    overlay_frame.stroke(&Path::circle(Point::new(sx, sy), 5.0), snap_stroke);
                }
                SnapKind::Midpoint => {
                    let tri = Path::new(|b| {
                        b.move_to(Point::new(sx, sy - 5.0));
                        b.line_to(Point::new(sx + 5.0, sy + 4.0));
                        b.line_to(Point::new(sx - 5.0, sy + 4.0));
                        b.close();
                    });
                    overlay_frame.stroke(&tri, snap_stroke);
                }
                SnapKind::Quadrant => {
                    let diamond = Path::new(|b| {
                        b.move_to(Point::new(sx, sy - 5.0));
                        b.line_to(Point::new(sx + 5.0, sy));
                        b.line_to(Point::new(sx, sy + 5.0));
                        b.line_to(Point::new(sx - 5.0, sy));
                        b.close();
                    });
                    overlay_frame.stroke(&diamond, snap_stroke);
                }
                SnapKind::Grid => {
                    overlay_frame.fill_rectangle(
                        Point::new(sx - 2.0, sy - 2.0),
                        Size::new(4.0, 4.0),
                        glyph_color,
                    );
                }
            }
        }

        vec![grid_geom, geom, overlay_frame.into_geometry()]
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
        let publish = |action| Some(iced::widget::Action::publish((self.map_action)(action)).and_capture());

        let zoom = self.zoom as f64;
        let pan = self.pan;

        let screen_to_model = |p: Point| -> SketchPoint {
            SketchPoint::New(
                (p.x as f64 - pan[0] as f64) / zoom,
                (p.y as f64 - pan[1] as f64) / zoom,
            )
        };

        match event {
            // Mouse wheel zoom centered on cursor
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) if is_over => {
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 25.0,
                };
                let zoom_factor = if lines > 0.0 { 1.15 } else { 1.0 / 1.15 };
                let focal = cursor.position_in(bounds).map(|p| [p.x, p.y]);
                return publish(SketchCanvasAction::Zoom(zoom_factor, focal));
            }

            // Mouse button down
            iced::Event::Mouse(mouse::Event::ButtonPressed(button)) if is_over => {
                if let Some(pos) = cursor.position_in(bounds) {
                    if *button == mouse::Button::Middle {
                        state.is_panning = true;
                        state.pan_start = Some(pos);
                        return Some(iced::widget::Action::capture());
                    }

                    if *button == mouse::Button::Left {
                        state.is_dragging = true;
                        state.click_origin = Some(pos);
                        let model_pt = screen_to_model(pos);
                        let input = SketchInputEvent::PointerDown {
                            pos:       model_pt,
                            button:    PointerButton::Left,
                            modifiers: KeyModifiers::default(),
                        };
                        return publish(SketchCanvasAction::Input(input));
                    }
                }
            }

            // Mouse move
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    if state.is_panning {
                        if let Some(prev) = state.pan_start {
                            let dx = pos.x - prev.x;
                            let dy = pos.y - prev.y;
                            state.pan_start = Some(pos);
                            return publish(SketchCanvasAction::Pan(dx, dy));
                        }
                    } else if state.is_dragging || is_over {
                        let model_pt = screen_to_model(pos);
                        let input = SketchInputEvent::PointerMove {
                            pos:       model_pt,
                            modifiers: KeyModifiers::default(),
                        };
                        return publish(SketchCanvasAction::Input(input));
                    }
                }
            }

            // Mouse button released
            iced::Event::Mouse(mouse::Event::ButtonReleased(button)) => {
                if *button == mouse::Button::Middle {
                    state.is_panning = false;
                    state.pan_start = None;
                    return Some(iced::widget::Action::capture());
                }

                if *button == mouse::Button::Left && state.is_dragging {
                    state.is_dragging = false;
                    if let Some(pos) = cursor.position_in(bounds) {
                        let model_pt = screen_to_model(pos);
                        let input = SketchInputEvent::PointerUp {
                            pos:       model_pt,
                            button:    PointerButton::Left,
                            modifiers: KeyModifiers::default(),
                        };
                        return publish(SketchCanvasAction::Input(input));
                    }
                }
            }

            // Window resize detection
            iced::Event::Window(iced::window::Event::RedrawRequested(_))
                if bounds.width > 0.0 && self.canvas_state.last_size != [bounds.width, bounds.height] =>
            {
                return publish(SketchCanvasAction::Resize(bounds.width, bounds.height));
            }

            _ => {}
        }

        None
    }
}

//-------------------------------------------------------------------------------------------------
