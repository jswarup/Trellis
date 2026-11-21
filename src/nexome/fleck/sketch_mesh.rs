// sketch_mesh.rs --------------------------------------------------------------------------------------------------
//! 3D wireframe mesh projection and workplane visualizer for 2D sketch documents in `fleck`.

use crate::fleck::geometry::{GeometryAsset, GeometryVertex};
use crate::fleck::sketch::{SketchDocument, SketchPlane, SketchPoint, SketchPrimitive};
use crate::fleck::sketch_tessellate::SketchTessellate;
use crate::silo::{Buff, IArr, Stash};

//-------------------------------------------------------------------------------------------------

/// Configuration options for projecting a 2D sketch into a 3D wireframe and datum grid asset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sketch3dProjectionOptions
{
    pub _IncludeWorkplaneGrid: bool,
    pub _IncludeTriad:         bool,
    pub _GridSize:             f64,
    pub _GridSubdivisions:     u32,
    pub _CurveTolerancePixels: f64,
}

impl Default for Sketch3dProjectionOptions
{
    fn default() -> Self
    {
        Self {
            _IncludeWorkplaneGrid: true,
            _IncludeTriad:         true,
            _GridSize:             100.0,
            _GridSubdivisions:     10,
            _CurveTolerancePixels: 0.5,
        }
    }
}

//-------------------------------------------------------------------------------------------------

pub struct Sketch3dProjection;

impl Sketch3dProjection
{
    /// Projects a 2D sketch document onto its 3D workplane, returning a renderable 3D `GeometryAsset`.
    pub fn ProjectDocument(
        doc: &SketchDocument,
        options: Sketch3dProjectionOptions,
    ) -> Result<GeometryAsset, String>
    {
        let plane = *doc.Plane();
        let mut vertices: Stash<GeometryVertex> = Stash::New();
        let mut edges: Stash<[u32; 2]> = Stash::New();

        if options._IncludeWorkplaneGrid && options._GridSubdivisions > 0 {
            Self::AddWorkplaneGrid(&plane, &options, &mut vertices, &mut edges);
        }

        if options._IncludeTriad {
            Self::AddTriad(&plane, options._GridSize * 0.25, &mut vertices, &mut edges);
        }

        doc.Entities().Traverse(|entity| {
            if let Some(layer) = doc.FindLayer(entity.LayerId())
                && !layer.IsVisible()
            {
                return;
            }
            let is_const = entity.IsConstruction()
                || doc.FindLayer(entity.LayerId()).is_some_and(|l| l.IsConstruction());
            let color = if is_const {
                [0.4, 0.7, 1.0, 0.6]
            } else {
                [1.0, 1.0, 1.0, 1.0]
            };
            Self::AddEntityGeometry(
                doc,
                entity.Primitive(),
                &plane,
                options._CurveTolerancePixels,
                color,
                &mut vertices,
                &mut edges,
            );
        });

        if vertices.IsEmpty() {
            let origin_pos = [plane._Origin[0] as f32, plane._Origin[1] as f32, plane._Origin[2] as f32];
            vertices.Push(GeometryVertex::New(origin_pos, [0.8, 0.8, 0.8, 1.0]));
            return Ok(GeometryAsset::FromPointCloudData(vertices.IntoBuff(), (origin_pos, origin_pos)));
        }

        let bounds = Self::ComputeBounds(vertices.Arr());

        GeometryAsset::FromMesh(
            vertices.IntoBuff(),
            Buff::New(),
            edges.IntoBuff(),
            Buff::New(),
            bounds,
        )
    }

    fn AddWorkplaneGrid(
        plane: &SketchPlane,
        options: &Sketch3dProjectionOptions,
        vertices: &mut Stash<GeometryVertex>,
        edges: &mut Stash<[u32; 2]>,
    )
    {
        let half = options._GridSize * 0.5;
        let divs = options._GridSubdivisions;
        let step = options._GridSize / (divs as f64);
        let grid_color = [0.35, 0.4, 0.5, 0.35];

        let mut i = 0u32;
        while i <= divs {
            let coord = -half + (i as f64) * step;

            let p0 = plane.ProjectTo3d(SketchPoint::New(coord, -half));
            let p1 = plane.ProjectTo3d(SketchPoint::New(coord, half));
            let idx0 = vertices.Size();
            vertices.Push(GeometryVertex::New([p0[0] as f32, p0[1] as f32, p0[2] as f32], grid_color));
            vertices.Push(GeometryVertex::New([p1[0] as f32, p1[1] as f32, p1[2] as f32], grid_color));
            edges.Push([idx0, idx0 + 1]);

            let q0 = plane.ProjectTo3d(SketchPoint::New(-half, coord));
            let q1 = plane.ProjectTo3d(SketchPoint::New(half, coord));
            let idx1 = vertices.Size();
            vertices.Push(GeometryVertex::New([q0[0] as f32, q0[1] as f32, q0[2] as f32], grid_color));
            vertices.Push(GeometryVertex::New([q1[0] as f32, q1[1] as f32, q1[2] as f32], grid_color));
            edges.Push([idx1, idx1 + 1]);

            i += 1;
        }
    }

