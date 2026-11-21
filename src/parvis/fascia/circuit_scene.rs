// circuit_scene.rs ---------------------------------------------------------------------------------
//! Shared UI presentation for hardware circuits, modules, and interconnect fabrics implementing
//! the generic `fenst::xplr::CircuitXplr` trait.

use crate::fascia::cask_view::cask2d_scheme_from_palette;
use crate::fascia::theme::ThemePalette;
use crate::fenst::cask2d::{Cask2dLayout, Cask2dLayoutMode, Cask2dOptions};
use crate::fenst::cask_scene::CaskScene;
use crate::fenst::xplr::CircuitXplr;
use crate::fenst::geometry::GeometryAsset;
use std::sync::atomic::{AtomicBool, Ordering};

/// Solves the 2D classical schematic layout for any `CircuitXplr` entity adhering to the theme palette.
pub fn CircuitSchematic(circuit: &dyn CircuitXplr, palette: &ThemePalette) -> Cask2dLayout
{
    let options = Cask2dOptions::New()
        .WithMode(Cask2dLayoutMode::CircuitSchematic)
        .WithColors(cask2d_scheme_from_palette(palette));
    Cask2dLayout::FromCircuit(circuit, &options)
}

/// Creates a 3D CaskScene hierarchy for any `CircuitXplr` entity (modules, submodules, ports, wires).
pub fn CircuitScene(circuit: &dyn CircuitXplr, cancelled: &AtomicBool) -> Result<CaskScene, String>
{
    CaskScene::FromCircuit(circuit, cancelled)
}

/// Builds labelled 3D geometry consumable by the shared Trellis viewport for any `CircuitXplr`.
pub fn CircuitBuild(circuit: &dyn CircuitXplr, cancelled: &AtomicBool) -> Result<GeometryAsset, String>
{
    if cancelled.load(Ordering::Acquire)
    {
        return Err("Loading cancelled.".into());
    }
    let scene = CircuitScene(circuit, cancelled)?;
    crate::fascia::cask_scene::Build(scene, cancelled)
}
