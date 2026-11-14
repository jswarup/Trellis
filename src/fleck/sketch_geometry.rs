// sketch_geometry.rs ------------------------------------------------------------------------------------------------
//! Analytic geometry routines, parametric curve evaluation, and 2D affine transforms for sketch primitives.

use crate::fenst::sketch::{Circle, CircularArc, Ellipse, LineSegment, SketchPoint};

//-------------------------------------------------------------------------------------------------
// Analytic Curve Evaluator & Metric Utilities

pub struct SketchGeometry;

impl SketchGeometry
{
    // Line Segment Analytics
    pub fn LinePointAt(line: &LineSegment, t: f64) -> SketchPoint
    {
        line.Evaluate(t.clamp(0.0, 1.0))
    }

    pub fn LineTangent(line: &LineSegment) -> (f64, f64)
    {
        line.Direction()
    }

    pub fn LineNormal(line: &LineSegment) -> (f64, f64)
    {
        let (dx, dy) = line.Direction();
        (-dy, dx)
    }

    // Circle Analytics
    pub fn CirclePointAt(circle: &Circle, angle_rad: f64) -> SketchPoint
    {
        circle.Evaluate(angle_rad)
    }

    pub fn CircleTangentAt(_circle: &Circle, angle_rad: f64) -> (f64, f64)
    {
        (-angle_rad.sin(), angle_rad.cos())
    }

    pub fn CircleNormalAt(_circle: &Circle, angle_rad: f64) -> (f64, f64)
    {
        (angle_rad.cos(), angle_rad.sin())
    }

    pub fn CircleCurvature(circle: &Circle) -> f64
    {
        if circle._Radius > 1e-12 {
            1.0 / circle._Radius
        } else {
            0.0
        }
    }

    // Circular Arc Analytics
    pub fn ArcPointAt(arc: &CircularArc, t: f64) -> SketchPoint
    {
        arc.Evaluate(t.clamp(0.0, 1.0))
    }

    pub fn ArcTangentAt(arc: &CircularArc, t: f64) -> (f64, f64)
    {
        let angle = arc._StartAngle + arc._SweepAngle * t.clamp(0.0, 1.0);
        let sign = if arc._SweepAngle >= 0.0 { 1.0 } else { -1.0 };
        (-angle.sin() * sign, angle.cos() * sign)
    }

    pub fn ArcSagitta(arc: &CircularArc) -> f64
    {
        let half_sweep = (arc._SweepAngle * 0.5).abs();
        arc._Radius * (1.0 - half_sweep.cos())
    }

    // Ellipse Analytics
    pub fn EllipsePointAt(ellipse: &Ellipse, t: f64) -> SketchPoint
    {
        ellipse.Evaluate(t)
    }

    pub fn EllipseCurvatureAt(ellipse: &Ellipse, t: f64) -> f64
    {
        let a = ellipse._SemiMajor;
        let b = ellipse._SemiMinor;
        let sin_t = t.sin();
        let cos_t = t.cos();
        let denom = (a * a * sin_t * sin_t + b * b * cos_t * cos_t).powf(1.5);
        if denom > 1e-12 {
            (a * b) / denom
        } else {
            0.0
        }
    }

    pub fn EllipseMaxCurvature(ellipse: &Ellipse) -> f64
    {
        if ellipse._SemiMinor > 1e-12 {
            ellipse._SemiMajor / (ellipse._SemiMinor * ellipse._SemiMinor)
        } else {
            0.0
        }
    }

    pub fn EllipseMinCurvature(ellipse: &Ellipse) -> f64
    {
        if ellipse._SemiMajor > 1e-12 {
            ellipse._SemiMinor / (ellipse._SemiMajor * ellipse._SemiMajor)
        } else {
            0.0
        }
    }

    // 2D Affine Transformations
    pub fn TranslatePoint(p: SketchPoint, dx: f64, dy: f64) -> SketchPoint
    {
        SketchPoint::New(p._X + dx, p._Y + dy)
    }

