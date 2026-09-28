// cask_scene.rs -----------------------------------------------------------------------------------
//! Flat, breadth-first hierarchy and parent-contained 3D layout, independent of graphics APIs.

use crate::fenst::cask::{Cask, CaskKind};
use crate::silo::{Arr, Buff, IArr, IArrMut, Stash, USeg};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

//-------------------------------------------------------------------------------------------------

/// A borrowed hierarchy source. Geometry owns labels and volumes, not the source tree.
/// Children must form a finite tree and are visited in presentation order.
pub trait ICaskHierarchy
{
    type Node: Copy;

    fn  Label( &self, node: Self::Node) -> String;
    fn  TraverseChildren( &self, node: Self::Node, visit: impl FnMut( Self::Node));
}

impl<'a> ICaskHierarchy for &'a Cask
{
    type Node = &'a Cask;

    fn  Label( &self, node: Self::Node) -> String
    {
        if let  CaskKind::Label( text) = node.Kind()
        {
            return text.clone();
        }
        let mut label   = String::new();
        let mut hasLabel    = false;
        let children: Arr<'_, Cask>     = node.Children().into();
        children.Traverse( |child| {
            if let  CaskKind::Label( text) = child.Kind()
            {
                if hasLabel
                {
                    label.push( '\n');
                }
                label.push_str( text);
                hasLabel = true;
            }
        });
        return if hasLabel { label } else { node.Id().into() };
    }

    fn  TraverseChildren( &self, node: Self::Node, mut visit: impl FnMut( Self::Node))
    {
        let children: Arr<'a, Cask>     = node.Children().into();
        children.USeg().Traverse( |index| {
            let child   = &node.Children()[index as usize];
            if matches!( child.Kind(), CaskKind::Window)
            {
                visit( child);
            }
        });
    }
}

//-------------------------------------------------------------------------------------------------

#[derive( Debug)]
pub struct CaskVolume
{
    _Name:     String,
    _Parent:   u32,
    _Children: USeg,
    _Depth:    u32,
    _Height:   u32,
    _Origin:   [f32; 3],
    _Size:     [f32; 3],
}

impl CaskVolume
{
    fn New( name: String, parent: u32, depth: u32) -> Self
    {
        Self { _Name:     name,
               _Parent:   parent,
               _Children: USeg::Empty(),
               _Depth:    depth,
               _Height:   0,
               _Origin:   [0.0; 3],
               _Size:     [0.0; 3], }
    }
    pub fn Name( &self) -> &str { &self._Name }
    pub fn Parent( &self) -> u32 { self._Parent }
    pub fn Depth( &self) -> u32 { self._Depth }
    pub fn Height( &self) -> u32 { self._Height }
    pub fn Origin( &self) -> [f32; 3] { self._Origin }
    pub fn Size( &self) -> [f32; 3] { self._Size }
    pub fn IsLeaf( &self) -> bool { self._Children.IsEmpty() }
}

#[derive( Debug)]
pub struct CaskScene
{
    _Nodes: Buff<CaskVolume>,
}

