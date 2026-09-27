// cask_scene.rs -----------------------------------------------------------------------------------
//! Converts a measured hierarchy into generic sampled geometry for the shared viewport.

use crate::fenst::cask_scene::CaskScene;
use crate::fleck::geometry::{GeometryAsset, GeometryLabels, GeometryVertex, LabelVertex};
use crate::silo::{Buff, IArr, Stash, USeg};
use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

//-------------------------------------------------------------------------------------------------

pub fn Build( mut scene: CaskScene, cancelled: &AtomicBool) -> Result<GeometryAsset, String>
{
    static FONTS: OnceLock<Mutex<FontSystem>> = OnceLock::new();
    let mut fonts = FONTS.get_or_init( || Mutex::new( FontSystem::new()))
                         .lock()
                         .map_err( |_| "Font measurement failed.".to_string())?;
    let mut text = Buffer::new( &mut fonts, Metrics::new( 14.0, 20.0));
    text.set_size( &mut fonts, None, None);
    let mut sizes = Stash::New();
    scene.Layout( |name| {
             text.set_text( &mut fonts, name, &Attrs::new(), Shaping::Advanced, None);
             let mut size = [1.0_f32, 20.0_f32];
             text.layout_runs().for_each( |run| {
                                   size[0] = size[0].max( run.line_w);
                                   size[1] = size[1].max( run.line_top + run.line_height);
                               });
             sizes.Push( size);
             size
         });
    // Shelf-pack one rasterized label per node. The atlas is uploaded once with the mesh.
    let mut regions = Stash::New();
    let mut x = 0;
    let mut y = 0;
    let mut rowHeight = 0;
    let mut widest = 1;
    let mut area = 0_u64;
    sizes.Arr().Traverse( |size| {
                   let w = size[0].ceil() as u32 + 4;
                   let h = size[1].ceil() as u32 + 4;
                   widest = widest.max( w);
                   area = area.saturating_add( u64::from( w) * u64::from( h));
               });
    if area > 4096 * 4096 {
        return Err( "Cask labels exceed the atlas budget. Open a smaller root.".into());
    }
    let width = widest.max( ( area as f64).sqrt().ceil() as u32)
                      .max( 512)
                      .checked_next_power_of_two()
                      .unwrap_or( u32::MAX);
    if width > 4096 {
        return Err( "A cask label exceeds the 4096-pixel atlas limit.".into());
    }
    sizes.Arr().Traverse( |size| {
                   let w = size[0].ceil() as u32 + 4;
                   let h = size[1].ceil() as u32 + 4;
                   if x + w > width {
                       x = 0;
                       y += rowHeight;
                       rowHeight = 0;
                   }
                   regions.Push( [x, y, w, h]);
                   x += w;
                   rowHeight = rowHeight.max( h);
               });
    let height = y + rowHeight;
    if height > 4096 {
        return Err( "Cask labels exceed the atlas budget. Open a smaller root.".into());
    }
    let mut pixels = Buff::FromDispenser( width * height, |_| 0_u8);
    let mut vertices = Stash::New();
    let mut levels = Buff::FromDispenser( scene.MaxDepth(), |_| 0);
    let mut cache = SwashCache::new();
    let mut index = 0;
    while index < scene.Nodes().Size() {
        if cancelled.load( Ordering::Acquire) {
            return Err( "Loading cancelled.".into());
        }
        let node = &scene.Nodes()[index];
        let [x, y, w, h] = regions[index];
        text.set_text( &mut fonts,
                      node.Name(),
                      &Attrs::new(),
                      Shaping::Advanced,
                      None);
        text.draw( &mut fonts,
                  &mut cache,
                  Color::rgb( 255, 255, 255),
                  |px, py, _, _, color| {
                      let px = px + 2;
                      let py = py + 2;
                      if px >= 0 && py >= 0 && px < w as i32 && py < h as i32 {
                          let pixel = ( y + py as u32) * width + x + px as u32;
                          pixels[pixel] = pixels[pixel].max( color.a());
                      }
                  });
        let origin = node.Origin();
        // XY label plane, positive-X baseline, on the front surface of its own cuboid.
        let corner = [origin[0] + 2.0,
                      origin[1] + sizes[index][1] + 6.0,
                      origin[2] + node.Size()[2] + 0.01];
        let corners = [[0.0, 0.0],
                       [1.0, 0.0],
                       [0.0, 1.0],
                       [0.0, 1.0],
                       [1.0, 0.0],
                       [1.0, 1.0]];
        USeg::FromLen( 6).Traverse( |i| {
                            let p = corners[i as usize];
                            vertices.Push( LabelVertex::New( [corner[0] + p[0] * w as f32,
                                                            corner[1] - p[1] * h as f32,
                                                            corner[2]],
                                                           [( x as f32 + p[0] * w as f32)
                                                            / width as f32,
                                                            ( y as f32 + p[1] * h as f32)
                                                            / height as f32]));
                        });
        levels[node.Depth() - 1] = vertices.Size();
        index += 1;
    }
    drop( fonts);
    let labels = GeometryLabels::New( vertices.ExtractBuff(), pixels, [width, height], levels)?;
    Mesh( &scene, cancelled)?.WithLabels( labels)
}

