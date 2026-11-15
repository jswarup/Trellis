// sketch_extrude.rs ------------------------------------------------------------------------------------------------
//! Solid 3D extrusion engine converting planar 2D sketch profiles into watertight 3D boundary representation meshes.

use crate::fenst::geometry::GeometryAsset;
use crate::fenst::sketch::{LineSegment, SketchDocument, SketchPlane, SketchPoint, SketchPrimitive};
use crate::fleck::sketch_tessellate::SketchTessellate;
use crate::silo::{Arr, Buff, IArr, Stash};

//-------------------------------------------------------------------------------------------------

pub struct SketchExtrude;

impl SketchExtrude
{
    /// Extrudes all closed profiles in the sketch document along the workplane normal by `depth`.
    pub fn ExtrudeDocument(doc: &SketchDocument, depth: f64) -> Result<GeometryAsset, String>
    {
        if !depth.is_finite() || depth.abs() < 1e-6 {
            return Err("Extrusion depth must be a non-zero finite number.".into());
        }

        let plane = *doc.Plane();
        let profiles = Self::ExtractClosedProfiles(doc);
        if profiles.IsEmpty() {
            return Err("No closed profile loops found in sketch to extrude.".into());
        }

        let mut all_positions: Stash<[f64; 3]> = Stash::New();
        let mut all_colors: Stash<[f32; 4]> = Stash::New();
        let mut all_triangles: Stash<[u32; 3]> = Stash::New();
        let mut all_edges: Stash<[u32; 2]> = Stash::New();

        profiles.Arr().Traverse(|profile| {
            Self::ExtrudeSingleProfile(
                profile.Arr(),
                &plane,
                depth,
                &mut all_positions,
                &mut all_colors,
                &mut all_triangles,
                &mut all_edges,
            );
        });

        if all_positions.IsEmpty() {
            return Err("Failed to generate solid geometry from sketch profiles.".into());
        }

        let bounds = Self::ComputeBoundsF64(all_positions.Arr());

        GeometryAsset::FromMeshF64(
            all_positions.Arr(),
            all_colors.Arr(),
            all_triangles.IntoBuff(),
            all_edges.IntoBuff(),
            Buff::New(),
            bounds,
        )
    }

    /// Extrudes a single closed polygon profile along the workplane normal.
    pub fn ExtrudeProfile(
        profile: Arr<'_, SketchPoint>,
        plane: &SketchPlane,
        depth: f64,
    ) -> Result<GeometryAsset, String>
    {
        if !depth.is_finite() || depth.abs() < 1e-6 {
            return Err("Extrusion depth must be a non-zero finite number.".into());
        }
        if profile.Len() < 3 {
            return Err("Extrusion profile requires at least 3 distinct vertices.".into());
        }

        let mut positions: Stash<[f64; 3]> = Stash::New();
        let mut colors: Stash<[f32; 4]> = Stash::New();
        let mut triangles: Stash<[u32; 3]> = Stash::New();
        let mut edges: Stash<[u32; 2]> = Stash::New();

        Self::ExtrudeSingleProfile(
            profile,
            plane,
            depth,
            &mut positions,
            &mut colors,
            &mut triangles,
            &mut edges,
        );

        let bounds = Self::ComputeBoundsF64(positions.Arr());

        GeometryAsset::FromMeshF64(
            positions.Arr(),
            colors.Arr(),
            triangles.IntoBuff(),
            edges.IntoBuff(),
            Buff::New(),
            bounds,
        )
    }