impl CaskScene
{
    /// Enumerates every accessible entry. Symlinks are leaves. An incomplete scan is an error.
    pub fn Read( path: &Path, cancelled: &AtomicBool) -> Result<Self, String>
    {
        let mut paths = Stash::New();
        let mut nodes = Stash::New();
        paths.Push( path.to_path_buf());
        nodes.Push( CaskVolume::New( path.file_name()
                                       .unwrap_or( path.as_os_str())
                                       .to_string_lossy()
                                       .into_owned(),
                                   u32::MAX,
                                   1));
        let mut index = 0;
        while index < nodes.Size() {
            if cancelled.load( Ordering::Acquire) {
                return Err( "Loading cancelled.".into());
            }
            let current = &paths[index];
            let metadata = std::fs::symlink_metadata( current).map_err( |error| {
                               format!( "Cannot inspect {}: {error}", current.display())
                           })?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                let mut entries = Stash::New();
                let mut reader = std::fs::read_dir( current).map_err( |error| {
                                                               format!( "Cannot read {}: {error}",
                                                                       current.display())
                                                           })?;
                reader.try_for_each( |entry| -> Result<(), String> {
                    if cancelled.load( Ordering::Acquire) { return Err( "Loading cancelled.".into()); }
                    entries.Push( entry.map_err( |error| error.to_string())?.path());
                    if u64::from( nodes.Size()) + u64::from( entries.Size()) > 100_000
                    {
                        return Err( "Hierarchy exceeds 100,000 entries. Open a smaller root; no partial tree was loaded.".into());
                    }
                    Ok( ())
                })?;
                entries.MutArr().QSort( |a, b| a < b);
                let first = nodes.Size();
                let depth = nodes[index]._Depth + 1;
                entries.Arr().Traverse( |entry| {
                                 nodes.Push( CaskVolume::New( entry.file_name()
                                                                 .unwrap_or_default()
                                                                 .to_string_lossy()
                                                                 .into_owned(),
                                                            index,
                                                            depth));
                                 paths.Push( entry.clone());
                             });
                nodes[index]._Children = USeg::WithLen( first, entries.Size());
            }
            index += 1;
        }
        let mut buff = nodes.ExtractBuff();
        Self::CalculateHeights( &mut buff);
        Ok( Self { _Nodes: buff, })
    }

    /// Adapts existing in-memory casks without deriving geometry from their 2D bounds.
    pub fn FromRoot( root: &Cask) -> Self
    {
        return Self::FromHierarchy( &root, root);
    }

    /// Builds contiguous sibling ranges directly from a borrowed domain tree, without nested Casks.
    pub fn  FromHierarchy<H: ICaskHierarchy>( source: &H, root: H::Node) -> Self
    {
        let mut sources = Stash::New();
        let mut nodes = Stash::New();
        sources.Push( root);
        nodes.Push( CaskVolume::New( source.Label( root), u32::MAX, 1));
        let mut index = 0;
        while index < sources.Size() {
            let first = nodes.Size();
            let depth = nodes[index]._Depth + 1;
            source.TraverseChildren( sources[index], |child| {
                nodes.Push( CaskVolume::New( source.Label( child), index, depth));
                sources.Push( child);
            });
            nodes[index]._Children = USeg::WithLen( first, nodes.Size() - first);
            index += 1;
        }
        let mut buff = nodes.ExtractBuff();
        Self::CalculateHeights( &mut buff);
        Self { _Nodes: buff, }
    }

    fn CalculateHeights( nodes: &mut Buff< CaskVolume>)
    {
        nodes.Arr().USeg().TraverseRev( |index| {
            let children = nodes[index]._Children;
            if !children.IsEmpty() {
                let mut maxChild = 0;
                children.Traverse( |child| {
                    maxChild = maxChild.max( nodes[child]._Height);
                });
                nodes[index]._Height = maxChild + 1;
            }
        });
    }

    pub fn Nodes( &self) -> Arr<'_, CaskVolume> { self._Nodes.Arr() }
    pub fn MaxDepth( &self) -> u32 { self._Nodes[self._Nodes.Size() - 1]._Depth }
    pub fn Bounds( &self) -> ( [f32; 3], [f32; 3]) { ( [0.0; 3], self._Nodes[0]._Size) }

    /// Text baselines follow X. Leaves have equal Y/Z extents. Parents pack in Y and Z.
    pub fn Layout( &mut self, mut measure: impl FnMut( &str) -> [f32; 2])
    {
        const PAD: f32 = 4.0;
        const GAP: f32 = 4.0;
        self._Nodes.MutArr().TraverseMut( |node| {
                                let text = measure( &node._Name);
                                let height = text[1].max( 1.0) + 2.0 * PAD;
                                node._Size = [text[0].max( 1.0) + 2.0 * PAD, height, height];
                                node._Origin = [0.0; 3];
                            });
        self._Nodes.Arr().USeg().TraverseRev( |index| {
                                    let children = self._Nodes[index]._Children;
                                    if children.IsEmpty() {
                                        return;
                                    }
                                    let title = self._Nodes[index]._Size;
                                    let mut width = title[0];
                                    let mut total = 0.0;
                                    let mut maxHeight = 0.0_f32;
                                    children.Traverse( |child| {
                                                let size = self._Nodes[child]._Size;
                                                width = width.max( size[0] + 2.0 * PAD);
                                                total += size[1] + GAP;
                                                maxHeight = maxHeight.max( size[1]);
                                            });
                                    // Bounded candidate search: balanced footprints without quadratic packing work.
                                    let mut best = ( f32::INFINITY, maxHeight, [0.0; 3]);
                                    USeg::FromLen( 17).Traverse( |candidate| {
                                                         let limit = maxHeight
                                                                     + ( total - maxHeight)
                                                                       * candidate as f32
                                                                       / 16.0;
                                                         let mut y = 0.0_f32;
                                                         let mut z = PAD;
                                                         let mut layerDepth = 0.0_f32;
                                                         let mut height = 0.0_f32;
                                                         children.Traverse( |child| {
                                                                     let size =
                                                                         self._Nodes[child]._Size;
                                                                     if y > 0.0
                                                                        && y + size[1] > limit
                                                                     {
                                                                         z += layerDepth + GAP;
                                                                         layerDepth = 0.0;
                                                                         y = 0.0;
                                                                     }
                                                                     height =
                                                                         height.max( y + size[1]);
                                                                     y += size[1] + GAP;
                                                                     layerDepth =
                                                                         layerDepth.max( size[2]);
                                                                 });
                                                         let size = [width,
                                                                     title[1] + height + PAD,
                                                                     z + layerDepth + PAD];
                                                         let longest =
                                                             size[0].max( size[1]).max( size[2]);
                                                         let shortest =
                                                             size[0].min( size[1]).min( size[2]);
                                                         let score =
                                                             longest / shortest
                                                             + size[0] * size[1] * size[2] * 1e-9;
                                                         if score < best.0 {
                                                             best = ( score, limit, size);
                                                         }
                                                     });
                                    let mut y = 0.0;
                                    let mut z = PAD;
                                    let mut layerDepth = 0.0_f32;
                                    children.Traverse( |child| {
                                                let size = self._Nodes[child]._Size;
                                                if y > 0.0 && y + size[1] > best.1 {
                                                    z += layerDepth + GAP;
                                                    layerDepth = 0.0;
                                                    y = 0.0;
                                                }
                                                self._Nodes[child]._Origin = [PAD, title[1] + y, z];
                                                y += size[1] + GAP;
                                                layerDepth = layerDepth.max( size[2]);
                                            });
                                    self._Nodes[index]._Size = best.2;
                                });
        self._Nodes.Arr().USeg().Traverse( |index| {
                                    let parent = self._Nodes[index]._Parent;
                                    if parent != u32::MAX {
                                        let offset = self._Nodes[parent]._Origin;
                                        let origin = self._Nodes[index]._Origin;
                                        self._Nodes[index]._Origin =
                                            std::array::from_fn( |axis| origin[axis] + offset[axis]);
                                    }
                                });
    }
}

//-------------------------------------------------------------------------------------------------