    pub fn RotatePoint(p: SketchPoint, pivot: SketchPoint, angle_rad: f64) -> SketchPoint
    {
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let dx = p._X - pivot._X;
        let dy = p._Y - pivot._Y;
        SketchPoint::New(
            pivot._X + dx * cos_a - dy * sin_a,
            pivot._Y + dx * sin_a + dy * cos_a,
        )
    }

    pub fn ScalePoint(p: SketchPoint, pivot: SketchPoint, scale: f64) -> SketchPoint
    {
        SketchPoint::New(
            pivot._X + (p._X - pivot._X) * scale,
            pivot._Y + (p._Y - pivot._Y) * scale,
        )
    }

    pub fn TranslateLine(line: LineSegment, dx: f64, dy: f64) -> LineSegment
    {
        LineSegment::New(
            Self::TranslatePoint(line._Start, dx, dy),
            Self::TranslatePoint(line._End, dx, dy),
        )
    }

    pub fn RotateLine(line: LineSegment, pivot: SketchPoint, angle_rad: f64) -> LineSegment
    {
        LineSegment::New(
            Self::RotatePoint(line._Start, pivot, angle_rad),
            Self::RotatePoint(line._End, pivot, angle_rad),
        )
    }

    pub fn ScaleLine(line: LineSegment, pivot: SketchPoint, scale: f64) -> LineSegment
    {
        LineSegment::New(
            Self::ScalePoint(line._Start, pivot, scale),
            Self::ScalePoint(line._End, pivot, scale),
        )
    }

    pub fn TranslateCircle(circle: Circle, dx: f64, dy: f64) -> Circle
    {
        Circle::New(Self::TranslatePoint(circle._Center, dx, dy), circle._Radius)
    }

    pub fn RotateCircle(circle: Circle, pivot: SketchPoint, angle_rad: f64) -> Circle
    {
        Circle::New(Self::RotatePoint(circle._Center, pivot, angle_rad), circle._Radius)
    }

    pub fn ScaleCircle(circle: Circle, pivot: SketchPoint, scale: f64) -> Circle
    {
        Circle::New(Self::ScalePoint(circle._Center, pivot, scale), circle._Radius * scale.abs())
    }

    pub fn TranslateArc(arc: CircularArc, dx: f64, dy: f64) -> CircularArc
    {
        CircularArc::New(
            Self::TranslatePoint(arc._Center, dx, dy),
            arc._Radius,
            arc._StartAngle,
            arc._SweepAngle,
        )
    }

    pub fn RotateArc(arc: CircularArc, pivot: SketchPoint, angle_rad: f64) -> CircularArc
    {
        CircularArc::New(
            Self::RotatePoint(arc._Center, pivot, angle_rad),
            arc._Radius,
            arc._StartAngle + angle_rad,
            arc._SweepAngle,
        )
    }

    pub fn ScaleArc(arc: CircularArc, pivot: SketchPoint, scale: f64) -> CircularArc
    {
        CircularArc::New(
            Self::ScalePoint(arc._Center, pivot, scale),
            arc._Radius * scale.abs(),
            arc._StartAngle,
            arc._SweepAngle,
        )
    }

    pub fn TranslateEllipse(ellipse: Ellipse, dx: f64, dy: f64) -> Ellipse
    {
        Ellipse::New(
            Self::TranslatePoint(ellipse._Center, dx, dy),
            ellipse._SemiMajor,
            ellipse._SemiMinor,
            ellipse._Rotation,
        )
    }

    pub fn RotateEllipse(ellipse: Ellipse, pivot: SketchPoint, angle_rad: f64) -> Ellipse
    {
        Ellipse::New(
            Self::RotatePoint(ellipse._Center, pivot, angle_rad),
            ellipse._SemiMajor,
            ellipse._SemiMinor,
            ellipse._Rotation + angle_rad,
        )
    }

    pub fn ScaleEllipse(ellipse: Ellipse, pivot: SketchPoint, scale: f64) -> Ellipse
    {
        Ellipse::New(
            Self::ScalePoint(ellipse._Center, pivot, scale),
            ellipse._SemiMajor * scale.abs(),
            ellipse._SemiMinor * scale.abs(),
            ellipse._Rotation,
        )
    }
}

//-------------------------------------------------------------------------------------------------