    fn ExtrudeSingleProfile(
        raw_pts: Arr<'_, SketchPoint>,
        plane: &SketchPlane,
        depth: f64,
        positions: &mut Stash<[f64; 3]>,
        colors: &mut Stash<[f32; 4]>,
        triangles: &mut Stash<[u32; 3]>,
        edges: &mut Stash<[u32; 2]>,
    )
    {
        let pts = Self::NormalizeWinding(raw_pts);
        let n = pts.Size();
        if n < 3 {
            return;
        }

        let base_idx = positions.Size();
        let bottom_color = [0.70, 0.74, 0.82, 1.0];
        let top_color = [0.82, 0.86, 0.92, 1.0];

        let norm_offset = [
            plane._Normal[0] * depth,
            plane._Normal[1] * depth,
            plane._Normal[2] * depth,
        ];

        // 1. Add Bottom Vertices (0..N-1)
        let mut i = 0u32;
        while i < n {
            let p2d = *pts.Arr().Get(i).unwrap();
            let p3d = plane.ProjectTo3d(p2d);
            positions.Push(p3d);
            colors.Push(bottom_color);
            i += 1;
        }

        // 2. Add Top Vertices (N..2N-1)
        let mut j = 0u32;
        while j < n {
            let p2d = *pts.Arr().Get(j).unwrap();
            let p3d = plane.ProjectTo3d(p2d);
            let top_pos = [
                p3d[0] + norm_offset[0],
                p3d[1] + norm_offset[1],
                p3d[2] + norm_offset[2],
            ];
            positions.Push(top_pos);
            colors.Push(top_color);
            j += 1;
        }

        // 3. Triangulate planar cap (Ear clipping)
        let cap_tris = Self::TriangulatePolygon(pts.Arr());

        // Bottom Cap (Reversed winding for outward normal pointing -N if depth > 0)
        let reverse_bottom = depth > 0.0;
        cap_tris.Arr().Traverse(|tri| {
            if reverse_bottom {
                triangles.Push([base_idx + tri[2], base_idx + tri[1], base_idx + tri[0]]);
            } else {
                triangles.Push([base_idx + tri[0], base_idx + tri[1], base_idx + tri[2]]);
            }
        });

        // Top Cap (Direct winding for outward normal pointing +N if depth > 0)
        cap_tris.Arr().Traverse(|tri| {
            if reverse_bottom {
                triangles.Push([base_idx + n + tri[0], base_idx + n + tri[1], base_idx + n + tri[2]]);
            } else {
                triangles.Push([base_idx + n + tri[2], base_idx + n + tri[1], base_idx + n + tri[0]]);
            }
        });

        // 4. Side Walls & Feature Edges
        let mut k = 0u32;
        while k < n {
            let next = (k + 1) % n;
            let b0 = base_idx + k;
            let b1 = base_idx + next;
            let t0 = base_idx + n + k;
            let t1 = base_idx + n + next;

            // Two triangles per side quad
            if depth > 0.0 {
                triangles.Push([b0, b1, t1]);
                triangles.Push([b0, t1, t0]);
            } else {
                triangles.Push([b0, t1, b1]);
                triangles.Push([b0, t0, t1]);
            }

            // Edges: bottom, top, and vertical upright
            edges.Push([b0, b1]);
            edges.Push([t0, t1]);
            edges.Push([b0, t0]);

            k += 1;
        }
    }

    /// Extracts all closed profiles from circles, closed ellipses, closed polylines, and connected lines/arcs.
    pub fn ExtractClosedProfiles(doc: &SketchDocument) -> Stash<Buff<SketchPoint>>
    {
        let mut profiles: Stash<Buff<SketchPoint>> = Stash::New();
        let mut segments: Stash<LineSegment> = Stash::New();

        doc.Entities().Traverse(|entity| {
            if entity.IsConstruction() {
                return;
            }
            match entity.Primitive() {
                SketchPrimitive::Circle(c) => {
                    let pts = SketchTessellate::TessellateCircle(c, 0.25, 1.0);
                    profiles.Push(Self::CleanLoop(pts.Arr()));
                }
                SketchPrimitive::Ellipse(e) => {
                    if e.IsClosed() {
                        let pts = SketchTessellate::TessellateEllipse(e, 0.25, 1.0);
                        profiles.Push(Self::CleanLoop(pts.Arr()));
                    }
                }
                SketchPrimitive::Polyline(p) => {
                    if p._IsClosed && p._PointCount >= 3 {
                        let pts = doc.PolylinePoints(p._PointOffset, p._PointCount);
                        profiles.Push(Self::CleanLoop(pts.Arr()));
                    }
                }
                SketchPrimitive::Line(l) => {
                    segments.Push(*l);
                }
                SketchPrimitive::Arc(a) => {
                    let pts = SketchTessellate::TessellateArc(a, 0.25, 1.0);
                    let mut i = 0u32;
                    while i + 1 < pts.Size() {
                        let p0 = *pts.Arr().Get(i).unwrap();
                        let p1 = *pts.Arr().Get(i + 1).unwrap();
                        segments.Push(LineSegment::New(p0, p1));
                        i += 1;
                    }
                }
            }
        });

        // Trace connected cycles from linear segments
        if segments.Size() >= 3 {
            let cycles = Self::TraceCyclesFromSegments(segments.Arr());
            cycles.Arr().Traverse(|cycle| profiles.Push(cycle.clone()));
        }

        profiles
    }

