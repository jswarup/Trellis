// sketch_sweep.rs ----------------------------------------------------------------------------------------------------
//! 3D Solid Sweep engine sweeping closed 2D sketch profiles along 3D guide paths.
//! Generates watertight 3D boundary representation meshes with Frenet / Bishop parallel transport framing.

use crate::fenst::geometry::GeometryAsset;
use crate::fenst::sketch::{SketchDocument, SketchPoint};
use crate::fleck::sketch_extrude::SketchExtrude;
use crate::silo::{Arr, Buff, IArr, Stash, USeg};

//-------------------------------------------------------------------------------------------------

/// Frame basis consisting of origin and two orthonormal normal/binormal axes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SweepFrame
{
    _Origin:   [f64; 3],
    _Normal:   [f64; 3],
    _Binormal: [f64; 3],
}

pub struct SketchSweep;

impl SketchSweep
{
    /// Sweeps all closed profiles in the document along a 3D polyline guide path.
    pub fn SweepDocument(doc: &SketchDocument, guide_path: Arr<'_, [f64; 3]>) -> Result<GeometryAsset, String>
    {
        if guide_path.Size() < 2 {
            return Err("Sweep guide path must contain at least 2 points.".into());
        }

        let profiles = SketchExtrude::ExtractClosedProfiles(doc);
        if profiles.IsEmpty() {
            return Err("No closed profile loops found in sketch to sweep.".into());
        }

        let mut all_positions: Stash<[f64; 3]> = Stash::New();
        let mut all_colors: Stash<[f32; 4]> = Stash::New();
        let mut all_triangles: Stash<[u32; 3]> = Stash::New();
        let mut all_edges: Stash<[u32; 2]> = Stash::New();

        profiles.Arr().Traverse(|profile| {
            Self::SweepSingleProfile(
                profile.Arr(),
                guide_path,
                &mut all_positions,
                &mut all_colors,
                &mut all_triangles,
                &mut all_edges,
            );
        });

        if all_positions.IsEmpty() {
            return Err("Failed to generate solid geometry from sweep profiles.".into());
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

    /// Sweeps a single closed profile along a 3D guide path into the target geometry buffers.
    pub fn SweepSingleProfile(
        profile: Arr<'_, SketchPoint>,
        guide: Arr<'_, [f64; 3]>,
        positions: &mut Stash<[f64; 3]>,
        colors: &mut Stash<[f32; 4]>,
        triangles: &mut Stash<[u32; 3]>,
        edges: &mut Stash<[u32; 2]>,
    )
    {
        let nPts = profile.Size();
        let nGuide = guide.Size();
        if nPts < 3 || nGuide < 2 {
            return;
        }

        let frames = Self::ComputeSweepFrames(guide);
        let baseIdx = positions.Size();
        let color = [0.72, 0.78, 0.88, 1.0];

        // 1. Emit vertices for every slice along the guide path
        frames.Arr().Traverse(|frame| {
            USeg::FromLen(nPts).Traverse(|pIdx| {
                let p2d = profile.Get(pIdx).unwrap();
                let p3d = [
                    frame._Origin[0] + p2d._X * frame._Normal[0] + p2d._Y * frame._Binormal[0],
                    frame._Origin[1] + p2d._X * frame._Normal[1] + p2d._Y * frame._Binormal[1],
                    frame._Origin[2] + p2d._X * frame._Normal[2] + p2d._Y * frame._Binormal[2],
                ];
                positions.Push(p3d);
                colors.Push(color);
            });
        });

        // 2. Skin side quads between consecutive slices
        USeg::FromLen(nGuide - 1).Traverse(|k| {
            let slice0 = baseIdx + k * nPts;
            let slice1 = slice0 + nPts;
            USeg::FromLen(nPts).Traverse(|i| {
                let next = (i + 1) % nPts;
                let b0 = slice0 + i;
                let b1 = slice0 + next;
                let t0 = slice1 + i;
                let t1 = slice1 + next;

                triangles.Push([b0, b1, t1]);
                triangles.Push([b0, t1, t0]);

                edges.Push([b0, b1]);
                edges.Push([b0, t0]);
            });
        });

        // 3. Triangulate Start and End End-Caps
        let capTris = SketchExtrude::TriangulatePolygon(profile);

        // Start cap (k = 0, reversed winding for outward normal pointing -tangent)
        capTris.Arr().Traverse(|tri| {
            triangles.Push([baseIdx + tri[2], baseIdx + tri[1], baseIdx + tri[0]]);
        });

        // End cap (k = nGuide - 1, forward winding pointing +tangent)
        let endSlice = baseIdx + (nGuide - 1) * nPts;
        capTris.Arr().Traverse(|tri| {
            triangles.Push([endSlice + tri[0], endSlice + tri[1], endSlice + tri[2]]);
        });

        // End cap perimeter edges
        USeg::FromLen(nPts).Traverse(|i| {
            edges.Push([baseIdx + i, baseIdx + (i + 1) % nPts]);
            edges.Push([endSlice + i, endSlice + (i + 1) % nPts]);
        });
    }

    fn ComputeSweepFrames(guide: Arr<'_, [f64; 3]>) -> Stash<SweepFrame>
    {
        let count = guide.Size();
        let mut frames = Stash::WithCapacity(count);
        let mut prevFrame: Option<SweepFrame> = None;

        USeg::FromLen(count).Traverse(|i| {
            let p = *guide.Get(i).unwrap();
            let tangent = if i == 0 {
                let pNext = *guide.Get(1).unwrap();
                Self::Normalize([pNext[0] - p[0], pNext[1] - p[1], pNext[2] - p[2]])
            } else if i == count - 1 {
                let pPrev = *guide.Get(i - 1).unwrap();
                Self::Normalize([p[0] - pPrev[0], p[1] - pPrev[1], p[2] - pPrev[2]])
            } else {
                let pPrev = *guide.Get(i - 1).unwrap();
                let pNext = *guide.Get(i + 1).unwrap();
                Self::Normalize([pNext[0] - pPrev[0], pNext[1] - pPrev[1], pNext[2] - pPrev[2]])
            };

            let (normal, binormal) = match prevFrame {
                None => {
                    let up = if tangent[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
                    let norm = Self::Normalize(Self::Cross(tangent, up));
                    let binorm = Self::Cross(tangent, norm);
                    (norm, binorm)
                }
                Some(prev) => {
                    let v = Self::Cross(prev._Normal, tangent);
                    let norm = if Self::LengthSq(v) > 1e-12 {
                        let bin = Self::Cross(tangent, prev._Normal);
                        Self::Normalize(Self::Cross(bin, tangent))
                    } else {
                        prev._Normal
                    };
                    let binorm = Self::Cross(tangent, norm);
                    (norm, binorm)
                }
            };

            let frame = SweepFrame { _Origin: p, _Normal: normal, _Binormal: binormal };
            frames.Push(frame);
            prevFrame = Some(frame);
        });

        frames
    }

    fn Normalize(v: [f64; 3]) -> [f64; 3]
    {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len > 1e-12 {
            [v[0] / len, v[1] / len, v[2] / len]
        } else {
            [0.0, 0.0, 1.0]
        }
    }

    fn Cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3]
    {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    fn LengthSq(v: [f64; 3]) -> f64
    {
        v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
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
