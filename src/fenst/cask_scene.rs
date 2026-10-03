// cask_scene.rs -----------------------------------------------------------------------------------
//! Flat, breadth-first hierarchy and parent-contained 3D layout, independent of graphics APIs.

use crate::fenst::cask::{Cask, CaskKind};
use crate::silo::{Arr, Buff, IArr, Stash, USeg};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_HIERARCHY_NODES: u32 = 100_000;

pub(crate) fn    CheckCancelled( cancelled: &AtomicBool) -> Result<(), String>
{
    if cancelled.load( Ordering::Acquire)
    {
        return Err( "Loading cancelled.".into());
    }
    Ok( ())
}

//-------------------------------------------------------------------------------------------------

/// A borrowed hierarchy source. Geometry owns labels and volumes, not the source tree.
/// Children must form a finite tree and are visited in presentation order.
pub trait ICaskHierarchy
{
    type Node: Copy;

    fn  Label( &self, node: Self::Node) -> String;

    /// Stops visiting children as soon as the visitor returns false.
    fn  SpanChildren( &self, node: Self::Node, visit: impl FnMut( Self::Node) -> bool);
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

    fn  SpanChildren( &self, node: Self::Node, mut visit: impl FnMut( Self::Node) -> bool)
    {
        let children: Arr<'a, Cask>     = node.Children().into();
        children.USeg().Span( |index| {
            let child   = &node.Children()[index as usize];
            if matches!( child.Kind(), CaskKind::Window)
            {
                return visit( child);
            }
            true
        });
    }
}

//-------------------------------------------------------------------------------------------------

/// A 3D viewing box specifying spatial bounds, depth range, and view-to-world scale.
#[derive( Debug, Clone, Copy, PartialEq)]
pub struct ViewBox
{
    _ViewSize:                  [f32; 2],
    _WorldOffset:               [f32; 3],
    _ScaleViewToWorld:          f32,
    _Depth:                     f32,
    _MinPixelThreshold:         f32,
    _GpuMemoryThresholdBytes:   u64,
}

impl Default for ViewBox
{
    fn default() -> Self
    {
        return Self::New( [800.0, 600.0], [0.0; 3], 1.0);
    }
}

impl ViewBox
{
    pub const DEFAULT_MIN_PIXELS: f32 = 3.0;
    pub const DEPTH_ORDER_FACTOR: f32 = 100.0;
    pub const DEFAULT_GPU_MEMORY_THRESHOLD_BYTES: u64 = 32 * 1024 * 1024; // 32 MiB
    pub const GPU_BYTES_PER_NODE: u64 = 1832; // 8v*44B + 12t*12B + 12e*8B + 26p*44B + 4lv*24B

    /// Constructs a viewbox where depth is a couple orders of magnitude more than the max view dimension.
    /// `scale_view_to_world` is the scale from view coordinates (pixels) to world coordinates.
    pub fn  New( view_size: [f32; 2], world_offset: [f32; 3], scale_view_to_world: f32) -> Self
    {
        let width = if view_size[0].is_finite() && view_size[0] > 0.0 { view_size[0] } else { 1.0 };
        let height = if view_size[1].is_finite() && view_size[1] > 0.0 { view_size[1] } else { 1.0 };
        let maxDim = width.max( height);
        let depth = Self::DEPTH_ORDER_FACTOR * maxDim;
        let v2w = if scale_view_to_world.is_finite() && scale_view_to_world > 0.0 {
            scale_view_to_world
        } else {
            1.0
        };
        return Self {
            _ViewSize:                  [width, height],
            _WorldOffset:               world_offset,
            _ScaleViewToWorld:          v2w,
            _Depth:                     depth,
            _MinPixelThreshold:         Self::DEFAULT_MIN_PIXELS,
            _GpuMemoryThresholdBytes:   Self::DEFAULT_GPU_MEMORY_THRESHOLD_BYTES,
        };
    }

    /// Constructs a viewbox fitting given world bounds with the specified viewport dimensions.
    pub fn  FromWorld( view_size: [f32; 2], world_bounds: ( [f32; 3], [f32; 3])) -> Self
    {
        let center = [
            ( world_bounds.0[0] + world_bounds.1[0]) * 0.5,
            ( world_bounds.0[1] + world_bounds.1[1]) * 0.5,
            ( world_bounds.0[2] + world_bounds.1[2]) * 0.5,
        ];
        let extentX = ( world_bounds.1[0] - world_bounds.0[0]).max( 1.0);
        let extentY = ( world_bounds.1[1] - world_bounds.0[1]).max( 1.0);
        let vWidth = if view_size[0].is_finite() && view_size[0] > 0.0 { view_size[0] } else { 1.0 };
        let vHeight = if view_size[1].is_finite() && view_size[1] > 0.0 { view_size[1] } else { 1.0 };
        let scale_v2w = ( extentX / vWidth).max( extentY / vHeight);
        return Self::New( [vWidth, vHeight], center, scale_v2w);
    }

