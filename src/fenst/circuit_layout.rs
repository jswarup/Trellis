// circuit_layout.rs ---------------------------------------------------------------------------------------------
//! Classical schematic layout engine for CircuitXplr entities: modules, perimeter pins, and Manhattan wires.
//!
//! Provides a decomposed layout pipeline:
//! 1. CircuitModel: Contiguous indexed module and port lookup using u32 indices.
//! 2. DagRanker: Cycle-aware forward DFS and longest-path topological column ranking.
//! 3. ModulePlacer: Solves module bounding rects, pin points, and pin bounding boxes.
//! 4. WireRouter: Classical Manhattan orthogonal wire routing, channel allocation, highway drops, and multi-sink junctions.
//! 5. SchematicEmitter: Generates render commands, Cask2dNode hierarchy, interactive Cask2dPinNodes, and Cask2dWire nets.

use crate::fenst::cask::{
    Cask, CaskRenderCommand, Color, LayoutDirection, Point, Rect,
};
use crate::fenst::cask2d::{Cask2dLayout, Cask2dNode, Cask2dOptions};
pub use crate::fenst::xplr::PortSide;
use crate::fenst::xplr::{CircuitXplr, PortDescriptor, PortDirection, WireDescriptor};
use crate::silo::USeg;

//-------------------------------------------------------------------------------------------------
// Layout Constants & Geometry Metrics

pub const MODULE_WIDTH: f32    = 160.0;
pub const HEADER_HEIGHT: f32   = 36.0;
pub const PIN_PITCH: f32       = 24.0;
pub const PIN_STUB_LEN: f32    = 12.0;
pub const CHANNEL_WIDTH: f32   = 72.0;
pub const ROW_GAP: f32         = 32.0;
pub const FRAME_PAD_X: f32     = 70.0;
pub const FRAME_PAD_Y: f32     = 60.0;
pub const PIN_HIT_RADIUS: f32  = 8.0;
pub const INVALID_INDEX: u32   = u32::MAX;

//-------------------------------------------------------------------------------------------------
// Interactive Pin & Wire Hit-Test Data Structures

/// Interactive terminal pin positioned on a module perimeter or canvas boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct Cask2dPinNode
{
    _ModuleName: String,
    _PortName:   String,
    _Direction:  PortDirection,
    _Side:       PortSide,
    _Bounds:     Rect,
    _Point:      Point,
    _NetName:    String,
}

impl Cask2dPinNode
{
    pub fn New(
        moduleName: impl Into<String>,
        portName: impl Into<String>,
        direction: PortDirection,
        bounds: Rect,
        point: Point,
        netName: impl Into<String>,
    ) -> Self
    {
        Self::NewWithSide(moduleName, portName, direction, PortSide::West, bounds, point, netName)
    }

    pub fn NewWithSide(
        moduleName: impl Into<String>,
        portName: impl Into<String>,
        direction: PortDirection,
        side: PortSide,
        bounds: Rect,
        point: Point,
        netName: impl Into<String>,
    ) -> Self
    {
        Self {
            _ModuleName: moduleName.into(),
            _PortName:   portName.into(),
            _Direction:  direction,
            _Side:       side,
            _Bounds:     bounds,
            _Point:      point,
            _NetName:    netName.into(),
        }
    }

    pub fn ModuleName(&self) -> &str { &self._ModuleName }

    pub fn PortName(&self) -> &str { &self._PortName }

    pub fn Direction(&self) -> PortDirection { self._Direction }

    pub fn Side(&self) -> PortSide { self._Side }

    pub fn Bounds(&self) -> Rect { self._Bounds }

    pub fn Point(&self) -> Point { self._Point }

    pub fn NetName(&self) -> &str { &self._NetName }

    pub fn ContainsPoint(&self, px: f32, py: f32) -> bool
    {
        self._Bounds.Contains(px, py)
    }
}

//-------------------------------------------------------------------------------------------------

/// Interactive wire net route between a driver pin and a sink pin.
#[derive(Clone, Debug, PartialEq)]
pub struct Cask2dWire
{
    _NetName:      String,
    _DriverModule: String,
    _DriverPort:   String,
    _SinkModule:   String,
    _SinkPort:     String,
    _Segments:     Vec<(Point, Point)>,
}

impl Cask2dWire
{
    pub fn New(
        netName: impl Into<String>,
        driverModule: impl Into<String>,
        driverPort: impl Into<String>,
        sinkModule: impl Into<String>,
        sinkPort: impl Into<String>,
        segments: Vec<(Point, Point)>,
    ) -> Self
    {
        Self {
            _NetName:      netName.into(),
            _DriverModule: driverModule.into(),
            _DriverPort:   driverPort.into(),
            _SinkModule:   sinkModule.into(),
            _SinkPort:     sinkPort.into(),
            _Segments:     segments,
        }
    }

    pub fn NetName(&self) -> &str { &self._NetName }

    pub fn DriverModule(&self) -> &str { &self._DriverModule }

    pub fn DriverPort(&self) -> &str { &self._DriverPort }

    pub fn SinkModule(&self) -> &str { &self._SinkModule }

    pub fn SinkPort(&self) -> &str { &self._SinkPort }

    pub fn Segments(&self) -> &[(Point, Point)] { &self._Segments }

    pub fn ContainsPoint(&self, px: f32, py: f32, tolerance: f32) -> bool
    {
        let mut hit = false;
        USeg::FromLen(self._Segments.len() as u32).Span(|i| {
            let (p1, p2) = self._Segments[i as usize];
            let minX = p1.x.min(p2.x) - tolerance;
            let maxX = p1.x.max(p2.x) + tolerance;
            let minY = p1.y.min(p2.y) - tolerance;
            let maxY = p1.y.max(p2.y) + tolerance;
            if px >= minX && px <= maxX && py >= minY && py <= maxY {
                let dx = p2.x - p1.x;
                let dy = p2.y - p1.y;
                let lenSq = dx * dx + dy * dy;
                if lenSq < 1e-4 {
                    let d = ((px - p1.x).powi(2) + (py - p1.y).powi(2)).sqrt();
                    if d <= tolerance {
                        hit = true;
                        return false;
                    }
                } else {
                    let t = (((px - p1.x) * dx + (py - p1.y) * dy) / lenSq).clamp(0.0, 1.0);
                    let projX = p1.x + t * dx;
                    let projY = p1.y + t * dy;
                    let dist = ((px - projX).powi(2) + (py - projY).powi(2)).sqrt();
                    if dist <= tolerance {
                        hit = true;
                        return false;
                    }
                }
            }
            return true;
        });
        hit
    }
}

//-------------------------------------------------------------------------------------------------
// Circuit Indexed Model

struct ModuleRecord
{
    name:            String,
    component_type:  String,
    path:            String,
    ports:           Vec<PortDescriptor>,
    submodule_count: u32,
    rect:            Rect,
    rank:            u32,
}

struct PlacedPortRecord
{
    module_name: String,
    port_name:   String,
    direction:   PortDirection,
    side:        PortSide,
    point:       Point,
    bounds:      Rect,
    net_name:    String,
}

struct DriverSinkGroup
{
    driver:   (String, String),
    sinks:    Vec<(String, String)>,
    net_name: String,
}

//-------------------------------------------------------------------------------------------------
// 1. Module Indexing & Model Extraction

