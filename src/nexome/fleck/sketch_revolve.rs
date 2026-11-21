// sketch_revolve.rs ------------------------------------------------------------------------------------------------
//! 3D Revolve / Lathe modeling engine converting planar 2D sketch profiles into rotational solid meshes.

use crate::fleck::geometry::GeometryAsset;
use crate::fleck::sketch::{SketchDocument, SketchPlane, SketchPoint};
use crate::fleck::sketch_extrude::SketchExtrude;
use crate::silo::{Arr, Buff, IArr, Stash};

//-------------------------------------------------------------------------------------------------

/// Axis about which a 2D sketch profile is revolved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RevolveAxis
{
    /// Workplane Y-axis (lathe spindle axis, x = 0). Radial distance is profile x.
    Y,
    /// Workplane X-axis (cylindrical shaft axis, y = 0). Radial distance is profile y.
    X,
}

/// Options controlling 3D rotational lathe revolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SketchRevolveOptions
{
    /// Axis of revolution on the workplane.
    pub axis: RevolveAxis,
    /// Total revolution angle in degrees (e.g. 360.0 for full lathe solid, 180.0, 90.0).
    pub angle_deg: f64,
    /// Number of angular tessellation segments around a full 360° circle (min 8, default 36).
    pub segments: u32,
}

impl Default for SketchRevolveOptions
{
    fn default() -> Self
    {
        Self {
            axis: RevolveAxis::Y,
            angle_deg: 360.0,
            segments: 36,
        }
    }
}

//-------------------------------------------------------------------------------------------------

pub struct SketchRevolve;