    pub fn  WithMinPixelThreshold( mut self, threshold: f32) -> Self
    {
        if threshold.is_finite() && threshold >= 0.0
        {
            self._MinPixelThreshold = threshold;
        }
        return self;
    }

    pub fn  WithDepth( mut self, depth: f32) -> Self
    {
        if depth.is_finite() && depth > 0.0
        {
            self._Depth = depth;
        }
        return self;
    }

    pub fn  WithDepthScale( mut self, factor: f32) -> Self
    {
        if factor.is_finite() && factor > 0.0
        {
            let maxDim = self._ViewSize[0].max( self._ViewSize[1]);
            self._Depth = factor * maxDim;
        }
        return self;
    }

    pub fn  WithGpuMemoryThreshold( mut self, bytes: u64) -> Self
    {
        if bytes > 0
        {
            self._GpuMemoryThresholdBytes = bytes;
        }
        return self;
    }

    pub fn  ViewSize( &self) -> [f32; 2] { return self._ViewSize; }
    pub fn  WorldOffset( &self) -> [f32; 3] { return self._WorldOffset; }
    pub fn  ScaleViewToWorld( &self) -> f32 { return self._ScaleViewToWorld; }
    pub fn  ScaleWorldToView( &self) -> f32 { return 1.0 / self._ScaleViewToWorld; }
    pub fn  Depth( &self) -> f32 { return self._Depth; }
    pub fn  MaxViewDimension( &self) -> f32 { return self._ViewSize[0].max( self._ViewSize[1]); }
    pub fn  MinPixelThreshold( &self) -> f32 { return self._MinPixelThreshold; }
    pub fn  GpuMemoryThreshold( &self) -> u64 { return self._GpuMemoryThresholdBytes; }

    /// Returns the maximum number of nodes permitted under the active GPU memory threshold.
    pub fn  MaxNodesForGpuBudget( &self) -> u32
    {
        let max_nodes = self._GpuMemoryThresholdBytes / Self::GPU_BYTES_PER_NODE;
        return max_nodes.min( MAX_HIERARCHY_NODES as u64 ) as u32;
    }

    /// Returns the viewbox bounds in view coordinates: ([0, 0, -depth/2], [width, height, depth/2]).
    pub fn  ViewBounds( &self) -> ( [f32; 3], [f32; 3])
    {
        return (
            [0.0, 0.0, -self._Depth * 0.5],
            [self._ViewSize[0], self._ViewSize[1], self._Depth * 0.5],
        );
    }

    /// Transforms an object's world box into view coordinates by applying the scale.
    pub fn  ScaledWorldBox( &self, world_origin: [f32; 3], world_size: [f32; 3]) -> ( [f32; 3], [f32; 3])
    {
        let scale = self.ScaleWorldToView();
        let scaled_size = [world_size[0] * scale, world_size[1] * scale, world_size[2] * scale];
        let scaled_origin = [
            ( world_origin[0] - self._WorldOffset[0]) * scale + self._ViewSize[0] * 0.5,
            ( world_origin[1] - self._WorldOffset[1]) * scale + self._ViewSize[1] * 0.5,
            ( world_origin[2] - self._WorldOffset[2]) * scale,
        ];
        return ( scaled_origin, scaled_size);
    }

    /// Computes the projected size of an entity in view pixels by applying scale to its world size.
    pub fn  ResolutionPixels( &self, world_size: [f32; 3]) -> f32
    {
        if !world_size.iter().all( |s| s.is_finite() && *s >= 0.0)
        {
            return 0.0;
        }
        let extent = world_size[0].max( world_size[1]).max( world_size[2]);
        return extent * self.ScaleWorldToView();
    }

