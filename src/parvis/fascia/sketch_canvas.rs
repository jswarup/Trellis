//-- sketch_canvas.rs -----------------------------------------------------------------------------------------------
use crate::fascia::sketch_tool::{
    DraftPrimitive, GripInfo, KeyModifiers, NumericFieldKind, PointerButton, SketchInputEvent,
    SketchToolController, SketchToolKind,
};
use crate::fascia::theme::ThemePalette;
use crate::fenst::sketch::{
    Circle, CircularArc, Ellipse, LineSegment, PolylineHeader, SketchDocument, SketchPoint,
    SketchPrimitive,
};
use crate::fleck::sketch_query::SnapKind;
use crate::fleck::sketch_tessellate::SketchTessellate;
use crate::silo::{Buff, IArr, Stash, USeg};
use iced::mouse;
use iced::widget::canvas::{Cache, Frame, Geometry, Path, Program, Stroke, Text};
use iced::{Color, Point, Rectangle, Size};

//-------------------------------------------------------------------------------------------------

/// Represents a cached polyline tessellation for a curved sketch primitive at a specific zoom bucket.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveCacheEntry
{
    pub _EntityId:   u32,
    pub _ZoomBucket: i32,
    pub _Points:     Buff<SketchPoint>,
}

impl CurveCacheEntry
{
    pub fn New(entityId: u32, zoomBucket: i32, points: Buff<SketchPoint>) -> Self
    {
        Self { _EntityId:   entityId,
               _ZoomBucket: zoomBucket,
               _Points:     points, }
    }
}

//-------------------------------------------------------------------------------------------------

/// State managing Iced canvas cache invalidation layers and interaction state.
#[derive(Debug, Default)]
pub struct SketchCanvasState
{
    _GridCache:     Cache,
    _GeometryCache: Cache,
    _CurveCache:    Buff<CurveCacheEntry>,
    _LastRevision:  u64,
    _LastZoom:      f32,
    _LastPan:       [f32; 2],
    _LastSize:      [f32; 2],
}

impl SketchCanvasState
{
    pub fn New() -> Self
    {
        Self { _GridCache:     Cache::default(),
               _GeometryCache: Cache::default(),
               _CurveCache:    Buff::New(),
               _LastRevision:  u64::MAX,
               _LastZoom:      0.0,
               _LastPan:       [0.0, 0.0],
               _LastSize:      [0.0, 0.0], }
    }

    #[inline]
    pub fn LastSize(&self) -> [f32; 2] { self._LastSize }

    #[inline]
    pub fn SetLastSize(&mut self, size: [f32; 2]) { self._LastSize = size; }

    #[inline]
    pub fn LastRevision(&self) -> u64 { self._LastRevision }

    #[inline]
    pub fn SetLastRevision(&mut self, rev: u64) { self._LastRevision = rev; }

    #[inline]
    pub fn GridCache(&self) -> &Cache { &self._GridCache }

    #[inline]
    pub fn GeometryCache(&self) -> &Cache { &self._GeometryCache }

    #[inline]
    pub fn CurveCacheCount(&self) -> u32 { self._CurveCache.Len() }

