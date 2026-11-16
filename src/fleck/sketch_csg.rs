// sketch_csg.rs ------------------------------------------------------------------------------------------------------
//! Constructive Solid Geometry (CSG) Boolean operations for solid 3D meshes.
//! Supports Union, Difference, and Intersection between manifold closed boundary representation meshes.

use crate::fenst::geometry::GeometryAsset;
use crate::silo::{Arr, Buff, IArr, Stash};

//-------------------------------------------------------------------------------------------------

/// Boolean operation kind for Constructive Solid Geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CsgOp
{
    Union,
    Difference,
    Intersection,
}

pub struct SketchCsg;

impl SketchCsg
{
    /// Evaluates a CSG Boolean operation between solid mesh A and solid mesh B.
    pub fn Apply(mesh_a: &GeometryAsset, mesh_b: &GeometryAsset, op: CsgOp) -> Result<GeometryAsset, String>
    {
        let vA = mesh_a.Vertices();
        let tA = mesh_a.Triangles();
        let vB = mesh_b.Vertices();
        let tB = mesh_b.Triangles();

        if tA.IsEmpty() && tB.IsEmpty() {
            return Err("Both input meshes for CSG operation are empty.".into());
        }
        if tA.IsEmpty() {
            return match op {
                CsgOp::Union => Ok(mesh_b.clone()),
                CsgOp::Difference | CsgOp::Intersection => Err("First operand mesh is empty.".into()),
            };
        }
        if tB.IsEmpty() {
            return match op {
                CsgOp::Union | CsgOp::Difference => Ok(mesh_a.clone()),
                CsgOp::Intersection => Err("Second operand mesh is empty.".into()),
            };
        }

        let mut out_positions: Stash<[f64; 3]> = Stash::New();
        let mut out_colors: Stash<[f32; 4]> = Stash::New();
        let mut out_triangles: Stash<[u32; 3]> = Stash::New();
        let mut out_edges: Stash<[u32; 2]> = Stash::New();

        // 1. Process triangles from Mesh A
        tA.Traverse(|tri| {
            let p0 = Self::VertexPos(vA.Get(tri[0]).unwrap());
            let p1 = Self::VertexPos(vA.Get(tri[1]).unwrap());
            let p2 = Self::VertexPos(vA.Get(tri[2]).unwrap());
            let centroid = [
                (p0[0] + p1[0] + p2[0]) / 3.0,
                (p0[1] + p1[1] + p2[1]) / 3.0,
                (p0[2] + p1[2] + p2[2]) / 3.0,
            ];
            let insideB = Self::IsPointInsideMesh(centroid, vB, tB);
            let keepA = match op {
                CsgOp::Union | CsgOp::Difference => !insideB,
                CsgOp::Intersection => insideB,
            };
            if keepA {
                let col = vA.Get(tri[0]).unwrap().Color();
                Self::EmitTriangle([p0, p1, p2], col, &mut out_positions, &mut out_colors, &mut out_triangles, &mut out_edges);
            }
        });

        // 2. Process triangles from Mesh B
        tB.Traverse(|tri| {
            let p0 = Self::VertexPos(vB.Get(tri[0]).unwrap());
            let p1 = Self::VertexPos(vB.Get(tri[1]).unwrap());
            let p2 = Self::VertexPos(vB.Get(tri[2]).unwrap());
            let centroid = [
                (p0[0] + p1[0] + p2[0]) / 3.0,
                (p0[1] + p1[1] + p2[1]) / 3.0,
                (p0[2] + p1[2] + p2[2]) / 3.0,
            ];
            let insideA = Self::IsPointInsideMesh(centroid, vA, tA);
            let col = vB.Get(tri[0]).unwrap().Color();

            match op {
                CsgOp::Union => {
                    if !insideA {
                        Self::EmitTriangle([p0, p1, p2], col, &mut out_positions, &mut out_colors, &mut out_triangles, &mut out_edges);
                    }
                }
                CsgOp::Difference => {
                    if insideA {
                        // Reverse winding for subtracted cavity facing inward
                        Self::EmitTriangle([p0, p2, p1], col, &mut out_positions, &mut out_colors, &mut out_triangles, &mut out_edges);
                    }
                }
                CsgOp::Intersection => {
                    if insideA {
                        Self::EmitTriangle([p0, p1, p2], col, &mut out_positions, &mut out_colors, &mut out_triangles, &mut out_edges);
                    }
                }
            }
        });

        if out_positions.IsEmpty() {
            return Err("CSG operation yielded empty volume.".into());
        }

        let bounds = Self::ComputeBoundsF64(out_positions.Arr());

        GeometryAsset::FromMeshF64(
            out_positions.Arr(),
            out_colors.Arr(),
            out_triangles.IntoBuff(),
            out_edges.IntoBuff(),
            Buff::New(),
            bounds,
        )
    }

