// sketch_fillet.rs --------------------------------------------------------------------------------------------------
//! 2D Fillet (tangent corner rounding) and Chamfer (corner beveling) geometry engines for sketch lines.

use std::f64::consts::PI;
use crate::fenst::sketch::{CircularArc, LineSegment, SketchDocument, SketchEntity, SketchPoint, SketchPrimitive};
use crate::fenst::sketch_edit::SketchPatch;
use crate::silo::{Buff, Stash};

//-------------------------------------------------------------------------------------------------

/// Result of a 2D corner fillet calculation between two line segments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilletResult
{
    pub trimmed_l1: LineSegment,
    pub trimmed_l2: LineSegment,
    pub arc:        CircularArc,
    pub tangent1:   SketchPoint,
    pub tangent2:   SketchPoint,
    pub center:     SketchPoint,
}

/// Result of a 2D corner chamfer calculation between two line segments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChamferResult
{
    pub trimmed_l1:   LineSegment,
    pub trimmed_l2:   LineSegment,
    pub chamfer_line: LineSegment,
    pub cut1:         SketchPoint,
    pub cut2:         SketchPoint,
}

//-------------------------------------------------------------------------------------------------

pub struct SketchFillet;

impl SketchFillet
{
    /// Finds the shared vertex between two line segments if they connect at their endpoints.
    /// Returns `(shared_vertex, l1_at_end, l2_at_end)`.
    pub fn FindSharedVertex(l1: &LineSegment, l2: &LineSegment) -> Option<(SketchPoint, bool, bool)>
    {
        const EPS_SQ: f64 = 1e-4;

        if l1._End.DistanceSqTo(l2._Start) < EPS_SQ {
            return Some((l1._End, true, false));
        }
        if l1._End.DistanceSqTo(l2._End) < EPS_SQ {
            return Some((l1._End, true, true));
        }
        if l1._Start.DistanceSqTo(l2._Start) < EPS_SQ {
            return Some((l1._Start, false, false));
        }
        if l1._Start.DistanceSqTo(l2._End) < EPS_SQ {
            return Some((l1._Start, false, true));
        }

        // Check virtual intersection if not strictly touching at endpoints
        Self::IntersectLines(l1, l2)
    }

    /// Computes intersection between two lines and whether they join near the intersection.
    fn IntersectLines(l1: &LineSegment, l2: &LineSegment) -> Option<(SketchPoint, bool, bool)>
    {
        let d1x = l1._End._X - l1._Start._X;
        let d1y = l1._End._Y - l1._Start._Y;
        let d2x = l2._End._X - l2._Start._X;
        let d2y = l2._End._Y - l2._Start._Y;

        let denom = d1x * d2y - d1y * d2x;
        if denom.abs() < 1e-6 {
            return None; // Parallel or collinear
        }

        let num_t = (l2._Start._X - l1._Start._X) * d2y - (l2._Start._Y - l1._Start._Y) * d2x;
        let t = num_t / denom;

        let ix = l1._Start._X + t * d1x;
        let iy = l1._Start._Y + t * d1y;
        let ip = SketchPoint::New(ix, iy);

        let l1_at_end = t >= 0.5;
        let d2_len_sq = d2x * d2x + d2y * d2y;
        let num_u = (l2._Start._X - l1._Start._X) * d1y - (l2._Start._Y - l1._Start._Y) * d1x;
        let u = if d2_len_sq > 1e-9 { num_u / denom } else { 0.0 };
        let l2_at_end = u >= 0.5;

        Some((ip, l1_at_end, l2_at_end))
    }

