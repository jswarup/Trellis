//-- sketch_tool.rs -------------------------------------------------------------------------------------------------
use crate::fenst::sketch::{
    Circle, CircularArc, Ellipse, LineSegment, PolylineHeader, SketchDocument,
    SketchEntity, SketchPoint, SketchPrimitive,
};
use crate::fenst::sketch_edit::{SketchPatch, SketchPatchBuilder};
use crate::fleck::sketch_geometry::SketchGeometry;
use crate::fleck::sketch_query::{SketchQuery, SnapCandidate};
use crate::silo::{Arr, Buff, IArr, Stash};

//-------------------------------------------------------------------------------------------------

/// Keyboard modifier keys held during an input event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyModifiers
{
    pub shift: bool,
    pub ctrl:  bool,
    pub alt:   bool,
}

impl KeyModifiers
{
    pub fn New(shift: bool, ctrl: bool, alt: bool) -> Self
    {
        Self { shift, ctrl, alt }
    }
}

//-------------------------------------------------------------------------------------------------

/// Pointer mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton
{
    Left,
    Middle,
    Right,
}

//-------------------------------------------------------------------------------------------------

/// Active sketch tool kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SketchToolKind
{
    #[default]
    Select,
    Line,
    Polyline,
    Circle,
    Arc,
    Ellipse,
    Move,
}

//-------------------------------------------------------------------------------------------------

/// Numeric HUD field target for direct dimension entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericFieldKind
{
    Primary,
    Secondary,
}

//-------------------------------------------------------------------------------------------------

/// Grip handle classification for direct manipulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GripKind
{
    LineStart,
    LineEnd,
    LineMidpoint,
    CircleCenter,
    CircleRadius,
    ArcCenter,
    ArcStart,
    ArcEnd,
    ArcMidpoint,
    EllipseCenter,
    EllipseMajor,
    EllipseMinor,
    PolylineVertex(u32),
}

//-------------------------------------------------------------------------------------------------

/// Interactive grip handle record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GripInfo
{
    pub kind:      GripKind,
    pub pos:       SketchPoint,
    pub entity_id: u32,
}