impl SketchRevolve
{
    /// Revolves all closed profile loops in the sketch document around the specified axis.
    pub fn RevolveDocument(
        doc: &SketchDocument,
        options: SketchRevolveOptions,
    ) -> Result<GeometryAsset, String>
    {
        let angle = options.angle_deg.clamp(1.0, 360.0);
        let plane = *doc.Plane();
        let profiles = SketchExtrude::ExtractClosedProfiles(doc);
        if profiles.IsEmpty() {
            return Err("No closed profile loops found in sketch to revolve.".into());
        }

        let mut all_positions: Stash<[f64; 3]> = Stash::New();
        let mut all_colors: Stash<[f32; 4]> = Stash::New();
        let mut all_triangles: Stash<[u32; 3]> = Stash::New();
        let mut all_edges: Stash<[u32; 2]> = Stash::New();

        // Check all profiles for axis crossing
        let mut cross_err = None;
        profiles.Arr().Traverse(|profile| {
            if cross_err.is_none()
                && let Err(e) = Self::ValidateProfileAxis(profile.Arr(), options.axis)
            {
                cross_err = Some(e);
            }
        });
        if let Some(err) = cross_err {
            return Err(err);
        }

        let effective_options = SketchRevolveOptions {
            axis: options.axis,
            angle_deg: angle,
            segments: options.segments.max(8),
        };

        profiles.Arr().Traverse(|profile| {
            Self::RevolveSingleProfile(
                profile.Arr(),
                &plane,
                effective_options,
                &mut all_positions,
                &mut all_colors,
                &mut all_triangles,
                &mut all_edges,
            );
        });

        if all_positions.IsEmpty() {
            return Err("Failed to generate revolved solid geometry from sketch profiles.".into());
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

    /// Revolves a single closed polygon profile around the specified axis.
    pub fn RevolveProfile(
        profile: Arr<'_, SketchPoint>,
        plane: &SketchPlane,
        options: SketchRevolveOptions,
    ) -> Result<GeometryAsset, String>
    {
        let angle = options.angle_deg.clamp(1.0, 360.0);
        if profile.Len() < 3 {
            return Err("Revolve profile requires at least 3 distinct vertices.".into());
        }
        Self::ValidateProfileAxis(profile, options.axis)?;

        let mut positions: Stash<[f64; 3]> = Stash::New();
        let mut colors: Stash<[f32; 4]> = Stash::New();
        let mut triangles: Stash<[u32; 3]> = Stash::New();
        let mut edges: Stash<[u32; 2]> = Stash::New();

        let effective_options = SketchRevolveOptions {
            axis: options.axis,
            angle_deg: angle,
            segments: options.segments.max(8),
        };

        Self::RevolveSingleProfile(
            profile,
            plane,
            effective_options,
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

    fn RevolveSingleProfile(
        raw_pts: Arr<'_, SketchPoint>,
        plane: &SketchPlane,
        options: SketchRevolveOptions,
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

        let is_full = (options.angle_deg - 360.0).abs() < 1e-4;
        let total_rad = options.angle_deg.to_radians();
        let num_steps = if is_full {
            options.segments.max(12)
        } else {
            ((options.segments as f64 * (options.angle_deg / 360.0)).round() as u32).max(4)
        };
        let num_rings = if is_full { num_steps } else { num_steps + 1 };
        let base_vertex_offset = positions.Size();

        let mut is_negative_side = false;
        pts.Arr().Traverse(|pt| {
            let r = match options.axis {
                RevolveAxis::Y => pt._X,
                RevolveAxis::X => pt._Y,
            };
            if r < -1e-4 {
                is_negative_side = true;
            }
        });

        Self::GenerateRevolveVertices(
            pts.Arr(),
            plane,
            options.axis,
            num_rings,
            num_steps,
            total_rad,
            positions,
            colors,
        );

        Self::GenerateRevolveQuads(
            pts.Arr(),
            options.axis,
            num_steps,
            num_rings,
            base_vertex_offset,
            is_negative_side,
            triangles,
            edges,
        );

        if !is_full {
            Self::CapPartialRevolve(
                pts.Arr(),
                base_vertex_offset,
                base_vertex_offset + num_steps * n,
                is_negative_side,
                positions,
                colors,
                triangles,
                edges,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn GenerateRevolveVertices(
        pts: Arr<'_, SketchPoint>,
        plane: &SketchPlane,
        axis: RevolveAxis,
        num_rings: u32,
        num_steps: u32,
        total_rad: f64,
        positions: &mut Stash<[f64; 3]>,
        colors: &mut Stash<[f32; 4]>,
    )
    {
        let n = pts.Size();
        let wall_color = [0.76, 0.80, 0.88, 1.0];
        let mut k = 0u32;
        while k < num_rings {
            let phi = total_rad * (k as f64 / num_steps as f64);
            let cos_phi = phi.cos();
            let sin_phi = phi.sin();

            let mut i = 0u32;
            while i < n {
                let pt = *pts.Get(i).unwrap();
                let (r, h) = match axis {
                    RevolveAxis::Y => (pt._X, pt._Y),
                    RevolveAxis::X => (pt._Y, pt._X),
                };
                let (px, py, pz) = match axis {
                    RevolveAxis::Y => (r * cos_phi, h, r * sin_phi),
                    RevolveAxis::X => (h, r * cos_phi, r * sin_phi),
                };
                positions.Push(Self::PlaneFrameToWorld(plane, px, py, pz));
                colors.Push(wall_color);
                i += 1;
            }
            k += 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn GenerateRevolveQuads(
        pts: Arr<'_, SketchPoint>,
        axis: RevolveAxis,
        num_steps: u32,
        num_rings: u32,
        base_offset: u32,
        is_negative_side: bool,
        triangles: &mut Stash<[u32; 3]>,
        edges: &mut Stash<[u32; 2]>,
    )
    {
        let n = pts.Size();
        let mut step = 0u32;
        while step < num_steps {
            let next_step = (step + 1) % num_rings;
            let r0_offset = base_offset + step * n;
            let r1_offset = base_offset + next_step * n;

            let mut i = 0u32;
            while i < n {
                let next_i = (i + 1) % n;
                let pt0 = *pts.Get(i).unwrap();
                let pt1 = *pts.Get(next_i).unwrap();
                let r0 = match axis {
                    RevolveAxis::Y => pt0._X.abs(),
                    RevolveAxis::X => pt0._Y.abs(),
                };
                let r1 = match axis {
                    RevolveAxis::Y => pt1._X.abs(),
                    RevolveAxis::X => pt1._Y.abs(),
                };
                let v00 = r0_offset + i;
                let v01 = r0_offset + next_i;
                let v10 = r1_offset + i;
                let v11 = r1_offset + next_i;

                Self::PushQuadTriangles(
                    [v00, v01, v10, v11],
                    r0 < 1e-4,
                    r1 < 1e-4,
                    is_negative_side,
                    triangles,
                    edges,
                );
                i += 1;
            }
            step += 1;
        }
    }

    fn PushQuadTriangles(
        v: [u32; 4],
        r0_zero: bool,
        r1_zero: bool,
        is_negative_side: bool,
        triangles: &mut Stash<[u32; 3]>,
        edges: &mut Stash<[u32; 2]>,
    )
    {
        let [v00, v01, v10, v11] = v;
        if r0_zero && r1_zero {
            return;
        }
        if r0_zero {
            let tri = if is_negative_side { [v00, v11, v01] } else { [v00, v01, v11] };
            triangles.Push(tri);
            edges.Push([v00, v01]);
            edges.Push([v01, v11]);
        } else if r1_zero {
            let tri = if is_negative_side { [v00, v10, v11] } else { [v00, v11, v10] };
            triangles.Push(tri);
            edges.Push([v00, v10]);
            edges.Push([v00, v11]);
        } else {
            let (t1, t2) = if is_negative_side {
                ([v00, v10, v11], [v00, v11, v01])
            } else {
                ([v00, v11, v10], [v00, v01, v11])
            };
            triangles.Push(t1);
            triangles.Push(t2);
            edges.Push([v00, v10]);
            edges.Push([v00, v01]);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn CapPartialRevolve(
        pts: Arr<'_, SketchPoint>,
        start_offset: u32,
        end_offset: u32,
        is_negative_side: bool,
        positions: &mut Stash<[f64; 3]>,
        colors: &mut Stash<[f32; 4]>,
        triangles: &mut Stash<[u32; 3]>,
        edges: &mut Stash<[u32; 2]>,
    )
    {
        let n = pts.Len();
        if n < 3 {
            return;
        }

        let cap_triangles = SketchExtrude::TriangulatePolygon(pts);
        let num_tri = cap_triangles.Size();
        let cap_color = [0.65, 0.70, 0.78, 1.0];

        // Start cap (at phi = 0)
        let cap_start_base = positions.Size();
        let mut i = 0u32;
        while i < n {
            let pos = *positions.Arr().Get(start_offset + i).unwrap();
            positions.Push(pos);
            colors.Push(cap_color);
            i += 1;
        }

        let mut t = 0u32;
        while t < num_tri {
            let tri = *cap_triangles.Arr().Get(t).unwrap();
            let winding = if is_negative_side {
                [cap_start_base + tri[0], cap_start_base + tri[1], cap_start_base + tri[2]]
            } else {
                [cap_start_base + tri[0], cap_start_base + tri[2], cap_start_base + tri[1]]
            };
            triangles.Push(winding);
            edges.Push([cap_start_base + tri[0], cap_start_base + tri[1]]);
            edges.Push([cap_start_base + tri[1], cap_start_base + tri[2]]);
            edges.Push([cap_start_base + tri[2], cap_start_base + tri[0]]);
            t += 1;
        }

        // End cap (at phi = angle)
        let cap_end_base = positions.Size();
        let mut j = 0u32;
        while j < n {
            let pos = *positions.Arr().Get(end_offset + j).unwrap();
            positions.Push(pos);
            colors.Push(cap_color);
            j += 1;
        }

        let mut t = 0u32;
        while t < num_tri {
            let tri = *cap_triangles.Arr().Get(t).unwrap();
            let winding = if is_negative_side {
                [cap_end_base + tri[0], cap_end_base + tri[2], cap_end_base + tri[1]]
            } else {
                [cap_end_base + tri[0], cap_end_base + tri[1], cap_end_base + tri[2]]
            };
            triangles.Push(winding);
            edges.Push([cap_end_base + tri[0], cap_end_base + tri[1]]);
            edges.Push([cap_end_base + tri[1], cap_end_base + tri[2]]);
            edges.Push([cap_end_base + tri[2], cap_end_base + tri[0]]);
            t += 1;
        }
    }

    fn PlaneFrameToWorld(plane: &SketchPlane, px: f64, py: f64, pz: f64) -> [f64; 3]
    {
        let o = plane.Origin();
        let u = plane.U();
        let v = plane.V();
        let n = plane.Normal();
        [
            o[0] + px * u[0] + py * v[0] + pz * n[0],
            o[1] + px * u[1] + py * v[1] + pz * n[1],
            o[2] + px * u[2] + py * v[2] + pz * n[2],
        ]
    }

    fn NormalizeWinding(pts: Arr<'_, SketchPoint>) -> Stash<SketchPoint>
    {
        let area = SketchExtrude::SignedArea(pts);
        let len = pts.Len();
        let mut out = Stash::WithCapacity(len);
        if area >= 0.0 {
            pts.Traverse(|p| out.Push(*p));
        } else {
            let mut i = len;
            while i > 0 {
                out.Push(*pts.Get(i - 1).unwrap());
                i -= 1;
            }
        }
        out
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

    /// Validates that a profile does not cross the revolution axis.
    pub fn ValidateProfileAxis(pts: Arr<'_, SketchPoint>, axis: RevolveAxis) -> Result<(), String>
    {
        let mut has_pos = false;
        let mut has_neg = false;
        pts.Traverse(|pt| {
            let r = match axis {
                RevolveAxis::Y => pt._X,
                RevolveAxis::X => pt._Y,
            };
            if r > 1e-4 {
                has_pos = true;
            } else if r < -1e-4 {
                has_neg = true;
            }
        });
        if has_pos && has_neg {
            return Err("Profile crosses the revolution axis, which would produce self-intersecting overlapping geometry. The sketch profile must lie entirely on one side of the axis.".into());
        }
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
