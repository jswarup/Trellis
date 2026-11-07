// scene.rs ----------------------------------------------------------------------------------------
//! Presentation adapters for Rube digital logic modules into `fenst::cask_scene` and `fenst::cask2d`.
//! This is part of the `rube` crate, keeping `fascia` independent of domain models.

use crate::fenst::cask2d::{Cask2dColorScheme, Cask2dLayout, Cask2dLayoutMode, Cask2dOptions};
use crate::fenst::cask_scene::{CaskScene, ICaskHierarchy};
use crate::rube::{KernelKind, Layout, ModuleId};
use std::fmt::Write;
use std::sync::atomic::AtomicBool;

//-------------------------------------------------------------------------------------------------
// 3D Module Hierarchy (Original module volume view)

struct ModuleHierarchy<'a>
{
    _Layout: &'a Layout,
}

impl ICaskHierarchy for ModuleHierarchy<'_>
{
    type Node = ModuleId;

    fn Label(&self, id: ModuleId) -> String
    {
        let module = self._Layout.Module(id);
        let mut label = format!("{}\n", self._Layout.LocalName(id));
        match module.Kernel()
        {
            KernelKind::None => label.push_str("Module"),
            KernelKind::Fast(op) => write!(label, "{op:?}").unwrap(),
            KernelKind::Coro(_) => label.push_str("Coroutine"),
        }
        write!(label, " | {} in | {} out", module.InPorts().Size(), module.OutPorts().Size()).unwrap();
        label
    }

    fn SpanChildren(&self, id: ModuleId, mut visit: impl FnMut(ModuleId) -> bool)
    {
        self._Layout.Children(id).USeg().Span(|index| visit(self._Layout.Children(id)[index]));
    }
}

/// Creates a 3D scene for any module subtree, before or after Freeze.
pub fn Scene(layout: &Layout, root: ModuleId, cancelled: &AtomicBool) -> Result<CaskScene, String>
{
    CaskScene::FromHierarchy(&ModuleHierarchy { _Layout: layout }, root, cancelled)
}

/// Classical 2D circuit schematic for a Rube module using the generic `CircuitXplr` pipeline.
pub fn CircuitSchematic(layout: &Layout, root: ModuleId, colors: Cask2dColorScheme) -> Cask2dLayout
{
    let options = Cask2dOptions::New()
        .WithMode(Cask2dLayoutMode::CircuitSchematic)
        .WithColors(colors);
    Cask2dLayout::FromCircuit(&layout.AsCircuit(root), &options)
}

/// 3D hierarchy scene for a Rube module using the generic `CircuitXplr` pipeline.
pub fn CircuitScene(layout: &Layout, root: ModuleId, cancelled: &AtomicBool) -> Result<CaskScene, String>
{
    CaskScene::FromCircuit(&layout.AsCircuit(root), cancelled)
}
