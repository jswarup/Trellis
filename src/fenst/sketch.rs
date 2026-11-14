// sketch.rs --------------------------------------------------------------------------------------------------------
//! Provider-neutral planar sketch domain models, records, and document hierarchy.
//! Strictly independent of UI rendering (`fascia`) or specific geometric algorithms (`fleck`).

use crate::silo::{Arr, Buff, IArr, Stash, USeg};

//-------------------------------------------------------------------------------------------------
// Planar Point Record (POD / DTO)

/// Represents a 2D planar point in model coordinates (f64).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SketchPoint
{
    pub _X: f64,
    pub _Y: f64,
}

impl SketchPoint
{
    pub const ORIGIN: Self = Self { _X: 0.0, _Y: 0.0 };

    pub fn New(x: f64, y: f64) -> Self
    {
        Self { _X: x, _Y: y }
    }

    pub fn DistanceTo(&self, other: SketchPoint) -> f64
    {
        self.DistanceSqTo(other).sqrt()
    }

    pub fn DistanceSqTo(&self, other: SketchPoint) -> f64
    {
        let dx = self._X - other._X;
        let dy = self._Y - other._Y;
        dx * dx + dy * dy
    }

    pub fn Lerp(&self, other: SketchPoint, t: f64) -> Self
    {
        Self {
            _X: self._X + (other._X - self._X) * t,
            _Y: self._Y + (other._Y - self._Y) * t,
        }
    }

    pub fn Midpoint(&self, other: SketchPoint) -> Self
    {
        self.Lerp(other, 0.5)
    }

    pub fn IsFinite(&self) -> bool
    {
        self._X.is_finite() && self._Y.is_finite()
    }
}

impl From<[f64; 2]> for SketchPoint
{
    fn from(arr: [f64; 2]) -> Self
    {
        Self::New(arr[0], arr[1])
    }
}

impl From<(f64, f64)> for SketchPoint
{
    fn from(pair: (f64, f64)) -> Self
    {
        Self::New(pair.0, pair.1)
    }
}

impl From<SketchPoint> for [f64; 2]
{
    fn from(pt: SketchPoint) -> Self
    {
        [pt._X, pt._Y]
    }
}

impl From<SketchPoint> for (f64, f64)
{
    fn from(pt: SketchPoint) -> Self
    {
        (pt._X, pt._Y)
    }
}

//-------------------------------------------------------------------------------------------------
// Planar Axis-Aligned Bounding Box Record

/// 2D Axis-aligned bounding box defined by minimum and maximum corners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SketchBounds
{
    pub _Min: SketchPoint,
    pub _Max: SketchPoint,
}

impl Default for SketchBounds
{
    fn default() -> Self
    {
        Self::Empty()
    }
}

impl SketchBounds
{
    pub fn New(min: SketchPoint, max: SketchPoint) -> Self
    {
        Self { _Min: min, _Max: max }
    }

    pub fn FromPoints(p1: SketchPoint, p2: SketchPoint) -> Self
    {
        Self {
            _Min: SketchPoint::New(p1._X.min(p2._X), p1._Y.min(p2._Y)),
            _Max: SketchPoint::New(p1._X.max(p2._X), p1._Y.max(p2._Y)),
        }
    }