    /// Computes a corner fillet of specified `radius` between two line segments.
    pub fn ComputeFillet(l1: &LineSegment, l2: &LineSegment, radius: f64) -> Result<FilletResult, String>
    {
        if !radius.is_finite() || radius <= 1e-4 {
            return Err("Fillet radius must be a positive non-zero number.".into());
        }

        let (vertex, l1_at_end, l2_at_end) = Self::FindSharedVertex(l1, l2)
            .ok_or_else(|| "Selected lines do not meet at a corner or intersection.".to_string())?;

        let other1 = if l1_at_end { l1._Start } else { l1._End };
        let other2 = if l2_at_end { l2._Start } else { l2._End };

        let len1 = vertex.DistanceTo(other1);
        let len2 = vertex.DistanceTo(other2);
        if len1 < 1e-4 || len2 < 1e-4 {
            return Err("Line length too small to form a corner fillet.".into());
        }

        let u1 = [(other1._X - vertex._X) / len1, (other1._Y - vertex._Y) / len1];
        let u2 = [(other2._X - vertex._X) / len2, (other2._Y - vertex._Y) / len2];

        let dot = (u1[0] * u2[0] + u1[1] * u2[1]).clamp(-0.9999, 0.9999);
        let theta = dot.acos();
        let half_theta = theta * 0.5;

        let tan_half = half_theta.tan();
        if tan_half.abs() < 1e-5 {
            return Err("Lines are collinear; cannot create a corner fillet.".into());
        }

        let tangent_dist = radius / tan_half;
        if tangent_dist >= len1 || tangent_dist >= len2 {
            return Err(format!(
                "Fillet radius {:.1} requires {:.1} mm tangent length, exceeding line lengths ({:.1} mm, {:.1} mm).",
                radius, tangent_dist, len1, len2
            ));
        }

        let t1 = SketchPoint::New(vertex._X + tangent_dist * u1[0], vertex._Y + tangent_dist * u1[1]);
        let t2 = SketchPoint::New(vertex._X + tangent_dist * u2[0], vertex._Y + tangent_dist * u2[1]);

        let sin_half = half_theta.sin();
        let bisector_dist = radius / sin_half;
        let mut bx = u1[0] + u2[0];
        let mut by = u1[1] + u2[1];
        let b_len = (bx * bx + by * by).sqrt();
        if b_len < 1e-6 {
            return Err("Lines are opposing parallel; cannot construct fillet arc.".into());
        }
        bx /= b_len;
        by /= b_len;

        let center = SketchPoint::New(vertex._X + bisector_dist * bx, vertex._Y + bisector_dist * by);

        // Compute start and sweep angle for the circular arc
        let mut ang1 = (t1._Y - center._Y).atan2(t1._X - center._X);
        let mut ang2 = (t2._Y - center._Y).atan2(t2._X - center._X);
        if ang1 < 0.0 { ang1 += 2.0 * PI; }
        if ang2 < 0.0 { ang2 += 2.0 * PI; }

        let mut sweep = ang2 - ang1;
        if sweep > PI {
            sweep -= 2.0 * PI;
        } else if sweep < -PI {
            sweep += 2.0 * PI;
        }

        let trimmed_l1 = if l1_at_end {
            LineSegment::New(l1._Start, t1)
        } else {
            LineSegment::New(t1, l1._End)
        };

        let trimmed_l2 = if l2_at_end {
            LineSegment::New(l2._Start, t2)
        } else {
            LineSegment::New(t2, l2._End)
        };

        let arc = CircularArc::New(center, radius, ang1, sweep);

        Ok(FilletResult {
            trimmed_l1,
            trimmed_l2,
            arc,
            tangent1: t1,
            tangent2: t2,
            center,
        })
    }

    /// Computes a corner chamfer (bevel) with distances `d1` and `d2` along the two lines.
    pub fn ComputeChamfer(
        l1: &LineSegment,
        l2: &LineSegment,
        d1: f64,
        d2: f64,
    ) -> Result<ChamferResult, String>
    {
        if !d1.is_finite() || d1 <= 1e-4 || !d2.is_finite() || d2 <= 1e-4 {
            return Err("Chamfer distances must be positive non-zero numbers.".into());
        }

        let (vertex, l1_at_end, l2_at_end) = Self::FindSharedVertex(l1, l2)
            .ok_or_else(|| "Selected lines do not meet at a corner or intersection.".to_string())?;

        let other1 = if l1_at_end { l1._Start } else { l1._End };
        let other2 = if l2_at_end { l2._Start } else { l2._End };

        let len1 = vertex.DistanceTo(other1);
        let len2 = vertex.DistanceTo(other2);
        if len1 < 1e-4 || len2 < 1e-4 {
            return Err("Line length too small to form a corner chamfer.".into());
        }

        if d1 >= len1 || d2 >= len2 {
            return Err(format!(
                "Chamfer distances ({:.1}, {:.1}) exceed line lengths ({:.1}, {:.1}).",
                d1, d2, len1, len2
            ));
        }

        let u1 = [(other1._X - vertex._X) / len1, (other1._Y - vertex._Y) / len1];
        let u2 = [(other2._X - vertex._X) / len2, (other2._Y - vertex._Y) / len2];

        let c1 = SketchPoint::New(vertex._X + d1 * u1[0], vertex._Y + d1 * u1[1]);
        let c2 = SketchPoint::New(vertex._X + d2 * u2[0], vertex._Y + d2 * u2[1]);

        let trimmed_l1 = if l1_at_end {
            LineSegment::New(l1._Start, c1)
        } else {
            LineSegment::New(c1, l1._End)
        };

        let trimmed_l2 = if l2_at_end {
            LineSegment::New(l2._Start, c2)
        } else {
            LineSegment::New(c2, l2._End)
        };

        let chamfer_line = LineSegment::New(c1, c2);

        Ok(ChamferResult {
            trimmed_l1,
            trimmed_l2,
            chamfer_line,
            cut1: c1,
            cut2: c2,
        })
    }

