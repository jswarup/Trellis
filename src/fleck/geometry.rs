// geometry.rs ------------------------------------------------------------------------------------
//! Fleck 3D geometry parsers producing `fenst::geometry::GeometryAsset`.
//! Implements `fenst::geometry::GeometryProvider` for Wavefront OBJ and PTS point clouds.

pub use crate::fenst::geometry::*;
use crate::fleck::{ParsePts, ParseWaveObj, PtsCloud, WaveObjModel};
use crate::silo::{Buff, IArr};

/// Extension trait providing OBJ and PTS construction for `GeometryAsset`.
pub trait FleckGeometryAssetExt
{
    fn FromPts(cloud: PtsCloud) -> Result<GeometryAsset, String>;
    fn FromObj(model: WaveObjModel) -> Result<GeometryAsset, String>;
}

impl FleckGeometryAssetExt for GeometryAsset
{
    fn FromPts(cloud: PtsCloud) -> Result<GeometryAsset, String>
    {
        GeometryAssetFromPts(cloud)
    }

    fn FromObj(model: WaveObjModel) -> Result<GeometryAsset, String>
    {
        GeometryAssetFromObj(model)
    }
}

pub fn GeometryAssetFromPts(cloud: PtsCloud) -> Result<GeometryAsset, String>
{
    if cloud.IsEmpty() {
        return Err("The file contains no points.".into());
    }
    let bounds = cloud.BoundingBox();
    let (center, scale) = GeometryAsset::Normalization(bounds)?;
    let mut valid = true;
    let mut intensity_min = f32::INFINITY;
    let mut intensity_max = f32::NEG_INFINITY;
    cloud.Points().Arr().Traverse(|p| {
        valid &= p._Pos.Pos().iter().all(|n| n.is_finite());
        if let Some(value) = p._Intensity.filter(|v| v.is_finite()) {
            intensity_min = intensity_min.min(value);
            intensity_max = intensity_max.max(value);
        }
    });
    if !valid {
        return Err("Point coordinates must be finite.".into());
    }
    let vertices = Buff::FromDispenser(cloud.Count(), |i| {
        let p = cloud.Points()[i];
        let color = p
            ._Color
            .map(|c| {
                [
                    f32::from(c._R) / 255.0,
                    f32::from(c._G) / 255.0,
                    f32::from(c._B) / 255.0,
                    1.0,
                ]
            })
            .unwrap_or([0.35, 0.76, 0.94, 1.0]);
        let intensity = p
            ._Intensity
            .filter(|v| v.is_finite())
            .map(|v| {
                if intensity_max > intensity_min {
                    ((f64::from(v) - f64::from(intensity_min))
                        / (f64::from(intensity_max) - f64::from(intensity_min))) as f32
                } else {
                    0.5
                }
            })
            .unwrap_or(0.5);
        GeometryVertex::New(
            GeometryAsset::Local(p._Pos.Pos(), center, scale),
            color,
        ).WithIntensity(intensity)
    });
    Ok(GeometryAsset::FromPointCloudData(vertices, bounds))
}

pub fn GeometryAssetFromObj(model: WaveObjModel) -> Result<GeometryAsset, String>
{
    if model.VertexCount() == 0 {
        return Err("The file contains no vertices.".into());
    }
    let bounds = model.BoundingBox();
    let _ = GeometryAsset::Normalization(bounds)?;
    let mut valid = true;
    model._Vertices.Arr().Traverse(|v| {
        valid &= v._X.is_finite() && v._Y.is_finite() && v._Z.is_finite();
    });
    model._Faces.Arr().Traverse(|face| {
        valid &= face.Len() >= 3;
        face._Vertices.Arr().Traverse(|v| {
            valid &= v._VertexIdx > 0 && v._VertexIdx <= model.VertexCount();
        });
    });
    if !valid {
        return Err("Invalid OBJ coordinates or face vertex indices.".into());
    }
    let mesh = model.ToMeshDto();
    let vertices = Buff::FromDispenser(model.VertexCount(), |i| {
        GeometryVertex::New(
            mesh._Points[i],
            [0.65, 0.72, 0.85, 1.0],
        )
    });
    GeometryAsset::FromMesh(vertices, mesh._Triangles, mesh._Edges, Buff::New(), bounds)
}

//-------------------------------------------------------------------------------------------------
// GeometryProvider Implementation for Fleck

pub struct FleckGeometryProvider;

impl GeometryProvider for FleckGeometryProvider
{
    fn CanLoad(&self, extension: &str) -> bool
    {
        extension.eq_ignore_ascii_case("obj") || extension.eq_ignore_ascii_case("pts")
    }

    fn LoadFromContent(&self, extension: &str, content: &str) -> Result<GeometryAsset, String>
    {
        if extension.eq_ignore_ascii_case("pts") {
            let cloud = ParsePts(content)?;
            GeometryAssetFromPts(cloud)
        } else if extension.eq_ignore_ascii_case("obj") {
            let model = ParseWaveObj(content)?;
            GeometryAssetFromObj(model)
        } else {
            Err(format!("Unsupported extension for fleck geometry provider: {extension}"))
        }
    }
}

static FLECK_PROVIDER: FleckGeometryProvider = FleckGeometryProvider;

inventory::submit! {
    GeometryProviderRegistration(&FLECK_PROVIDER)
}
