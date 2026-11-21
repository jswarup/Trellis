// sketch_constraint.rs ------------------------------------------------------------------------------------------------
//! 2D Geometric Constraint Solver Engine for planar sketch systems.
//! Supports iterative DoF relaxation for coincident, horizontal, vertical, parallel,
//! perpendicular, distance, length, concentric, tangent, and symmetric constraints.

use crate::fenst::sketch::{SketchDocument, SketchPoint, SketchPrimitive};
use crate::silo::{Buff, Stash, USeg};

//-------------------------------------------------------------------------------------------------

/// Target reference to a geometric control point on a sketch entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointTarget
{
    LineStart(u32),
    LineEnd(u32),
    CircleCenter(u32),
    ArcStart(u32),
    ArcEnd(u32),
    ArcCenter(u32),
    FreePoint(u32),
}

/// Geometric constraint types supported by the 2D sketch solver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeometricConstraintKind
{
    Coincident { _PtA: u32, _PtB: u32 },
    Horizontal { _PtA: u32, _PtB: u32 },
    Vertical { _PtA: u32, _PtB: u32 },
    Distance { _PtA: u32, _PtB: u32, _Distance: f64 },
    Length { _LineStart: u32, _LineEnd: u32, _Length: f64 },
    Parallel { _Line1Start: u32, _Line1End: u32, _Line2Start: u32, _Line2End: u32 },
    Perpendicular { _Line1Start: u32, _Line1End: u32, _Line2Start: u32, _Line2End: u32 },
    Concentric { _CenterA: u32, _CenterB: u32 },
    Tangent { _LineStart: u32, _LineEnd: u32, _Center: u32, _Radius: f64 },
    Symmetric { _PtA: u32, _PtB: u32, _AxisStart: u32, _AxisEnd: u32 },
    Fixed { _Pt: u32, _Position: SketchPoint },
}

impl GeometricConstraintKind
{
    pub fn Glyph(&self) -> &'static str
    {
        match self {
            Self::Coincident { .. } => "•",
            Self::Horizontal { .. } => "H",
            Self::Vertical { .. } => "V",
            Self::Distance { .. } => "D",
            Self::Length { .. } => "L",
            Self::Parallel { .. } => "//",
            Self::Perpendicular { .. } => "⊥",
            Self::Concentric { .. } => "◎",
            Self::Tangent { .. } => "T",
            Self::Symmetric { .. } => "SYM",
            Self::Fixed { .. } => "⚓",
        }
    }
}

/// Result of an iterative constraint relaxation solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolverResult
{
    pub _Converged:   bool,
    pub _Iterations:  u32,
    pub _MaxResidual: f64,
}

//-------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
struct SolverPoint
{
    _Pos:   SketchPoint,
    _Fixed: bool,
}

/// 2D Geometric Constraint Solver managing points, constraints, and iterative relaxation.
#[derive(Clone, Debug, Default)]
pub struct ConstraintSolver
{
    _Points:      Buff<SolverPoint>,
    _Constraints: Buff<GeometricConstraintKind>,
}

impl ConstraintSolver
{
    pub fn New() -> Self
    {
        Self {
            _Points:      Buff::New(),
            _Constraints: Buff::New(),
        }
    }

    pub fn AddPoint(&mut self, pos: SketchPoint, fixed: bool) -> u32
    {
        let mut stash = Stash::WithCapacity(self._Points.Size() + 1);
        let arr = self._Points.Arr();
        USeg::FromLen(arr.Len()).Traverse(|i| {
            stash.Push(*arr.Get(i).unwrap());
        });
        let id = stash.Size();
        stash.Push(SolverPoint { _Pos: pos, _Fixed: fixed });
        self._Points = stash.IntoBuff();
        id
    }

    pub fn PointCount(&self) -> u32 { self._Points.Size() }

    pub fn GetPoint(&self, id: u32) -> Option<SketchPoint>
    {
        self._Points.Arr().Get(id).map(|p| p._Pos)
    }