    fn AddTriad(
        plane: &SketchPlane,
        length: f64,
        vertices: &mut Stash<GeometryVertex>,
        edges: &mut Stash<[u32; 2]>,
    )
    {
        let o = plane._Origin;
        let u_end = [o[0] + plane._U[0] * length, o[1] + plane._U[1] * length, o[2] + plane._U[2] * length];
        let v_end = [o[0] + plane._V[0] * length, o[1] + plane._V[1] * length, o[2] + plane._V[2] * length];
        let n_end = [o[0] + plane._Normal[0] * length, o[1] + plane._Normal[1] * length, o[2] + plane._Normal[2] * length];

        let o_f32 = [o[0] as f32, o[1] as f32, o[2] as f32];

        let idx_u = vertices.Size();
        vertices.Push(GeometryVertex::New(o_f32, [0.95, 0.2, 0.2, 1.0]));
        vertices.Push(GeometryVertex::New([u_end[0] as f32, u_end[1] as f32, u_end[2] as f32], [0.95, 0.2, 0.2, 1.0]));
        edges.Push([idx_u, idx_u + 1]);

        let idx_v = vertices.Size();
        vertices.Push(GeometryVertex::New(o_f32, [0.2, 0.9, 0.2, 1.0]));
        vertices.Push(GeometryVertex::New([v_end[0] as f32, v_end[1] as f32, v_end[2] as f32], [0.2, 0.9, 0.2, 1.0]));
        edges.Push([idx_v, idx_v + 1]);

        let idx_n = vertices.Size();
        vertices.Push(GeometryVertex::New(o_f32, [0.2, 0.4, 0.95, 1.0]));
        vertices.Push(GeometryVertex::New([n_end[0] as f32, n_end[1] as f32, n_end[2] as f32], [0.2, 0.4, 0.95, 1.0]));
        edges.Push([idx_n, idx_n + 1]);
    }

    fn AddEntityGeometry(
        doc: &SketchDocument,
        prim: &SketchPrimitive,
        plane: &SketchPlane,
        tol_pixels: f64,
        color: [f32; 4],
        vertices: &mut Stash<GeometryVertex>,
        edges: &mut Stash<[u32; 2]>,
    )
    {
        match prim {
            SketchPrimitive::Line(l) => {
                let p0 = plane.ProjectTo3d(l._Start);
                let p1 = plane.ProjectTo3d(l._End);
                let base = vertices.Size();
                vertices.Push(GeometryVertex::New([p0[0] as f32, p0[1] as f32, p0[2] as f32], color));
                vertices.Push(GeometryVertex::New([p1[0] as f32, p1[1] as f32, p1[2] as f32], color));
                edges.Push([base, base + 1]);
            }
            SketchPrimitive::Circle(c) => {
                let pts = SketchTessellate::TessellateCircle(c, tol_pixels, 1.0);
                Self::AddPolygonLoop(pts.Arr(), plane, color, true, vertices, edges);
            }
            SketchPrimitive::Arc(a) => {
                let pts = SketchTessellate::TessellateArc(a, tol_pixels, 1.0);
                Self::AddPolygonLoop(pts.Arr(), plane, color, false, vertices, edges);
            }
            SketchPrimitive::Ellipse(e) => {
                let pts = SketchTessellate::TessellateEllipse(e, tol_pixels, 1.0);
                Self::AddPolygonLoop(pts.Arr(), plane, color, e.IsClosed(), vertices, edges);
            }
            SketchPrimitive::Polyline(p) => {
                let pts = doc.PolylinePoints(p._PointOffset, p._PointCount);
                Self::AddPolygonLoop(pts.Arr(), plane, color, p._IsClosed, vertices, edges);
            }
        }
    }

    fn AddPolygonLoop(
        pts: crate::silo::Arr<'_, SketchPoint>,
        plane: &SketchPlane,
        color: [f32; 4],
        closed: bool,
        vertices: &mut Stash<GeometryVertex>,
        edges: &mut Stash<[u32; 2]>,
    )
    {
        let len = pts.Len();
        if len < 2 {
            return;
        }
        let base = vertices.Size();
        let mut i = 0u32;
        while i < len {
            let pt = pts.Get(i).unwrap();
            let p3d = plane.ProjectTo3d(*pt);
            vertices.Push(GeometryVertex::New([p3d[0] as f32, p3d[1] as f32, p3d[2] as f32], color));
            if i > 0 {
                edges.Push([base + i - 1, base + i]);
            }
            i += 1;
        }
        if closed && len >= 3 {
            let first = pts.Get(0).unwrap();
            let last = pts.Get(len - 1).unwrap();
            if first.DistanceSqTo(*last) > 1e-9 {
                edges.Push([base + len - 1, base]);
            }
        }
    }

    fn ComputeBounds(verts: crate::silo::Arr<'_, GeometryVertex>) -> ([f32; 3], [f32; 3])
    {
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        verts.Traverse(|v| {
            let p = v.Position();
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
