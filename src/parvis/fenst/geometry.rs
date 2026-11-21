// geometry.rs ------------------------------------------------------------------------------------
//! Explorer registry and presentation provider interfaces for geometry assets in `fenst`.
//! Core geometry domain structures are canonically defined in `nexome::fleck::geometry`.

pub use crate::fleck::geometry::*;
use std::sync::{OnceLock, RwLock};

//-------------------------------------------------------------------------------------------------
// Geometry Provider Trait & Registry

/// Trait implemented by file parsers / loaders (`fleck` for OBJ/PTS) to produce `GeometryAsset`.
pub trait GeometryProvider: Send + Sync
{
    fn CanLoad(&self, extension: &str) -> bool;
    fn LoadFromContent(&self, extension: &str, content: &str) -> Result<GeometryAsset, String>;
}

pub struct GeometryProviderRegistration(pub &'static dyn GeometryProvider);
inventory::collect!(GeometryProviderRegistration);

static GEOMETRY_PROVIDERS: OnceLock<RwLock<Vec<Box<dyn GeometryProvider>>>> = OnceLock::new();

fn GeometryProvidersList() -> &'static RwLock<Vec<Box<dyn GeometryProvider>>>
{
    GEOMETRY_PROVIDERS.get_or_init(|| RwLock::new(Vec::new()))
}

/// Registers a geometry provider (e.g. from `fleck`).
pub fn RegisterGeometryProvider(provider: Box<dyn GeometryProvider>)
{
    let mut lock = GeometryProvidersList().write().unwrap();
    lock.push(provider);
}

/// Loads a geometry asset from content using registered geometry providers.
pub fn LoadGeometry(extension: &str, content: &str) -> Result<GeometryAsset, String>
{
    for reg in inventory::iter::<GeometryProviderRegistration>
    {
        if reg.0.CanLoad(extension)
        {
            return reg.0.LoadFromContent(extension, content);
        }
    }
    let lock = GeometryProvidersList().read().unwrap();
    for p in lock.iter()
    {
        if p.CanLoad(extension)
        {
            return p.LoadFromContent(extension, content);
        }
    }
    Err(format!("No geometry provider registered for format '{extension}'"))
}

//-------------------------------------------------------------------------------------------------
// Fleck Geometry Provider Registration

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
            let cloud = crate::fleck::ParsePts(content)?;
            crate::fleck::geometry::GeometryAssetFromPts(cloud)
        } else if extension.eq_ignore_ascii_case("obj") {
            let model = crate::fleck::ParseWaveObj(content)?;
            crate::fleck::geometry::GeometryAssetFromObj(model)
        } else {
            Err(format!("Unsupported extension for fleck geometry provider: {extension}"))
        }
    }
}

static FLECK_PROVIDER: FleckGeometryProvider = FleckGeometryProvider;

inventory::submit! {
    GeometryProviderRegistration(&FLECK_PROVIDER)
}