fn ExtractModuleModel(
    root: &dyn CircuitXplr,
    isHierarchical: bool,
) -> (Vec<ModuleRecord>, Vec<WireDescriptor>)
{
    let mut modules: Vec<ModuleRecord> = Vec::new();
    if isHierarchical {
        let submodules = root.Submodules();
        USeg::FromLen(submodules.len() as u32).Traverse(|i| {
            let m = &submodules[i as usize];
            modules.push(ModuleRecord {
                name:            m.Name().to_string(),
                component_type:  m.ComponentType().to_string(),
                path:            m.Path().to_string(),
                ports:           m.Ports(),
                submodule_count: m.Submodules().len() as u32,
                rect:            Rect::Zero(),
                rank:            0,
            });
        });
        let wires = root.Wires();
        (modules, wires)
    } else {
        modules.push(ModuleRecord {
            name:            root.Name().to_string(),
            component_type:  root.ComponentType().to_string(),
            path:            root.Path().to_string(),
            ports:           root.Ports(),
            submodule_count: 0,
            rect:            Rect::Zero(),
            rank:            0,
        });
        (modules, Vec::new())
    }
}

fn FindModuleIndex(modules: &[ModuleRecord], name: &str) -> u32
{
    let mut found = INVALID_INDEX;
    USeg::FromLen(modules.len() as u32).Span(|i| {
        if modules[i as usize].name == name {
            found = i;
            return false;
        }
        return true;
    });
    found
}

//-------------------------------------------------------------------------------------------------
// 2. DAG Ranker (Cycle-Aware Topological Column Ranking)

fn DfsRank(
    u: u32,
    adj: &[Vec<u32>],
    color: &mut [u8],
    forwardEdges: &mut Vec<(u32, u32)>,
)
{
    color[u as usize] = 1;
    let neighbors = &adj[u as usize];
    USeg::FromLen(neighbors.len() as u32).Traverse(|i| {
        let v = neighbors[i as usize];
        if color[v as usize] == 1 {
            return;
        }
        forwardEdges.push((u, v));
        if color[v as usize] == 0 {
            DfsRank(v, adj, color, forwardEdges);
        }
    });
    color[u as usize] = 2;
}

fn RankModules(modules: &mut [ModuleRecord], wires: &[WireDescriptor]) -> (u32, Vec<Vec<u32>>)
{
    let n = modules.len() as u32;
    if n <= 1 {
        return (0, vec![vec![0]]);
    }

    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n as usize];
    USeg::FromLen(wires.len() as u32).Traverse(|i| {
        let w = &wires[i as usize];
        if w.is_feedback {
            return;
        }
        let u = FindModuleIndex(modules, &w.driver_module);
        let v = FindModuleIndex(modules, &w.sink_module);
        if u != INVALID_INDEX && v != INVALID_INDEX && u != v {
            let mut exists = false;
            let neighbors = &adj[u as usize];
            USeg::FromLen(neighbors.len() as u32).Span(|j| {
                if neighbors[j as usize] == v {
                    exists = true;
                    return false;
                }
                return true;
            });
            if !exists {
                adj[u as usize].push(v);
            }
        }
    });

    // Resolve any remaining 2-cycles (mutual edges u <-> v) using response/ack heuristics
    USeg::FromLen(n).Traverse(|u| {
        let neighbors = adj[u as usize].clone();
        USeg::FromLen(neighbors.len() as u32).Traverse(|j| {
            let v = neighbors[j as usize];
            if v > u && adj[v as usize].contains(&u) {
                // Both u -> v and v -> u exist. Detect which one is response/feedback.
                let mut u_to_v_is_resp = false;
                let mut v_to_u_is_resp = false;
                USeg::FromLen(wires.len() as u32).Traverse(|k| {
                    let w = &wires[k as usize];
                    let d = FindModuleIndex(modules, &w.driver_module);
                    let s = FindModuleIndex(modules, &w.sink_module);
                    let is_resp = w.name.contains("Resp")
                        || w.name.contains("Ack")
                        || w.driver_port.contains("Resp")
                        || w.sink_port.contains("Resp");
                    if d == u && s == v && is_resp {
                        u_to_v_is_resp = true;
                    }
                    if d == v && s == u && is_resp {
                        v_to_u_is_resp = true;
                    }
                });
                if u_to_v_is_resp && !v_to_u_is_resp {
                    adj[u as usize].retain(|&x| x != v);
                } else if v_to_u_is_resp && !u_to_v_is_resp {
                    adj[v as usize].retain(|&x| x != u);
                }
            }
        });
    });

    let mut color = vec![0u8; n as usize];
    let mut forwardEdges: Vec<(u32, u32)> = Vec::new();
    USeg::FromLen(n).Traverse(|i| {
        if color[i as usize] == 0 {
            DfsRank(i, &adj, &mut color, &mut forwardEdges);
        }
    });

    let mut ranks = vec![0u32; n as usize];
    USeg::FromLen(n).Span(|_| {
        let mut changed = false;
        USeg::FromLen(forwardEdges.len() as u32).Traverse(|i| {
            let (u, v) = forwardEdges[i as usize];
            let uIdx = u as usize;
            let vIdx = v as usize;
            if ranks[vIdx] < ranks[uIdx] + 1 && ranks[uIdx] + 1 < n {
                ranks[vIdx] = ranks[uIdx] + 1;
                changed = true;
            }
        });
        if !changed {
            return false;
        }
        return true;
    });

    let mut maxRank: u32 = 0;
    USeg::FromLen(n).Traverse(|i| {
        let r = ranks[i as usize];
        modules[i as usize].rank = r;
        if r > maxRank {
            maxRank = r;
        }
    });

    let mut cols: Vec<Vec<u32>> = vec![Vec::new(); (maxRank + 1) as usize];
    USeg::FromLen(n).Traverse(|i| {
        let r = modules[i as usize].rank as usize;
        cols[r].push(i);
    });

    (maxRank, cols)
}

//-------------------------------------------------------------------------------------------------
// 2b. Port Orientation & Column Connectivity Sorting

fn DeterminePortSide(
    modIdx: usize,
    port: &PortDescriptor,
    modules: &[ModuleRecord],
    wires: &[WireDescriptor],
    rootName: &str,
) -> PortSide
{
    if let Some(side) = port.side {
        return side;
    }

    let m = &modules[modIdx];
    let mut westVotes: u32 = 0;
    let mut eastVotes: u32 = 0;

    USeg::FromLen(wires.len() as u32).Traverse(|i| {
        let w = &wires[i as usize];

        // 1. Port is driver
        if w.driver_module == m.name && w.driver_port == port.name {
            if w.sink_module == rootName || w.sink_module.is_empty() {
                eastVotes += 1;
            } else {
                let sIdx = FindModuleIndex(modules, &w.sink_module);
                if sIdx != INVALID_INDEX {
                    let sRank = modules[sIdx as usize].rank;
                    if sRank < m.rank {
                        westVotes += 1;
                    } else if sRank > m.rank {
                        eastVotes += 1;
                    }
                }
            }
        }

        // 2. Port is sink
        if w.sink_module == m.name && w.sink_port == port.name {
            if w.driver_module == rootName || w.driver_module.is_empty() {
                westVotes += 1;
            } else {
                let dIdx = FindModuleIndex(modules, &w.driver_module);
                if dIdx != INVALID_INDEX {
                    let dRank = modules[dIdx as usize].rank;
                    if dRank < m.rank {
                        westVotes += 1;
                    } else if dRank > m.rank {
                        eastVotes += 1;
                    }
                }
            }
        }
    });

    if westVotes > eastVotes {
        PortSide::West
    } else if eastVotes > westVotes {
        PortSide::East
    } else {
        match port.direction {
            PortDirection::In => PortSide::West,
            PortDirection::Out | PortDirection::InOut => PortSide::East,
        }
    }
}

