// cask_labels.rs ----------------------------------------------------------------------------------
//! Measures, preflights and rasterizes cask labels independently of mesh construction.

use crate::fenst::cask_scene::{CaskScene, CheckCancelled};
use crate::fleck::geometry::{GeometryLabels, LabelVertex};
use crate::silo::{Buff, Stash, USeg};
use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

//-------------------------------------------------------------------------------------------------

static FONTS: OnceLock<Mutex<FontSystem>> = OnceLock::new();

pub(super) struct LabelPlan
{
    _Sizes:   Buff<[f32; 2]>,
    _Regions: Buff<[u32; 4]>,
    _Size:    [u32; 2],
}

impl LabelPlan
{
    pub(super) fn Measure(scene: &mut CaskScene, cancelled: &AtomicBool) -> Result<Self, String>
    {
        CheckCancelled(cancelled)?;
        let mut fonts = FONTS
            .get_or_init(|| Mutex::new(FontSystem::new()))
            .lock()
            .map_err(|_| "Font measurement failed.".to_string())?;
        CheckCancelled(cancelled)?;
        let mut text = Buffer::new(&mut fonts, Metrics::new(14.0, 20.0));
        text.set_size(&mut fonts, None, None);
        let mut sizes = Stash::WithCapacity(scene.Nodes().Size());
        scene.Layout(cancelled, |name| {
            text.set_text(&mut fonts, name, &Attrs::new(), Shaping::Advanced, None);
            let mut size = [1.0_f32, 20.0_f32];
            text.layout_runs().for_each(|run| {
                size[0] = size[0].max(run.line_w);
                size[1] = size[1].max(run.line_top + run.line_height);
            });
            sizes.Push(size);
            size
        })?;
        // Shelf-pack one rasterized label per node. The atlas is uploaded once with the mesh.
        let mut regions = Stash::WithCapacity(scene.Nodes().Size());
        let mut x = 0;
        let mut y = 0;
        let mut rowHeight = 0;
        let mut widest = 1;
        let mut area = 0_u64;
        let mut valid = true;
        sizes.Arr().USeg().Span(|index| {
            if cancelled.load(Ordering::Acquire)
            {
                return false;
            }
            let size = sizes[index];
            valid &= size
                .iter()
                .all(|n| n.is_finite() && *n > 0.0 && *n <= 4092.0);
            if !valid
            {
                return false;
            }
            let w = size[0].ceil() as u32 + 4;
            let h = size[1].ceil() as u32 + 4;
            widest = widest.max(w);
            area = area.saturating_add(u64::from(w) * u64::from(h));
            true
        });
        CheckCancelled(cancelled)?;
        if !valid || area > 4096 * 4096
        {
            return Err("Cask labels exceed the atlas budget. Open a smaller root.".into());
        }
        let width = widest
            .max((area as f64).sqrt().ceil() as u32)
            .max(512)
            .checked_next_power_of_two()
            .unwrap_or(u32::MAX);
        if width > 4096
        {
            return Err("A cask label exceeds the 4096-pixel atlas limit.".into());
        }
        sizes.Arr().USeg().Span(|index| {
            if cancelled.load(Ordering::Acquire)
            {
                return false;
            }
            let size = sizes[index];
            let w = size[0].ceil() as u32 + 4;
            let h = size[1].ceil() as u32 + 4;
            if x + w > width
            {
                x = 0;
                y += rowHeight;
                rowHeight = 0;
            }
            regions.Push([x, y, w, h]);
            x += w;
            rowHeight = rowHeight.max(h);
            true
        });
        CheckCancelled(cancelled)?;
        let height = y + rowHeight;
        if height > 4096
        {
            return Err("Cask labels exceed the atlas budget. Open a smaller root.".into());
        }

        Ok(Self {
            _Sizes:   sizes.ExtractBuff(),
            _Regions: regions.ExtractBuff(),
            _Size:    [width, height],
        })
    }

    pub(super) fn Raster(
        &self, scene: &CaskScene, cancelled: &AtomicBool,
    ) -> Result<GeometryLabels, String>
    {
        CheckCancelled(cancelled)?;
        let mut fonts = FONTS
            .get_or_init(|| Mutex::new(FontSystem::new()))
            .lock()
            .map_err(|_| "Font rasterization failed.".to_string())?;
        CheckCancelled(cancelled)?;
        let mut text = Buffer::new(&mut fonts, Metrics::new(14.0, 20.0));
        text.set_size(&mut fonts, None, None);
        let [width, height] = self._Size;
        let mut pixels = Buff::FromDispenser(width * height, |_| 0_u8);
        let mut vertices = Stash::WithCapacity(scene.Nodes().Size() * 6);
        let mut levels = Buff::FromDispenser(scene.MaxDepth(), |_| 0);
        let mut cache = SwashCache::new();
        let mut index = 0;
        while index < scene.Nodes().Size()
        {
            if cancelled.load(Ordering::Acquire)
            {
                return Err("Loading cancelled.".into());
            }
            let node = &scene.Nodes()[index];
            let [x, y, w, h] = self._Regions[index];
            text.set_text(
                &mut fonts,
                node.Name(),
                &Attrs::new(),
                Shaping::Advanced,
                None,
            );
            text.draw(
                &mut fonts,
                &mut cache,
                Color::rgb(255, 255, 255),
                |px, py, _, _, color| {
                    let px = px + 2;
                    let py = py + 2;
                    if px >= 0 && py >= 0 && px < w as i32 && py < h as i32
                    {
                        let pixel = (y + py as u32) * width + x + px as u32;
                        pixels[pixel] = pixels[pixel].max(color.a());
                    }
                },
            );
            let origin = node.Origin();
            // XY label plane, positive-X baseline, on the front surface of its own cuboid.
            let corner = [
                origin[0] + 2.0,
                origin[1] + self._Sizes[index][1] + 6.0,
                origin[2] + node.Size()[2] + 0.01,
            ];
            let corners = [
                [0.0, 0.0],
                [1.0, 0.0],
                [0.0, 1.0],
                [0.0, 1.0],
                [1.0, 0.0],
                [1.0, 1.0],
            ];
            USeg::FromLen(6).Traverse(|i| {
                let p = corners[i as usize];
                vertices.Push(LabelVertex::New(
                    [
                        corner[0] + p[0] * w as f32,
                        corner[1] - p[1] * h as f32,
                        corner[2],
                    ],
                    [
                        (x as f32 + p[0] * w as f32) / width as f32,
                        (y as f32 + p[1] * h as f32) / height as f32,
                    ],
                ));
            });
            levels[node.Depth() - 1] = vertices.Size();
            index += 1;
        }
        drop(fonts);
        CheckCancelled(cancelled)?;
        GeometryLabels::New(vertices.ExtractBuff(), pixels, [width, height], levels)
    }
}

//-------------------------------------------------------------------------------------------------