    pub fn SetPoint(&mut self, id: u32, pos: SketchPoint) -> bool
    {
        if id >= self._Points.Size() {
            return false;
        }
        let mut stash = Stash::WithCapacity(self._Points.Size());
        let arr = self._Points.Arr();
        USeg::FromLen(arr.Len()).Traverse(|i| {
            let mut pt = *arr.Get(i).unwrap();
            if i == id {
                pt._Pos = pos;
            }
            stash.Push(pt);
        });
        self._Points = stash.IntoBuff();
        true
    }

    pub fn AddConstraint(&mut self, constraint: GeometricConstraintKind) -> u32
    {
        let mut stash = Stash::WithCapacity(self._Constraints.Size() + 1);
        let arr = self._Constraints.Arr();
        USeg::FromLen(arr.Len()).Traverse(|i| {
            stash.Push(*arr.Get(i).unwrap());
        });
        let id = stash.Size();
        stash.Push(constraint);
        self._Constraints = stash.IntoBuff();
        id
    }

    pub fn ConstraintCount(&self) -> u32 { self._Constraints.Size() }

    pub fn Solve(&mut self, max_iterations: u32, tolerance: f64) -> SolverResult
    {
        let pCount = self._Points.Size() as usize;
        if pCount == 0 || self._Constraints.Size() == 0 {
            return SolverResult { _Converged: true, _Iterations: 0, _MaxResidual: 0.0 };
        }

        let mut pts = Vec::with_capacity(pCount);
        let mut fixed = Vec::with_capacity(pCount);
        let pArr = self._Points.Arr();
        USeg::FromLen(pArr.Len()).Traverse(|i| {
            let pt = pArr.Get(i).unwrap();
            pts.push(pt._Pos);
            fixed.push(pt._Fixed);
        });

        let mut iter = 0u32;
        let mut maxRes = f64::INFINITY;

        while iter < max_iterations && maxRes > tolerance {
            maxRes = 0.0;
            let cArr = self._Constraints.Arr();
            USeg::FromLen(cArr.Len()).Traverse(|cIdx| {
                let c = cArr.Get(cIdx).unwrap();
                let res = Self::RelaxConstraint(c, &mut pts, &fixed);
                if res > maxRes {
                    maxRes = res;
                }
            });
            iter += 1;
        }

        // Write back relaxed points
        let mut stash = Stash::WithCapacity(self._Points.Size());
        for (i, p) in pts.iter().enumerate() {
            stash.Push(SolverPoint { _Pos: *p, _Fixed: fixed[i] });
        }
        self._Points = stash.IntoBuff();

        SolverResult {
            _Converged:   maxRes <= tolerance,
            _Iterations:  iter,
            _MaxResidual: maxRes,
        }
    }

    fn RelaxConstraint(c: &GeometricConstraintKind, pts: &mut [SketchPoint], fixed: &[bool]) -> f64
    {
        match *c {
            GeometricConstraintKind::Coincident { _PtA: a, _PtB: b }
            | GeometricConstraintKind::Concentric { _CenterA: a, _CenterB: b } => {
                Self::RelaxCoincident(a as usize, b as usize, pts, fixed)
            }
            GeometricConstraintKind::Horizontal { _PtA: a, _PtB: b } => {
                Self::RelaxHorizontal(a as usize, b as usize, pts, fixed)
            }
            GeometricConstraintKind::Vertical { _PtA: a, _PtB: b } => {
                Self::RelaxVertical(a as usize, b as usize, pts, fixed)
            }
            GeometricConstraintKind::Distance { _PtA: a, _PtB: b, _Distance: d }
            | GeometricConstraintKind::Length { _LineStart: a, _LineEnd: b, _Length: d } => {
                Self::RelaxDistance(a as usize, b as usize, d, pts, fixed)
            }
            GeometricConstraintKind::Parallel { _Line1Start: s1, _Line1End: e1, _Line2Start: s2, _Line2End: e2 } => {
                Self::RelaxParallel(s1 as usize, e1 as usize, s2 as usize, e2 as usize, pts, fixed)
            }
            GeometricConstraintKind::Perpendicular { _Line1Start: s1, _Line1End: e1, _Line2Start: s2, _Line2End: e2 } => {
                Self::RelaxPerpendicular(s1 as usize, e1 as usize, s2 as usize, e2 as usize, pts, fixed)
            }
            GeometricConstraintKind::Tangent { _LineStart: s, _LineEnd: e, _Center: c, _Radius: r } => {
                Self::RelaxTangent(s as usize, e as usize, c as usize, r, pts, fixed)
            }
            GeometricConstraintKind::Symmetric { _PtA: a, _PtB: b, _AxisStart: s, _AxisEnd: e } => {
                Self::RelaxSymmetric(a as usize, b as usize, s as usize, e as usize, pts, fixed)
            }
            GeometricConstraintKind::Fixed { _Pt: p, _Position: target } => {
                let idx = p as usize;
                if idx < pts.len() {
                    let err = pts[idx].DistanceTo(target);
                    pts[idx] = target;
                    err
                } else {
                    0.0
                }
            }
        }
    }

