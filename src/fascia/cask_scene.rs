// cask_scene.rs -----------------------------------------------------------------------------------
//! Converts a measured hierarchy into bounded surface geometry and independent point samples.

use crate::fascia::cask_labels::LabelPlan;
use crate::fenst::cask_scene::{CaskScene, CheckCancelled};
use crate::fleck::geometry::{GeometryAsset, GeometryVertex};
use crate::silo::{Buff, Stash, USeg};
use std::sync::atomic::{AtomicBool, Ordering};

//-------------------------------------------------------------------------------------------------

pub fn Build(mut scene: CaskScene, cancelled: &AtomicBool) -> Result<GeometryAsset, String>
{
    let labels = LabelPlan::Measure(&mut scene, cancelled)?;
    let mesh = Mesh(&scene, cancelled)?;
    CheckCancelled(cancelled)?;
    mesh.WithLabels(labels.Raster(&scene, cancelled)?)
}

fn Preflight(scene: &CaskScene, cancelled: &AtomicBool) -> Result<(), String>
{
    CheckCancelled(cancelled)?;
    let count = u64::from(scene.Nodes().Size());
    if count == 0 || count * 8 > 4_000_000 || count * 26 > 4_000_000 || count * 12 > 8_000_000
    {
        return Err("Cask geometry exceeds the mesh budget. Open a smaller root.".into());
    }
    let mut valid = true;
    scene.Nodes().USeg().Span(|index| {
        if cancelled.load(Ordering::Acquire)
        {
            return false;
        }
        let node = &scene.Nodes()[index];
        valid &= node
            .Size()
            .iter()
            .all(|size| size.is_finite() && *size > 0.0);
        valid &= node.Origin().iter().all(|origin| origin.is_finite());
        valid
    });
    CheckCancelled(cancelled)?;
    if !valid
    {
        return Err("Invalid cask dimensions.".into());
    }
    Ok(())
}

pub fn Mesh(scene: &CaskScene, cancelled: &AtomicBool) -> Result<GeometryAsset, String>
{
    Preflight(scene, cancelled)?;
    let count = scene.Nodes().Size();
    let mut vertices = Stash::WithCapacity(count * 8);
    let mut triangles = Stash::WithCapacity(count * 12);
    let mut edges = Stash::WithCapacity(count * 12);
    let mut samples = Stash::WithCapacity(count * 26);
    let mut levels = Buff::FromDispenser(scene.MaxDepth(), |_| [0; 3]);
    let mut sampleLevels = Buff::FromDispenser(scene.MaxDepth(), |_| 0);
    scene.Nodes().USeg().Span(|index| {
        if cancelled.load(Ordering::Acquire)
        {
            return false;
        }
        let node = &scene.Nodes()[index];
        let origin = node.Origin();
        let size = node.Size();
        let color = Pastel(index);
        let base = vertices.Size();
        USeg::FromLen(8).Traverse(|corner| {
            let position = std::array::from_fn(|axis| {
                origin[axis] + size[axis] * ((corner >> axis) & 1) as f32
            });
            vertices.Push(GeometryVertex::New(position, color));
        });
        // A fixed 3x3x3 grid without its interior point: 26 samples, independent of tree height.
        USeg::FromLen(27).Traverse(|sample| {
            if sample != 13
            {
                let grid = [sample % 3, sample / 3 % 3, sample / 9];
                let position =
                    std::array::from_fn(|axis| origin[axis] + size[axis] * grid[axis] as f32 * 0.5);
                samples.Push(GeometryVertex::New(position, color));
            }
        });
        // One outward-facing quad per cuboid face, sharing eight corner vertices.
        USeg::FromLen(3).Traverse(|axis| {
            let u = (axis + 1) % 3;
            let v = (axis + 2) % 3;
            USeg::FromLen(2).Traverse(|side| {
                let i0 = base + (side << axis);
                let i1 = i0 + (1 << u);
                let i2 = i1 + (1 << v);
                let i3 = i0 + (1 << v);
                if side == 1
                {
                    triangles.Push([i0, i1, i2]);
                    triangles.Push([i0, i2, i3]);
                }
                else
                {
                    triangles.Push([i0, i2, i1]);
                    triangles.Push([i0, i3, i2]);
                }
            });
            USeg::FromLen(4).Traverse(|corner| {
                let first = base + ((corner & 1) << u) + ((corner >> 1) << v);
                edges.Push([first, first + (1 << axis)]);
            });
        });
        levels[node.Depth() - 1] = [vertices.Size(), triangles.Size(), edges.Size()];
        sampleLevels[node.Depth() - 1] = samples.Size();
        true
    });
    CheckCancelled(cancelled)?;
    GeometryAsset::FromMesh(
        vertices.ExtractBuff(),
        triangles.ExtractBuff(),
        edges.ExtractBuff(),
        levels,
        scene.Bounds(),
    )?
    .WithSamples(samples.ExtractBuff(), sampleLevels)
}

fn Pastel(index: u32) -> [f32; 4]
{
    // Seeded permutation of a pastel cycle: stable during interaction and refresh.
    const COLORS: [[f32; 4]; 8] = [
        [0.96, 0.70, 0.74, 1.0],
        [0.71, 0.82, 0.98, 1.0],
        [0.68, 0.90, 0.76, 1.0],
        [0.98, 0.84, 0.62, 1.0],
        [0.83, 0.74, 0.96, 1.0],
        [0.67, 0.89, 0.89, 1.0],
        [0.95, 0.75, 0.92, 1.0],
        [0.88, 0.91, 0.68, 1.0],
    ];
    const ORDER: [usize; 8] = [3, 0, 5, 7, 2, 6, 1, 4];
    COLORS[ORDER[(index % 8) as usize]]
}

//-------------------------------------------------------------------------------------------------