fn DetermineRootPortSide(
    port: &PortDescriptor,
    modules: &[ModuleRecord],
    wires: &[WireDescriptor],
    rootName: &str,
    colsCount: usize,
) -> PortSide
{
    if let Some(side) = port.side {
        return side;
    }

    let mut westVotes: u32 = 0;
    let mut eastVotes: u32 = 0;
    let midCol = (colsCount as f32) / 2.0;

    USeg::FromLen(wires.len() as u32).Traverse(|i| {
        let w = &wires[i as usize];

        // 1. Root port is driver (drives into circuit)
        if (w.driver_module == rootName || w.driver_module.is_empty()) && w.driver_port == port.name {
            let sIdx = FindModuleIndex(modules, &w.sink_module);
            if sIdx != INVALID_INDEX {
                let sRank = modules[sIdx as usize].rank as f32;
                if sRank < midCol {
                    westVotes += 1;
                } else {
                    eastVotes += 1;
                }
            }
        }

        // 2. Root port is sink (receives from circuit)
        if (w.sink_module == rootName || w.sink_module.is_empty()) && w.sink_port == port.name {
            let dIdx = FindModuleIndex(modules, &w.driver_module);
            if dIdx != INVALID_INDEX {
                let dRank = modules[dIdx as usize].rank as f32;
                if dRank >= midCol {
                    eastVotes += 1;
                } else {
                    westVotes += 1;
                }
            }
        }
    });

    if westVotes > eastVotes {
        PortSide::West
    } else if eastVotes > westVotes {
        PortSide::East
    } else {
        match port.direction {
            PortDirection::In => PortSide::West,
            PortDirection::Out | PortDirection::InOut => PortSide::East,
        }
    }
}

fn SortColumnsByConnectivity(
    cols: &mut [Vec<u32>],
    modules: &[ModuleRecord],
    wires: &[WireDescriptor],
)
{
    let numCols = cols.len();
    if numCols <= 1 {
        return;
    }

    let findRowInCol = |colIdx: usize, mIdx: u32, cols: &[Vec<u32>]| -> Option<f32> {
        let col = &cols[colIdx];
        let mut foundRow = None;
        USeg::FromLen(col.len() as u32).Span(|r| {
            if col[r as usize] == mIdx {
                foundRow = Some(r as f32);
                return false;
            }
            return true;
        });
        foundRow
    };

    // Forward sweep: columns 1 to maxRank (align with column c - 1)
    USeg::FromLen(numCols as u32).Traverse(|c_u32| {
        let c = c_u32 as usize;
        if c == 0 || cols[c].len() <= 1 {
            return;
        }

        let mut barycenters: Vec<(u32, f32, u32)> = Vec::new();
        USeg::FromLen(cols[c].len() as u32).Traverse(|row_u32| {
            let mIdx = cols[c][row_u32 as usize];
            let m = &modules[mIdx as usize];
            let mut sumRows = 0.0f32;
            let mut count = 0u32;

            USeg::FromLen(wires.len() as u32).Traverse(|wIdx| {
                let w = &wires[wIdx as usize];
                if w.driver_module == m.name {
                    let sIdx = FindModuleIndex(modules, &w.sink_module);
                    if sIdx != INVALID_INDEX
                        && modules[sIdx as usize].rank == (c - 1) as u32
                        && let Some(r) = findRowInCol(c - 1, sIdx, cols)
                    {
                        sumRows += r;
                        count += 1;
                    }
                }
                if w.sink_module == m.name {
                    let dIdx = FindModuleIndex(modules, &w.driver_module);
                    if dIdx != INVALID_INDEX
                        && modules[dIdx as usize].rank == (c - 1) as u32
                        && let Some(r) = findRowInCol(c - 1, dIdx, cols)
                    {
                        sumRows += r;
                        count += 1;
                    }
                }
            });

            let bary = if count > 0 {
                sumRows / (count as f32)
            } else {
                row_u32 as f32
            };
            barycenters.push((mIdx, bary, row_u32));
        });

        barycenters.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.cmp(&b.2))
        });

        cols[c] = barycenters.into_iter().map(|(mIdx, _, _)| mIdx).collect();
    });

    // Backward sweep: columns maxRank - 1 down to 0 (align with column c + 1)
    let mut c_back = numCols - 1;
    while c_back > 0 {
        c_back -= 1;
        if cols[c_back].len() <= 1 {
            continue;
        }

        let mut barycenters: Vec<(u32, f32, u32)> = Vec::new();
        let colLen = cols[c_back].len() as u32;
        USeg::FromLen(colLen).Traverse(|row_u32| {
            let mIdx = cols[c_back][row_u32 as usize];
            let m = &modules[mIdx as usize];
            let mut sumRows = 0.0f32;
            let mut count = 0u32;

            USeg::FromLen(wires.len() as u32).Traverse(|wIdx| {
                let w = &wires[wIdx as usize];
                if w.driver_module == m.name {
                    let sIdx = FindModuleIndex(modules, &w.sink_module);
                    if sIdx != INVALID_INDEX
                        && modules[sIdx as usize].rank == (c_back + 1) as u32
                        && let Some(r) = findRowInCol(c_back + 1, sIdx, cols)
                    {
                        sumRows += r;
                        count += 1;
                    }
                }
                if w.sink_module == m.name {
                    let dIdx = FindModuleIndex(modules, &w.driver_module);
                    if dIdx != INVALID_INDEX
                        && modules[dIdx as usize].rank == (c_back + 1) as u32
                        && let Some(r) = findRowInCol(c_back + 1, dIdx, cols)
                    {
                        sumRows += r;
                        count += 1;
                    }
                }
            });

            let bary = if count > 0 {
                sumRows / (count as f32)
            } else {
                row_u32 as f32
            };
            barycenters.push((mIdx, bary, row_u32));
        });

        barycenters.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.cmp(&b.2))
        });

        cols[c_back] = barycenters.into_iter().map(|(mIdx, _, _)| mIdx).collect();
    }
}