    pub fn FindCachedCurve(&self, entityId: u32, zoomBucket: i32) -> Option<&Buff<SketchPoint>>
    {
        let arr = self._CurveCache.Arr();
        let seg = USeg::FromLen(arr.Len());
        let pos = seg.BinarySearch(|idx| {
            let entry = arr.Get(idx).unwrap();
            entry._EntityId.cmp(&entityId)
        });
        match pos {
            Ok(idx) => {
                let entry = arr.Get(idx).unwrap();
                if entry._ZoomBucket == zoomBucket {
                    Some(&entry._Points)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    pub fn InsertCachedCurve(&mut self, entityId: u32, zoomBucket: i32, pts: Buff<SketchPoint>)
    {
        let arr = self._CurveCache.Arr();
        let seg = USeg::FromLen(arr.Len());
        let pos = seg.BinarySearch(|idx| {
            let entry = arr.Get(idx).unwrap();
            entry._EntityId.cmp(&entityId)
        });
        let mut stash = Stash::WithCapacity(arr.Len() + 1);
        match pos {
            Ok(idx) => {
                USeg::FromLen(arr.Len()).Traverse(|i| {
                    if i == idx {
                        stash.Push(CurveCacheEntry::New(entityId, zoomBucket, pts.clone()));
                    } else {
                        stash.Push(arr.Get(i).unwrap().clone());
                    }
                });
            }
            Err(insertIdx) => {
                USeg::FromLen(insertIdx).Traverse(|i| {
                    stash.Push(arr.Get(i).unwrap().clone());
                });
                stash.Push(CurveCacheEntry::New(entityId, zoomBucket, pts));
                let tailLen = arr.Len() - insertIdx;
                USeg::FromLen(tailLen).Traverse(|i| {
                    stash.Push(arr.Get(insertIdx + i).unwrap().clone());
                });
            }
        }
        self._CurveCache = stash.IntoBuff();
    }

    pub fn InvalidateGrid(&mut self) { self._GridCache.clear(); }

    pub fn InvalidateGeometry(&mut self)
    {
        self._GeometryCache.clear();
        self._CurveCache = Buff::New();
    }

    pub fn InvalidateAll(&mut self)
    {
        self._GridCache.clear();
        self._GeometryCache.clear();
    }
}

//-------------------------------------------------------------------------------------------------

/// Canvas interaction state tracking drag gestures and mouse state.
#[derive(Debug, Default)]
pub struct SketchCanvasInteraction
{
    pub is_panning:     bool,
    pub pan_start:      Option<Point>,
    pub click_origin:   Option<Point>,
    pub is_dragging:    bool,
    pub numeric_buffer: String,
    pub numeric_field:  NumericFieldKind,
    _KeyboardFocus:     bool,
    _Modifiers:         KeyModifiers,
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
    Undo,
    Redo,
    Save,
    Tool(SketchToolKind),
    ContextMenu([f32; 2]),
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

    fn draw(&self, _state: &Self::State, renderer: &iced::Renderer, _theme: &iced::Theme,
            bounds: Rectangle, _cursor: mouse::Cursor)
            -> Vec<Geometry>
    {
        let zoom = self.zoom;
        let pan = self.pan;
        let palette = self.palette;
        let doc = self.doc;
        let ctrl = self.ctrl;

        // Layer 1: Background & Adaptive Grid Cache
        let grid_geom =
            self.canvas_state
                .GridCache()
                .draw(renderer, bounds.size(), |frame| {
                    // Background fill
                    frame.fill_rectangle(Point::ORIGIN, bounds.size(), palette.content_bg);

                    // Origin crosshair & axes
                    let origin_x = pan[0];
                    let origin_y = pan[1];

                    // Draw X and Y axes
                    let axis_color = Color::from_rgba(palette.border.r,
                                                      palette.border.g,
                                                      palette.border.b,
                                                      0.45);
                    let axis_stroke = Stroke::default().with_color(axis_color).with_width(1.0);

                    if origin_y >= 0.0 && origin_y <= bounds.height {
                        frame.stroke(&Path::line(Point::new(0.0, origin_y),
                                                 Point::new(bounds.width, origin_y)),
                                     axis_stroke);
                    }
                    if origin_x >= 0.0 && origin_x <= bounds.width {
                        frame.stroke(&Path::line(Point::new(origin_x, 0.0),
                                                 Point::new(origin_x, bounds.height)),
                                     axis_stroke);
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
                        let dot_color = Color::from_rgba(palette.border.r,
                                                         palette.border.g,
                                                         palette.border.b,
                                                         0.25);
                        let start_x = pan[0].rem_euclid(grid_step);
                        let start_y = pan[1].rem_euclid(grid_step);

                        let mut x = start_x;
                        while x < bounds.width {
                            let mut y = start_y;
                            while y < bounds.height {
                                frame.fill_rectangle(Point::new(x, y),
                                                     Size::new(1.5, 1.5),
                                                     dot_color);
                                y += grid_step;
                            }
                            x += grid_step;
                        }
                    }

                    // Origin circle
                    let origin_marker_stroke =
                        Stroke::default().with_color(palette.accent).with_width(1.5);
                    frame.stroke(&Path::circle(Point::new(origin_x, origin_y), 4.0),
                                 origin_marker_stroke);
                });

        // Layer 2: Committed Geometry Cache
        let geom = self.canvas_state.GeometryCache().draw(renderer, bounds.size(), |frame| {
            let arena = doc.PointArena();
            let zoom_f64 = zoom as f64;

            doc.Entities().Traverse(|entity| {
                let mut visible = false;
                let mut construction = entity.IsConstruction();
                doc.Layers().Traverse(|layer| {
                    if layer.Id() == entity.LayerId() {
                        visible = layer.IsVisible();
                        construction |= layer.IsConstruction();
                    }
                });
                if !visible { return; }
                let stroke_color = if construction {
                    palette.text_secondary
                } else {
                    palette.text_primary
                };
                let stroke_width = if construction { 1.0 } else { 1.5 };
                let stroke = Stroke::default().with_color(stroke_color).with_width(stroke_width);
                let stroke = if construction {
                    Stroke { line_dash: iced::widget::canvas::LineDash { segments: &[6.0, 4.0], offset: 0 }, ..stroke }
                } else { stroke };

                let zoom_bucket = (zoom_f64.max(0.001).log2().floor()) as i32;
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
                        let pts = match self.canvas_state.FindCachedCurve(entity.Id(), zoom_bucket) {
                            Some(cached) => cached.clone(),
                            None => SketchTessellate::TessellateCircle(circle, 0.5, zoom_f64),
                        };
                        Self::draw_polyline_pts(frame, &pts, zoom_f64, pan, stroke, true);
                    }
                    SketchPrimitive::Arc(arc) => {
                        let pts = match self.canvas_state.FindCachedCurve(entity.Id(), zoom_bucket) {
                            Some(cached) => cached.clone(),
                            None => SketchTessellate::TessellateArc(arc, 0.5, zoom_f64),
                        };
                        Self::draw_polyline_pts(frame, &pts, zoom_f64, pan, stroke, false);
                    }
                    SketchPrimitive::Ellipse(ellipse) => {
                        let pts = match self.canvas_state.FindCachedCurve(entity.Id(), zoom_bucket) {
                            Some(cached) => cached.clone(),
                            None => SketchTessellate::TessellateEllipse(ellipse, 0.5, zoom_f64),
                        };
                        Self::draw_polyline_pts(frame, &pts, zoom_f64, pan, stroke, ellipse.IsClosed());
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

            // Draw Engineering Dimensions & Annotations
            let sel_dim_id = ctrl.SelectedDimensionId();
            doc.Dimensions().Traverse(|dim| {
                Self::draw_dimension(frame, dim, zoom_f64, pan, palette, doc.Units(), dim._Id == sel_dim_id);
            });
        });

        // Layer 3: Dynamic Interactive Overlay (Zero-Cache, Instant Previews & Grips)
        let mut overlay_frame = Frame::new(renderer, bounds.size());
        let zoom_f64 = zoom as f64;

        // Selection highlight halo
        let sel_id = ctrl.SelectedEntityId();
        if sel_id != u32::MAX {
            if let Some(sel_ent) = doc.FindEntity(sel_id)
               && SketchToolController::IsLayerEditable(doc, sel_ent.LayerId())
            {
                let sel_color = palette.accent;
                let sel_stroke = Stroke::default().with_color(sel_color).with_width(3.0);
                match sel_ent.Primitive() {
                    SketchPrimitive::Line(LineSegment { _Start, _End }) => {
                        let p1 = Point::new((_Start._X * zoom_f64 + pan[0] as f64) as f32,
                                            (_Start._Y * zoom_f64 + pan[1] as f64) as f32);
                        let p2 = Point::new((_End._X * zoom_f64 + pan[0] as f64) as f32,
                                            (_End._Y * zoom_f64 + pan[1] as f64) as f32);
                        overlay_frame.stroke(&Path::line(p1, p2), sel_stroke);
                    }
                    SketchPrimitive::Circle(circle) => {
                        let pts = SketchTessellate::TessellateCircle(circle, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new((p0._X * zoom_f64 + pan[0] as f64)
                                                           as f32,
                                                           (p0._Y * zoom_f64 + pan[1] as f64)
                                                           as f32));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new((pt._X * zoom_f64 + pan[0] as f64)
                                                               as f32,
                                                               (pt._Y * zoom_f64 + pan[1] as f64)
                                                               as f32));
                                    i += 1;
                                }
                                builder.close();
                            });
                            overlay_frame.stroke(&path, sel_stroke);
                        }
                    }
                    SketchPrimitive::Arc(arc) => {
                        let pts = SketchTessellate::TessellateArc(arc, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new((p0._X * zoom_f64 + pan[0] as f64)
                                                           as f32,
                                                           (p0._Y * zoom_f64 + pan[1] as f64)
                                                           as f32));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new((pt._X * zoom_f64 + pan[0] as f64)
                                                               as f32,
                                                               (pt._Y * zoom_f64 + pan[1] as f64)
                                                               as f32));
                                    i += 1;
                                }
                            });
                            overlay_frame.stroke(&path, sel_stroke);
                        }
                    }
                    SketchPrimitive::Ellipse(ellipse) => {
                        let pts = SketchTessellate::TessellateEllipse(ellipse, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new((p0._X * zoom_f64 + pan[0] as f64)
                                                           as f32,
                                                           (p0._Y * zoom_f64 + pan[1] as f64)
                                                           as f32));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new((pt._X * zoom_f64 + pan[0] as f64)
                                                               as f32,
                                                               (pt._Y * zoom_f64 + pan[1] as f64)
                                                               as f32));
                                    i += 1;
                                }
                                if ellipse.IsClosed() {
                                    builder.close();
                                }
                            });
                            overlay_frame.stroke(&path, sel_stroke);
                        }
                    }
                    SketchPrimitive::Polyline(header) => {
                        let arena = doc.PointArena();
                        if header._PointCount >= 2 {
                            let path = Path::new(|builder| {
                                if let Some(p0) = arena.Get(header._PointOffset) {
                                    builder.move_to(Point::new((p0._X * zoom_f64 + pan[0] as f64)
                                                               as f32,
                                                               (p0._Y * zoom_f64 + pan[1] as f64)
                                                               as f32));
                                    let mut i = 1u32;
                                    while i < header._PointCount {
                                        if let Some(pt) = arena.Get(header._PointOffset + i) {
                                            builder.line_to(Point::new((pt._X * zoom_f64
                                                                        + pan[0] as f64)
                                                                       as f32,
                                                                       (pt._Y * zoom_f64
                                                                        + pan[1] as f64)
                                                                       as f32));
                                        }
                                        i += 1;
                                    }
                                    if header._IsClosed {
                                        builder.close();
                                    }
                                }
                            });
                            overlay_frame.stroke(&path, sel_stroke);
                        }
                    }
                }
            }

            // Draw active grip handles
            let grips = ctrl.ActiveGrips(doc);
            grips.Arr().Traverse(|grip: &GripInfo| {
                           let sx = (grip.pos._X * zoom_f64 + pan[0] as f64) as f32;
                           let sy = (grip.pos._Y * zoom_f64 + pan[1] as f64) as f32;
                           let grip_size = 7.0;
                           let rect = Rectangle { x:      sx - grip_size * 0.5,
                                                  y:      sy - grip_size * 0.5,
                                                  width:  grip_size,
                                                  height: grip_size, };
                           overlay_frame.fill_rectangle(Point::new(rect.x, rect.y),
                                                        Size::new(rect.width, rect.height),
                                                        palette.content_bg);
                           overlay_frame.stroke(&Path::rectangle(Point::new(rect.x, rect.y),
                                                                 Size::new(rect.width,
                                                                           rect.height)),
                                                Stroke::default().with_color(palette.accent)
                                                                 .with_width(1.5));
                       });
        }

        // Draft geometry preview
        if let Some(draft) = ctrl.DraftGeometry() {
            let draft_stroke = Stroke::default().with_color(palette.accent).with_width(2.0);

            if let Some(prim) = &draft.primitive {
                match prim {
                    DraftPrimitive::Line { start, end } => {
                        let p1 = Point::new((start._X * zoom_f64 + pan[0] as f64) as f32,
                                            (start._Y * zoom_f64 + pan[1] as f64) as f32);
                        let p2 = Point::new((end._X * zoom_f64 + pan[0] as f64) as f32,
                                            (end._Y * zoom_f64 + pan[1] as f64) as f32);
                        overlay_frame.stroke(&Path::line(p1, p2), draft_stroke);
                    }
                    DraftPrimitive::Circle { center, radius } => {
                        let c = Circle::New(*center, *radius);
                        let pts = SketchTessellate::TessellateCircle(&c, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new((p0._X * zoom_f64 + pan[0] as f64)
                                                           as f32,
                                                           (p0._Y * zoom_f64 + pan[1] as f64)
                                                           as f32));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new((pt._X * zoom_f64 + pan[0] as f64)
                                                               as f32,
                                                               (pt._Y * zoom_f64 + pan[1] as f64)
                                                               as f32));
                                    i += 1;
                                }
                                builder.close();
                            });
                            overlay_frame.stroke(&path, draft_stroke);
                        }
                    }
                    DraftPrimitive::Arc { center,
                                          radius,
                                          start_angle,
                                          sweep_angle, } => {
                        let arc = CircularArc::New(*center, *radius, *start_angle, *sweep_angle);
                        let pts = SketchTessellate::TessellateArc(&arc, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new((p0._X * zoom_f64 + pan[0] as f64)
                                                           as f32,
                                                           (p0._Y * zoom_f64 + pan[1] as f64)
                                                           as f32));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new((pt._X * zoom_f64 + pan[0] as f64)
                                                               as f32,
                                                               (pt._Y * zoom_f64 + pan[1] as f64)
                                                               as f32));
                                    i += 1;
                                }
                            });
                            overlay_frame.stroke(&path, draft_stroke);
                        }
                    }
                    DraftPrimitive::Ellipse { center,
                                              semi_major,
                                              semi_minor,
                                              rotation,
                                              start_angle,
                                              sweep_angle, } => {
                        let ell = Ellipse::New(*center, *semi_major, *semi_minor, *rotation).WithArc(*start_angle, *sweep_angle);
                        let pts = SketchTessellate::TessellateEllipse(&ell, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new((p0._X * zoom_f64 + pan[0] as f64)
                                                           as f32,
                                                           (p0._Y * zoom_f64 + pan[1] as f64)
                                                           as f32));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new((pt._X * zoom_f64 + pan[0] as f64)
                                                               as f32,
                                                               (pt._Y * zoom_f64 + pan[1] as f64)
                                                               as f32));
                                    i += 1;
                                }
                                if ell.IsClosed() {
                                    builder.close();
                                }
                            });
                            overlay_frame.stroke(&path, draft_stroke);
                        }
                    }
                    DraftPrimitive::Polyline { points, closed } => {
                        if points.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = points.Arr().Get(0).unwrap();
                                builder.move_to(Point::new((p0._X * zoom_f64 + pan[0] as f64)
                                                           as f32,
                                                           (p0._Y * zoom_f64 + pan[1] as f64)
                                                           as f32));
                                let mut i = 1u32;
                                while i < points.Len() {
                                    let pt = points.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new((pt._X * zoom_f64 + pan[0] as f64)
                                                               as f32,
                                                               (pt._Y * zoom_f64 + pan[1] as f64)
                                                               as f32));
                                    i += 1;
                                }
                                if *closed {
                                    builder.close();
                                }
                            });
                            overlay_frame.stroke(&path, draft_stroke);
                        }
                    }
                    DraftPrimitive::FilletPreview { trimmed_l1,
                                                    trimmed_l2,
                                                    arc, } => {
                        let p1_start =
                            Point::new((trimmed_l1._Start._X * zoom_f64 + pan[0] as f64) as f32,
                                       (trimmed_l1._Start._Y * zoom_f64 + pan[1] as f64) as f32);
                        let p1_end =
                            Point::new((trimmed_l1._End._X * zoom_f64 + pan[0] as f64) as f32,
                                       (trimmed_l1._End._Y * zoom_f64 + pan[1] as f64) as f32);
                        let p2_start =
                            Point::new((trimmed_l2._Start._X * zoom_f64 + pan[0] as f64) as f32,
                                       (trimmed_l2._Start._Y * zoom_f64 + pan[1] as f64) as f32);
                        let p2_end =
                            Point::new((trimmed_l2._End._X * zoom_f64 + pan[0] as f64) as f32,
                                       (trimmed_l2._End._Y * zoom_f64 + pan[1] as f64) as f32);
                        overlay_frame.stroke(&Path::line(p1_start, p1_end), draft_stroke);
                        overlay_frame.stroke(&Path::line(p2_start, p2_end), draft_stroke);

                        let pts = SketchTessellate::TessellateArc(arc, 0.5, zoom_f64);
                        if pts.Len() >= 2 {
                            let path = Path::new(|builder| {
                                let p0 = pts.Arr().Get(0).unwrap();
                                builder.move_to(Point::new((p0._X * zoom_f64 + pan[0] as f64)
                                                           as f32,
                                                           (p0._Y * zoom_f64 + pan[1] as f64)
                                                           as f32));
                                let mut i = 1u32;
                                while i < pts.Len() {
                                    let pt = pts.Arr().Get(i).unwrap();
                                    builder.line_to(Point::new((pt._X * zoom_f64 + pan[0] as f64)
                                                               as f32,
                                                               (pt._Y * zoom_f64 + pan[1] as f64)
                                                               as f32));
                                    i += 1;
                                }
                            });
                            overlay_frame.stroke(&path, draft_stroke);
                        }
                    }
                    DraftPrimitive::ChamferPreview { trimmed_l1,
                                                     trimmed_l2,
                                                     chamfer_line, } => {
                        let p1_start =
                            Point::new((trimmed_l1._Start._X * zoom_f64 + pan[0] as f64) as f32,
                                       (trimmed_l1._Start._Y * zoom_f64 + pan[1] as f64) as f32);
                        let p1_end =
                            Point::new((trimmed_l1._End._X * zoom_f64 + pan[0] as f64) as f32,
                                       (trimmed_l1._End._Y * zoom_f64 + pan[1] as f64) as f32);
                        let p2_start =
                            Point::new((trimmed_l2._Start._X * zoom_f64 + pan[0] as f64) as f32,
                                       (trimmed_l2._Start._Y * zoom_f64 + pan[1] as f64) as f32);
                        let p2_end =
                            Point::new((trimmed_l2._End._X * zoom_f64 + pan[0] as f64) as f32,
                                       (trimmed_l2._End._Y * zoom_f64 + pan[1] as f64) as f32);
                        let pc_start =
                            Point::new((chamfer_line._Start._X * zoom_f64 + pan[0] as f64) as f32,
                                       (chamfer_line._Start._Y * zoom_f64 + pan[1] as f64) as f32);
                        let pc_end =
                            Point::new((chamfer_line._End._X * zoom_f64 + pan[0] as f64) as f32,
                                       (chamfer_line._End._Y * zoom_f64 + pan[1] as f64) as f32);
                        overlay_frame.stroke(&Path::line(p1_start, p1_end), draft_stroke);
                        overlay_frame.stroke(&Path::line(p2_start, p2_end), draft_stroke);
                        overlay_frame.stroke(&Path::line(pc_start, pc_end), draft_stroke);
                    }
                    DraftPrimitive::DimensionPreview(dim) => {
                        Self::draw_dimension(&mut overlay_frame,
                                             dim,
                                             zoom_f64,
                                             pan,
                                             palette,
                                             doc.Units(),
                                             false);
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

                let marquee_fill =
                    Color::from_rgba(palette.accent.r, palette.accent.g, palette.accent.b, 0.15);
                let marquee_stroke = Stroke::default().with_color(palette.accent).with_width(1.0);

                overlay_frame.fill_rectangle(Point::new(min_x, min_y),
                                             Size::new(w, h),
                                             marquee_fill);
                overlay_frame.stroke(&Path::rectangle(Point::new(min_x, min_y), Size::new(w, h)),
                                     marquee_stroke);
            }

            // Floating dimension badge
            if let (Some(text), Some(pos)) = (&draft.dimension_badge, &draft.dimension_pos) {
                let badgeWidth =
                    (text.chars().count() as f32 * 7.0 + 12.0).max(120.0)
                                                              .min(bounds.width.max(1.0));
                let sx =
                    ((pos._X * zoom_f64 + pan[0] as f64) as f32 + 15.0).clamp(4.0,
                                                                              (bounds.width
                                                                               - badgeWidth
                                                                               + 4.0)
                                                                                     .max(4.0));
                let sy =
                    ((pos._Y * zoom_f64 + pan[1] as f64) as f32 - 20.0).clamp(4.0,
                                                                              (bounds.height
                                                                               - 20.0)
                                                                                      .max(4.0));

                let badge_bg = Color::from_rgba(palette.toolbar_bg.r,
                                                palette.toolbar_bg.g,
                                                palette.toolbar_bg.b,
                                                0.9);
                overlay_frame.fill_rectangle(Point::new(sx - 4.0, sy - 2.0),
                                             Size::new(badgeWidth, 20.0),
                                             badge_bg);
                overlay_frame.stroke(&Path::rectangle(Point::new(sx - 4.0, sy - 2.0),
                                                      Size::new(badgeWidth, 20.0)),
                                     Stroke::default().with_color(palette.border).with_width(1.0));
                overlay_frame.fill_text(Text { content: text.clone(),
                                               position: Point::new(sx, sy),
                                               color: palette.text_primary,
                                               size: 11.0.into(),
                                               ..Default::default() });
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
                    overlay_frame.stroke(&Path::rectangle(Point::new(sx - 4.0, sy - 4.0),
                                                          Size::new(8.0, 8.0)),
                                         snap_stroke);
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
                    overlay_frame.fill_rectangle(Point::new(sx - 2.0, sy - 2.0),
                                                 Size::new(4.0, 4.0),
                                                 glyph_color);
                }
            }
        }

        vec![grid_geom, geom, overlay_frame.into_geometry()]
    }

    fn update(&self, state: &mut Self::State, event: &iced::Event, bounds: Rectangle,
              cursor: mouse::Cursor)
              -> Option<iced::widget::Action<Message>>
    {
        let is_over = cursor.is_over(bounds);
        let publish =
            |action| Some(iced::widget::Action::publish((self.map_action)(action)).and_capture());

        let zoom = self.zoom as f64;
        let pan = self.pan;

        let screen_to_model = |p: Point| -> SketchPoint {
            SketchPoint::New((p.x as f64 - pan[0] as f64) / zoom,
                             (p.y as f64 - pan[1] as f64) / zoom)
        };

        match event {
            iced::Event::Window(iced::window::Event::Unfocused) => {
                state.is_dragging = false;
                state.is_panning = false;
                state.pan_start = None;
                state._KeyboardFocus = false;
                state.numeric_buffer.clear();
                state.numeric_field = NumericFieldKind::Primary;
                return publish(SketchCanvasAction::Input(SketchInputEvent::Cancel));
            }
            iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) => {
                state._Modifiers =
                    KeyModifiers::New(modifiers.shift(), modifiers.control(), modifiers.alt());
            }
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. })
                if state._KeyboardFocus =>
            {
                use iced::keyboard::{Key, key::Named};
                if let Key::Character(character) = key {
                    let first_char = character.chars().next().unwrap_or('\0');
                    if self.ctrl.DraftGeometry().is_some()
                       && (first_char.is_ascii_digit() || first_char == '.' || first_char == '-')
                       && !modifiers.control()
                    {
                        state.numeric_buffer.push_str(character.as_str());
                        if let Ok(val) = state.numeric_buffer.parse::<f64>() {
                            return publish(SketchCanvasAction::Input(SketchInputEvent::Numeric {
                                field: state.numeric_field,
                                value: val,
                            }));
                        }
                        return Some(iced::widget::Action::capture());
                    }

                    let name = character.to_ascii_lowercase();
                    if modifiers.control() {
                        return match name.as_str() {
                            "s" => publish(SketchCanvasAction::Save),
                            "z" if modifiers.shift() => publish(SketchCanvasAction::Redo),
                            "z" => publish(SketchCanvasAction::Undo),
                            "y" => publish(SketchCanvasAction::Redo),
                            _ => None,
                        };
                    }
                    let tool = match name.as_str() {
                        "v" => Some(SketchToolKind::Select),
                        "l" => Some(SketchToolKind::Line),
                        "p" => Some(SketchToolKind::Polyline),
                        "c" => Some(SketchToolKind::Circle),
                        "a" => Some(SketchToolKind::Arc),
                        "e" => Some(SketchToolKind::Ellipse),
                        "f" if modifiers.shift() => Some(SketchToolKind::Chamfer),
                        "f" => Some(SketchToolKind::Fillet),
                        "d" => Some(SketchToolKind::Dimension),
                        _ => None,
                    };
                    if let Some(tool) = tool {
                        return publish(SketchCanvasAction::Tool(tool));
                    }
                }
                if matches!(key, Key::Named(Named::Tab)) && self.ctrl.DraftGeometry().is_some() {
                    state.numeric_field = match state.numeric_field {
                        NumericFieldKind::Primary => NumericFieldKind::Secondary,
                        NumericFieldKind::Secondary => NumericFieldKind::Primary,
                    };
                    state.numeric_buffer.clear();
                    return Some(iced::widget::Action::capture());
                }
                if matches!(key, Key::Named(Named::Backspace)) && !state.numeric_buffer.is_empty() {
                    state.numeric_buffer.pop();
                    if !state.numeric_buffer.is_empty()
                       && let Ok(val) = state.numeric_buffer.parse::<f64>()
                    {
                        return publish(SketchCanvasAction::Input(SketchInputEvent::Numeric {
                            field: state.numeric_field,
                            value: val,
                        }));
                    }
                    return Some(iced::widget::Action::capture());
                }
                if matches!(key,
                            Key::Named(Named::Escape
                                       | Named::Enter
                                       | Named::Delete
                                       | Named::Backspace))
                {
                    if matches!(key, Key::Named(Named::Escape | Named::Enter)) {
                        state.numeric_buffer.clear();
                        state.numeric_field = NumericFieldKind::Primary;
                    }
                    return publish(SketchCanvasAction::Input(SketchInputEvent::KeyDown {
                        key_escape: matches!(key, Key::Named(Named::Escape)),
                        key_enter:  matches!(key, Key::Named(Named::Enter)),
                        key_delete: matches!(key, Key::Named(Named::Delete | Named::Backspace)),
                        modifiers:  state._Modifiers,
                    }));
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(_)) if !is_over => {
                state._KeyboardFocus = false;
            }
            // Mouse wheel zoom centered on cursor
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) if is_over => {
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 25.0,
                };
                if !lines.is_finite() || lines == 0.0 {
                    return None;
                }
                let zoom_factor = 1.15f32.powf(lines.clamp(-10.0, 10.0));
                let focal = cursor.position_in(bounds).map(|p| [p.x, p.y]);
                return publish(SketchCanvasAction::Zoom(zoom_factor, focal));
            }

            // Mouse button down
            iced::Event::Mouse(mouse::Event::ButtonPressed(button)) if is_over => {
                state._KeyboardFocus = true;
                if *button == mouse::Button::Right {
                    state.is_dragging = false;
                    state.numeric_buffer.clear();
                    state.numeric_field = NumericFieldKind::Primary;
                    if self.ctrl.DraftGeometry().is_some() {
                        return publish(SketchCanvasAction::Input(SketchInputEvent::Cancel));
                    }
                    if let Some(pos) = cursor.position_in(bounds) {
                        return publish(SketchCanvasAction::ContextMenu([pos.x, pos.y]));
                    }
                    return publish(SketchCanvasAction::Input(SketchInputEvent::Cancel));
                }
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
                        let input = SketchInputEvent::PointerDown { pos:       model_pt,
                                                                    button:    PointerButton::Left,
                                                                    modifiers: state._Modifiers, };
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
                        let input = SketchInputEvent::PointerMove { pos:       model_pt,
                                                                    modifiers: state._Modifiers, };
                        return publish(SketchCanvasAction::Input(input));
                    }
                }
            }

            // Mouse button released
            iced::Event::Mouse(mouse::Event::ButtonReleased(button)) => {
                if *button == mouse::Button::Middle && state.is_panning {
                    state.is_panning = false;
                    state.pan_start = None;
                    return Some(iced::widget::Action::capture());
                }

                if *button == mouse::Button::Left && state.is_dragging {
                    state.is_dragging = false;
                    if let Some(pos) = cursor.position_in(bounds) {
                        let model_pt = screen_to_model(pos);
                        let input = SketchInputEvent::PointerUp { pos:       model_pt,
                                                                  button:    PointerButton::Left,
                                                                  modifiers: state._Modifiers, };
                        return publish(SketchCanvasAction::Input(input));
                    }
                    return publish(SketchCanvasAction::Input(SketchInputEvent::Cancel));
                }
            }

            // Window resize detection
            iced::Event::Window(iced::window::Event::RedrawRequested(_))
                if bounds.width > 0.0
                   && self.canvas_state.LastSize() != [bounds.width, bounds.height] =>
            {
                return publish(SketchCanvasAction::Resize(bounds.width, bounds.height));
            }

            _ => {}
        }

        None
    }

    fn mouse_interaction(&self, state: &Self::State, bounds: Rectangle, cursor: mouse::Cursor)
                         -> mouse::Interaction
    {
        if state.is_panning {
            return mouse::Interaction::Grabbing;
        }
        if cursor.is_over(bounds) {
            return if self.ctrl.Tool() == SketchToolKind::Select {
                mouse::Interaction::default()
            } else {
                mouse::Interaction::Crosshair
            };
        }
        return mouse::Interaction::default();
    }
}