    fn RelaxCoincident(a: usize, b: usize, pts: &mut [SketchPoint], fixed: &[bool]) -> f64
    {
        if a >= pts.len() || b >= pts.len() || a == b { return 0.0; }
        let pA = pts[a];
        let pB = pts[b];
        let dx = pB._X - pA._X;
        let dy = pB._Y - pA._Y;
        let dist = (dx * dx + dy * dy).sqrt();
        if fixed[a] && fixed[b] { return dist; }
        if fixed[a] {
            pts[b] = pA;
        } else if fixed[b] {
            pts[a] = pB;
        } else {
            let midX = (pA._X + pB._X) * 0.5;
            let midY = (pA._Y + pB._Y) * 0.5;
            pts[a] = SketchPoint::New(midX, midY);
            pts[b] = SketchPoint::New(midX, midY);
        }
        dist
    }

    fn RelaxHorizontal(a: usize, b: usize, pts: &mut [SketchPoint], fixed: &[bool]) -> f64
    {
        if a >= pts.len() || b >= pts.len() || a == b { return 0.0; }
        let diff = (pts[b]._Y - pts[a]._Y).abs();
        if fixed[a] && fixed[b] { return diff; }
        if fixed[a] {
            pts[b]._Y = pts[a]._Y;
        } else if fixed[b] {
            pts[a]._Y = pts[b]._Y;
        } else {
            let midY = (pts[a]._Y + pts[b]._Y) * 0.5;
            pts[a]._Y = midY;
            pts[b]._Y = midY;
        }
        diff
    }

    fn RelaxVertical(a: usize, b: usize, pts: &mut [SketchPoint], fixed: &[bool]) -> f64
    {
        if a >= pts.len() || b >= pts.len() || a == b { return 0.0; }
        let diff = (pts[b]._X - pts[a]._X).abs();
        if fixed[a] && fixed[b] { return diff; }
        if fixed[a] {
            pts[b]._X = pts[a]._X;
        } else if fixed[b] {
            pts[a]._X = pts[b]._X;
        } else {
            let midX = (pts[a]._X + pts[b]._X) * 0.5;
            pts[a]._X = midX;
            pts[b]._X = midX;
        }
        diff
    }

    fn RelaxDistance(a: usize, b: usize, target: f64, pts: &mut [SketchPoint], fixed: &[bool]) -> f64
    {
        if a >= pts.len() || b >= pts.len() || a == b { return 0.0; }
        let pA = pts[a];
        let pB = pts[b];
        let dx = pB._X - pA._X;
        let dy = pB._Y - pA._Y;
        let curDist = (dx * dx + dy * dy).sqrt();
        let err = (curDist - target).abs();
        if curDist < 1e-12 || (fixed[a] && fixed[b]) { return err; }

        let factor = (curDist - target) / curDist;
        let shiftX = dx * factor;
        let shiftY = dy * factor;

        if fixed[a] {
            pts[b]._X -= shiftX;
            pts[b]._Y -= shiftY;
        } else if fixed[b] {
            pts[a]._X += shiftX;
            pts[a]._Y += shiftY;
        } else {
            pts[a]._X += shiftX * 0.5;
            pts[a]._Y += shiftY * 0.5;
            pts[b]._X -= shiftX * 0.5;
            pts[b]._Y -= shiftY * 0.5;
        }
        err
    }