fn PortPeerRowOrder(
    modIdx: usize,
    port: &PortDescriptor,
    modules: &[ModuleRecord],
    wires: &[WireDescriptor],
    rootName: &str,
    rootPorts: &[PortDescriptor],
    cols: &[Vec<u32>],
) -> f32
{
    let m = &modules[modIdx];
    let mut sumRows = 0.0f32;
    let mut count = 0u32;

    USeg::FromLen(wires.len() as u32).Traverse(|wIdx| {
        let w = &wires[wIdx as usize];
        if w.driver_module == m.name && w.driver_port == port.name {
            if w.sink_module == rootName || w.sink_module.is_empty() {
                let mut rootIdx = 0.0f32;
                USeg::FromLen(rootPorts.len() as u32).Span(|rpI| {
                    if rootPorts[rpI as usize].name == w.sink_port {
                        rootIdx = rpI as f32;
                        return false;
                    }
                    return true;
                });
                sumRows += 1000.0 + rootIdx;
                count += 1;
            } else {
                let sIdx = FindModuleIndex(modules, &w.sink_module);
                if sIdx != INVALID_INDEX {
                    let sRank = modules[sIdx as usize].rank as usize;
                    if sRank < cols.len() {
                        USeg::FromLen(cols[sRank].len() as u32).Span(|r| {
                            if cols[sRank][r as usize] == sIdx {
                                sumRows += r as f32;
                                count += 1;
                                return false;
                            }
                            return true;
                        });
                    }
                }
            }
        }
        if w.sink_module == m.name && w.sink_port == port.name {
            if w.driver_module == rootName || w.driver_module.is_empty() {
                let mut rootIdx = 0.0f32;
                USeg::FromLen(rootPorts.len() as u32).Span(|rpI| {
                    if rootPorts[rpI as usize].name == w.driver_port {
                        rootIdx = rpI as f32;
                        return false;
                    }
                    return true;
                });
                sumRows += 1000.0 + rootIdx;
                count += 1;
            } else {
                let dIdx = FindModuleIndex(modules, &w.driver_module);
                if dIdx != INVALID_INDEX {
                    let dRank = modules[dIdx as usize].rank as usize;
                    if dRank < cols.len() {
                        USeg::FromLen(cols[dRank].len() as u32).Span(|r| {
                            if cols[dRank][r as usize] == dIdx {
                                sumRows += r as f32;
                                count += 1;
                                return false;
                            }
                            return true;
                        });
                    }
                }
            }
        }
    });

    if count > 0 {
        sumRows / (count as f32)
    } else {
        500.0
    }
}

//-------------------------------------------------------------------------------------------------
// 3. Module & Perimeter Port Placement

fn PlaceModulesAndPorts(
    root: &dyn CircuitXplr,
    modules: &mut [ModuleRecord],
    wires: &[WireDescriptor],
    cols: &[Vec<u32>],
    maxRank: u32,
    isHierarchical: bool,
) -> (Rect, Vec<PlacedPortRecord>)
{
    let mut placedPorts: Vec<PlacedPortRecord> = Vec::new();
    let mut maxColY: f32 = 0.0;
    let rootPorts = root.Ports();

    if isHierarchical {
        USeg::FromLen(maxRank + 1).Traverse(|col| {
            let x = FRAME_PAD_X + (col as f32) * (MODULE_WIDTH + CHANNEL_WIDTH);
            let mut y = FRAME_PAD_Y + 20.0;
            let colMods = &cols[col as usize];
            USeg::FromLen(colMods.len() as u32).Traverse(|i| {
                let modIdx = colMods[i as usize] as usize;
                let modName = modules[modIdx].name.clone();
                let ports = modules[modIdx].ports.clone();

                let mut westPorts: Vec<(f32, u32, PortDescriptor)> = Vec::new();
                let mut eastPorts: Vec<(f32, u32, PortDescriptor)> = Vec::new();

                USeg::FromLen(ports.len() as u32).Traverse(|pIdx| {
                    let p = &ports[pIdx as usize];
                    let side = DeterminePortSide(modIdx, p, modules, wires, root.Name());
                    let order = PortPeerRowOrder(modIdx, p, modules, wires, root.Name(), &rootPorts, cols);
                    match side {
                        PortSide::West => westPorts.push((order, pIdx, p.clone())),
                        PortSide::East => eastPorts.push((order, pIdx, p.clone())),
                    }
                });

                westPorts.sort_by(|a, b| {
                    a.0.partial_cmp(&b.0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.1.cmp(&b.1))
                });
                eastPorts.sort_by(|a, b| {
                    a.0.partial_cmp(&b.0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.1.cmp(&b.1))
                });

                let maxPorts = westPorts.len().max(eastPorts.len()).max(1);
                let height = HEADER_HEIGHT + (maxPorts as f32) * PIN_PITCH + 12.0;
                let rect = Rect::New(x, y, MODULE_WIDTH, height);
                modules[modIdx].rect = rect;

                USeg::FromLen(westPorts.len() as u32).Traverse(|wIdx| {
                    let p = &westPorts[wIdx as usize].2;
                    let py = y + HEADER_HEIGHT + (wIdx as f32) * PIN_PITCH + PIN_PITCH * 0.5;
                    let pt = Point::New(x - PIN_STUB_LEN, py);
                    let bounds = Rect::New(
                        pt.x - PIN_HIT_RADIUS,
                        pt.y - PIN_HIT_RADIUS,
                        PIN_HIT_RADIUS * 2.0,
                        PIN_HIT_RADIUS * 2.0,
                    );
                    placedPorts.push(PlacedPortRecord {
                        module_name: modName.clone(),
                        port_name:   p.name.clone(),
                        direction:   p.direction,
                        side:        PortSide::West,
                        point:       pt,
                        bounds,
                        net_name:    String::new(),
                    });
                });

                USeg::FromLen(eastPorts.len() as u32).Traverse(|eIdx| {
                    let p = &eastPorts[eIdx as usize].2;
                    let py = y + HEADER_HEIGHT + (eIdx as f32) * PIN_PITCH + PIN_PITCH * 0.5;
                    let pt = Point::New(x + MODULE_WIDTH + PIN_STUB_LEN, py);
                    let bounds = Rect::New(
                        pt.x - PIN_HIT_RADIUS,
                        pt.y - PIN_HIT_RADIUS,
                        PIN_HIT_RADIUS * 2.0,
                        PIN_HIT_RADIUS * 2.0,
                    );
                    placedPorts.push(PlacedPortRecord {
                        module_name: modName.clone(),
                        port_name:   p.name.clone(),
                        direction:   p.direction,
                        side:        PortSide::East,
                        point:       pt,
                        bounds,
                        net_name:    String::new(),
                    });
                });

                y += height + ROW_GAP;
            });
            if y > maxColY {
                maxColY = y;
            }
        });
    } else {
        let modName = modules[0].name.clone();
        let ports = modules[0].ports.clone();
        let mut westPorts: Vec<PortDescriptor> = Vec::new();
        let mut eastPorts: Vec<PortDescriptor> = Vec::new();
        USeg::FromLen(ports.len() as u32).Traverse(|pIdx| {
            let p = &ports[pIdx as usize];
            let side = p.side.unwrap_or(match p.direction {
                PortDirection::In => PortSide::West,
                PortDirection::Out | PortDirection::InOut => PortSide::East,
            });
            match side {
                PortSide::West => westPorts.push(p.clone()),
                PortSide::East => eastPorts.push(p.clone()),
            }
        });
        let maxPorts = westPorts.len().max(eastPorts.len()).max(1);
        let height = HEADER_HEIGHT + (maxPorts as f32) * PIN_PITCH + 12.0;
        let rect = Rect::New(FRAME_PAD_X, FRAME_PAD_Y + 20.0, MODULE_WIDTH, height);
        modules[0].rect = rect;

        USeg::FromLen(westPorts.len() as u32).Traverse(|wIdx| {
            let p = &westPorts[wIdx as usize];
            let py = rect.y + HEADER_HEIGHT + (wIdx as f32) * PIN_PITCH + PIN_PITCH * 0.5;
            let pt = Point::New(rect.x - PIN_STUB_LEN, py);
            let bounds = Rect::New(
                pt.x - PIN_HIT_RADIUS,
                pt.y - PIN_HIT_RADIUS,
                PIN_HIT_RADIUS * 2.0,
                PIN_HIT_RADIUS * 2.0,
            );
            placedPorts.push(PlacedPortRecord {
                module_name: modName.clone(),
                port_name:   p.name.clone(),
                direction:   p.direction,
                side:        PortSide::West,
                point:       pt,
                bounds,
                net_name:    String::new(),
            });
        });

        USeg::FromLen(eastPorts.len() as u32).Traverse(|eIdx| {
            let p = &eastPorts[eIdx as usize];
            let py = rect.y + HEADER_HEIGHT + (eIdx as f32) * PIN_PITCH + PIN_PITCH * 0.5;
            let pt = Point::New(rect.Right() + PIN_STUB_LEN, py);
            let bounds = Rect::New(
                pt.x - PIN_HIT_RADIUS,
                pt.y - PIN_HIT_RADIUS,
                PIN_HIT_RADIUS * 2.0,
                PIN_HIT_RADIUS * 2.0,
            );
            placedPorts.push(PlacedPortRecord {
                module_name: modName.clone(),
                port_name:   p.name.clone(),
                direction:   p.direction,
                side:        PortSide::East,
                point:       pt,
                bounds,
                net_name:    String::new(),
            });
        });
        maxColY = rect.Bottom();
    }

    let mut westRootPorts: Vec<PortDescriptor> = Vec::new();
    let mut eastRootPorts: Vec<PortDescriptor> = Vec::new();
    USeg::FromLen(rootPorts.len() as u32).Traverse(|i| {
        let p = &rootPorts[i as usize];
        let side = DetermineRootPortSide(p, modules, wires, root.Name(), cols.len());
        match side {
            PortSide::West => westRootPorts.push(p.clone()),
            PortSide::East => eastRootPorts.push(p.clone()),
        }
    });
    let rootPortsH = (westRootPorts.len().max(eastRootPorts.len()) as f32) * 36.0 + 80.0;

    let numCols = (maxRank + 1) as f32;
    let totalWidth = (FRAME_PAD_X
                      + numCols * MODULE_WIDTH
                      + (numCols - 1.0).max(0.0) * CHANNEL_WIDTH
                      + FRAME_PAD_X)
                          .max(480.0);
    let totalHeight = (maxColY.max(rootPortsH) + FRAME_PAD_Y).max(360.0);
    let totalBounds = Rect::New(0.0, 0.0, totalWidth, totalHeight);

    if isHierarchical {
        USeg::FromLen(westRootPorts.len() as u32).Traverse(|i| {
            let p = &westRootPorts[i as usize];
            let py = FRAME_PAD_Y + 24.0 + (i as f32) * 36.0 + 18.0;
            let pt = Point::New(28.0, py);
            let bounds = Rect::New(
                pt.x - PIN_HIT_RADIUS,
                pt.y - PIN_HIT_RADIUS,
                PIN_HIT_RADIUS * 2.0,
                PIN_HIT_RADIUS * 2.0,
            );
            placedPorts.push(PlacedPortRecord {
                module_name: root.Name().to_string(),
                port_name:   p.name.clone(),
                direction:   p.direction,
                side:        PortSide::West,
                point:       pt,
                bounds,
                net_name:    String::new(),
            });
            placedPorts.push(PlacedPortRecord {
                module_name: String::new(),
                port_name:   p.name.clone(),
                direction:   p.direction,
                side:        PortSide::West,
                point:       pt,
                bounds,
                net_name:    String::new(),
            });
        });

        USeg::FromLen(eastRootPorts.len() as u32).Traverse(|i| {
            let p = &eastRootPorts[i as usize];
            let py = FRAME_PAD_Y + 24.0 + (i as f32) * 36.0 + 18.0;
            let pt = Point::New(totalWidth - 28.0, py);
            let bounds = Rect::New(
                pt.x - PIN_HIT_RADIUS,
                pt.y - PIN_HIT_RADIUS,
                PIN_HIT_RADIUS * 2.0,
                PIN_HIT_RADIUS * 2.0,
            );
            placedPorts.push(PlacedPortRecord {
                module_name: root.Name().to_string(),
                port_name:   p.name.clone(),
                direction:   p.direction,
                side:        PortSide::East,
                point:       pt,
                bounds,
                net_name:    String::new(),
            });
            placedPorts.push(PlacedPortRecord {
                module_name: String::new(),
                port_name:   p.name.clone(),
                direction:   p.direction,
                side:        PortSide::East,
                point:       pt,
                bounds,
                net_name:    String::new(),
            });
        });
    }

    (totalBounds, placedPorts)
}

