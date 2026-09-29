// module_scene.rs ---------------------------------------------------------------------------------
//! Presents Rube modules through the shared Cask layout and geometry pipeline.

use crate::fenst::cask_scene::{CaskScene, ICaskHierarchy};
use crate::fleck::geometry::GeometryAsset;
use crate::rube::{KernelKind, Layout, ModuleId};
use crate::silo::IArr;
use std::fmt::Write;
use std::sync::atomic::{AtomicBool, Ordering};

//-------------------------------------------------------------------------------------------------

struct ModuleHierarchy<'a>
{
    _Layout: &'a Layout,
}

impl ICaskHierarchy for ModuleHierarchy<'_>
{
    type Node = ModuleId;

    fn  Label( &self, id: ModuleId) -> String
    {
        let module      = self._Layout.Module( id);
        let mut label   = format!( "{}\n", self._Layout.LocalName( id));
        match module.Kernel()
        {
            KernelKind::None => label.push_str( "Module"),
            KernelKind::Fast( op) => write!( label, "{op:?}").unwrap(),
            KernelKind::Coro( _) => label.push_str( "Coroutine"),
        }
        write!( label, " | {} in | {} out", module.InPorts().Size(), module.OutPorts().Size()).unwrap();
        return label;
    }

    fn  TraverseChildren( &self, id: ModuleId, mut visit: impl FnMut( ModuleId))
    {
        self._Layout.Children( id).Traverse( |&child| visit( child));
    }
}

/// Creates a scene for any module subtree, before or after Freeze. Use current layout ModuleIds.
pub fn  Scene( layout: &Layout, root: ModuleId) -> CaskScene
{
    return CaskScene::FromHierarchy( &ModuleHierarchy { _Layout: layout }, root);
}

/// Builds labelled geometry consumable by the existing shared 3D viewport.
pub fn  Build( layout: &Layout, root: ModuleId, cancelled: &AtomicBool)
             -> Result<GeometryAsset, String>
{
    if cancelled.load( Ordering::Acquire)
    {
        return Err( "Loading cancelled.".into());
    }
    return crate::fascia::cask_scene::Build( Scene( layout, root), cancelled);
}

//-------------------------------------------------------------------------------------------------