    fn RelaxParallel(s1: usize, e1: usize, s2: usize, e2: usize, pts: &mut [SketchPoint], fixed: &[bool]) -> f64
    {
        if s1 >= pts.len() || e1 >= pts.len() || s2 >= pts.len() || e2 >= pts.len() { return 0.0; }
        let d1x = pts[e1]._X - pts[s1]._X;
        let d1y = pts[e1]._Y - pts[s1]._Y;
        let d2x = pts[e2]._X - pts[s2]._X;
        let d2y = pts[e2]._Y - pts[s2]._Y;
        let len1 = (d1x * d1x + d1y * d1y).sqrt();
        let len2 = (d2x * d2x + d2y * d2y).sqrt();
        if len1 < 1e-12 || len2 < 1e-12 { return 0.0; }

        let cross = (d1x * d2y - d1y * d2x) / (len1 * len2);
        let angErr = cross.abs();
        if angErr < 1e-9 { return angErr; }

        // Align line 2 direction with line 1 if line 1 fixed, else average
        let ang1 = d1y.atan2(d1x);
        let ang2 = d2y.atan2(d2x);
        let targetAng = if fixed[s1] && fixed[e1] {
            ang1
        } else if fixed[s2] && fixed[e2] {
            ang2
        } else {
            ang1 + (ang2 - ang1) * 0.5
        };

        if !fixed[e2] {
            pts[e2]._X = pts[s2]._X + len2 * targetAng.cos();
            pts[e2]._Y = pts[s2]._Y + len2 * targetAng.sin();
        }
        if !fixed[e1] && !(fixed[s1] && fixed[e1]) {
            pts[e1]._X = pts[s1]._X + len1 * targetAng.cos();
            pts[e1]._Y = pts[s1]._Y + len1 * targetAng.sin();
        }
        angErr
    }

    fn RelaxPerpendicular(s1: usize, e1: usize, s2: usize, e2: usize, pts: &mut [SketchPoint], fixed: &[bool]) -> f64
    {
        if s1 >= pts.len() || e1 >= pts.len() || s2 >= pts.len() || e2 >= pts.len() { return 0.0; }
        let d1x = pts[e1]._X - pts[s1]._X;
        let d1y = pts[e1]._Y - pts[s1]._Y;
        let d2x = pts[e2]._X - pts[s2]._X;
        let d2y = pts[e2]._Y - pts[s2]._Y;
        let len1 = (d1x * d1x + d1y * d1y).sqrt();
        let len2 = (d2x * d2x + d2y * d2y).sqrt();
        if len1 < 1e-12 || len2 < 1e-12 { return 0.0; }

        let dot = (d1x * d2x + d1y * d2y) / (len1 * len2);
        let dotErr = dot.abs();
        if dotErr < 1e-9 { return dotErr; }

        let ang1 = d1y.atan2(d1x);
        let perpAng = ang1 + std::f64::consts::FRAC_PI_2;

        if !fixed[e2] {
            pts[e2]._X = pts[s2]._X + len2 * perpAng.cos();
            pts[e2]._Y = pts[s2]._Y + len2 * perpAng.sin();
        }
        dotErr
    }

    fn RelaxTangent(s: usize, e: usize, c: usize, r: f64, pts: &mut [SketchPoint], fixed: &[bool]) -> f64
    {
        if s >= pts.len() || e >= pts.len() || c >= pts.len() { return 0.0; }
        let pS = pts[s];
        let pE = pts[e];
        let pC = pts[c];
        let dx = pE._X - pS._X;
        let dy = pE._Y - pS._Y;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 { return 0.0; }

        let cross = (dx * (pS._Y - pC._Y) - dy * (pS._X - pC._X)).abs();
        let curDist = cross / len;
        let err = (curDist - r).abs();
        if err < 1e-9 { return err; }

        let nx = -dy / len;
        let ny = dx / len;
        let shift = curDist - r;

        if !fixed[c] {
            pts[c]._X += nx * shift;
            pts[c]._Y += ny * shift;
        } else {
            if !fixed[s] {
                pts[s]._X -= nx * shift * 0.5;
                pts[s]._Y -= ny * shift * 0.5;
            }
            if !fixed[e] {
                pts[e]._X -= nx * shift * 0.5;
                pts[e]._Y -= ny * shift * 0.5;
            }
        }
        err
    }