fn FindPlacedPortRecord<'a>(
    ports: &'a [PlacedPortRecord],
    moduleName: &str,
    portName: &str,
) -> Option<&'a PlacedPortRecord>
{
    let mut found = None;
    USeg::FromLen(ports.len() as u32).Span(|i| {
        let p = &ports[i as usize];
        if (p.module_name == moduleName || (moduleName.is_empty() && p.module_name.is_empty()))
            && p.port_name == portName
        {
            found = Some(p);
            return false;
        }
        return true;
    });
    found
}

fn FindPlacedPort(
    ports: &[PlacedPortRecord],
    moduleName: &str,
    portName: &str,
) -> Option<Point>
{
    FindPlacedPortRecord(ports, moduleName, portName).map(|p| p.point)
}

//-------------------------------------------------------------------------------------------------
// 4. Manhattan Wire Router & Multi-Sink Grouping

fn PinFacingDirection(port: &PlacedPortRecord, totalWidth: f32) -> PortSide
{
    let isBoundary = port.point.x <= FRAME_PAD_X || port.point.x >= totalWidth - FRAME_PAD_X;
    if isBoundary {
        if port.point.x <= FRAME_PAD_X {
            PortSide::East
        } else {
            PortSide::West
        }
    } else {
        port.side
    }
}

