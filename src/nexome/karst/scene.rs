// scene.rs ----------------------------------------------------------------------------------------
//! Presentation adapters for Karst multi-die SoC fabrics into `fenst::cask_scene` and `fenst::cask2d`.
//! This is part of the `karst` crate, keeping `fascia` independent of domain models.

use crate::fenst::cask2d::{Cask2dColorScheme, Cask2dLayout, Cask2dLayoutMode, Cask2dOptions};
use crate::fenst::cask_scene::CaskScene;
use crate::karst::circuit_xplr::KarstDieXplr;
use crate::karst::fabric::KarstFabric;
use std::sync::atomic::AtomicBool;

/// Solves classical 2D schematic layout for the full Karst multi-die SoC fabric.
pub fn FabricSchematic(fabric: &KarstFabric, colors: Cask2dColorScheme) -> Cask2dLayout
{
    let options = Cask2dOptions::New()
        .WithMode(Cask2dLayoutMode::CircuitSchematic)
        .WithColors(colors);
    Cask2dLayout::FromCircuit(&fabric.AsCircuit(), &options)
}

/// Solves classical 2D schematic layout for an individual Karst die (housing on-die crossbar NoC and perimeter routing).
pub fn DieSchematic(die: &KarstDieXplr, colors: Cask2dColorScheme) -> Cask2dLayout
{
    let options = Cask2dOptions::New()
        .WithMode(Cask2dLayoutMode::CircuitSchematic)
        .WithColors(colors);
    Cask2dLayout::FromCircuit(die, &options)
}

/// Creates a 3D CaskScene hierarchy for the Karst multi-die SoC fabric.
pub fn FabricScene(fabric: &KarstFabric, cancelled: &AtomicBool) -> Result<CaskScene, String>
{
    CaskScene::FromCircuit(&fabric.AsCircuit(), cancelled)
}