    fn VertexPos(v: &crate::fenst::geometry::GeometryVertex) -> [f64; 3]
    {
        let p = v.Position();
        [p[0] as f64, p[1] as f64, p[2] as f64]
    }

    fn EmitTriangle(
        pts: [[f64; 3]; 3],
        color: [f32; 4],
        positions: &mut Stash<[f64; 3]>,
        colors: &mut Stash<[f32; 4]>,
        triangles: &mut Stash<[u32; 3]>,
        edges: &mut Stash<[u32; 2]>,
    )
    {
        let base = positions.Size();
        positions.Push(pts[0]);
        positions.Push(pts[1]);
        positions.Push(pts[2]);
        colors.Push(color);
        colors.Push(color);
        colors.Push(color);
        triangles.Push([base, base + 1, base + 2]);
        edges.Push([base, base + 1]);
        edges.Push([base + 1, base + 2]);
        edges.Push([base + 2, base]);
    }

    /// Point-in-polyhedron parity test via ray casting along fixed non-axis-aligned ray.
    fn IsPointInsideMesh(
        point: [f64; 3],
        vertices: Arr<'_, crate::fenst::geometry::GeometryVertex>,
        triangles: Arr<'_, [u32; 3]>,
    ) -> bool
    {
        // Fixed non-degenerate unit ray vector
        let rayDir = [0.2672612419, 0.5345224838, 0.8017837257];
        let mut hitCount = 0u32;

        triangles.Traverse(|tri| {
            let v0 = Self::VertexPos(vertices.Get(tri[0]).unwrap());
            let v1 = Self::VertexPos(vertices.Get(tri[1]).unwrap());
            let v2 = Self::VertexPos(vertices.Get(tri[2]).unwrap());

            if Self::RayIntersectsTriangle(point, rayDir, v0, v1, v2) {
                hitCount += 1;
            }
        });

        (hitCount % 2) == 1
    }

    /// Möller–Trumbore ray-triangle intersection algorithm.
    fn RayIntersectsTriangle(
        orig: [f64; 3],
        dir: [f64; 3],
        v0: [f64; 3],
        v1: [f64; 3],
        v2: [f64; 3],
    ) -> bool
    {
        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        let h = [
            dir[1] * e2[2] - dir[2] * e2[1],
            dir[2] * e2[0] - dir[0] * e2[2],
            dir[0] * e2[1] - dir[1] * e2[0],
        ];

        let a = e1[0] * h[0] + e1[1] * h[1] + e1[2] * h[2];
        if a.abs() < 1e-9 {
            return false;
        }

        let f = 1.0 / a;
        let s = [orig[0] - v0[0], orig[1] - v0[1], orig[2] - v0[2]];
        let u = f * (s[0] * h[0] + s[1] * h[1] + s[2] * h[2]);
        if !(0.0..=1.0).contains(&u) {
            return false;
        }

        let q = [
            s[1] * e1[2] - s[2] * e1[1],
            s[2] * e1[0] - s[0] * e1[2],
            s[0] * e1[1] - s[1] * e1[0],
        ];

        let v = f * (dir[0] * q[0] + dir[1] * q[1] + dir[2] * q[2]);
        if v < 0.0 || u + v > 1.0 {
            return false;
        }

        let t = f * (e2[0] * q[0] + e2[1] * q[1] + e2[2] * q[2]);
        t > 1e-9
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