impl GripInfo
{
    pub fn New(kind: GripKind, pos: SketchPoint, entity_id: u32) -> Self
    {
        Self {
            kind,
            pos,
            entity_id,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Ephemeral draft primitive being constructed or previewed.
#[derive(Clone, Debug, PartialEq)]
pub enum DraftPrimitive
{
    Line
    {
        start: SketchPoint,
        end:   SketchPoint,
    },
    Circle
    {
        center: SketchPoint,
        radius: f64,
    },
    Arc
    {
        center:      SketchPoint,
        radius:      f64,
        start_angle: f64,
        sweep_angle: f64,
    },
    Ellipse
    {
        center:     SketchPoint,
        semi_major: f64,
        semi_minor: f64,
        rotation:   f64,
    },
    Polyline
    {
        points: Buff<SketchPoint>,
        closed: bool,
    },
}

//-------------------------------------------------------------------------------------------------

/// Ephemeral draft geometry rendered on the interactive canvas layer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DraftGeometry
{
    pub primitive:       Option<DraftPrimitive>,
    pub snap_candidate:  Option<SnapCandidate>,
    pub dimension_badge: Option<String>,
    pub dimension_pos:   Option<SketchPoint>,
    pub marquee:         Option<(SketchPoint, SketchPoint)>,
}

//-------------------------------------------------------------------------------------------------

/// Decoupled input event consumed by the headless tool controller.
#[derive(Clone, Debug, PartialEq)]
pub enum SketchInputEvent
{
    PointerDown
    {
        pos:       SketchPoint,
        button:    PointerButton,
        modifiers: KeyModifiers,
    },
    PointerMove
    {
        pos:       SketchPoint,
        modifiers: KeyModifiers,
    },
    PointerUp
    {
        pos:       SketchPoint,
        button:    PointerButton,
        modifiers: KeyModifiers,
    },
    KeyDown
    {
        key_escape: bool,
        key_enter:  bool,
        key_delete: bool,
        modifiers:  KeyModifiers,
    },
    Numeric
    {
        field: NumericFieldKind,
        value: f64,
    },
    Cancel,
}

//-------------------------------------------------------------------------------------------------

/// Headless FSM tool controller managing interaction state, drafting, snapping, and transactions.
pub struct SketchToolController
{
    _ToolKind:             SketchToolKind,
    _Zoom:                 f64,
    _AnchorPoint:          Option<SketchPoint>,
    _SecondaryAnchor:      Option<SketchPoint>,
    _ArcRadius:            f64,
    _ArcStartAngle:        f64,
    _PolylinePoints:       Stash<SketchPoint>,
    _CurrentCursor:        SketchPoint,
    _ActiveSnap:           Option<SnapCandidate>,
    _Draft:                Option<DraftGeometry>,
    _SelectedEntityId:     u32,
    _ActiveGrip:           Option<GripInfo>,
    _DragStart:            Option<SketchPoint>,
    _OriginalEntityOnDrag: Option<SketchEntity>,
    _CommittedPatch:       Option<SketchPatch>,
    _NextEntityId:         u32,
    _ActiveLayerId:        u32,
}

impl SketchToolController
{
    pub fn New() -> Self
    {
        Self {
            _ToolKind:             SketchToolKind::Select,
            _Zoom:                 1.0,
            _AnchorPoint:          None,
            _SecondaryAnchor:      None,
            _ArcRadius:            0.0,
            _ArcStartAngle:        0.0,
            _PolylinePoints:       Stash::New(),
            _CurrentCursor:        SketchPoint::New(0.0, 0.0),
            _ActiveSnap:           None,
            _Draft:                None,
            _SelectedEntityId:     u32::MAX,
            _ActiveGrip:           None,
            _DragStart:            None,
            _OriginalEntityOnDrag: None,
            _CommittedPatch:       None,
            _NextEntityId:         1000,
            _ActiveLayerId:        0,
        }
    }

    pub fn Tool(&self) -> SketchToolKind
    {
        self._ToolKind
    }

    pub fn SetTool(&mut self, tool: SketchToolKind)
    {
        self.ResetDraft();
        self._ToolKind = tool;
    }

    pub fn Zoom(&self) -> f64
    {
        self._Zoom
    }

    pub fn SetZoom(&mut self, zoom: f64)
    {
        self._Zoom = zoom.max(0.0001);
    }

    pub fn ActiveLayerId(&self) -> u32
    {
        self._ActiveLayerId
    }

    pub fn SetActiveLayerId(&mut self, id: u32)
    {
        self._ActiveLayerId = id;
    }

    pub fn SelectedEntityId(&self) -> u32
    {
        self._SelectedEntityId
    }

    pub fn SetSelectedEntityId(&mut self, id: u32)
    {
        self._SelectedEntityId = id;
    }

    pub fn DraftGeometry(&self) -> Option<&DraftGeometry>
    {
        self._Draft.as_ref()
    }

    pub fn ActiveSnap(&self) -> Option<&SnapCandidate>
    {
        self._ActiveSnap.as_ref()
    }

    pub fn TakeCommittedPatch(&mut self) -> Option<SketchPatch>
    {
        self._CommittedPatch.take()
    }

    pub fn ResetDraft(&mut self)
    {
        self._AnchorPoint = None;
        self._SecondaryAnchor = None;
        self._ArcRadius = 0.0;
        self._ArcStartAngle = 0.0;
        self._PolylinePoints = Stash::New();
        self._ActiveSnap = None;
        self._Draft = None;
        self._ActiveGrip = None;
        self._DragStart = None;
        self._OriginalEntityOnDrag = None;
    }

    /// Evaluates snapping candidate with 8px acquisition / 12px release hysteresis.
    pub fn EvaluateSnap(&mut self, doc: &SketchDocument, raw_pos: SketchPoint) -> SketchPoint
    {
        let acquisitionModel = 8.0 / self._Zoom;
        let releaseModel = 12.0 / self._Zoom;

        // Check hysteresis release on existing active snap
        if let Some(active) = &self._ActiveSnap {
            let dx = raw_pos._X - active._Point._X;
            let dy = raw_pos._Y - active._Point._Y;
            let dist = dx.hypot(dy);
            if dist < releaseModel {
                return active._Point;
            }
            self._ActiveSnap = None;
        }

        // Search new snap candidates within acquisition radius
        let mut candidates = Stash::New();
        let arena = doc.PointArena();
        doc.Entities().Traverse(|entity| {
            SketchQuery::CollectSnapCandidates(entity.Primitive(), entity.Id(), arena, &mut candidates);
        });

        if candidates.Len() > 0 {
            let mut best: Option<SnapCandidate> = None;
            let mut bestDist = acquisitionModel;
            candidates.Arr().Traverse(|cand| {
                let dx = raw_pos._X - cand._Point._X;
                let dy = raw_pos._Y - cand._Point._Y;
                let d = dx.hypot(dy);
                if d < bestDist {
                    bestDist = d;
                    best = Some(*cand);
                }
            });
            if let Some(snap) = best {
                self._ActiveSnap = Some(snap);
                return snap._Point;
            }
        }

        raw_pos
    }

    /// Collects active grip handles for the currently selected entity.
    pub fn ActiveGrips(&self, doc: &SketchDocument) -> Buff<GripInfo>
    {
        if self._SelectedEntityId == u32::MAX {
            return Buff::New();
        }

        let mut grips = Stash::New();
        if let Some(entity) = doc.FindEntity(self._SelectedEntityId) {
            let id = entity.Id();
            match entity.Primitive() {
                SketchPrimitive::Line(LineSegment { _Start, _End }) => {
                    grips.Push(GripInfo::New(GripKind::LineStart, *_Start, id));
                    grips.Push(GripInfo::New(GripKind::LineEnd, *_End, id));
                    let mid = SketchPoint::New(
                        (_Start._X + _End._X) * 0.5,
                        (_Start._Y + _End._Y) * 0.5,
                    );
                    grips.Push(GripInfo::New(GripKind::LineMidpoint, mid, id));
                }
                SketchPrimitive::Circle(Circle { _Center, _Radius }) => {
                    grips.Push(GripInfo::New(GripKind::CircleCenter, *_Center, id));
                    let radPt = SketchPoint::New(_Center._X + _Radius, _Center._Y);
                    grips.Push(GripInfo::New(GripKind::CircleRadius, radPt, id));
                }
                SketchPrimitive::Arc(CircularArc {
                    _Center,
                    _Radius,
                    _StartAngle,
                    _SweepAngle,
                }) => {
                    grips.Push(GripInfo::New(GripKind::ArcCenter, *_Center, id));
                    let startPt = SketchPoint::New(
                        _Center._X + _Radius * _StartAngle.cos(),
                        _Center._Y + _Radius * _StartAngle.sin(),
                    );
                    grips.Push(GripInfo::New(GripKind::ArcStart, startPt, id));
                    let endAng = _StartAngle + _SweepAngle;
                    let endPt = SketchPoint::New(
                        _Center._X + _Radius * endAng.cos(),
                        _Center._Y + _Radius * endAng.sin(),
                    );
                    grips.Push(GripInfo::New(GripKind::ArcEnd, endPt, id));
                    let midAng = _StartAngle + _SweepAngle * 0.5;
                    let midPt = SketchPoint::New(
                        _Center._X + _Radius * midAng.cos(),
                        _Center._Y + _Radius * midAng.sin(),
                    );
                    grips.Push(GripInfo::New(GripKind::ArcMidpoint, midPt, id));
                }
                SketchPrimitive::Ellipse(Ellipse {
                    _Center,
                    _SemiMajor,
                    _SemiMinor,
                    _Rotation,
                }) => {
                    grips.Push(GripInfo::New(GripKind::EllipseCenter, *_Center, id));
                    let majPt = SketchGeometry::RotatePoint(
                        SketchPoint::New(_Center._X + _SemiMajor, _Center._Y),
                        *_Center,
                        *_Rotation,
                    );
                    grips.Push(GripInfo::New(GripKind::EllipseMajor, majPt, id));
                    let minPt = SketchGeometry::RotatePoint(
                        SketchPoint::New(_Center._X, _Center._Y + _SemiMinor),
                        *_Center,
                        *_Rotation,
                    );
                    grips.Push(GripInfo::New(GripKind::EllipseMinor, minPt, id));
                }
                SketchPrimitive::Polyline(PolylineHeader {
                    _PointOffset,
                    _PointCount,
                    ..
                }) => {
                    let pts = doc.PolylinePoints(*_PointOffset, *_PointCount);
                    let mut vIdx = 0u32;
                    pts.Traverse(|pt| {
                        grips.Push(GripInfo::New(GripKind::PolylineVertex(vIdx), *pt, id));
                        vIdx += 1;
                    });
                }
            }
        }
        grips.IntoBuff()
    }

    /// Hit-tests active grips against cursor within given screen pixel tolerance.
    pub fn HitTestGrip(
        &self,
        grips: Arr<GripInfo>,
        cursor: SketchPoint,
        tolerancePx: f64,
    ) -> Option<GripInfo>
    {
        let tolModel = tolerancePx / self._Zoom;
        let mut hit: Option<GripInfo> = None;
        let mut hitDist = tolModel;
        grips.Traverse(|grip| {
            let dx = cursor._X - grip.pos._X;
            let dy = cursor._Y - grip.pos._Y;
            let dist = dx.hypot(dy);
            if dist < hitDist {
                hitDist = dist;
                hit = Some(*grip);
            }
        });
        hit
    }

    /// Hit-tests entities in `doc` against cursor within given screen pixel tolerance.
    pub fn HitTestEntity(
        &self,
        doc: &SketchDocument,
        cursor: SketchPoint,
        tolerancePx: f64,
    ) -> u32
    {
        let tolModel = tolerancePx / self._Zoom;
        let mut hitId = u32::MAX;
        let mut hitDist = tolModel;
        let arena = doc.PointArena();

        doc.Entities().Traverse(|entity| {
            let dist = SketchQuery::PointToPrimitiveDistance(cursor, entity.Primitive(), arena);
            if dist < hitDist {
                hitDist = dist;
                hitId = entity.Id();
            }
        });

        hitId
    }

    /// Primary event dispatch loop.
    pub fn ProcessEvent(&mut self, doc: &SketchDocument, event: SketchInputEvent)
    {
        match event {
            SketchInputEvent::PointerDown {
                pos,
                button,
                modifiers,
            } => {
                if button == PointerButton::Left {
                    self.HandlePointerDownLeft(doc, pos, modifiers);
                } else if button == PointerButton::Right {
                    self.ResetDraft();
                }
            }
            SketchInputEvent::PointerMove { pos, modifiers } => {
                self.HandlePointerMove(doc, pos, modifiers);
            }
            SketchInputEvent::PointerUp {
                pos,
                button,
                modifiers,
            } => {
                if button == PointerButton::Left {
                    self.HandlePointerUpLeft(doc, pos, modifiers);
                }
            }
            SketchInputEvent::KeyDown {
                key_escape,
                key_enter,
                key_delete,
                ..
            } => {
                if key_escape {
                    self.ResetDraft();
                    self._SelectedEntityId = u32::MAX;
                } else if key_delete {
                    self.HandleDelete(doc);
                } else if key_enter {
                    self.HandleEnter(doc);
                }
            }
            SketchInputEvent::Numeric { field, value } => {
                self.HandleNumeric(doc, field, value);
            }
            SketchInputEvent::Cancel => {
                self.ResetDraft();
            }
        }
    }

    fn HandlePointerDownLeft(
        &mut self,
        doc: &SketchDocument,
        rawPos: SketchPoint,
        _modifiers: KeyModifiers,
    )
    {
        let pos = self.EvaluateSnap(doc, rawPos);
        self._CurrentCursor = pos;

        match self._ToolKind {
            SketchToolKind::Select => {
                let grips = self.ActiveGrips(doc);
                if let Some(grip) = self.HitTestGrip(grips.Arr(), rawPos, 8.0) {
                    self._ActiveGrip = Some(grip);
                    self._DragStart = Some(pos);
                    if let Some(entity) = doc.FindEntity(grip.entity_id) {
                        self._OriginalEntityOnDrag = Some(entity.clone());
                    }
                    return;
                }

                let hit = self.HitTestEntity(doc, rawPos, 8.0);
                if hit != u32::MAX {
                    self._SelectedEntityId = hit;
                    self._DragStart = Some(pos);
                    if let Some(entity) = doc.FindEntity(hit) {
                        self._OriginalEntityOnDrag = Some(entity.clone());
                    }
                } else {
                    self._SelectedEntityId = u32::MAX;
                    self._DragStart = Some(rawPos);
                }
            }
            SketchToolKind::Line => {
                if self._AnchorPoint.is_none() {
                    self._AnchorPoint = Some(pos);
                    self._Draft = Some(DraftGeometry {
                        primitive:       Some(DraftPrimitive::Line {
                            start: pos,
                            end:   pos,
                        }),
                        snap_candidate:  self._ActiveSnap,
                        dimension_badge: Some("L: 0.00mm".to_string()),
                        dimension_pos:   Some(pos),
                        marquee:         None,
                    });
                } else {
                    let start = self._AnchorPoint.take().unwrap();
                    let len = (pos._X - start._X).hypot(pos._Y - start._Y);
                    if len > 1e-4 {
                        self.CommitPrimitive(
                            doc,
                            SketchPrimitive::Line(LineSegment::New(start, pos)),
                        );
                    }
                    self.ResetDraft();
                }
            }
            SketchToolKind::Circle => {
                if self._AnchorPoint.is_none() {
                    self._AnchorPoint = Some(pos);
                    self._Draft = Some(DraftGeometry {
                        primitive:       Some(DraftPrimitive::Circle {
                            center: pos,
                            radius: 0.0,
                        }),
                        snap_candidate:  self._ActiveSnap,
                        dimension_badge: Some("R: 0.00mm".to_string()),
                        dimension_pos:   Some(pos),
                        marquee:         None,
                    });
                } else {
                    let center = self._AnchorPoint.take().unwrap();
                    let radius = (pos._X - center._X).hypot(pos._Y - center._Y);
                    if radius > 1e-4 {
                        self.CommitPrimitive(
                            doc,
                            SketchPrimitive::Circle(Circle::New(center, radius)),
                        );
                    }
                    self.ResetDraft();
                }
            }
            SketchToolKind::Arc => {
                if self._AnchorPoint.is_none() {
                    self._AnchorPoint = Some(pos);
                } else if self._SecondaryAnchor.is_none() {
                    let center = self._AnchorPoint.unwrap();
                    let r = (pos._X - center._X).hypot(pos._Y - center._Y);
                    if r > 1e-4 {
                        self._SecondaryAnchor = Some(pos);
                        self._ArcRadius = r;
                        self._ArcStartAngle = (pos._Y - center._Y).atan2(pos._X - center._X);
                    }
                } else {
                    let center = self._AnchorPoint.unwrap();
                    let r = self._ArcRadius;
                    let startAng = self._ArcStartAngle;
                    let curAng = (pos._Y - center._Y).atan2(pos._X - center._X);
                    let mut sweep = curAng - startAng;
                    if sweep <= 0.0 {
                        sweep += std::f64::consts::TAU;
                    }
                    if sweep > 1e-4 && r > 1e-4 {
                        self.CommitPrimitive(
                            doc,
                            SketchPrimitive::Arc(CircularArc::New(center, r, startAng, sweep)),
                        );
                    }
                    self.ResetDraft();
                }
            }
            SketchToolKind::Ellipse => {
                if self._AnchorPoint.is_none() {
                    self._AnchorPoint = Some(pos);
                } else if self._SecondaryAnchor.is_none() {
                    let center = self._AnchorPoint.unwrap();
                    let a = (pos._X - center._X).hypot(pos._Y - center._Y);
                    if a > 1e-4 {
                        self._SecondaryAnchor = Some(pos);
                    }
                } else {
                    let center = self._AnchorPoint.unwrap();
                    let pMajor = self._SecondaryAnchor.unwrap();
                    let a = (pMajor._X - center._X).hypot(pMajor._Y - center._Y);
                    let rot = (pMajor._Y - center._Y).atan2(pMajor._X - center._X);

                    // Minor radius is projection distance perpendicular to major axis
                    let rel = SketchPoint::New(pos._X - center._X, pos._Y - center._Y);
                    let unrot = SketchGeometry::RotatePoint(rel, SketchPoint::New(0.0, 0.0), -rot);
                    let b = unrot._Y.abs().min(a).max(0.001);

                    if a > 1e-4 && b > 1e-4 {
                        self.CommitPrimitive(
                            doc,
                            SketchPrimitive::Ellipse(Ellipse::New(center, a, b, rot)),
                        );
                    }
                    self.ResetDraft();
                }
            }
            SketchToolKind::Polyline => {
                if self._PolylinePoints.Len() == 0 {
                    self._PolylinePoints.Push(pos);
                } else {
                    let first = *self._PolylinePoints.Arr().Get(0).unwrap();
                    let dFirst = (pos._X - first._X).hypot(pos._Y - first._Y);
                    if dFirst < 8.0 / self._Zoom && self._PolylinePoints.Len() >= 3 {
                        // Close polyline and commit
                        self.CommitPolyline(doc, true);
                        self.ResetDraft();
                        return;
                    }
                    self._PolylinePoints.Push(pos);
                }
            }
            SketchToolKind::Move => {
                if self._SelectedEntityId != u32::MAX {
                    self._DragStart = Some(pos);
                    if let Some(entity) = doc.FindEntity(self._SelectedEntityId) {
                        self._OriginalEntityOnDrag = Some(entity.clone());
                    }
                }
            }
        }
    }

    fn HandlePointerMove(
        &mut self,
        doc: &SketchDocument,
        rawPos: SketchPoint,
        modifiers: KeyModifiers,
    )
    {
        let pos = self.EvaluateSnap(doc, rawPos);
        self._CurrentCursor = pos;

        match self._ToolKind {
            SketchToolKind::Select => {
                if let Some(grip) = self._ActiveGrip
                    && let Some(orig) = self._OriginalEntityOnDrag.clone()
                {
                    self.UpdateGripReshapeDraft(doc, &orig, grip.kind, pos);
                } else if let Some(dragStart) = self._DragStart
                    && self._SelectedEntityId != u32::MAX
                    && let Some(orig) = self._OriginalEntityOnDrag.clone()
                {
                    let dx = pos._X - dragStart._X;
                    let dy = pos._Y - dragStart._Y;
                    self.UpdateEntityTranslateDraft(doc, &orig, dx, dy);
                } else if let Some(dragStart) = self._DragStart
                    && self._SelectedEntityId == u32::MAX
                {
                    self._Draft = Some(DraftGeometry {
                        primitive:       None,
                        snap_candidate:  None,
                        dimension_badge: None,
                        dimension_pos:   None,
                        marquee:         Some((dragStart, rawPos)),
                    });
                }
            }
            SketchToolKind::Line => {
                if let Some(start) = self._AnchorPoint {
                    let mut end = pos;
                    if modifiers.shift {
                        // Angle lock to 15 degree increments
                        let dx = end._X - start._X;
                        let dy = end._Y - start._Y;
                        let r = dx.hypot(dy);
                        let ang = dy.atan2(dx);
                        let step = std::f64::consts::PI / 12.0;
                        let lockedAng = (ang / step).round() * step;
                        end = SketchPoint::New(
                            start._X + r * lockedAng.cos(),
                            start._Y + r * lockedAng.sin(),
                        );
                    }

                    let len = (end._X - start._X).hypot(end._Y - start._Y);
                    let angDeg = (end._Y - start._Y).atan2(end._X - start._X).to_degrees();
                    let badge = format!("L: {:.2}mm  ∠: {:.1}°", len, angDeg);

                    self._Draft = Some(DraftGeometry {
                        primitive:       Some(DraftPrimitive::Line { start, end }),
                        snap_candidate:  self._ActiveSnap,
                        dimension_badge: Some(badge),
                        dimension_pos:   Some(end),
                        marquee:         None,
                    });
                }
            }
            SketchToolKind::Circle => {
                if let Some(center) = self._AnchorPoint {
                    let r = (pos._X - center._X).hypot(pos._Y - center._Y);
                    let badge = format!("R: {:.2}mm  (D: {:.2}mm)", r, r * 2.0);
                    self._Draft = Some(DraftGeometry {
                        primitive:       Some(DraftPrimitive::Circle { center, radius: r }),
                        snap_candidate:  self._ActiveSnap,
                        dimension_badge: Some(badge),
                        dimension_pos:   Some(pos),
                        marquee:         None,
                    });
                }
            }
            SketchToolKind::Arc => {
                if let Some(center) = self._AnchorPoint {
                    if self._SecondaryAnchor.is_none() {
                        let r = (pos._X - center._X).hypot(pos._Y - center._Y);
                        let badge = format!("R: {:.2}mm", r);
                        self._Draft = Some(DraftGeometry {
                            primitive:       Some(DraftPrimitive::Line {
                                start: center,
                                end:   pos,
                            }),
                            snap_candidate:  self._ActiveSnap,
                            dimension_badge: Some(badge),
                            dimension_pos:   Some(pos),
                            marquee:         None,
                        });
                    } else {
                        let r = self._ArcRadius;
                        let startAng = self._ArcStartAngle;
                        let curAng = (pos._Y - center._Y).atan2(pos._X - center._X);
                        let mut sweep = curAng - startAng;
                        if sweep <= 0.0 {
                            sweep += std::f64::consts::TAU;
                        }
                        let badge = format!(
                            "R: {:.2}mm  Sweep: {:.1}°",
                            r,
                            sweep.to_degrees()
                        );
                        self._Draft = Some(DraftGeometry {
                            primitive:       Some(DraftPrimitive::Arc {
                                center,
                                radius: r,
                                start_angle: startAng,
                                sweep_angle: sweep,
                            }),
                            snap_candidate:  self._ActiveSnap,
                            dimension_badge: Some(badge),
                            dimension_pos:   Some(pos),
                            marquee:         None,
                        });
                    }
                }
            }
            SketchToolKind::Ellipse => {
                if let Some(center) = self._AnchorPoint {
                    if let Some(pMajor) = self._SecondaryAnchor {
                        let a = (pMajor._X - center._X).hypot(pMajor._Y - center._Y);
                        let rot = (pMajor._Y - center._Y).atan2(pMajor._X - center._X);
                        let rel = SketchPoint::New(pos._X - center._X, pos._Y - center._Y);
                        let unrot = SketchGeometry::RotatePoint(rel, SketchPoint::New(0.0, 0.0), -rot);
                        let b = unrot._Y.abs().min(a).max(0.001);
                        let badge = format!(
                            "A: {:.2}mm  B: {:.2}mm  ∠: {:.1}°",
                            a,
                            b,
                            rot.to_degrees()
                        );
                        self._Draft = Some(DraftGeometry {
                            primitive:       Some(DraftPrimitive::Ellipse {
                                center,
                                semi_major: a,
                                semi_minor: b,
                                rotation: rot,
                            }),
                            snap_candidate:  self._ActiveSnap,
                            dimension_badge: Some(badge),
                            dimension_pos:   Some(pos),
                            marquee:         None,
                        });
                    } else {
                        let a = (pos._X - center._X).hypot(pos._Y - center._Y);
                        let rot = (pos._Y - center._Y).atan2(pos._X - center._X);
                        let badge = format!("A: {:.2}mm  ∠: {:.1}°", a, rot.to_degrees());
                        self._Draft = Some(DraftGeometry {
                            primitive:       Some(DraftPrimitive::Line {
                                start: center,
                                end:   pos,
                            }),
                            snap_candidate:  self._ActiveSnap,
                            dimension_badge: Some(badge),
                            dimension_pos:   Some(pos),
                            marquee:         None,
                        });
                    }
                }
            }
            SketchToolKind::Polyline => {
                if self._PolylinePoints.Len() > 0 {
                    let mut draftPts = Stash::New();
                    self._PolylinePoints.Arr().Traverse(|pt| {
                        draftPts.Push(*pt);
                    });
                    draftPts.Push(pos);
                    let count = draftPts.Len();
                    self._Draft = Some(DraftGeometry {
                        primitive:       Some(DraftPrimitive::Polyline {
                            points: draftPts.IntoBuff(),
                            closed: false,
                        }),
                        snap_candidate:  self._ActiveSnap,
                        dimension_badge: Some(format!("Points: {}", count)),
                        dimension_pos:   Some(pos),
                        marquee:         None,
                    });
                }
            }
            SketchToolKind::Move => {
                if let Some(dragStart) = self._DragStart
                    && let Some(orig) = self._OriginalEntityOnDrag.clone()
                {
                    let dx = pos._X - dragStart._X;
                    let dy = pos._Y - dragStart._Y;
                    self.UpdateEntityTranslateDraft(doc, &orig, dx, dy);
                }
            }
        }
    }

    fn HandlePointerUpLeft(
        &mut self,
        doc: &SketchDocument,
        rawPos: SketchPoint,
        _modifiers: KeyModifiers,
    )
    {
        let pos = self.EvaluateSnap(doc, rawPos);

        // Drag-release gesture support for Line and Circle tools
        if self._ToolKind == SketchToolKind::Line
            && let Some(start) = self._AnchorPoint
        {
            let dist = (pos._X - start._X).hypot(pos._Y - start._Y);
            if dist * self._Zoom >= 6.0 {
                self.CommitPrimitive(
                    doc,
                    SketchPrimitive::Line(LineSegment::New(start, pos)),
                );
                self.ResetDraft();
                return;
            }
        }

        if self._ToolKind == SketchToolKind::Circle
            && let Some(center) = self._AnchorPoint
        {
            let r = (pos._X - center._X).hypot(pos._Y - center._Y);
            if r * self._Zoom >= 6.0 {
                self.CommitPrimitive(
                    doc,
                    SketchPrimitive::Circle(Circle::New(center, r)),
                );
                self.ResetDraft();
                return;
            }
        }

        if let Some(grip) = self._ActiveGrip.take()
            && let Some(orig) = self._OriginalEntityOnDrag.take()
        {
            if let Some(modified) = self.ComputeReshapedEntity(doc, &orig, grip.kind, pos) {
                let mut builder = SketchPatchBuilder::New();
                builder.ReplaceEntity(orig, modified);
                self._CommittedPatch = Some(builder.Build());
            }
            self.ResetDraft();
            return;
        }

        if let Some(dragStart) = self._DragStart.take()
            && let Some(orig) = self._OriginalEntityOnDrag.take()
        {
            let dx = pos._X - dragStart._X;
            let dy = pos._Y - dragStart._Y;
            if dx.hypot(dy) > 1e-4
                && let Some(translated) = self.ComputeTranslatedEntity(doc, &orig, dx, dy)
            {
                let mut builder = SketchPatchBuilder::New();
                builder.ReplaceEntity(orig, translated);
                self._CommittedPatch = Some(builder.Build());
            }
            self.ResetDraft();
            return;
        }

        // Marquee selection check
        if let Some(DraftGeometry {
            marquee: Some((start, end)),
            ..
        }) = &self._Draft
        {
            let minX = start._X.min(end._X);
            let maxX = start._X.max(end._X);
            let minY = start._Y.min(end._Y);
            let maxY = start._Y.max(end._Y);

            let mut selected = u32::MAX;
            let arena = doc.PointArena();
            doc.Entities().Traverse(|ent| {
                if selected == u32::MAX {
                    let b = ent.Bounds(arena);
                    if b._Min._X >= minX
                        && b._Max._X <= maxX
                        && b._Min._Y >= minY
                        && b._Max._Y <= maxY
                    {
                        selected = ent.Id();
                    }
                }
            });
            self._SelectedEntityId = selected;
            self.ResetDraft();
        }
    }

    fn HandleDelete(&mut self, doc: &SketchDocument)
    {
        if self._SelectedEntityId != u32::MAX {
            if let Some(entity) = doc.FindEntity(self._SelectedEntityId) {
                let mut builder = SketchPatchBuilder::New();
                builder.RemoveEntity(entity.clone());
                self._CommittedPatch = Some(builder.Build());
            }
            self._SelectedEntityId = u32::MAX;
            self.ResetDraft();
        }
    }

    fn HandleEnter(&mut self, doc: &SketchDocument)
    {
        if self._ToolKind == SketchToolKind::Polyline && self._PolylinePoints.Len() >= 2 {
            self.CommitPolyline(doc, false);
            self.ResetDraft();
        }
    }

    fn HandleNumeric(&mut self, _doc: &SketchDocument, field: NumericFieldKind, value: f64)
    {
        if let Some(DraftGeometry {
            primitive: Some(prim),
            ..
        }) = &mut self._Draft
        {
            match prim {
                DraftPrimitive::Line { start, end } => {
                    if field == NumericFieldKind::Primary && value > 0.0 {
                        let ang = (end._Y - start._Y).atan2(end._X - start._X);
                        *end = SketchPoint::New(
                            start._X + value * ang.cos(),
                            start._Y + value * ang.sin(),
                        );
                    }
                }
                DraftPrimitive::Circle { radius, .. } => {
                    if field == NumericFieldKind::Primary && value > 0.0 {
                        *radius = value;
                    }
                }
                DraftPrimitive::Arc {
                    radius,
                    sweep_angle,
                    ..
                } => {
                    if field == NumericFieldKind::Primary && value > 0.0 {
                        *radius = value;
                    } else if field == NumericFieldKind::Secondary && value > 0.0 {
                        *sweep_angle = value.to_radians();
                    }
                }
                DraftPrimitive::Ellipse {
                    semi_major,
                    semi_minor,
                    ..
                } => {
                    if field == NumericFieldKind::Primary && value > 0.0 {
                        *semi_major = value;
                    } else if field == NumericFieldKind::Secondary && value > 0.0 {
                        *semi_minor = value;
                    }
                }
                _ => {}
            }
        }
    }

    fn CommitPrimitive(&mut self, _doc: &SketchDocument, prim: SketchPrimitive)
    {
        let id = self.AllocateNextId();
        let entity = SketchEntity::New(id, prim).WithLayer(self._ActiveLayerId);
        let mut builder = SketchPatchBuilder::New();
        builder.AddEntity(entity);
        self._CommittedPatch = Some(builder.Build());
    }

    fn CommitPolyline(&mut self, doc: &SketchDocument, closed: bool)
    {
        let count = self._PolylinePoints.Len();
        if count < 2 {
            return;
        }
        let id = self.AllocateNextId();
        let offset = doc.PointArena().Len();
        let header = PolylineHeader::New(offset, count, closed);
        let entity = SketchEntity::New(id, SketchPrimitive::Polyline(header))
            .WithLayer(self._ActiveLayerId);

        let mut builder = SketchPatchBuilder::New();
        builder.AddEntity(entity);
        self._CommittedPatch = Some(builder.Build());
    }

    fn AllocateNextId(&mut self) -> u32
    {
        let id = self._NextEntityId;
        self._NextEntityId += 1;
        id
    }

    fn UpdateGripReshapeDraft(
        &mut self,
        doc: &SketchDocument,
        orig: &SketchEntity,
        kind: GripKind,
        cursor: SketchPoint,
    )
    {
        if let Some(modified) = self.ComputeReshapedEntity(doc, orig, kind, cursor) {
            let prim = match modified.Primitive() {
                SketchPrimitive::Line(LineSegment { _Start, _End }) => {
                    DraftPrimitive::Line {
                        start: *_Start,
                        end:   *_End,
                    }
                }
                SketchPrimitive::Circle(Circle { _Center, _Radius }) => {
                    DraftPrimitive::Circle {
                        center: *_Center,
                        radius: *_Radius,
                    }
                }
                SketchPrimitive::Arc(CircularArc {
                    _Center,
                    _Radius,
                    _StartAngle,
                    _SweepAngle,
                }) => {
                    DraftPrimitive::Arc {
                        center:      *_Center,
                        radius:      *_Radius,
                        start_angle: *_StartAngle,
                        sweep_angle: *_SweepAngle,
                    }
                }
                SketchPrimitive::Ellipse(Ellipse {
                    _Center,
                    _SemiMajor,
                    _SemiMinor,
                    _Rotation,
                }) => {
                    DraftPrimitive::Ellipse {
                        center:     *_Center,
                        semi_major: *_SemiMajor,
                        semi_minor: *_SemiMinor,
                        rotation:   *_Rotation,
                    }
                }
                SketchPrimitive::Polyline(PolylineHeader {
                    _PointOffset,
                    _PointCount,
                    _IsClosed,
                }) => {
                    let pts = doc.PolylinePoints(*_PointOffset, *_PointCount);
                    DraftPrimitive::Polyline {
                        points: pts,
                        closed: *_IsClosed,
                    }
                }
            };

            self._Draft = Some(DraftGeometry {
                primitive:       Some(prim),
                snap_candidate:  self._ActiveSnap,
                dimension_badge: None,
                dimension_pos:   Some(cursor),
                marquee:         None,
            });
        }
    }

    fn UpdateEntityTranslateDraft(
        &mut self,
        doc: &SketchDocument,
        orig: &SketchEntity,
        dx: f64,
        dy: f64,
    )
    {
        if let Some(trans) = self.ComputeTranslatedEntity(doc, orig, dx, dy) {
            let prim = match trans.Primitive() {
                SketchPrimitive::Line(LineSegment { _Start, _End }) => {
                    DraftPrimitive::Line {
                        start: *_Start,
                        end:   *_End,
                    }
                }
                SketchPrimitive::Circle(Circle { _Center, _Radius }) => {
                    DraftPrimitive::Circle {
                        center: *_Center,
                        radius: *_Radius,
                    }
                }
                SketchPrimitive::Arc(CircularArc {
                    _Center,
                    _Radius,
                    _StartAngle,
                    _SweepAngle,
                }) => {
                    DraftPrimitive::Arc {
                        center:      *_Center,
                        radius:      *_Radius,
                        start_angle: *_StartAngle,
                        sweep_angle: *_SweepAngle,
                    }
                }
                SketchPrimitive::Ellipse(Ellipse {
                    _Center,
                    _SemiMajor,
                    _SemiMinor,
                    _Rotation,
                }) => {
                    DraftPrimitive::Ellipse {
                        center:     *_Center,
                        semi_major: *_SemiMajor,
                        semi_minor: *_SemiMinor,
                        rotation:   *_Rotation,
                    }
                }
                SketchPrimitive::Polyline(PolylineHeader {
                    _PointOffset,
                    _PointCount,
                    _IsClosed,
                }) => {
                    let pts = doc.PolylinePoints(*_PointOffset, *_PointCount);
                    let mut moved = Stash::New();
                    pts.Traverse(|p| {
                        moved.Push(SketchPoint::New(p._X + dx, p._Y + dy));
                    });
                    DraftPrimitive::Polyline {
                        points: moved.IntoBuff(),
                        closed: *_IsClosed,
                    }
                }
            };

            self._Draft = Some(DraftGeometry {
                primitive:       Some(prim),
                snap_candidate:  self._ActiveSnap,
                dimension_badge: Some(format!("Δ: ({:.2}, {:.2})", dx, dy)),
                dimension_pos:   Some(self._CurrentCursor),
                marquee:         None,
            });
        }
    }

    fn ComputeReshapedEntity(
        &self,
        _doc: &SketchDocument,
        orig: &SketchEntity,
        kind: GripKind,
        cursor: SketchPoint,
    ) -> Option<SketchEntity>
    {
        let newPrim = match (orig.Primitive(), kind) {
            (
                SketchPrimitive::Line(LineSegment { _End, .. }),
                GripKind::LineStart,
            ) => {
                if (cursor._X - _End._X).hypot(cursor._Y - _End._Y) > 1e-4 {
                    SketchPrimitive::Line(LineSegment::New(cursor, *_End))
                } else {
                    return None;
                }
            }
            (
                SketchPrimitive::Line(LineSegment { _Start, .. }),
                GripKind::LineEnd,
            ) => {
                if (cursor._X - _Start._X).hypot(cursor._Y - _Start._Y) > 1e-4 {
                    SketchPrimitive::Line(LineSegment::New(*_Start, cursor))
                } else {
                    return None;
                }
            }
            (
                SketchPrimitive::Line(LineSegment { _Start, _End }),
                GripKind::LineMidpoint,
            ) => {
                let midX = (_Start._X + _End._X) * 0.5;
                let midY = (_Start._Y + _End._Y) * 0.5;
                let dx = cursor._X - midX;
                let dy = cursor._Y - midY;
                SketchPrimitive::Line(LineSegment::New(
                    SketchPoint::New(_Start._X + dx, _Start._Y + dy),
                    SketchPoint::New(_End._X + dx, _End._Y + dy),
                ))
            }
            (
                SketchPrimitive::Circle(Circle { _Radius, .. }),
                GripKind::CircleCenter,
            ) => SketchPrimitive::Circle(Circle::New(cursor, *_Radius)),
            (
                SketchPrimitive::Circle(Circle { _Center, .. }),
                GripKind::CircleRadius,
            ) => {
                let r = (cursor._X - _Center._X).hypot(cursor._Y - _Center._Y);
                if r > 1e-4 {
                    SketchPrimitive::Circle(Circle::New(*_Center, r))
                } else {
                    return None;
                }
            }
            (
                SketchPrimitive::Arc(CircularArc {
                    _Radius,
                    _StartAngle,
                    _SweepAngle,
                    ..
                }),
                GripKind::ArcCenter,
            ) => SketchPrimitive::Arc(CircularArc::New(
                cursor,
                *_Radius,
                *_StartAngle,
                *_SweepAngle,
            )),
            (
                SketchPrimitive::Arc(CircularArc {
                    _Center,
                    _SweepAngle,
                    ..
                }),
                GripKind::ArcStart,
            ) => {
                let r = (cursor._X - _Center._X).hypot(cursor._Y - _Center._Y);
                let startAng = (cursor._Y - _Center._Y).atan2(cursor._X - _Center._X);
                if r > 1e-4 {
                    SketchPrimitive::Arc(CircularArc::New(
                        *_Center,
                        r,
                        startAng,
                        *_SweepAngle,
                    ))
                } else {
                    return None;
                }
            }
            (
                SketchPrimitive::Arc(CircularArc {
                    _Center,
                    _Radius,
                    _StartAngle,
                    ..
                }),
                GripKind::ArcEnd,
            ) => {
                let curAng = (cursor._Y - _Center._Y).atan2(cursor._X - _Center._X);
                let mut sweep = curAng - *_StartAngle;
                if sweep <= 0.0 {
                    sweep += std::f64::consts::TAU;
                }
                if sweep > 1e-4 {
                    SketchPrimitive::Arc(CircularArc::New(
                        *_Center,
                        *_Radius,
                        *_StartAngle,
                        sweep,
                    ))
                } else {
                    return None;
                }
            }
            (
                SketchPrimitive::Ellipse(Ellipse {
                    _SemiMajor,
                    _SemiMinor,
                    _Rotation,
                    ..
                }),
                GripKind::EllipseCenter,
            ) => SketchPrimitive::Ellipse(Ellipse::New(
                cursor,
                *_SemiMajor,
                *_SemiMinor,
                *_Rotation,
            )),
            (
                SketchPrimitive::Ellipse(Ellipse {
                    _Center,
                    _SemiMinor,
                    ..
                }),
                GripKind::EllipseMajor,
            ) => {
                let a = (cursor._X - _Center._X).hypot(cursor._Y - _Center._Y);
                let rot = (cursor._Y - _Center._Y).atan2(cursor._X - _Center._X);
                if a > 1e-4 {
                    SketchPrimitive::Ellipse(Ellipse::New(
                        *_Center,
                        a,
                        (*_SemiMinor).min(a),
                        rot,
                    ))
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        let mut res = orig.clone();
        res.SetPrimitive(newPrim);
        Some(res)
    }

    fn ComputeTranslatedEntity(
        &self,
        _doc: &SketchDocument,
        orig: &SketchEntity,
        dx: f64,
        dy: f64,
    ) -> Option<SketchEntity>
    {
        let newPrim = match orig.Primitive() {
            SketchPrimitive::Line(LineSegment { _Start, _End }) => {
                SketchPrimitive::Line(LineSegment::New(
                    SketchPoint::New(_Start._X + dx, _Start._Y + dy),
                    SketchPoint::New(_End._X + dx, _End._Y + dy),
                ))
            }
            SketchPrimitive::Circle(Circle { _Center, _Radius }) => {
                SketchPrimitive::Circle(Circle::New(
                    SketchPoint::New(_Center._X + dx, _Center._Y + dy),
                    *_Radius,
                ))
            }
            SketchPrimitive::Arc(CircularArc {
                _Center,
                _Radius,
                _StartAngle,
                _SweepAngle,
            }) => SketchPrimitive::Arc(CircularArc::New(
                SketchPoint::New(_Center._X + dx, _Center._Y + dy),
                *_Radius,
                *_StartAngle,
                *_SweepAngle,
            )),
            SketchPrimitive::Ellipse(Ellipse {
                _Center,
                _SemiMajor,
                _SemiMinor,
                _Rotation,
            }) => SketchPrimitive::Ellipse(Ellipse::New(
                SketchPoint::New(_Center._X + dx, _Center._Y + dy),
                *_SemiMajor,
                *_SemiMinor,
                *_Rotation,
            )),
            SketchPrimitive::Polyline(header) => SketchPrimitive::Polyline(*header),
        };

        let mut res = orig.clone();
        res.SetPrimitive(newPrim);
        Some(res)
    }
}

impl Default for SketchToolController
{
    fn default() -> Self
    {
        Self::New()
    }
}

//-------------------------------------------------------------------------------------------------