impl<'a, Message: 'static + Clone, F> SketchCanvasProgram<'a, Message, F>
{
    fn draw_polyline_pts(frame: &mut Frame, pts: &Buff<SketchPoint>, zoom_f64: f64, pan: [f32; 2],
                         stroke: Stroke, close: bool)
    {
        if pts.Len() < 2 {
            return;
        }
        let path = Path::new(|builder| {
            let p0 = pts.Arr().Get(0).unwrap();
            builder.move_to(Point::new((p0._X * zoom_f64 + pan[0] as f64) as f32,
                                       (p0._Y * zoom_f64 + pan[1] as f64) as f32));
            let mut i = 1u32;
            while i < pts.Len() {
                let pt = pts.Arr().Get(i).unwrap();
                builder.line_to(Point::new((pt._X * zoom_f64 + pan[0] as f64) as f32,
                                           (pt._Y * zoom_f64 + pan[1] as f64) as f32));
                i += 1;
            }
            if close {
                builder.close();
            }
        });
        frame.stroke(&path, stroke);
    }

    fn draw_dimension(frame: &mut Frame, dim: &crate::fenst::sketch_dimension::SketchDimension,
                      zoom_f64: f64, pan: [f32; 2], palette: ThemePalette, units: &str,
                      is_selected: bool)
    {
        let to_screen = |p: SketchPoint| -> Point {
            Point::new((p._X * zoom_f64 + pan[0] as f64) as f32,
                       (p._Y * zoom_f64 + pan[1] as f64) as f32)
        };

        let dim_color = palette.accent;
        let ext_color = if is_selected {
            palette.accent
        } else {
            palette.text_secondary
        };
        let dim_stroke = Stroke::default().with_color(dim_color)
                                          .with_width(if is_selected { 2.5 } else { 1.0 });
        let ext_stroke = Stroke::default().with_color(ext_color)
                                          .with_width(if is_selected { 1.5 } else { 0.75 });

        // 1. Draw Extension Lines
        let ((e1_start, e1_end), (e2_start, e2_end)) = dim.ExtensionLines();
        if e1_start != e1_end {
            frame.stroke(&Path::line(to_screen(e1_start), to_screen(e1_end)),
                         ext_stroke);
        }
        if e2_start != e2_end {
            frame.stroke(&Path::line(to_screen(e2_start), to_screen(e2_end)),
                         ext_stroke);
        }

        // 2. Draw Dimension Line
        let (d1, d2) = dim.DimensionLine();
        let sd1 = to_screen(d1);
        let sd2 = to_screen(d2);
        frame.stroke(&Path::line(sd1, sd2), dim_stroke);

        // 3. Draw Arrowheads
        let (a1, a2) = dim.Arrowheads(8.0 / zoom_f64.max(0.1));
        let path_arrow1 = Path::new(|builder| {
            builder.move_to(to_screen(a1[0]));
            builder.line_to(to_screen(a1[1]));
            builder.line_to(to_screen(a1[2]));
            builder.close();
        });
        frame.fill(&path_arrow1, dim_color);

        let path_arrow2 = Path::new(|builder| {
            builder.move_to(to_screen(a2[0]));
            builder.line_to(to_screen(a2[1]));
            builder.line_to(to_screen(a2[2]));
            builder.close();
        });
        frame.fill(&path_arrow2, dim_color);

        // 4. Draw Callout Text Badge
        let label = dim.FormattedText(units);
        let text_screen = to_screen(dim._TextPos);
        let text_render = iced::widget::canvas::Text { content: label,
                                                       position: Point::new(text_screen.x,
                                                                            text_screen.y - 6.0),
                                                       color: palette.text_primary,
                                                       size: iced::Pixels(11.0),
                                                       ..Default::default() };
        frame.fill_text(text_render);
    }
}

//-------------------------------------------------------------------------------------------------