fn RouteManhattanWires(
    wires: &[WireDescriptor],
    placedPorts: &mut [PlacedPortRecord],
    totalWidth: f32,
    totalHeight: f32,
    wireColor: Color,
    commands: &mut Vec<CaskRenderCommand>,
) -> Vec<Cask2dWire>
{
    let mut interactiveWires: Vec<Cask2dWire> = Vec::new();
    let mut driverGroups: Vec<DriverSinkGroup> = Vec::new();

    USeg::FromLen(wires.len() as u32).Traverse(|i| {
        let w = &wires[i as usize];
        let driverKey = (w.driver_module.clone(), w.driver_port.clone());
        let sinkKey = (w.sink_module.clone(), w.sink_port.clone());

        if FindPlacedPort(placedPorts, &driverKey.0, &driverKey.1).is_some()
            && FindPlacedPort(placedPorts, &sinkKey.0, &sinkKey.1).is_some()
        {
            let netName = if !w.name.is_empty() {
                w.name.clone()
            } else {
                format!("{}_{}", driverKey.0, driverKey.1)
            };

            let mut groupIdx = INVALID_INDEX;
            USeg::FromLen(driverGroups.len() as u32).Span(|gIdx| {
                if driverGroups[gIdx as usize].driver == driverKey {
                    groupIdx = gIdx;
                    return false;
                }
                return true;
            });

            if groupIdx != INVALID_INDEX {
                driverGroups[groupIdx as usize].sinks.push(sinkKey);
            } else {
                driverGroups.push(DriverSinkGroup {
                    driver:   driverKey,
                    sinks:    vec![sinkKey],
                    net_name: netName,
                });
            }
        }
    });

    // Assign net names to all matching placed ports
    USeg::FromLen(driverGroups.len() as u32).Traverse(|gIdx| {
        let group = &driverGroups[gIdx as usize];
        USeg::FromLen(placedPorts.len() as u32).Traverse(|pIdx| {
            let p = &mut placedPorts[pIdx as usize];
            if p.module_name == group.driver.0 && p.port_name == group.driver.1 {
                p.net_name = group.net_name.clone();
            }
            USeg::FromLen(group.sinks.len() as u32).Traverse(|sIdx| {
                let s = &group.sinks[sIdx as usize];
                if p.module_name == s.0 && p.port_name == s.1 {
                    p.net_name = group.net_name.clone();
                }
            });
        });
    });

    let mut wireIndex: usize = 0;
    USeg::FromLen(driverGroups.len() as u32).Traverse(|gIdx| {
        let group = &driverGroups[gIdx as usize];
        let Some(rSrc) = FindPlacedPortRecord(placedPorts, &group.driver.0, &group.driver.1) else {
            return;
        };
        let pSrc = rSrc.point;
        let facingSrc = PinFacingDirection(rSrc, totalWidth);

        USeg::FromLen(group.sinks.len() as u32).Traverse(|sIdx| {
            let sink = &group.sinks[sIdx as usize];
            let Some(rDst) = FindPlacedPortRecord(placedPorts, &sink.0, &sink.1) else {
                return;
            };
            let pDst = rDst.point;
            let facingDst = PinFacingDirection(rDst, totalWidth);

            let mut segments: Vec<(Point, Point)> = Vec::new();

            // Direct channel facing connections:
            // 1. East-to-West: pDst.x > pSrc.x, driver faces East, sink faces West, across adjacent channel
            // 2. West-to-East: pSrc.x > pDst.x, driver faces West, sink faces East, across adjacent channel
            let channelThreshold = MODULE_WIDTH + CHANNEL_WIDTH * 0.5;
            let isDirectEastToWest = pDst.x > pSrc.x
                && facingSrc == PortSide::East
                && facingDst == PortSide::West
                && (pDst.x - pSrc.x) <= channelThreshold;
            let isDirectWestToEast = pSrc.x > pDst.x
                && facingSrc == PortSide::West
                && facingDst == PortSide::East
                && (pSrc.x - pDst.x) <= channelThreshold;

            if isDirectEastToWest || isDirectWestToEast {
                let chanMid = (pSrc.x + pDst.x) * 0.5;
                let xMid = chanMid + (((wireIndex * 7) % 25) as f32 - 12.0);

                let seg1 = (pSrc, Point::New(xMid, pSrc.y));
                let seg2 = (Point::New(xMid, pSrc.y), Point::New(xMid, pDst.y));
                let seg3 = (Point::New(xMid, pDst.y), pDst);

                commands.push(CaskRenderCommand::Line {
                    start: seg1.0,
                    end:   seg1.1,
                    color: wireColor,
                    width: 2.0,
                });
                commands.push(CaskRenderCommand::Line {
                    start: seg2.0,
                    end:   seg2.1,
                    color: wireColor,
                    width: 2.0,
                });
                commands.push(CaskRenderCommand::Line {
                    start: seg3.0,
                    end:   seg3.1,
                    color: wireColor,
                    width: 2.0,
                });

                if sIdx > 0 {
                    commands.push(CaskRenderCommand::Rectangle {
                        bounds:        Rect::New(xMid - 3.0, pSrc.y - 3.0, 6.0, 6.0),
                        color:         wireColor,
                        corner_radius: 3.0,
                    });
                }

                segments.push(seg1);
                segments.push(seg2);
                segments.push(seg3);

                interactiveWires.push(Cask2dWire::New(
                    group.net_name.clone(),
                    group.driver.0.clone(),
                    group.driver.1.clone(),
                    sink.0.clone(),
                    sink.1.clone(),
                    segments,
                ));
            } else if pDst.x > pSrc.x {
                let yHighway = totalHeight - FRAME_PAD_Y * 0.6 + ((wireIndex * 6) % 24) as f32;
                let xDrop = pSrc.x + 14.0;
                let xRise = pDst.x - 14.0;

                let seg1 = (pSrc, Point::New(xDrop, pSrc.y));
                let seg2 = (Point::New(xDrop, pSrc.y), Point::New(xDrop, yHighway));
                let seg3 = (Point::New(xDrop, yHighway), Point::New(xRise, yHighway));
                let seg4 = (Point::New(xRise, yHighway), Point::New(xRise, pDst.y));
                let seg5 = (Point::New(xRise, pDst.y), pDst);

                commands.push(CaskRenderCommand::Line {
                    start: seg1.0,
                    end:   seg1.1,
                    color: wireColor,
                    width: 2.0,
                });
                commands.push(CaskRenderCommand::Line {
                    start: seg2.0,
                    end:   seg2.1,
                    color: wireColor,
                    width: 2.0,
                });
                commands.push(CaskRenderCommand::Line {
                    start: seg3.0,
                    end:   seg3.1,
                    color: wireColor,
                    width: 2.0,
                });
                commands.push(CaskRenderCommand::Line {
                    start: seg4.0,
                    end:   seg4.1,
                    color: wireColor,
                    width: 2.0,
                });
                commands.push(CaskRenderCommand::Line {
                    start: seg5.0,
                    end:   seg5.1,
                    color: wireColor,
                    width: 2.0,
                });

                if sIdx > 0 {
                    commands.push(CaskRenderCommand::Rectangle {
                        bounds:        Rect::New(xDrop - 3.0, pSrc.y - 3.0, 6.0, 6.0),
                        color:         wireColor,
                        corner_radius: 3.0,
                    });
                }

                segments.push(seg1);
                segments.push(seg2);
                segments.push(seg3);
                segments.push(seg4);
                segments.push(seg5);

                interactiveWires.push(Cask2dWire::New(
                    group.net_name.clone(),
                    group.driver.0.clone(),
                    group.driver.1.clone(),
                    sink.0.clone(),
                    sink.1.clone(),
                    segments,
                ));
            } else {
                let yLoop = totalHeight - FRAME_PAD_Y * 0.4 + ((wireIndex * 5) % 20) as f32;
                let xOut = if facingSrc == PortSide::East { pSrc.x + 14.0 } else { pSrc.x - 14.0 };
                let xIn = if facingDst == PortSide::West { pDst.x - 14.0 } else { pDst.x + 14.0 };

                let seg1 = (pSrc, Point::New(xOut, pSrc.y));
                let seg2 = (Point::New(xOut, pSrc.y), Point::New(xOut, yLoop));
                let seg3 = (Point::New(xOut, yLoop), Point::New(xIn, yLoop));
                let seg4 = (Point::New(xIn, yLoop), Point::New(xIn, pDst.y));
                let seg5 = (Point::New(xIn, pDst.y), pDst);

                commands.push(CaskRenderCommand::Line {
                    start: seg1.0,
                    end:   seg1.1,
                    color: wireColor,
                    width: 2.0,
                });
                commands.push(CaskRenderCommand::Line {
                    start: seg2.0,
                    end:   seg2.1,
                    color: wireColor,
                    width: 2.0,
                });
                commands.push(CaskRenderCommand::Line {
                    start: seg3.0,
                    end:   seg3.1,
                    color: wireColor,
                    width: 2.0,
                });
                commands.push(CaskRenderCommand::Line {
                    start: seg4.0,
                    end:   seg4.1,
                    color: wireColor,
                    width: 2.0,
                });
                commands.push(CaskRenderCommand::Line {
                    start: seg5.0,
                    end:   seg5.1,
                    color: wireColor,
                    width: 2.0,
                });

                if sIdx > 0 {
                    commands.push(CaskRenderCommand::Rectangle {
                        bounds:        Rect::New(xOut - 3.0, pSrc.y - 3.0, 6.0, 6.0),
                        color:         wireColor,
                        corner_radius: 3.0,
                    });
                }

                segments.push(seg1);
                segments.push(seg2);
                segments.push(seg3);
                segments.push(seg4);
                segments.push(seg5);

                interactiveWires.push(Cask2dWire::New(
                    group.net_name.clone(),
                    group.driver.0.clone(),
                    group.driver.1.clone(),
                    sink.0.clone(),
                    sink.1.clone(),
                    segments,
                ));
            }
            wireIndex += 1;
        });
    });

    interactiveWires
}