    pub fn Empty() -> Self
    {
        Self {
            _Min: SketchPoint::New(f64::INFINITY, f64::INFINITY),
            _Max: SketchPoint::New(f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    pub fn Width(&self) -> f64
    {
        if self._Max._X >= self._Min._X {
            self._Max._X - self._Min._X
        } else {
            0.0
        }
    }

    pub fn Height(&self) -> f64
    {
        if self._Max._Y >= self._Min._Y {
            self._Max._Y - self._Min._Y
        } else {
            0.0
        }
    }

    pub fn Center(&self) -> SketchPoint
    {
        SketchPoint::New(
            (self._Min._X + self._Max._X) * 0.5,
            (self._Min._Y + self._Max._Y) * 0.5,
        )
    }

    pub fn ContainsPoint(&self, p: SketchPoint) -> bool
    {
        p._X >= self._Min._X && p._X <= self._Max._X && p._Y >= self._Min._Y && p._Y <= self._Max._Y
    }

    pub fn ContainsBounds(&self, other: SketchBounds) -> bool
    {
        other._Min._X >= self._Min._X
            && other._Max._X <= self._Max._X
            && other._Min._Y >= self._Min._Y
            && other._Max._Y <= self._Max._Y
    }

    pub fn IntersectsBounds(&self, other: SketchBounds) -> bool
    {
        self._Min._X <= other._Max._X
            && self._Max._X >= other._Min._X
            && self._Min._Y <= other._Max._Y
            && self._Max._Y >= other._Min._Y
    }

    pub fn ExpandByPoint(&mut self, p: SketchPoint)
    {
        self._Min._X = self._Min._X.min(p._X);
        self._Min._Y = self._Min._Y.min(p._Y);
        self._Max._X = self._Max._X.max(p._X);
        self._Max._Y = self._Max._Y.max(p._Y);
    }

    pub fn ExpandByBounds(&mut self, other: SketchBounds)
    {
        if other.IsValid() {
            self._Min._X = self._Min._X.min(other._Min._X);
            self._Min._Y = self._Min._Y.min(other._Min._Y);
            self._Max._X = self._Max._X.max(other._Max._X);
            self._Max._Y = self._Max._Y.max(other._Max._Y);
        }
    }

    pub fn IsValid(&self) -> bool
    {
        self._Min._X <= self._Max._X && self._Min._Y <= self._Max._Y && self._Min.IsFinite() && self._Max.IsFinite()
    }
}

//-------------------------------------------------------------------------------------------------
// 3D Workplane Reference Basis Record

/// 3D Orthonormal coordinate frame establishing the workplane in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SketchPlane
{
    pub _Origin: [f64; 3],
    pub _U:      [f64; 3],
    pub _V:      [f64; 3],
    pub _Normal: [f64; 3],
}

impl Default for SketchPlane
{
    fn default() -> Self
    {
        Self::Xy()
    }
}

impl SketchPlane
{
    pub fn Xy() -> Self
    {
        Self {
            _Origin: [0.0, 0.0, 0.0],
            _U:      [1.0, 0.0, 0.0],
            _V:      [0.0, 1.0, 0.0],
            _Normal: [0.0, 0.0, 1.0],
        }
    }

    pub fn New(origin: [f64; 3], u: [f64; 3], v: [f64; 3], normal: [f64; 3]) -> Self
    {
        Self {
            _Origin: origin,
            _U:      u,
            _V:      v,
            _Normal: normal,
        }
    }

    pub fn ProjectTo3d(&self, p: SketchPoint) -> [f64; 3]
    {
        [
            self._Origin[0] + p._X * self._U[0] + p._Y * self._V[0],
            self._Origin[1] + p._X * self._U[1] + p._Y * self._V[1],
            self._Origin[2] + p._X * self._U[2] + p._Y * self._V[2],
        ]
    }
}

//-------------------------------------------------------------------------------------------------
// Analytic Primitive Records (POD / DTO)

/// Finite line segment defined by two distinct endpoints.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineSegment
{
    pub _Start: SketchPoint,
    pub _End:   SketchPoint,
}

impl LineSegment
{
    pub fn New(start: SketchPoint, end: SketchPoint) -> Self
    {
        Self { _Start: start, _End: end }
    }

    pub fn Length(&self) -> f64
    {
        self._Start.DistanceTo(self._End)
    }

    pub fn LengthSq(&self) -> f64
    {
        self._Start.DistanceSqTo(self._End)
    }

    pub fn Midpoint(&self) -> SketchPoint
    {
        self._Start.Midpoint(self._End)
    }

    pub fn Bounds(&self) -> SketchBounds
    {
        SketchBounds::FromPoints(self._Start, self._End)
    }

    pub fn Evaluate(&self, t: f64) -> SketchPoint
    {
        self._Start.Lerp(self._End, t)
    }

    pub fn Direction(&self) -> (f64, f64)
    {
        let len = self.Length();
        if len > 1e-12 {
            ((self._End._X - self._Start._X) / len, (self._End._Y - self._Start._Y) / len)
        } else {
            (1.0, 0.0)
        }
    }
}

/// Planar circle defined by center and positive radius.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Circle
{
    pub _Center: SketchPoint,
    pub _Radius: f64,
}

impl Circle
{
    pub fn New(center: SketchPoint, radius: f64) -> Self
    {
        Self { _Center: center, _Radius: radius.abs() }
    }

    pub fn Circumference(&self) -> f64
    {
        2.0 * std::f64::consts::PI * self._Radius
    }

    pub fn Area(&self) -> f64
    {
        std::f64::consts::PI * self._Radius * self._Radius
    }

    pub fn Bounds(&self) -> SketchBounds
    {
        SketchBounds::New(
            SketchPoint::New(self._Center._X - self._Radius, self._Center._Y - self._Radius),
            SketchPoint::New(self._Center._X + self._Radius, self._Center._Y + self._Radius),
        )
    }

    pub fn Evaluate(&self, angle_rad: f64) -> SketchPoint
    {
        SketchPoint::New(
            self._Center._X + self._Radius * angle_rad.cos(),
            self._Center._Y + self._Radius * angle_rad.sin(),
        )
    }
}

/// Circular arc defined by center, radius, start angle, and signed sweep angle.
/// Signed sweep avoids branch-cut angle ambiguity and supports continuous dragging.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CircularArc
{
    pub _Center:     SketchPoint,
    pub _Radius:     f64,
    pub _StartAngle: f64,
    pub _SweepAngle: f64,
}

impl CircularArc
{
    pub fn New(center: SketchPoint, radius: f64, start_angle: f64, sweep_angle: f64) -> Self
    {
        Self {
            _Center:     center,
            _Radius:     radius.abs(),
            _StartAngle: start_angle,
            _SweepAngle: sweep_angle,
        }
    }

    pub fn EndAngle(&self) -> f64
    {
        self._StartAngle + self._SweepAngle
    }

    pub fn ArcLength(&self) -> f64
    {
        self._Radius * self._SweepAngle.abs()
    }

    pub fn StartPoint(&self) -> SketchPoint
    {
        self.Evaluate(0.0)
    }

    pub fn EndPoint(&self) -> SketchPoint
    {
        self.Evaluate(1.0)
    }

    pub fn Midpoint(&self) -> SketchPoint
    {
        self.Evaluate(0.5)
    }

    pub fn Evaluate(&self, t: f64) -> SketchPoint
    {
        let angle = self._StartAngle + self._SweepAngle * t;
        SketchPoint::New(
            self._Center._X + self._Radius * angle.cos(),
            self._Center._Y + self._Radius * angle.sin(),
        )
    }

    pub fn Bounds(&self) -> SketchBounds
    {
        let p_start = self.StartPoint();
        let p_end = self.EndPoint();
        let mut bounds = SketchBounds::FromPoints(p_start, p_end);

        // Include any quadrant extrema (0, pi/2, pi, 3pi/2) falling within the arc's angular span
        let a1 = self._StartAngle.min(self.EndAngle());
        let a2 = self._StartAngle.max(self.EndAngle());
        let half_pi = std::f64::consts::FRAC_PI_2;

        let k_min = (a1 / half_pi).floor() as i64;
        let k_max = (a2 / half_pi).ceil() as i64;

        let mut k = k_min;
        while k <= k_max {
            let angle = (k as f64) * half_pi;
            if angle >= a1 && angle <= a2 {
                let pt = SketchPoint::New(
                    self._Center._X + self._Radius * angle.cos(),
                    self._Center._Y + self._Radius * angle.sin(),
                );
                bounds.ExpandByPoint(pt);
            }
            k += 1;
        }

        bounds
    }
}

/// Rotated ellipse defined by center, semimajor axis a, semiminor axis b, and rotation angle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ellipse
{
    pub _Center:    SketchPoint,
    pub _SemiMajor: f64,
    pub _SemiMinor: f64,
    pub _Rotation:  f64,
}

impl Ellipse
{
    pub fn New(center: SketchPoint, semi_major: f64, semi_minor: f64, rotation: f64) -> Self
    {
        Self {
            _Center:    center,
            _SemiMajor: semi_major.abs(),
            _SemiMinor: semi_minor.abs(),
            _Rotation:  rotation,
        }
    }

    pub fn Evaluate(&self, t: f64) -> SketchPoint
    {
        let cos_r = self._Rotation.cos();
        let sin_r = self._Rotation.sin();
        let lx = self._SemiMajor * t.cos();
        let ly = self._SemiMinor * t.sin();
        SketchPoint::New(
            self._Center._X + lx * cos_r - ly * sin_r,
            self._Center._Y + lx * sin_r + ly * cos_r,
        )
    }

    pub fn Bounds(&self) -> SketchBounds
    {
        // Analytic bounding box of rotated ellipse
        let cos_r = self._Rotation.cos();
        let sin_r = self._Rotation.sin();
        let a = self._SemiMajor;
        let b = self._SemiMinor;

        let half_w = (a * a * cos_r * cos_r + b * b * sin_r * sin_r).sqrt();
        let half_h = (a * a * sin_r * sin_r + b * b * cos_r * cos_r).sqrt();

        SketchBounds::New(
            SketchPoint::New(self._Center._X - half_w, self._Center._Y - half_h),
            SketchPoint::New(self._Center._X + half_w, self._Center._Y + half_h),
        )
    }
}

/// Polyline slice header into the document-wide contiguous vertex arena.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PolylineHeader
{
    pub _PointOffset: u32,
    pub _PointCount:  u32,
    pub _IsClosed:    bool,
}

impl PolylineHeader
{
    pub fn New(offset: u32, count: u32, is_closed: bool) -> Self
    {
        Self {
            _PointOffset: offset,
            _PointCount:  count,
            _IsClosed:    is_closed,
        }
    }
}

/// Visual styling attributes for sketch stroke presentation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SketchStyle
{
    pub _Color:       [f32; 4],
    pub _LineWidth:   f32,
    pub _LinePattern: u8,
}

impl Default for SketchStyle
{
    fn default() -> Self
    {
        Self {
            _Color:       [1.0, 1.0, 1.0, 1.0],
            _LineWidth:   1.5,
            _LinePattern: 0,
        }
    }
}

impl SketchStyle
{
    pub fn Construction() -> Self
    {
        Self {
            _Color:       [0.5, 0.7, 1.0, 0.7],
            _LineWidth:   1.0,
            _LinePattern: 1,
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Tagged Union of Supported Planar Primitives

/// Tagged union representing any planar sketch primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SketchPrimitive
{
    Line(LineSegment),
    Circle(Circle),
    Arc(CircularArc),
    Ellipse(Ellipse),
    Polyline(PolylineHeader),
}

impl SketchPrimitive
{
    pub fn KindName(&self) -> &'static str
    {
        match self {
            Self::Line(_) => "Line",
            Self::Circle(_) => "Circle",
            Self::Arc(_) => "Arc",
            Self::Ellipse(_) => "Ellipse",
            Self::Polyline(_) => "Polyline",
        }
    }

    pub fn Bounds(&self, point_arena: Arr<'_, SketchPoint>) -> SketchBounds
    {
        match self {
            Self::Line(l) => l.Bounds(),
            Self::Circle(c) => c.Bounds(),
            Self::Arc(a) => a.Bounds(),
            Self::Ellipse(e) => e.Bounds(),
            Self::Polyline(p) => {
                if p._PointCount == 0 {
                    return SketchBounds::Empty();
                }
                let mut bounds = SketchBounds::Empty();
                let end = p._PointOffset + p._PointCount;
                let seg = USeg::New(p._PointOffset, end - 1);
                seg.Traverse(|idx| {
                    if let Some(pt) = point_arena.Get(idx) {
                        bounds.ExpandByPoint(*pt);
                    }
                });
                bounds
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Sketch Entity (Non-Record, Invariant Enforced)

/// An entity in the sketch document, possessing unique monotonic identity, layer assignment,
/// and geometric primitive definition.
#[derive(Clone, Debug, PartialEq)]
pub struct SketchEntity
{
    _Id:             u32,
    _Primitive:      SketchPrimitive,
    _LayerId:        u32,
    _GroupId:        u32,
    _IsConstruction: bool,
}

impl SketchEntity
{
    pub fn New(id: u32, primitive: SketchPrimitive) -> Self
    {
        Self {
            _Id:             id,
            _Primitive:      primitive,
            _LayerId:        0,
            _GroupId:        u32::MAX,
            _IsConstruction: false,
        }
    }

    pub fn Id(&self) -> u32
    {
        self._Id
    }

    pub fn Primitive(&self) -> &SketchPrimitive
    {
        &self._Primitive
    }

    pub fn PrimitiveMut(&mut self) -> &mut SketchPrimitive
    {
        &mut self._Primitive
    }

    pub fn LayerId(&self) -> u32
    {
        self._LayerId
    }

    pub fn GroupId(&self) -> u32
    {
        self._GroupId
    }

    pub fn IsConstruction(&self) -> bool
    {
        self._IsConstruction
    }

    pub fn WithLayer(mut self, layer_id: u32) -> Self
    {
        self._LayerId = layer_id;
        self
    }

    pub fn WithGroup(mut self, group_id: u32) -> Self
    {
        self._GroupId = group_id;
        self
    }

    pub fn WithConstruction(mut self, is_construction: bool) -> Self
    {
        self._IsConstruction = is_construction;
        self
    }

    pub fn SetPrimitive(&mut self, primitive: SketchPrimitive)
    {
        self._Primitive = primitive;
    }

    pub fn Bounds(&self, point_arena: Arr<'_, SketchPoint>) -> SketchBounds
    {
        self._Primitive.Bounds(point_arena)
    }
}

//-------------------------------------------------------------------------------------------------
// Sketch Layer & Group (Non-Records)

/// Layer properties grouping entities with shared visibility, lock state, and style defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct SketchLayer
{
    _Id:             u32,
    _Name:           String,
    _Style:          SketchStyle,
    _IsVisible:      bool,
    _IsLocked:       bool,
    _IsConstruction: bool,
}

impl SketchLayer
{
    pub fn New(id: u32, name: impl Into<String>) -> Self
    {
        Self {
            _Id:             id,
            _Name:           name.into(),
            _Style:          SketchStyle::default(),
            _IsVisible:      true,
            _IsLocked:       false,
            _IsConstruction: false,
        }
    }

    pub fn Id(&self) -> u32
    {
        self._Id
    }

    pub fn Name(&self) -> &str
    {
        &self._Name
    }

    pub fn Style(&self) -> &SketchStyle
    {
        &self._Style
    }

    pub fn IsVisible(&self) -> bool
    {
        self._IsVisible
    }

    pub fn IsLocked(&self) -> bool
    {
        self._IsLocked
    }

    pub fn IsConstruction(&self) -> bool
    {
        self._IsConstruction
    }

    pub fn SetVisible(&mut self, visible: bool)
    {
        self._IsVisible = visible;
    }

    pub fn SetLocked(&mut self, locked: bool)
    {
        self._IsLocked = locked;
    }

    pub fn SetStyle(&mut self, style: SketchStyle)
    {
        self._Style = style;
    }
}

/// Logical grouping of multiple entities for simultaneous selection and transformation.
#[derive(Clone, Debug, PartialEq)]
pub struct SketchGroup
{
    _Id:        u32,
    _Name:      String,
    _MemberIds: Buff<u32>,
}

impl SketchGroup
{
    pub fn New(id: u32, name: impl Into<String>, members: Buff<u32>) -> Self
    {
        Self {
            _Id:        id,
            _Name:      name.into(),
            _MemberIds: members,
        }
    }

    pub fn Id(&self) -> u32
    {
        self._Id
    }

    pub fn Name(&self) -> &str
    {
        &self._Name
    }

    pub fn MemberIds(&self) -> Arr<'_, u32>
    {
        self._MemberIds.Arr()
    }
}

//-------------------------------------------------------------------------------------------------
// Monotonic ID Map Entry for Binary Search

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SketchIdSlot
{
    _Id:    u32,
    _Index: u32,
}

//-------------------------------------------------------------------------------------------------
// Sketch Document (Non-Record, Single Owner of Geometry & History State)

/// Document root owning all entities, point pools, layers, groups, and committed revision state.
pub struct SketchDocument
{
    _Entities:     Stash<SketchEntity>,
    _PointArena:   Stash<SketchPoint>,
    _IdSlots:      Stash<SketchIdSlot>,
    _Layers:       Stash<SketchLayer>,
    _Groups:       Stash<SketchGroup>,
    _NextEntityId: u32,
    _Revision:     u64,
    _Units:        String,
    _Plane:        SketchPlane,
}

impl Default for SketchDocument
{
    fn default() -> Self
    {
        Self::New()
    }
}

impl SketchDocument
{
    pub fn New() -> Self
    {
        let mut layers = Stash::New();
        layers.Push(SketchLayer::New(0, "Default"));

        Self {
            _Entities:     Stash::New(),
            _PointArena:   Stash::New(),
            _IdSlots:      Stash::New(),
            _Layers:       layers,
            _Groups:       Stash::New(),
            _NextEntityId: 1,
            _Revision:     0,
            _Units:        "mm".to_string(),
            _Plane:        SketchPlane::Xy(),
        }
    }

    pub fn Revision(&self) -> u64
    {
        self._Revision
    }

    pub fn Units(&self) -> &str
    {
        &self._Units
    }

    pub fn SetUnits(&mut self, units: impl Into<String>)
    {
        self._Units = units.into();
        self._Revision += 1;
    }

    pub fn Plane(&self) -> &SketchPlane
    {
        &self._Plane
    }

    pub fn SetPlane(&mut self, plane: SketchPlane)
    {
        self._Plane = plane;
        self._Revision += 1;
    }

    pub fn EntityCount(&self) -> u32
    {
        self._Entities.Size()
    }

    pub fn Entities(&self) -> Arr<'_, SketchEntity>
    {
        self._Entities.Arr()
    }

    pub fn PointArena(&self) -> Arr<'_, SketchPoint>
    {
        self._PointArena.Arr()
    }

    pub fn PolylinePoints(&self, offset: u32, count: u32) -> Buff<SketchPoint>
    {
        let mut stash = Stash::New();
        let arena = self._PointArena.Arr();
        let mut i = 0u32;
        while i < count {
            if let Some(pt) = arena.Get(offset + i) {
                stash.Push(*pt);
            }
            i += 1;
        }
        stash.IntoBuff()
    }

    pub fn LayerCount(&self) -> u32
    {
        self._Layers.Size()
    }

    pub fn Layers(&self) -> Arr<'_, SketchLayer>
    {
        self._Layers.Arr()
    }

    pub fn GroupCount(&self) -> u32
    {
        self._Groups.Size()
    }

    pub fn Groups(&self) -> Arr<'_, SketchGroup>
    {
        self._Groups.Arr()
    }

    // Binary search ID lookup
    fn FindSlot(&self, id: u32) -> Option<u32>
    {
        let sz = self._IdSlots.Size();
        if sz == 0 {
            return None;
        }
        let slots = self._IdSlots.Arr();
        let seg = USeg::FromLen(sz);
        let pos = seg.BinarySearch(|idx| {
            let slot = slots.Get(idx).unwrap();
            slot._Id.cmp(&id)
        });
        match pos {
            Ok(idx) => Some(slots.Get(idx).unwrap()._Index),
            Err(_) => None,
        }
    }

    pub fn FindEntity(&self, id: u32) -> Option<&SketchEntity>
    {
        let idx = self.FindSlot(id)?;
        Some(&self._Entities[idx])
    }

    pub fn FindEntityMut(&mut self, id: u32) -> Option<&mut SketchEntity>
    {
        let idx = self.FindSlot(id)?;
        Some(&mut self._Entities[idx])
    }

    pub fn InsertEntity(&mut self, entity: SketchEntity) -> u32
    {
        if entity.Id() >= self._NextEntityId {
            self._NextEntityId = entity.Id() + 1;
        }
        self.InsertEntityInternal(entity)
    }

    fn InsertEntityInternal(&mut self, entity: SketchEntity) -> u32
    {
        let id = entity.Id();
        let idx = self._Entities.Size();
        self._Entities.Push(entity);

        // Maintain sorted _IdSlots
        let new_slot = SketchIdSlot { _Id: id, _Index: idx };
        let mut inserted = false;
        let sz = self._IdSlots.Size();
        if sz == 0 {
            self._IdSlots.Push(new_slot);
        } else {
            let mut temp = Stash::WithCapacity(sz + 1);
            let mut i = 0u32;
            let slots = self._IdSlots.Arr();
            while i < sz {
                let s = *slots.Get(i).unwrap();
                if !inserted && new_slot._Id < s._Id {
                    temp.Push(new_slot);
                    inserted = true;
                }
                temp.Push(s);
                i += 1;
            }
            if !inserted {
                temp.Push(new_slot);
            }
            self._IdSlots = temp;
        }

        self._Revision += 1;
        id
    }

    fn AllocateId(&mut self) -> u32
    {
        let id = self._NextEntityId;
        self._NextEntityId += 1;
        id
    }

    pub fn AddLine(&mut self, start: SketchPoint, end: SketchPoint) -> Result<u32, String>
    {
        if !start.IsFinite() || !end.IsFinite() {
            return Err("Line endpoints must be finite numbers".into());
        }
        if start.DistanceSqTo(end) < 1e-12 {
            return Err("Line segment length must be strictly positive".into());
        }
        let id = self.AllocateId();
        let line = LineSegment::New(start, end);
        let entity = SketchEntity::New(id, SketchPrimitive::Line(line));
        Ok(self.InsertEntityInternal(entity))
    }

    pub fn AddCircle(&mut self, center: SketchPoint, radius: f64) -> Result<u32, String>
    {
        if !center.IsFinite() || !radius.is_finite() {
            return Err("Circle parameters must be finite numbers".into());
        }
        if radius <= 1e-9 {
            return Err("Circle radius must be strictly positive".into());
        }
        let id = self.AllocateId();
        let circle = Circle::New(center, radius);
        let entity = SketchEntity::New(id, SketchPrimitive::Circle(circle));
        Ok(self.InsertEntityInternal(entity))
    }

    pub fn AddArc(&mut self, center: SketchPoint, radius: f64, start: f64, sweep: f64) -> Result<u32, String>
    {
        if !center.IsFinite() || !radius.is_finite() || !start.is_finite() || !sweep.is_finite() {
            return Err("Arc parameters must be finite numbers".into());
        }
        if radius <= 1e-9 {
            return Err("Arc radius must be strictly positive".into());
        }
        if sweep.abs() <= 1e-9 || sweep.abs() >= (2.0 * std::f64::consts::PI + 1e-9) {
            return Err("Arc sweep must be strictly between 0 and 2*pi radians".into());
        }
        let id = self.AllocateId();
        let arc = CircularArc::New(center, radius, start, sweep);
        let entity = SketchEntity::New(id, SketchPrimitive::Arc(arc));
        Ok(self.InsertEntityInternal(entity))
    }

    pub fn AddEllipse(&mut self, center: SketchPoint, semi_major: f64, semi_minor: f64, rotation: f64) -> Result<u32, String>
    {
        if !center.IsFinite() || !semi_major.is_finite() || !semi_minor.is_finite() || !rotation.is_finite() {
            return Err("Ellipse parameters must be finite numbers".into());
        }
        if semi_major <= 1e-9 || semi_minor <= 1e-9 {
            return Err("Ellipse semiaxes must be strictly positive".into());
        }
        let id = self.AllocateId();
        let ellipse = Ellipse::New(center, semi_major, semi_minor, rotation);
        let entity = SketchEntity::New(id, SketchPrimitive::Ellipse(ellipse));
        Ok(self.InsertEntityInternal(entity))
    }

    pub fn AddPolyline(&mut self, points: Arr<'_, SketchPoint>, is_closed: bool) -> Result<u32, String>
    {
        let count = points.Len();
        if is_closed && count < 3 {
            return Err("Closed polyline requires at least 3 points".into());
        }
        if !is_closed && count < 2 {
            return Err("Open polyline requires at least 2 points".into());
        }

        let mut all_finite = true;
        points.Traverse(|pt| {
            all_finite &= pt.IsFinite();
        });
        if !all_finite {
            return Err("All polyline points must be finite numbers".into());
        }

        let offset = self._PointArena.Size();
        points.Traverse(|pt| {
            self._PointArena.Push(*pt);
        });

        let id = self.AllocateId();
        let header = PolylineHeader::New(offset, count, is_closed);
        let entity = SketchEntity::New(id, SketchPrimitive::Polyline(header));
        Ok(self.InsertEntityInternal(entity))
    }

    pub fn RemoveEntity(&mut self, id: u32) -> Result<SketchEntity, String>
    {
        let idx = self.FindSlot(id).ok_or_else(|| format!("Entity {} not found", id))?;

        // Extract entity
        let mut new_entities = Stash::WithCapacity(self._Entities.Size() - 1);
        let mut removed = None;
        let mut i = 0u32;
        let arr = self._Entities.Arr();
        while i < arr.Len() {
            let e = arr.Get(i).unwrap().clone();
            if i == idx {
                removed = Some(e);
            } else {
                new_entities.Push(e);
            }
            i += 1;
        }
        self._Entities = new_entities;

        // Rebuild _IdSlots
        let mut new_slots = Stash::WithCapacity(self._Entities.Size());
        let mut j = 0u32;
        let updated_entities = self._Entities.Arr();
        while j < updated_entities.Len() {
            let ent = updated_entities.Get(j).unwrap();
            new_slots.Push(SketchIdSlot { _Id: ent.Id(), _Index: j });
            j += 1;
        }
        // Sort slots by ID
        let mut sorted = false;
        let mut pass = 0u32;
        while pass < new_slots.Size() && !sorted {
            sorted = true;
            let mut k = 0u32;
            let sz = new_slots.Size() - 1;
            while k < sz {
                let s1 = *new_slots.Arr().Get(k).unwrap();
                let s2 = *new_slots.Arr().Get(k + 1).unwrap();
                if s1._Id > s2._Id {
                    new_slots[k] = s2;
                    new_slots[k + 1] = s1;
                    sorted = false;
                }
                k += 1;
            }
            pass += 1;
        }
        self._IdSlots = new_slots;
        self._Revision += 1;

        removed.ok_or_else(|| "Failed to extract entity".into())
    }

    pub fn ComputeBounds(&self) -> SketchBounds
    {
        let mut bounds = SketchBounds::Empty();
        let arena = self._PointArena.Arr();
        self._Entities.Arr().Traverse(|e| {
            bounds.ExpandByBounds(e.Bounds(arena));
        });
        bounds
    }
}

//-------------------------------------------------------------------------------------------------