    fn RelaxSymmetric(a: usize, b: usize, s: usize, e: usize, pts: &mut [SketchPoint], fixed: &[bool]) -> f64
    {
        if a >= pts.len() || b >= pts.len() || s >= pts.len() || e >= pts.len() { return 0.0; }
        let pA = pts[a];
        let pB = pts[b];
        let pS = pts[s];
        let pE = pts[e];
        let axisDx = pE._X - pS._X;
        let axisDy = pE._Y - pS._Y;
        let axisLen = (axisDx * axisDx + axisDy * axisDy).sqrt();
        if axisLen < 1e-12 { return 0.0; }

        let midX = (pA._X + pB._X) * 0.5;
        let midY = (pA._Y + pB._Y) * 0.5;

        // Project midpoint onto axis
        let t = ((midX - pS._X) * axisDx + (midY - pS._Y) * axisDy) / (axisLen * axisLen);
        let projX = pS._X + t * axisDx;
        let projY = pS._Y + t * axisDy;

        let shiftX = projX - midX;
        let shiftY = projY - midY;
        let err = (shiftX * shiftX + shiftY * shiftY).sqrt();

        if !fixed[a] {
            pts[a]._X += shiftX;
            pts[a]._Y += shiftY;
        }
        if !fixed[b] {
            pts[b]._X += shiftX;
            pts[b]._Y += shiftY;
        }
        err
    }

    /// Extracts control points from a SketchDocument into the solver, returning the point mapping.
    pub fn ExtractFromDocument(doc: &SketchDocument) -> (Self, Vec<(u32, PointTarget)>)
    {
        let mut solver = Self::New();
        let mut mapping = Vec::new();
        let entities = doc.Entities();
        USeg::FromLen(entities.Len()).Traverse(|i| {
            let entity = entities.Get(i).unwrap();
            let id = entity.Id();
            match entity.Primitive() {
                SketchPrimitive::Line(l) => {
                    let pt0 = solver.AddPoint(l._Start, false);
                    mapping.push((pt0, PointTarget::LineStart(id)));
                    let pt1 = solver.AddPoint(l._End, false);
                    mapping.push((pt1, PointTarget::LineEnd(id)));
                }
                SketchPrimitive::Circle(c) => {
                    let pt0 = solver.AddPoint(c._Center, false);
                    mapping.push((pt0, PointTarget::CircleCenter(id)));
                }
                SketchPrimitive::Arc(a) => {
                    let pt0 = solver.AddPoint(a._Center, false);
                    mapping.push((pt0, PointTarget::ArcCenter(id)));
                    let pt1 = solver.AddPoint(a.StartPoint(), false);
                    mapping.push((pt1, PointTarget::ArcStart(id)));
                    let pt2 = solver.AddPoint(a.EndPoint(), false);
                    mapping.push((pt2, PointTarget::ArcEnd(id)));
                }
                _ => {}
            }
        });
        (solver, mapping)
    }

    /// Applies resolved solver point positions back into the SketchDocument.
    pub fn ApplyToDocument(&self, doc: &mut SketchDocument, mapping: &[(u32, PointTarget)])
    {
        for &(pId, target) in mapping {
            let Some(pt) = self.GetPoint(pId) else { continue; };
            match target {
                PointTarget::LineStart(eId) => {
                    if let Some(entity) = doc.FindEntityMut(eId)
                        && let SketchPrimitive::Line(l) = entity.PrimitiveMut()
                    {
                        l._Start = pt;
                    }
                }
                PointTarget::LineEnd(eId) => {
                    if let Some(entity) = doc.FindEntityMut(eId)
                        && let SketchPrimitive::Line(l) = entity.PrimitiveMut()
                    {
                        l._End = pt;
                    }
                }
                PointTarget::CircleCenter(eId) => {
                    if let Some(entity) = doc.FindEntityMut(eId)
                        && let SketchPrimitive::Circle(c) = entity.PrimitiveMut()
                    {
                        c._Center = pt;
                    }
                }
                PointTarget::ArcCenter(eId) => {
                    if let Some(entity) = doc.FindEntityMut(eId)
                        && let SketchPrimitive::Arc(a) = entity.PrimitiveMut()
                    {
                        a._Center = pt;
                    }
                }
                _ => {}
            }
        }
        doc.Touch();
    }
}