    /// Builds a corner fillet patch for two line entities in the document.
    pub fn BuildFilletPatch(
        doc: &SketchDocument,
        id1: u32,
        id2: u32,
        radius: f64,
    ) -> Result<SketchPatch, String>
    {
        let ent1 = doc.FindEntity(id1).cloned().ok_or("First line entity not found.")?;
        let ent2 = doc.FindEntity(id2).cloned().ok_or("Second line entity not found.")?;

        let (l1, l2) = match (ent1.Primitive(), ent2.Primitive()) {
            (SketchPrimitive::Line(a), SketchPrimitive::Line(b)) => (*a, *b),
            _ => return Err("Fillet requires two line entities.".into()),
        };

        let result = Self::ComputeFillet(&l1, &l2, radius)?;

        let layer_id = if ent1.LayerId() == ent2.LayerId() { ent1.LayerId() } else { 0 };
        let is_construction = ent1.IsConstruction() && ent2.IsConstruction();
        let group_id = if ent1.GroupId() == ent2.GroupId() { ent1.GroupId() } else { u32::MAX };

        let mut replaced = Stash::New();
        let mut new_ent1 = ent1.clone();
        new_ent1.SetPrimitive(SketchPrimitive::Line(result.trimmed_l1));
        replaced.Push((ent1, new_ent1));

        let mut new_ent2 = ent2.clone();
        new_ent2.SetPrimitive(SketchPrimitive::Line(result.trimmed_l2));
        replaced.Push((ent2, new_ent2));

        let new_arc_id = doc.NextEntityId();
        let mut arc_ent = SketchEntity::New(new_arc_id, SketchPrimitive::Arc(result.arc));
        if layer_id != 0 {
            arc_ent = arc_ent.WithLayer(layer_id);
        }
        arc_ent = arc_ent.WithConstruction(is_construction);
        if group_id != u32::MAX {
            arc_ent = arc_ent.WithGroup(group_id);
        }
        let mut added = Stash::New();
        added.Push(arc_ent);

        Ok(SketchPatch::New(added.IntoBuff(), Buff::New(), replaced.IntoBuff()))
    }

    /// Builds a corner chamfer patch for two line entities in the document.
    pub fn BuildChamferPatch(
        doc: &SketchDocument,
        id1: u32,
        id2: u32,
        distance: f64,
    ) -> Result<SketchPatch, String>
    {
        let ent1 = doc.FindEntity(id1).cloned().ok_or("First line entity not found.")?;
        let ent2 = doc.FindEntity(id2).cloned().ok_or("Second line entity not found.")?;

        let (l1, l2) = match (ent1.Primitive(), ent2.Primitive()) {
            (SketchPrimitive::Line(a), SketchPrimitive::Line(b)) => (*a, *b),
            _ => return Err("Chamfer requires two line entities.".into()),
        };

        let result = Self::ComputeChamfer(&l1, &l2, distance, distance)?;

        let layer_id = if ent1.LayerId() == ent2.LayerId() { ent1.LayerId() } else { 0 };
        let is_construction = ent1.IsConstruction() && ent2.IsConstruction();
        let group_id = if ent1.GroupId() == ent2.GroupId() { ent1.GroupId() } else { u32::MAX };

        let mut replaced = Stash::New();
        let mut new_ent1 = ent1.clone();
        new_ent1.SetPrimitive(SketchPrimitive::Line(result.trimmed_l1));
        replaced.Push((ent1, new_ent1));

        let mut new_ent2 = ent2.clone();
        new_ent2.SetPrimitive(SketchPrimitive::Line(result.trimmed_l2));
        replaced.Push((ent2, new_ent2));

        let new_line_id = doc.NextEntityId();
        let mut line_ent = SketchEntity::New(new_line_id, SketchPrimitive::Line(result.chamfer_line));
        if layer_id != 0 {
            line_ent = line_ent.WithLayer(layer_id);
        }
        line_ent = line_ent.WithConstruction(is_construction);
        if group_id != u32::MAX {
            line_ent = line_ent.WithGroup(group_id);
        }
        let mut added = Stash::New();
        added.Push(line_ent);

        Ok(SketchPatch::New(added.IntoBuff(), Buff::New(), replaced.IntoBuff()))
    }
}

//-------------------------------------------------------------------------------------------------
