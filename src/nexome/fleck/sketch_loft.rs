// sketch_loft.rs -----------------------------------------------------------------------------------------------------
//! 3D Solid Loft engine skinning across multiple planar 2D/3D profile sections into a solid CAD mesh.

use crate::fenst::geometry::GeometryAsset;
use crate::fenst::sketch::{SketchPlane, SketchPoint};
use crate::fleck::sketch_extrude::SketchExtrude;
use crate::silo::{Arr, Buff, IArr, Stash, USeg};

//-------------------------------------------------------------------------------------------------

/// A section profile in 3D space ready for loft skinning.
#[derive(Clone, Debug)]
pub struct LoftSection
{
    _Points: Buff<[f64; 3]>,
}

impl LoftSection
{
    pub fn From3dPoints(pts: Arr<'_, [f64; 3]>) -> Self
    {
        let mut stash = Stash::WithCapacity(pts.Size());
        pts.Traverse(|p| stash.Push(*p));
        Self { _Points: stash.IntoBuff() }
    }

    pub fn FromPlanarProfile(profile: Arr<'_, SketchPoint>, plane: &SketchPlane) -> Self
    {
        let mut stash = Stash::WithCapacity(profile.Size());
        profile.Traverse(|pt| {
            stash.Push(plane.ProjectTo3d(*pt));
        });
        Self { _Points: stash.IntoBuff() }
    }

    pub fn Points(&self) -> Arr<'_, [f64; 3]> { self._Points.Arr() }

    pub fn Resample(&self, target_count: u32) -> Self
    {
        let orig = self.Points();
        let n = orig.Size();
        if n == target_count || n < 3 || target_count < 3 {
            return self.clone();
        }

        // Compute cumulative perimeter lengths
        let mut lengths = Stash::WithCapacity(n + 1);
        lengths.Push(0.0);
        let mut total = 0.0f64;
        USeg::FromLen(n).Traverse(|i| {
            let p0 = *orig.Get(i).unwrap();
            let p1 = *orig.Get((i + 1) % n).unwrap();
            let d = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2) + (p1[2] - p0[2]).powi(2)).sqrt();
            total += d;
            lengths.Push(total);
        });

        let mut resampled = Stash::WithCapacity(target_count);
        let step = total / (target_count as f64);

        USeg::FromLen(target_count).Traverse(|k| {
            let targetD = (k as f64) * step;
            let mut found = 0u32;
            let lenArr = lengths.Arr();
            USeg::FromLen(n).Traverse(|i| {
                if *lenArr.Get(i).unwrap() <= targetD {
                    found = i;
                }
            });
            let next = (found + 1) % n;
            let d0 = *lenArr.Get(found).unwrap();
            let pFound = *orig.Get(found).unwrap();
            let pNext = *orig.Get(next).unwrap();
            let segLen = ((pNext[0] - pFound[0]).powi(2)
                + (pNext[1] - pFound[1]).powi(2)
                + (pNext[2] - pFound[2]).powi(2))
            .sqrt();

            let t = if segLen > 1e-12 { ((targetD - d0) / segLen).clamp(0.0, 1.0) } else { 0.0 };
            let pInterp = [
                pFound[0] + t * (pNext[0] - pFound[0]),
                pFound[1] + t * (pNext[1] - pFound[1]),
                pFound[2] + t * (pNext[2] - pFound[2]),
            ];
            resampled.Push(pInterp);
        });

        Self { _Points: resampled.IntoBuff() }
    }
}

//-------------------------------------------------------------------------------------------------

pub struct SketchLoft;

impl SketchLoft
{
    /// Lofts a solid across an ordered sequence of profile sections.
    pub fn LoftSections(sections: &[LoftSection]) -> Result<GeometryAsset, String>
    {
        if sections.len() < 2 {
            return Err("Loft operation requires at least 2 profile sections.".into());
        }

        // Standardize vertex count across all sections to the maximum section resolution
        let mut maxPts = 0u32;
        for s in sections {
            maxPts = maxPts.max(s.Points().Size());
        }
        if maxPts < 3 {
            return Err("Loft sections must have at least 3 points per profile.".into());
        }

        let mut resampledSections = Vec::with_capacity(sections.len());
        for s in sections {
            resampledSections.push(s.Resample(maxPts));
        }

        let nSections = resampledSections.len() as u32;
        let nPts = maxPts;

        let mut all_positions: Stash<[f64; 3]> = Stash::New();
        let mut all_colors: Stash<[f32; 4]> = Stash::New();
        let mut all_triangles: Stash<[u32; 3]> = Stash::New();
        let mut all_edges: Stash<[u32; 2]> = Stash::New();

        let color = [0.80, 0.75, 0.85, 1.0];

        // 1. Emit vertices for every section
        for s in &resampledSections {
            s.Points().Traverse(|p| {
                all_positions.Push(*p);
                all_colors.Push(color);
            });
        }

        // 2. Skin side quads between consecutive sections
        USeg::FromLen(nSections - 1).Traverse(|k| {
            let slice0 = k * nPts;
            let slice1 = slice0 + nPts;
            USeg::FromLen(nPts).Traverse(|i| {
                let next = (i + 1) % nPts;
                let b0 = slice0 + i;
                let b1 = slice0 + next;
                let t0 = slice1 + i;
                let t1 = slice1 + next;

                all_triangles.Push([b0, b1, t1]);
                all_triangles.Push([b0, t1, t0]);

                all_edges.Push([b0, b1]);
                all_edges.Push([b0, t0]);
            });
        });

        // 3. Triangulate Bottom Cap (Section 0)
        let bottomPts2d = Self::ProjectToCentroidPlane(resampledSections[0].Points());
        let capTris = SketchExtrude::TriangulatePolygon(bottomPts2d.Arr());
        capTris.Arr().Traverse(|tri| {
            all_triangles.Push([tri[2], tri[1], tri[0]]);
        });

        // 4. Triangulate Top Cap (Section N-1)
        let topPts2d = Self::ProjectToCentroidPlane(resampledSections[(nSections - 1) as usize].Points());
        let topTris = SketchExtrude::TriangulatePolygon(topPts2d.Arr());
        let topSlice = (nSections - 1) * nPts;
        topTris.Arr().Traverse(|tri| {
            all_triangles.Push([topSlice + tri[0], topSlice + tri[1], topSlice + tri[2]]);
        });

        // Perimeter edges
        USeg::FromLen(nPts).Traverse(|i| {
            all_edges.Push([i, (i + 1) % nPts]);
            all_edges.Push([topSlice + i, topSlice + (i + 1) % nPts]);
        });

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

    fn ProjectToCentroidPlane(pts: Arr<'_, [f64; 3]>) -> Buff<SketchPoint>
    {
        let count = pts.Size();
        if count == 0 {
            return Buff::New();
        }
        let mut center = [0.0, 0.0, 0.0];
        pts.Traverse(|p| {
            center[0] += p[0];
            center[1] += p[1];
            center[2] += p[2];
        });
        center[0] /= count as f64;
        center[1] /= count as f64;
        center[2] /= count as f64;

        // Project onto primary dominant axis
        let mut stash = Stash::WithCapacity(count);
        pts.Traverse(|p| {
            stash.Push(SketchPoint::New(p[0] - center[0], p[1] - center[1]));
        });
        stash.IntoBuff()
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