//-------------------------------------------------------------------------------------------------
// 5. Schematic Command & Scene Emitter

/// Solves 2D classical schematic layout for a `CircuitXplr` entity (modules, perimeter pins, wires).
pub fn LayoutCircuit2d(root: &dyn CircuitXplr, options: &Cask2dOptions) -> Cask2dLayout
{
    let colors = &options.colors;
    let bgRoot = colors.background_root;
    let bgModule = colors.background_branch;
    let bgSubbox = colors.background_leaf;
    let borderModule = colors.border_highlight;
    let borderSubtle = colors.border_color;
    let textPrimary = colors.text_primary;
    let textSecondary = colors.text_secondary;
    let wireColor = colors.accent_color;

    let submodules = root.Submodules();
    let isHierarchical = !submodules.is_empty();

    let (mut modules, wires) = ExtractModuleModel(root, isHierarchical);
    let (maxRank, mut cols) = if isHierarchical {
        RankModules(&mut modules, &wires)
    } else {
        (0, vec![vec![0]])
    };

    if isHierarchical {
        SortColumnsByConnectivity(&mut cols, &modules, &wires);
    }

    let (totalBounds, mut placedPorts) =
        PlaceModulesAndPorts(root, &mut modules, &wires, &cols, maxRank, isHierarchical);

    let mut commands: Vec<CaskRenderCommand> = Vec::new();

    // 1. Root canvas background & frame
    commands.push(CaskRenderCommand::Rectangle {
        bounds:        totalBounds,
        color:         bgRoot,
        corner_radius: 8.0,
    });
    commands.push(CaskRenderCommand::Border {
        bounds:        totalBounds,
        color:         borderSubtle,
        width:         1.5,
        corner_radius: 8.0,
    });
    commands.push(CaskRenderCommand::Text {
        bounds:    Rect::New(16.0, 12.0, totalBounds.width - 32.0, 24.0),
        text:      format!("SCHEMATIC: {} [{}]", root.Name(), root.ComponentType()),
        color:     textPrimary,
        font_size: 14.0,
    });

    // 2. Primary external ports on outer boundary frame
    let rootPorts = root.Ports();
    if isHierarchical {
        USeg::FromLen(rootPorts.len() as u32).Traverse(|i| {
            let p = &rootPorts[i as usize];
            if let Some(placed) = FindPlacedPortRecord(&placedPorts, root.Name(), &p.name) {
                let pt = placed.point;
                let side = placed.side;
                if side == PortSide::West {
                    commands.push(CaskRenderCommand::Line {
                        start: Point::New(4.0, pt.y),
                        end:   Point::New(28.0, pt.y),
                        color: borderModule,
                        width: 2.0,
                    });
                    if p.direction == PortDirection::In {
                        commands.push(CaskRenderCommand::Triangle {
                            p1:    Point::New(20.0, pt.y - 4.5),
                            p2:    Point::New(20.0, pt.y + 4.5),
                            p3:    Point::New(28.0, pt.y),
                            color: wireColor,
                        });
                    } else {
                        commands.push(CaskRenderCommand::Triangle {
                            p1:    Point::New(12.0, pt.y - 4.5),
                            p2:    Point::New(12.0, pt.y + 4.5),
                            p3:    Point::New(4.0, pt.y),
                            color: wireColor,
                        });
                    }
                    commands.push(CaskRenderCommand::Rectangle {
                        bounds:        Rect::New(2.0, pt.y - 3.0, 6.0, 6.0),
                        color:         wireColor,
                        corner_radius: 3.0,
                    });
                    commands.push(CaskRenderCommand::Text {
                        bounds:    Rect::New(32.0, pt.y - 7.0, 64.0, 16.0),
                        text:      p.name.clone(),
                        color:     textPrimary,
                        font_size: 11.0,
                    });
                } else {
                    let leftX = totalBounds.width - 28.0;
                    let rightX = totalBounds.width - 4.0;
                    commands.push(CaskRenderCommand::Line {
                        start: Point::New(leftX, pt.y),
                        end:   Point::New(rightX, pt.y),
                        color: borderModule,
                        width: 2.0,
                    });
                    if p.direction == PortDirection::In {
                        commands.push(CaskRenderCommand::Triangle {
                            p1:    Point::New(leftX + 8.0, pt.y - 4.5),
                            p2:    Point::New(leftX + 8.0, pt.y + 4.5),
                            p3:    Point::New(leftX, pt.y),
                            color: wireColor,
                        });
                    } else {
                        commands.push(CaskRenderCommand::Triangle {
                            p1:    Point::New(rightX - 8.0, pt.y - 4.5),
                            p2:    Point::New(rightX - 8.0, pt.y + 4.5),
                            p3:    Point::New(rightX, pt.y),
                            color: wireColor,
                        });
                    }
                    commands.push(CaskRenderCommand::Rectangle {
                        bounds:        Rect::New(rightX - 4.0, pt.y - 3.0, 6.0, 6.0),
                        color:         wireColor,
                        corner_radius: 3.0,
                    });
                    commands.push(CaskRenderCommand::Text {
                        bounds:    Rect::New(leftX - 68.0, pt.y - 7.0, 64.0, 16.0),
                        text:      p.name.clone(),
                        color:     textPrimary,
                        font_size: 11.0,
                    });
                }
            }
        });
    }

    // 3. Module functional blocks & perimeter pins
    USeg::FromLen(modules.len() as u32).Traverse(|i| {
        let m = &modules[i as usize];
        let rect = m.rect;
        commands.push(CaskRenderCommand::Rectangle {
            bounds:        rect,
            color:         bgModule,
            corner_radius: 6.0,
        });
        commands.push(CaskRenderCommand::Border {
            bounds:        rect,
            color:         borderModule,
            width:         1.5,
            corner_radius: 6.0,
        });

        // Header
        let headerRect = Rect::New(rect.x, rect.y, rect.width, HEADER_HEIGHT);
        commands.push(CaskRenderCommand::Rectangle {
            bounds:        headerRect,
            color:         bgSubbox,
            corner_radius: 6.0,
        });
        commands.push(CaskRenderCommand::Border {
            bounds:        headerRect,
            color:         borderSubtle,
            width:         1.0,
            corner_radius: 6.0,
        });
        commands.push(CaskRenderCommand::Text {
            bounds:    Rect::New(rect.x + 8.0, rect.y + 4.0, rect.width - 16.0, 16.0),
            text:      m.name.clone(),
            color:     textPrimary,
            font_size: 13.0,
        });
        commands.push(CaskRenderCommand::Text {
            bounds:    Rect::New(rect.x + 8.0, rect.y + 20.0, rect.width - 16.0, 12.0),
            text:      m.component_type.clone(),
            color:     textSecondary,
            font_size: 10.0,
        });

        // Pins
        USeg::FromLen(m.ports.len() as u32).Traverse(|pIdx| {
            let p = &m.ports[pIdx as usize];
            if let Some(placed) = FindPlacedPortRecord(&placedPorts, &m.name, &p.name) {
                let pt = placed.point;
                let side = placed.side;
                if side == PortSide::West {
                    commands.push(CaskRenderCommand::Line {
                        start: pt,
                        end:   Point::New(rect.x, pt.y),
                        color: borderModule,
                        width: 1.5,
                    });
                    if p.direction == PortDirection::In {
                        commands.push(CaskRenderCommand::Triangle {
                            p1:    Point::New(rect.x - 7.0, pt.y - 4.5),
                            p2:    Point::New(rect.x - 7.0, pt.y + 4.5),
                            p3:    Point::New(rect.x, pt.y),
                            color: wireColor,
                        });
                    } else {
                        commands.push(CaskRenderCommand::Triangle {
                            p1:    Point::New(rect.x, pt.y - 4.5),
                            p2:    Point::New(rect.x, pt.y + 4.5),
                            p3:    Point::New(rect.x - 7.0, pt.y),
                            color: wireColor,
                        });
                    }
                    commands.push(CaskRenderCommand::Rectangle {
                        bounds:        Rect::New(pt.x - 2.0, pt.y - 2.0, 4.0, 4.0),
                        color:         borderModule,
                        corner_radius: 1.0,
                    });
                    commands.push(CaskRenderCommand::Text {
                        bounds:    Rect::New(rect.x + 6.0, pt.y - 6.0, 48.0, 14.0),
                        text:      p.name.clone(),
                        color:     textSecondary,
                        font_size: 11.0,
                    });
                } else {
                    commands.push(CaskRenderCommand::Line {
                        start: Point::New(rect.Right(), pt.y),
                        end:   pt,
                        color: borderModule,
                        width: 1.5,
                    });
                    if p.direction == PortDirection::In {
                        commands.push(CaskRenderCommand::Triangle {
                            p1:    Point::New(rect.Right() + 7.0, pt.y - 4.5),
                            p2:    Point::New(rect.Right() + 7.0, pt.y + 4.5),
                            p3:    Point::New(rect.Right(), pt.y),
                            color: wireColor,
                        });
                    } else {
                        commands.push(CaskRenderCommand::Triangle {
                            p1:    Point::New(pt.x - 7.0, pt.y - 4.5),
                            p2:    Point::New(pt.x - 7.0, pt.y + 4.5),
                            p3:    Point::New(pt.x, pt.y),
                            color: wireColor,
                        });
                    }
                    commands.push(CaskRenderCommand::Rectangle {
                        bounds:        Rect::New(pt.x - 2.0, pt.y - 2.0, 4.0, 4.0),
                        color:         borderModule,
                        corner_radius: 1.0,
                    });
                    commands.push(CaskRenderCommand::Text {
                        bounds:    Rect::New(rect.Right() - 54.0, pt.y - 6.0, 48.0, 14.0),
                        text:      p.name.clone(),
                        color:     textSecondary,
                        font_size: 11.0,
                    });
                }
            }
        });
    });

    // 4. Manhattan wire routing
    let interactiveWires = RouteManhattanWires(
        &wires,
        &mut placedPorts,
        totalBounds.width,
        totalBounds.height,
        wireColor,
        &mut commands,
    );

    // 5. Interactive Pin Nodes (deduplicated by module + port)
    let mut interactivePins: Vec<Cask2dPinNode> = Vec::new();
    USeg::FromLen(placedPorts.len() as u32).Traverse(|i| {
        let p = &placedPorts[i as usize];
        if p.module_name.is_empty() {
            return;
        }
        let mut exists = false;
        USeg::FromLen(interactivePins.len() as u32).Span(|j| {
            let existing = &interactivePins[j as usize];
            if existing.ModuleName() == p.module_name && existing.PortName() == p.port_name {
                exists = true;
                return false;
            }
            return true;
        });
        if !exists {
            interactivePins.push(Cask2dPinNode::NewWithSide(
                p.module_name.clone(),
                p.port_name.clone(),
                p.direction,
                p.side,
                p.bounds,
                p.point,
                p.net_name.clone(),
            ));
        }
    });

    // 6. Cask2dNode list (Canvas root + Submodules for navigation and selection)
    let mut nodes: Vec<Cask2dNode> = Vec::new();
    nodes.push(Cask2dNode {
        id:               format!("circuit-{}", root.Name()),
        name:             root.Name().to_string(),
        path:             root.Path().to_string(),
        is_leaf:          !isHierarchical,
        extension:        "circuit".to_string(),
        size:             modules.len() as u64,
        depth:            0,
        bounds:           totalBounds,
        parent_index:     None,
        children_indices: (1..=modules.len()).collect(),
    });

    USeg::FromLen(modules.len() as u32).Traverse(|i| {
        let m = &modules[i as usize];
        nodes.push(Cask2dNode {
            id:               format!("mod-{}", m.name),
            name:             m.name.clone(),
            path:             m.path.clone(),
            is_leaf:          m.submodule_count == 0,
            extension:        "module".to_string(),
            size:             m.submodule_count as u64,
            depth:            1,
            bounds:           m.rect,
            parent_index:     Some(0),
            children_indices: Vec::new(),
        });
    });

    // 7. Cask Flexbox Tree
    let mut rootCask = Cask::NewWindow(format!("Circuit: {}", root.Name()));
    rootCask = rootCask.WithDirection(LayoutDirection::LeftToRight);
    USeg::FromLen(modules.len() as u32).Traverse(|i| {
        let m = &modules[i as usize];
        let mut modCask = Cask::NewWindow(m.name.clone());
        modCask = modCask.WithDirection(LayoutDirection::TopToBottom);
        rootCask.AddChild(modCask);
    });

    Cask2dLayout {
        bounds: totalBounds,
        nodes,
        pins: interactivePins,
        wires: interactiveWires,
        commands,
        cask_tree: Some(rootCask),
    }
}

//-------------------------------------------------------------------------------------------------
// Submodule Port Placement Solver

/// Solves the port placements (West vs East side) decided for all submodules in a parent circuit.
pub fn DecideSubmodulePortSides(root: &dyn CircuitXplr) -> Vec<(String, Vec<(String, PortSide)>)>
{
    let submodules = root.Submodules();
    if submodules.is_empty() {
        return Vec::new();
    }

    let (mut modules, wires) = ExtractModuleModel(root, true);
    let (_maxRank, _cols) = RankModules(&mut modules, &wires);

    let mut result: Vec<(String, Vec<(String, PortSide)>)> = Vec::new();
    USeg::FromLen(modules.len() as u32).Traverse(|modIdx| {
        let m = &modules[modIdx as usize];
        let mut portSides: Vec<(String, PortSide)> = Vec::new();
        USeg::FromLen(m.ports.len() as u32).Traverse(|pIdx| {
            let p = &m.ports[pIdx as usize];
            let side = DeterminePortSide(modIdx as usize, p, &modules, &wires, root.Name());
            portSides.push((p.name.clone(), side));
        });
        result.push((m.name.clone(), portSides));
    });

    result
}

//-------------------------------------------------------------------------------------------------