pub fn Mesh( scene: &CaskScene, cancelled: &AtomicBool) -> Result<GeometryAsset, String>
{
    let mut minimum = f32::INFINITY;
    scene.Nodes().Traverse( |node| {
                     let size = node.Size();
                     minimum = minimum.min( size[0]).min( size[1]).min( size[2]);
                 });
    if !minimum.is_finite() || minimum <= 0.0 {
        return Err( "Invalid cask dimensions.".into());
    }
    let interval = minimum / 3.0;
    let mut vertices = Stash::New();
    let mut triangles = Stash::New();
    let mut edges = Stash::New();
    let mut levels = Buff::FromDispenser( scene.MaxDepth(), |_| [0; 3]);
    let mut index = 0;
    while index < scene.Nodes().Size() {
        if cancelled.load( Ordering::Acquire) {
            return Err( "Loading cancelled.".into());
        }
        let node = &scene.Nodes()[index];
        let size = node.Size();
        let steps: [u32; 3] =
            std::array::from_fn( |axis| ( size[axis] / interval).ceil().max( 3.0) as u32);
        if steps.iter().any( |n| *n > 4_000_000) {
            return Err( "Cask sampling exceeds the mesh budget. Open a smaller root.".into());
        }
        let [nx, ny, nz] = steps.map( u64::from);
        let count = 2 * ( nx + 1) * ( ny + 1) + ( nz - 1) * 2 * ( nx + ny);
        let faceCount = 4 * ( nx * ny + nx * nz + ny * nz);
        if count + u64::from( vertices.Size()) > 4_000_000
           || faceCount + u64::from( triangles.Size()) > 8_000_000
        {
            return Err( "Cask sampling exceeds the mesh budget. Open a smaller root; sampling was not reduced.".into());
        }
        let [nx, ny, nz] = steps;
        let base = vertices.Size();
        let plane = ( nx + 1) * ( ny + 1);
        let perimeter = 2 * ( nx + ny);
        let color = Pastel( index);
        USeg::FromLen( count as u32).Traverse( |sample| {
                                       let grid = if sample < 2 * plane {
                                           [sample % plane % ( nx + 1),
                                            sample % plane / ( nx + 1),
                                            if sample < plane { 0 } else { nz }]
                                       } else {
                                           let edge = ( sample - 2 * plane) % perimeter;
                                           let z = ( sample - 2 * plane) / perimeter + 1;
                                           if edge <= nx {
                                               [edge, 0, z]
                                           } else if edge <= nx + ny {
                                               [nx, edge - nx, z]
                                           } else if edge <= 2 * nx + ny {
                                               [2 * nx + ny - edge, ny, z]
                                           } else {
                                               [0, perimeter - edge, z]
                                           }
                                       };
                                       let origin = node.Origin();
                                       let position = std::array::from_fn( |axis| {
                                           origin[axis]
                                           + size[axis] * grid[axis] as f32 / steps[axis] as f32
                                       });
                                       vertices.Push( GeometryVertex::New( position, color));
                                   });
        let vertex = |p: [u32; 3]| {
            let [x, y, z] = p;
            if z == 0 || z == nz {
                base + if z == 0 { 0 } else { plane } + y * ( nx + 1) + x
            } else {
                let edge = if y == 0 {
                    x
                } else if x == nx {
                    nx + y
                } else if y == ny {
                    2 * nx + ny - x
                } else {
                    perimeter - y
                };
                base + 2 * plane + ( z - 1) * perimeter + edge
            }
        };
        USeg::FromLen( 3).Traverse( |axis| {
                            let a = axis as usize;
                            let u = ( a + 1) % 3;
                            let v = ( a + 2) % 3;
                            USeg::FromLen( 2).Traverse( |side| {
                                                USeg::FromLen( steps[u]).Traverse( |x| {
                                                    USeg::FromLen( steps[v]).Traverse( |y| {
                                                        let mut p = [0; 3];
                                                        p[a] = side * steps[a];
                                                        p[u] = x;
                                                        p[v] = y;
                                                        let i0 = vertex( p);
                                                        p[u] += 1;
                                                        let i1 = vertex( p);
                                                        p[v] += 1;
                                                        let i2 = vertex( p);
                                                        p[u] -= 1;
                                                        let i3 = vertex( p);
                                                        if side == 1 {
                                                            triangles.Push( [i0, i1, i2]);
                                                            triangles.Push( [i0, i2, i3]);
                                                        } else {
                                                            triangles.Push( [i0, i2, i1]);
                                                            triangles.Push( [i0, i3, i2]);
                                                        }
                                                    });
                                                });
                                            });
                        });
        // Wire mode uses subdivided box boundaries; surface triangulation stays visually quiet.
        USeg::FromLen( 3).Traverse( |axis| {
                            let a = axis as usize;
                            USeg::FromLen( 4).Traverse( |corner| {
                                                let mut p = [0; 3];
                                                p[( a + 1) % 3] = ( corner & 1) * steps[( a + 1) % 3];
                                                p[( a + 2) % 3] = ( corner >> 1) * steps[( a + 2) % 3];
                                                USeg::FromLen( steps[a]).Traverse( |segment| {
                                                                           p[a] = segment;
                                                                           let first = vertex( p);
                                                                           p[a] += 1;
                                                                           edges.Push( [first,
                                                                                       vertex( p)]);
                                                                       });
                                            });
                        });
        levels[node.Depth() - 1] = [vertices.Size(), triangles.Size(), edges.Size()];
        index += 1;
    }
    GeometryAsset::FromMesh( vertices.ExtractBuff(),
                            triangles.ExtractBuff(),
                            edges.ExtractBuff(),
                            levels,
                            scene.Bounds())
}

fn Pastel( index: u32) -> [f32; 4]
{
    // Seeded permutation of a pastel cycle: stable during interaction and refresh.
    const COLORS: [[f32; 4]; 8] = [[0.96, 0.70, 0.74, 1.0],
                                   [0.71, 0.82, 0.98, 1.0],
                                   [0.68, 0.90, 0.76, 1.0],
                                   [0.98, 0.84, 0.62, 1.0],
                                   [0.83, 0.74, 0.96, 1.0],
                                   [0.67, 0.89, 0.89, 1.0],
                                   [0.95, 0.75, 0.92, 1.0],
                                   [0.88, 0.91, 0.68, 1.0]];
    const ORDER: [usize; 8] = [3, 0, 5, 7, 2, 6, 1, 4];
    COLORS[ORDER[( index % 8) as usize]]
}

//-------------------------------------------------------------------------------------------------
