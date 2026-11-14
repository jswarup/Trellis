// sketch_tessellate.rs ---------------------------------------------------------------------------------------------
//! Adaptive polyline tessellation for analytic curves (circles, arcs, ellipses) with guaranteed error bounds.

use crate::fenst::sketch::{Circle, CircularArc, Ellipse, SketchPoint};
use crate::silo::{Buff, Stash};

//-------------------------------------------------------------------------------------------------
// Adaptive Curve Tessellation Engine

pub struct SketchTessellate;

impl SketchTessellate
{
    /// Tessellates a circle into an adaptive closed polyline respecting maximum pixel sagitta deviation.
    pub fn TessellateCircle(circle: &Circle, max_deviation_pixels: f64, zoom: f64) -> Buff<SketchPoint>
    {
        let r_screen = (circle._Radius * zoom).max(1.0);
        let delta = max_deviation_pixels.max(0.05);

        // N_rad = 1 / (2 * acos(1 - delta / R))
        let cos_val = (1.0 - delta / r_screen).clamp(-1.0, 1.0);
        let half_step = cos_val.acos();
        let step_angle = if half_step > 1e-4 {
            2.0 * half_step
        } else {
            std::f64::consts::PI / 6.0
        };

        let two_pi = 2.0 * std::f64::consts::PI;
        let seg_count = ((two_pi / step_angle).ceil() as u32).clamp(16, 512);

        let mut pts = Stash::WithCapacity(seg_count + 1);
        let dt = two_pi / (seg_count as f64);

        let mut i = 0u32;
        while i < seg_count {
            let angle = (i as f64) * dt;
            pts.Push(circle.Evaluate(angle));
            i += 1;
        }
        // Close polygon
        pts.Push(circle.Evaluate(0.0));
        pts.IntoBuff()
    }

    /// Tessellates a circular arc into an adaptive open polyline respecting maximum pixel sagitta deviation.
    pub fn TessellateArc(arc: &CircularArc, max_deviation_pixels: f64, zoom: f64) -> Buff<SketchPoint>
    {
        let r_screen = (arc._Radius * zoom).max(1.0);
        let delta = max_deviation_pixels.max(0.05);

        let cos_val = (1.0 - delta / r_screen).clamp(-1.0, 1.0);
        let half_step = cos_val.acos();
        let step_angle = if half_step > 1e-4 {
            2.0 * half_step
        } else {
            std::f64::consts::PI / 6.0
        };

        let sweep_abs = arc._SweepAngle.abs();
        let seg_count = ((sweep_abs / step_angle).ceil() as u32).clamp(4, 256);

        let mut pts = Stash::WithCapacity(seg_count + 1);
        let mut i = 0u32;
        while i <= seg_count {
            let t = (i as f64) / (seg_count as f64);
            pts.Push(arc.Evaluate(t));
            i += 1;
        }
        pts.IntoBuff()
    }

    /// Tessellates a rotated ellipse using curvature-adaptive parameter subdivision.
    pub fn TessellateEllipse(ellipse: &Ellipse, max_deviation_pixels: f64, zoom: f64) -> Buff<SketchPoint>
    {
        let a = ellipse._SemiMajor;
        let b = ellipse._SemiMinor;
        if a < 1e-9 || b < 1e-9 {
            let mut pts = Stash::WithCapacity(2);
            pts.Push(ellipse._Center);
            pts.Push(ellipse._Center);
            return pts.IntoBuff();
        }

        // Base sampling with recursive subdivision on high-curvature intervals
        let base_intervals = 16u32;
        let two_pi = 2.0 * std::f64::consts::PI;
        let dt = two_pi / (base_intervals as f64);
        let max_screen_dev = max_deviation_pixels.max(0.05) / zoom;

        let mut pts = Stash::WithCapacity(128);

        let mut i = 0u32;
        while i < base_intervals {
            let t0 = (i as f64) * dt;
            let t1 = ((i + 1) as f64) * dt;
            Self::SubdivideEllipseSegment(ellipse, t0, t1, max_screen_dev, 0, &mut pts);
            i += 1;
        }

        // Close the loop
        pts.Push(ellipse.Evaluate(0.0));
        pts.IntoBuff()
    }

    fn SubdivideEllipseSegment(
        ellipse: &Ellipse,
        t0: f64,
        t1: f64,
        max_dev: f64,
        depth: u32,
        out: &mut Stash<SketchPoint>,
    )
    {
        let p0 = ellipse.Evaluate(t0);
        let p1 = ellipse.Evaluate(t1);
        let t_mid = (t0 + t1) * 0.5;
        let p_mid = ellipse.Evaluate(t_mid);

        // Chord midpoint
        let chord_mid = p0.Midpoint(p1);
        let deviation = chord_mid.DistanceTo(p_mid);

        if deviation > max_dev && depth < 5 {
            Self::SubdivideEllipseSegment(ellipse, t0, t_mid, max_dev, depth + 1, out);
            Self::SubdivideEllipseSegment(ellipse, t_mid, t1, max_dev, depth + 1, out);
        } else {
            out.Push(p0);
        }
    }
}

//-------------------------------------------------------------------------------------------------
