// sketch_query.rs ---------------------------------------------------------------------------------------------------
//! Distance calculation, hit-testing, snap candidate searching, and marquee queries for sketch primitives.

use crate::fenst::sketch::{Circle, CircularArc, Ellipse, LineSegment, SketchPoint, SketchPrimitive};
use crate::silo::{Arr, Stash};

//-------------------------------------------------------------------------------------------------
// Snap Candidate Records

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SnapKind
{
    Endpoint,
    Midpoint,
    Center,
    Quadrant,
    Grid,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapCandidate
{
    pub _Point:    SketchPoint,
    pub _Kind:     SnapKind,
    pub _EntityId: u32,
    pub _Distance: f64,
}

impl SnapCandidate
{
    pub fn New(point: SketchPoint, kind: SnapKind, entity_id: u32, distance: f64) -> Self
    {
        Self {
            _Point:    point,
            _Kind:     kind,
            _EntityId: entity_id,
            _Distance: distance,
        }
    }
}

//-------------------------------------------------------------------------------------------------
// Geometric Queries & Distance Solvers

pub struct SketchQuery;

impl SketchQuery
{
    // Distance from point to finite line segment
    pub fn PointToLineDistance(p: SketchPoint, line: &LineSegment) -> f64
    {
        let l2 = line.LengthSq();
        if l2 < 1e-12 {
            return p.DistanceTo(line._Start);
        }
        let t = ((p._X - line._Start._X) * (line._End._X - line._Start._X)
            + (p._Y - line._Start._Y) * (line._End._Y - line._Start._Y))
            / l2;
        let t_clamped = t.clamp(0.0, 1.0);
        let proj = line.Evaluate(t_clamped);
        p.DistanceTo(proj)
    }

    // Distance from point to circle perimeter
    pub fn PointToCircleDistance(p: SketchPoint, circle: &Circle) -> f64
    {
        let d_center = p.DistanceTo(circle._Center);
        (d_center - circle._Radius).abs()
    }

    // Distance from point to circular arc
    pub fn PointToArcDistance(p: SketchPoint, arc: &CircularArc) -> f64
    {
        let dx = p._X - arc._Center._X;
        let dy = p._Y - arc._Center._Y;
        let angle = dy.atan2(dx);

        let mut diff = angle - arc._StartAngle;
        let two_pi = 2.0 * std::f64::consts::PI;

        if arc._SweepAngle >= 0.0 {
            while diff < 0.0 {
                diff += two_pi;
            }
            while diff >= two_pi {
                diff -= two_pi;
            }
            if diff <= arc._SweepAngle {
                let d_center = (dx * dx + dy * dy).sqrt();
                return (d_center - arc._Radius).abs();
            }
        } else {
            while diff > 0.0 {
                diff -= two_pi;
            }
            while diff <= -two_pi {
                diff += two_pi;
            }
            if diff >= arc._SweepAngle {
                let d_center = (dx * dx + dy * dy).sqrt();
                return (d_center - arc._Radius).abs();
            }
        }

        // Outside angular sweep -> distance to closest endpoint
        let d_start = p.DistanceTo(arc.StartPoint());
        let d_end = p.DistanceTo(arc.EndPoint());
        d_start.min(d_end)
    }

    // Distance from point to rotated ellipse (analytic multi-sample projection)
    pub fn PointToEllipseDistance(p: SketchPoint, ellipse: &Ellipse) -> f64
    {
        // Transform point into unrotated, center-aligned coordinate space
        let cos_r = (-ellipse._Rotation).cos();
        let sin_r = (-ellipse._Rotation).sin();
        let dx = p._X - ellipse._Center._X;
        let dy = p._Y - ellipse._Center._Y;
        let px = (dx * cos_r - dy * sin_r).abs();
        let py = (dx * sin_r + dy * cos_r).abs();

        let a = ellipse._SemiMajor;
        let b = ellipse._SemiMinor;

        if a < 1e-9 || b < 1e-9 {
            return p.DistanceTo(ellipse._Center);
        }

        // Bounded Newton iteration to solve closest point on quadrant
        let mut t = std::f64::consts::FRAC_PI_4; // initial guess: 45 degrees
        let mut iter = 0;
        while iter < 8 {
            let cos_t = t.cos();
            let sin_t = t.sin();
            let x = a * cos_t;
            let y = b * sin_t;
            let ex = (a * a - b * b) * cos_t * cos_t * cos_t / a;
            let ey = (b * b - a * a) * sin_t * sin_t * sin_t / b;
            let rx = x - ex;
            let ry = y - ey;
            let qx = px - ex;
            let qy = py - ey;
            let r = rx.hypot(ry);
            let q = qx.hypot(qy);
            let delta_c = r * ((qy * rx - qx * ry) / (r * q)).asin();
            let delta_t = delta_c / (a * a * sin_t * sin_t + b * b * cos_t * cos_t).sqrt();
            t = (t + delta_t).clamp(0.0, std::f64::consts::FRAC_PI_2);
            iter += 1;
        }

        let closest_x = a * t.cos();
        let closest_y = b * t.sin();
        (px - closest_x).hypot(py - closest_y)
    }

    // Distance from point to polyline
    pub fn PointToPolylineDistance(p: SketchPoint, offset: u32, count: u32, is_closed: bool, arena: Arr<'_, SketchPoint>) -> f64
    {
        if count < 2 {
            return f64::INFINITY;
        }
        let mut min_dist = f64::INFINITY;
        let seg_count = if is_closed { count } else { count - 1 };

        let mut i = 0u32;
        while i < seg_count {
            let idx1 = offset + i;
            let idx2 = if is_closed && i == count - 1 {
                offset
            } else {
                offset + i + 1
            };
            if let (Some(p1), Some(p2)) = (arena.Get(idx1), arena.Get(idx2)) {
                let seg = LineSegment::New(*p1, *p2);
                min_dist = min_dist.min(Self::PointToLineDistance(p, &seg));
            }
            i += 1;
        }

        min_dist
    }

    // Distance from point to any primitive
    pub fn PointToPrimitiveDistance(p: SketchPoint, prim: &SketchPrimitive, arena: Arr<'_, SketchPoint>) -> f64
    {
        match prim {
            SketchPrimitive::Line(l) => Self::PointToLineDistance(p, l),
            SketchPrimitive::Circle(c) => Self::PointToCircleDistance(p, c),
            SketchPrimitive::Arc(a) => Self::PointToArcDistance(p, a),
            SketchPrimitive::Ellipse(e) => Self::PointToEllipseDistance(p, e),
            SketchPrimitive::Polyline(poly) => {
                Self::PointToPolylineDistance(p, poly._PointOffset, poly._PointCount, poly._IsClosed, arena)
            }
        }
    }

    // Extract snap feature candidates from a primitive
    pub fn CollectSnapCandidates(
        prim: &SketchPrimitive,
        entity_id: u32,
        arena: Arr<'_, SketchPoint>,
        out: &mut Stash<SnapCandidate>,
    )
    {
        match prim {
            SketchPrimitive::Line(l) => {
                out.Push(SnapCandidate::New(l._Start, SnapKind::Endpoint, entity_id, 0.0));
                out.Push(SnapCandidate::New(l._End, SnapKind::Endpoint, entity_id, 0.0));
                out.Push(SnapCandidate::New(l.Midpoint(), SnapKind::Midpoint, entity_id, 0.0));
            }
            SketchPrimitive::Circle(c) => {
                out.Push(SnapCandidate::New(c._Center, SnapKind::Center, entity_id, 0.0));
                out.Push(SnapCandidate::New(
                    c.Evaluate(0.0),
                    SnapKind::Quadrant,
                    entity_id,
                    0.0,
                ));
                out.Push(SnapCandidate::New(
                    c.Evaluate(std::f64::consts::FRAC_PI_2),
                    SnapKind::Quadrant,
                    entity_id,
                    0.0,
                ));
                out.Push(SnapCandidate::New(
                    c.Evaluate(std::f64::consts::PI),
                    SnapKind::Quadrant,
                    entity_id,
                    0.0,
                ));
                out.Push(SnapCandidate::New(
                    c.Evaluate(3.0 * std::f64::consts::FRAC_PI_2),
                    SnapKind::Quadrant,
                    entity_id,
                    0.0,
                ));
            }
            SketchPrimitive::Arc(a) => {
                out.Push(SnapCandidate::New(a._Center, SnapKind::Center, entity_id, 0.0));
                out.Push(SnapCandidate::New(a.StartPoint(), SnapKind::Endpoint, entity_id, 0.0));
                out.Push(SnapCandidate::New(a.EndPoint(), SnapKind::Endpoint, entity_id, 0.0));
                out.Push(SnapCandidate::New(a.Midpoint(), SnapKind::Midpoint, entity_id, 0.0));
            }
            SketchPrimitive::Ellipse(e) => {
                out.Push(SnapCandidate::New(e._Center, SnapKind::Center, entity_id, 0.0));
                out.Push(SnapCandidate::New(e.Evaluate(0.0), SnapKind::Quadrant, entity_id, 0.0));
                out.Push(SnapCandidate::New(
                    e.Evaluate(std::f64::consts::FRAC_PI_2),
                    SnapKind::Quadrant,
                    entity_id,
                    0.0,
                ));
                out.Push(SnapCandidate::New(
                    e.Evaluate(std::f64::consts::PI),
                    SnapKind::Quadrant,
                    entity_id,
                    0.0,
                ));
                out.Push(SnapCandidate::New(
                    e.Evaluate(3.0 * std::f64::consts::FRAC_PI_2),
                    SnapKind::Quadrant,
                    entity_id,
                    0.0,
                ));
            }
            SketchPrimitive::Polyline(poly) => {
                let mut i = 0u32;
                while i < poly._PointCount {
                    if let Some(pt) = arena.Get(poly._PointOffset + i) {
                        out.Push(SnapCandidate::New(*pt, SnapKind::Endpoint, entity_id, 0.0));
                    }
                    if i + 1 < poly._PointCount
                        && let (Some(p1), Some(p2)) =
                            (arena.Get(poly._PointOffset + i), arena.Get(poly._PointOffset + i + 1))
                    {
                        out.Push(SnapCandidate::New(p1.Midpoint(*p2), SnapKind::Midpoint, entity_id, 0.0));
                    }
                    i += 1;
                }
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------