    fn CleanLoop(pts: Arr<'_, SketchPoint>) -> Buff<SketchPoint>
    {
        let len = pts.Len();
        if len < 3 {
            let mut s = Stash::New();
            pts.Traverse(|p| s.Push(*p));
            return s.IntoBuff();
        }
        let first = pts.Get(0).unwrap();
        let last = pts.Get(len - 1).unwrap();
        let count = if first.DistanceSqTo(*last) < 1e-9 { len - 1 } else { len };
        let mut s = Stash::WithCapacity(count);
        let mut i = 0u32;
        while i < count {
            s.Push(*pts.Get(i).unwrap());
            i += 1;
        }
        s.IntoBuff()
    }

    fn TraceCyclesFromSegments(segs: Arr<'_, LineSegment>) -> Stash<Buff<SketchPoint>>
    {
        let mut cycles = Stash::New();
        let mut visited = Stash::WithCapacity(segs.Len());
        let mut i = 0u32;
        while i < segs.Len() {
            visited.Push(false);
            i += 1;
        }

        let mut start_idx = 0u32;
        while start_idx < segs.Len() {
            if *visited.Arr().Get(start_idx).unwrap() {
                start_idx += 1;
                continue;
            }

            let first_seg = segs.Get(start_idx).unwrap();
            let mut chain = Stash::New();
            chain.Push(first_seg._Start);
            let mut current_tip = first_seg._End;
            visited[start_idx] = true;

            let mut closed = false;
            let mut advanced = true;

            while advanced && !closed {
                advanced = false;
                let mut candidate = 0u32;
                while candidate < segs.Len() {
                    if !*visited.Arr().Get(candidate).unwrap() {
                        let seg = segs.Get(candidate).unwrap();
                        if seg._Start.DistanceSqTo(current_tip) < 1e-6 {
                            chain.Push(current_tip);
                            current_tip = seg._End;
                            visited[candidate] = true;
                            advanced = true;
                            break;
                        } else if seg._End.DistanceSqTo(current_tip) < 1e-6 {
                            chain.Push(current_tip);
                            current_tip = seg._Start;
                            visited[candidate] = true;
                            advanced = true;
                            break;
                        }
                    }
                    candidate += 1;
                }

                if current_tip.DistanceSqTo(first_seg._Start) < 1e-6 && chain.Size() >= 2 {
                    closed = true;
                }
            }

            if closed && chain.Size() >= 3 {
                cycles.Push(chain.IntoBuff());
            }

            start_idx += 1;
        }

        cycles
    }

    pub fn SignedArea(pts: Arr<'_, SketchPoint>) -> f64
    {
        let len = pts.Len();
        if len < 3 {
            return 0.0;
        }
        let mut sum = 0.0;
        let mut i = 0u32;
        while i < len {
            let next = (i + 1) % len;
            let p1 = pts.Get(i).unwrap();
            let p2 = pts.Get(next).unwrap();
            sum += p1._X * p2._Y - p2._X * p1._Y;
            i += 1;
        }
        sum * 0.5
    }

    fn NormalizeWinding(pts: Arr<'_, SketchPoint>) -> Stash<SketchPoint>
    {
        let area = Self::SignedArea(pts);
        let len = pts.Len();
        let mut out = Stash::WithCapacity(len);
        if area >= 0.0 {
            // Already CCW
            pts.Traverse(|p| out.Push(*p));
        } else {
            // CW -> reverse to CCW
            let mut i = len;
            while i > 0 {
                out.Push(*pts.Get(i - 1).unwrap());
                i -= 1;
            }
        }
        out
    }

    pub fn TriangulatePolygon(pts: Arr<'_, SketchPoint>) -> Buff<[u32; 3]>
    {
        let n = pts.Len();
        if n < 3 {
            return Buff::New();
        }
        if n == 3 {
            let mut b = Stash::WithCapacity(1);
            b.Push([0, 1, 2]);
            return b.IntoBuff();
        }

        let mut active: Stash<u32> = Stash::WithCapacity(n);
        let mut i = 0u32;
        while i < n {
            active.Push(i);
            i += 1;
        }

        let mut triangles: Stash<[u32; 3]> = Stash::WithCapacity(n - 2);
        let mut count = active.Size();
        let mut guard = 0u32;
        let max_guard = n * n * 2;
        let mut curr = 0u32;

        while count > 3 && guard < max_guard {
            guard += 1;
            let prev = if curr == 0 { count - 1 } else { curr - 1 };
            let next = if curr + 1 >= count { 0 } else { curr + 1 };

            let idx_prev = *active.Arr().Get(prev).unwrap();
            let idx_curr = *active.Arr().Get(curr).unwrap();
            let idx_next = *active.Arr().Get(next).unwrap();

            if Self::IsEar(idx_curr, idx_prev, idx_next, pts, active.Arr()) {
                triangles.Push([idx_prev, idx_curr, idx_next]);
                let mut new_active = Stash::WithCapacity(count - 1);
                let mut k = 0u32;
                while k < active.Size() {
                    if k != curr {
                        new_active.Push(*active.Arr().Get(k).unwrap());
                    }
                    k += 1;
                }
                active = new_active;
                count -= 1;
                if curr >= count {
                    curr = 0;
                }
            } else {
                curr = (curr + 1) % count;
            }
        }

        if count == 3 {
            let v0 = *active.Arr().Get(0).unwrap();
            let v1 = *active.Arr().Get(1).unwrap();
            let v2 = *active.Arr().Get(2).unwrap();
            triangles.Push([v0, v1, v2]);
        } else if triangles.IsEmpty() {
            // Triangle fan fallback for self-intersecting or complex shapes
            let mut k = 1u32;
            while k + 1 < n {
                triangles.Push([0, k, k + 1]);
                k += 1;
            }
        }

        triangles.IntoBuff()
    }

    fn IsEar(i: u32, prev: u32, next: u32, pts: Arr<'_, SketchPoint>, active: Arr<'_, u32>) -> bool
    {
        let a = *pts.Get(prev).unwrap();
        let b = *pts.Get(i).unwrap();
        let c = *pts.Get(next).unwrap();

        // Convexity check: 2D cross product of (b - a) and (c - b)
        let cross = (b._X - a._X) * (c._Y - b._Y) - (b._Y - a._Y) * (c._X - b._X);
        if cross <= 1e-9 {
            return false;
        }

        // Point-in-triangle check for all other remaining vertices
        let mut j = 0u32;
        while j < active.Len() {
            let idx = *active.Get(j).unwrap();
            if idx != prev && idx != i && idx != next {
                let p = *pts.Get(idx).unwrap();
                if Self::PointInTriangle(p, a, b, c) {
                    return false;
                }
            }
            j += 1;
        }
        true
    }

    fn PointInTriangle(p: SketchPoint, a: SketchPoint, b: SketchPoint, c: SketchPoint) -> bool
    {
        let cp1 = (b._X - a._X) * (p._Y - a._Y) - (b._Y - a._Y) * (p._X - a._X);
        let cp2 = (c._X - b._X) * (p._Y - b._Y) - (c._Y - b._Y) * (p._X - b._X);
        let cp3 = (a._X - c._X) * (p._Y - c._Y) - (a._Y - c._Y) * (p._X - c._X);

        (cp1 >= -1e-9 && cp2 >= -1e-9 && cp3 >= -1e-9) || (cp1 <= 1e-9 && cp2 <= 1e-9 && cp3 <= 1e-9)
    }

    fn ComputeBoundsF64(positions: Arr<'_, [f64; 3]>) -> ([f64; 3], [f64; 3])
    {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        positions.Traverse(|p| {
            min[0] = min[0].min(p[0]);
            min[1] = min[1].min(p[1]);
            min[2] = min[2].min(p[2]);
            max[0] = max[0].max(p[0]);
            max[1] = max[1].max(p[1]);
            max[2] = max[2].max(p[2]);
        });
        (min, max)
    }
}