    /// Tests whether the object's scaled world box overlaps this viewbox volume.
    pub fn  ContainsOrIntersects( &self, world_origin: [f32; 3], world_size: [f32; 3]) -> bool
    {
        let ( scaled_origin, scaled_size) = self.ScaledWorldBox( world_origin, world_size);
        let view_min = [0.0_f32, 0.0_f32, -self._Depth * 0.5];
        let view_max = [self._ViewSize[0], self._ViewSize[1], self._Depth * 0.5];

        let mut intersects = true;
        USeg::FromLen( 3).Span( |axis| {
            let i = axis as usize;
            let obj_min = scaled_origin[i];
            let obj_max = scaled_origin[i] + scaled_size[i];
            if !obj_min.is_finite() || !obj_max.is_finite()
               || obj_max < view_min[i] || obj_min > view_max[i]
            {
                intersects = false;
                return false;
            }
            return true;
        });
        return intersects;
    }

    /// Determines whether a geometric entity deserves further unfurling:
    /// Returns false if it lies outside the view box or has resolution less than the pixel threshold.
    pub fn  ShouldUnfurl( &self, world_origin: [f32; 3], world_size: [f32; 3]) -> bool
    {
        if !self.ContainsOrIntersects( world_origin, world_size)
        {
            return false;
        }
        if self.ResolutionPixels( world_size) < self._MinPixelThreshold
        {
            return false;
        }
        return true;
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
    pub fn Children( &self) -> USeg { self._Children }
    pub fn DeservesUnfurling( &self, viewbox: &ViewBox) -> bool
    {
        return viewbox.ShouldUnfurl( self._Origin, self._Size);
    }
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
        CheckCancelled( cancelled)?;
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
                    CheckCancelled( cancelled)?;
                    if u64::from( nodes.Size()) + u64::from( entries.Size())
                       >= u64::from( MAX_HIERARCHY_NODES)
                    {
                        return Err( "Hierarchy exceeds 100,000 entries. Open a smaller root; no partial tree was loaded.".into());
                    }
                    entries.Push( entry.map_err( |error| error.to_string())?.path());
                    Ok( ())
                })?;
                entries.MutArr().QSort( |a, b| a < b);
                CheckCancelled( cancelled)?;
                let first = nodes.Size();
                let depth = nodes[index]._Depth + 1;
                entries.Arr().USeg().Span( |entryIndex| {
                                 if cancelled.load( Ordering::Acquire)
                                 {
                                     return false;
                                 }
                                 let entry = &entries[entryIndex];
                                 nodes.Push( CaskVolume::New( entry.file_name()
                                                                 .unwrap_or_default()
                                                                 .to_string_lossy()
                                                                 .into_owned(),
                                                            index,
                                                            depth));
                                 paths.Push( entry.clone());
                                 true
                             });
                CheckCancelled( cancelled)?;
                nodes[index]._Children = USeg::WithLen( first, entries.Size());
            }
            index += 1;
        }
        let mut buff = nodes.ExtractBuff();
        Self::CalculateHeights( &mut buff, cancelled)?;
        Ok( Self { _Nodes: buff, })
    }

    /// Adapts existing in-memory casks without deriving geometry from their 2D bounds.
    pub fn FromRoot( root: &Cask) -> Result<Self, String>
    {
        return Self::FromHierarchy( &root, root, &AtomicBool::new( false));
    }

    /// Builds contiguous sibling ranges directly from a borrowed domain tree, without nested Casks.
    pub fn  FromHierarchy<H: ICaskHierarchy>( source: &H, root: H::Node, cancelled: &AtomicBool)
                                          -> Result<Self, String>
    {
        CheckCancelled( cancelled)?;
        let mut sources = Stash::New();
        let mut nodes = Stash::New();
        sources.Push( root);
        nodes.Push( CaskVolume::New( source.Label( root), u32::MAX, 1));
        let mut index = 0;
        while index < sources.Size() {
            CheckCancelled( cancelled)?;
            let first = nodes.Size();
            let depth = nodes[index]._Depth + 1;
            let mut exceeded = false;
            source.SpanChildren( sources[index], |child| {
                if cancelled.load( Ordering::Acquire)
                {
                    return false;
                }
                if nodes.Size() >= MAX_HIERARCHY_NODES
                {
                    exceeded = true;
                    return false;
                }
                nodes.Push( CaskVolume::New( source.Label( child), index, depth));
                sources.Push( child);
                !cancelled.load( Ordering::Acquire)
            });
            CheckCancelled( cancelled)?;
            if exceeded
            {
                return Err( "Hierarchy exceeds 100,000 entries. Open a smaller root; no partial tree was loaded.".into());
            }
            nodes[index]._Children = USeg::WithLen( first, nodes.Size() - first);
            index += 1;
        }
        let mut buff = nodes.ExtractBuff();
        Self::CalculateHeights( &mut buff, cancelled)?;
        Ok( Self { _Nodes: buff, })
    }

    fn CalculateHeights( nodes: &mut Buff<CaskVolume>, cancelled: &AtomicBool) -> Result<(), String>
    {
        nodes.Arr().USeg().Span( |step| {
            if cancelled.load( Ordering::Acquire)
            {
                return false;
            }
            let index = nodes.Size() - step - 1;
            let children = nodes[index]._Children;
            if !children.IsEmpty() {
                let mut maxChild = 0;
                children.Span( |child| {
                    if cancelled.load( Ordering::Acquire)
                    {
                        return false;
                    }
                    maxChild = maxChild.max( nodes[child]._Height);
                    true
                });
                nodes[index]._Height = maxChild + 1;
            }
            true
        });
        CheckCancelled( cancelled)
    }

    pub fn Nodes( &self) -> Arr<'_, CaskVolume> { self._Nodes.Arr() }
    pub fn MaxDepth( &self) -> u32 { self._Nodes[self._Nodes.Size() - 1]._Depth }
    pub fn Bounds( &self) -> ( [f32; 3], [f32; 3]) { ( [0.0; 3], self._Nodes[0]._Size) }

    /// Checks whether the entity at `index` deserves further child unfurling.
    pub fn  DeservesUnfurling( &self, index: u32, viewbox: &ViewBox) -> bool
    {
        if index >= self._Nodes.Size()
        {
            return false;
        }
        let node = &self._Nodes[index];
        return node.DeservesUnfurling( viewbox);
    }

    /// Prunes children of entities that do not deserve further unfurling (outside viewbox or sub-pixel).
    pub fn  Unfurl( &self, viewbox: &ViewBox, cancelled: &AtomicBool) -> Result<Self, String>
    {
        CheckCancelled( cancelled)?;
        if self._Nodes.IsEmpty()
        {
            return Ok( Self { _Nodes: Buff::New() });
        }
        if self._Nodes[0]._Size.iter().any( |s| *s <= 0.0)
        {
            return Err( "Cannot unfurl a CaskScene before layout.".into());
        }

        let mut sources = Stash::New();
        let mut nodes = Stash::New();

        sources.Push( 0_u32);
        let root = &self._Nodes[0];
        nodes.Push( CaskVolume {
            _Name:     root._Name.clone(),
            _Parent:   u32::MAX,
            _Children: USeg::Empty(),
            _Depth:    1,
            _Height:   0,
            _Origin:   root._Origin,
            _Size:     root._Size,
        });

        let gpu_node_limit = viewbox.MaxNodesForGpuBudget();
        let mut index = 0_u32;
        let mut cur_depth = 1_u32;
        let mut cur_depth_unfurled = false;
        let mut depth_cutoff_reached = false;

        while index < sources.Size() {
            CheckCancelled( cancelled)?;
            let origIdx = sources[index];
            let origNode = &self._Nodes[origIdx];
            let node_depth = nodes[index]._Depth;

            if node_depth != cur_depth
            {
                // If the previous depth level had nodes, but NONE of them deserved unfurling,
                // that depth is uniformly heuristically assumed to hold for all deeper levels.
                if !cur_depth_unfurled
                {
                    depth_cutoff_reached = true;
                }
                cur_depth = node_depth;
                cur_depth_unfurled = false;
            }

            if !depth_cutoff_reached
               && origNode.DeservesUnfurling( viewbox)
               && !origNode._Children.IsEmpty()
            {
                // Account for GPU memory threshold: if adding these children would exceed
                // the GPU budget, stop unfurling this branch so performance remains manageable.
                if nodes.Size() + origNode._Children.Len() <= gpu_node_limit
                {
                    cur_depth_unfurled = true;
                    let first = nodes.Size();
                    let depth = node_depth + 1;
                    let mut exceeded = false;

                    origNode._Children.Span( |childOrigIdx| {
                        if cancelled.load( Ordering::Acquire)
                        {
                            return false;
                        }
                        if nodes.Size() >= MAX_HIERARCHY_NODES
                        {
                            exceeded = true;
                            return false;
                        }
                        let childNode = &self._Nodes[childOrigIdx];
                        nodes.Push( CaskVolume {
                            _Name:     childNode._Name.clone(),
                            _Parent:   index,
                            _Children: USeg::Empty(),
                            _Depth:    depth,
                            _Height:   0,
                            _Origin:   childNode._Origin,
                            _Size:     childNode._Size,
                        });
                        sources.Push( childOrigIdx);
                        return true;
                    });

                    CheckCancelled( cancelled)?;
                    if exceeded
                    {
                        return Err( "Hierarchy exceeds 100,000 entries. Open a smaller root; no partial tree was loaded.".into());
                    }
                    nodes[index]._Children = USeg::WithLen( first, nodes.Size() - first);
                }
            }
            index += 1;
        }

        let mut buff = nodes.ExtractBuff();
        Self::CalculateHeights( &mut buff, cancelled)?;
        return Ok( Self { _Nodes: buff });
    }

    /// Text baselines follow X. Leaves have equal Y/Z extents. Parents pack in Y and Z.
    pub fn Layout( &mut self, cancelled: &AtomicBool, mut measure: impl FnMut( &str) -> [f32; 2])
                 -> Result<(), String>
    {
        const PAD: f32 = 4.0;
        const GAP: f32 = 4.0;
        self._Nodes.Arr().USeg().Span( |index| {
                                if cancelled.load( Ordering::Acquire)
                                {
                                    return false;
                                }
                                let node = &mut self._Nodes[index];
                                let text = measure( &node._Name);
                                let height = text[1].max( 1.0) + 2.0 * PAD;
                                node._Size = [text[0].max( 1.0) + 2.0 * PAD, height, height];
                                node._Origin = [0.0; 3];
                                true
                            });
        CheckCancelled( cancelled)?;
        self._Nodes.Arr().USeg().Span( |step| {
                                    if cancelled.load( Ordering::Acquire)
                                    {
                                        return false;
                                    }
                                    let index = self._Nodes.Size() - step - 1;
                                    let children = self._Nodes[index]._Children;
                                    if children.IsEmpty() {
                                        return true;
                                    }
                                    let title = self._Nodes[index]._Size;
                                    let mut width = title[0];
                                    let mut total = 0.0;
                                    let mut maxHeight = 0.0_f32;
                                    children.Span( |child| {
                                                if cancelled.load( Ordering::Acquire)
                                                {
                                                    return false;
                                                }
                                                let size = self._Nodes[child]._Size;
                                                width = width.max( size[0] + 2.0 * PAD);
                                                total += size[1] + GAP;
                                                maxHeight = maxHeight.max( size[1]);
                                                true
                                            });
                                    // Bounded candidate search: balanced footprints without quadratic packing work.
                                    let mut best = ( f32::INFINITY, maxHeight, [0.0; 3]);
                                    USeg::FromLen( 17).Span( |candidate| {
                                                         if cancelled.load( Ordering::Acquire)
                                                         {
                                                             return false;
                                                         }
                                                         let limit = maxHeight
                                                                     + ( total - maxHeight)
                                                                       * candidate as f32
                                                                       / 16.0;
                                                         let mut y = 0.0_f32;
                                                         let mut z = PAD;
                                                         let mut layerDepth = 0.0_f32;
                                                         let mut height = 0.0_f32;
                                                         children.Span( |child| {
                                                                     if cancelled.load( Ordering::Acquire)
                                                                     {
                                                                         return false;
                                                                     }
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
                                                                     true
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
                                                         true
                                                     });
                                    let mut y = 0.0;
                                    let mut z = PAD;
                                    let mut layerDepth = 0.0_f32;
                                    children.Span( |child| {
                                                if cancelled.load( Ordering::Acquire)
                                                {
                                                    return false;
                                                }
                                                let size = self._Nodes[child]._Size;
                                                if y > 0.0 && y + size[1] > best.1 {
                                                    z += layerDepth + GAP;
                                                    layerDepth = 0.0;
                                                    y = 0.0;
                                                }
                                                self._Nodes[child]._Origin = [PAD, title[1] + y, z];
                                                y += size[1] + GAP;
                                                layerDepth = layerDepth.max( size[2]);
                                                true
                                            });
                                    self._Nodes[index]._Size = best.2;
                                    true
                                });
        CheckCancelled( cancelled)?;
        self._Nodes.Arr().USeg().Span( |index| {
                                    if cancelled.load( Ordering::Acquire)
                                    {
                                        return false;
                                    }
                                    let parent = self._Nodes[index]._Parent;
                                    if parent != u32::MAX {
                                        let offset = self._Nodes[parent]._Origin;
                                        let origin = self._Nodes[index]._Origin;
                                        self._Nodes[index]._Origin =
                                            std::array::from_fn( |axis| origin[axis] + offset[axis]);
                                    }
                                    true
                                });
        CheckCancelled( cancelled)
    }
}

//-------------------------------------------------------------------------------------------------
