// examples_workspace.rs --------------------------------------------------------------------------
//! Dedicated Examples Workspace: Gallery discovery, Heist-backed execution, run-owned
//! result trees, and integrated Circuit / Fresco / Waveform / Tree inspection.

use crate::fascia::cask_view::{Cask2dAction, view_cask2d};
use crate::fascia::circuit_scene::{CircuitBuild, CircuitSchematic};
use crate::fascia::examples::ExampleCompletion;
use crate::fascia::geometry_view::{GeometryAction, GeometryViewerState, ViewGeometry};
use crate::fascia::icons::{IconId, ViewIcon};
use crate::fascia::math_view::{FrescoAction, FrescoViewerState, ViewMathTab};
use crate::fascia::output::JobId;
use crate::fascia::plot2d::{Plot2dAction, Plot2dState, SampleFrescoPlot2d, ViewPlot2d};
use crate::fascia::plot3d::SampleFrescoSurface3d;
use crate::fascia::theme::{FasciaStyle, ThemePalette, default_code_font};
use crate::fascia::waveform::{WaveformAction, WaveformState, view_waveform};
use crate::fenst::Xplr;
use crate::fenst::cask2d::Cask2dLayout;
use crate::fenst::example::XplrExample;
use crate::fenst::rube_resource::DADDA_EXAMPLE_URI;
use crate::fenst::waveform::ParseWaveform;
use crate::silo::Buff;
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text, text_input};
use iced::{Alignment, Background, Element, Length};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

//-------------------------------------------------------------------------------------------------

/// Curated presentation metadata for known `XplrExample` registrations.
#[derive(Debug, Clone, Copy)]
pub struct CatalogDescriptor
{
    pub suite:         &'static str,
    pub name:          &'static str,
    pub title:         &'static str,
    pub summary:       &'static str,
    pub featured_rank: Option<u8>,
}

static DESCRIPTORS: &[CatalogDescriptor] = &[
    CatalogDescriptor {
        suite: "Rube",
        name: "DaddaMultiplier4x4",
        title: "4x4 Dadda multiplier",
        summary: "Inspect gates, module hierarchy, and signal flow",
        featured_rank: Some(1),
    },
    CatalogDescriptor {
        suite: "Fresco",
        name: "FunctionPlotting2dAnd3d",
        title: "Function plotting in 2D and 3D",
        summary: "Explore curves and 3D heightfield surfaces",
        featured_rank: Some(2),
    },
    CatalogDescriptor {
        suite: "Fenst",
        name: "SketchExample",
        title: "Mechanical sketch",
        summary: "Inspect layers, groups, and geometric entities",
        featured_rank: Some(3),
    },
    CatalogDescriptor {
        suite: "Crew",
        name: "HubTopology",
        title: "Hub topology",
        summary: "Follow a multicast and its routing replies",
        featured_rank: Some(4),
    },
    CatalogDescriptor {
        suite: "Karst",
        name: "System",
        title: "System circuit",
        summary: "Explore the top-level module hierarchy",
        featured_rank: Some(5),
    },
    CatalogDescriptor {
        suite: "Rube",
        name: "FullAdderExample",
        title: "Full adder",
        summary: "One-bit adder schematic with selectable gates and ports",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Rube",
        name: "Adder8Example",
        title: "8-bit ripple adder",
        summary: "Multi-bit adder schematic with hierarchy drill-down",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fresco",
        name: "ExpressionExplorerExample",
        title: "Expression explorer",
        summary: "Scalar calculus, matrix algebra, and evaluated values",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fresco",
        name: "SymbolicParsingAndDag",
        title: "Symbolic parsing & DAG",
        summary: "Parsed expressions, shared DAG nodes, and checkpoints",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fresco",
        name: "PolynomialsAndFunctions",
        title: "Polynomials & functions",
        summary: "Canonical polynomials and transcendental functions",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fresco",
        name: "LogicAndConditionals",
        title: "Logic & conditionals",
        summary: "Boolean predicates and piecewise conditional evaluation",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fresco",
        name: "TensorsAndShapes",
        title: "Tensors & shapes",
        summary: "Vectors, matrices, and validated tensor dimensions",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fresco",
        name: "SymbolicDiffAndJacobian",
        title: "Symbolic differentiation & Jacobian",
        summary: "Partial derivatives and 2x2 Jacobian assembly",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fresco",
        name: "CompiledProgramAndAd",
        title: "Compiled program & AD",
        summary: "Topological bytecode compilation and evaluation",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fresco",
        name: "LeastSquaresOptimization",
        title: "Least-squares optimization",
        summary: "Levenberg-Marquardt solve on Rosenbrock with Reverse AD",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fenst",
        name: "FilesystemExplorer",
        title: "Filesystem explorer",
        summary: "Lazy directory tree with bounded file previews",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fenst",
        name: "TraverseDepthFullOrder",
        title: "Depth traversal order",
        summary: "Mock hierarchy verifying depth-first tree traversal",
        featured_rank: None,
    },
    CatalogDescriptor {
        suite: "Fenst",
        name: "XplrExampleProtocolDemo",
        title: "Protocol demo",
        summary: "Subsystem hierarchy and inspectable result leaves",
        featured_rank: None,
    },
];

/// Resolved catalog entry preserving its original index in `XplrExample::Registered()`.
#[derive(Debug, Clone)]
pub struct CatalogItem
{
    pub catalog_index: usize,
    pub suite:         &'static str,
    pub name:          &'static str,
    pub uri:           String,
    pub title:         String,
    pub summary:       String,
    pub featured_rank: Option<u8>,
}

fn build_catalog(examples: &[&'static XplrExample]) -> Vec<CatalogItem>
{
    let mut items = Vec::with_capacity(examples.len());
    for (idx, ex) in examples.iter().enumerate() {
        let desc = DESCRIPTORS
            .iter()
            .find(|d| d.suite == ex.suite && d.name == ex.name);
        let uri = format!("example://{}/{}", ex.suite, ex.name);
        let title = desc.map(|d| d.title.to_string()).unwrap_or_else(|| ex.name.to_string());
        let summary = desc.map(|d| d.summary.to_string()).unwrap_or_else(|| uri.clone());
        let featured_rank = desc.and_then(|d| d.featured_rank);
        items.push(CatalogItem {
            catalog_index: idx,
            suite: ex.suite,
            name: ex.name,
            uri,
            title,
            summary,
            featured_rank,
        });
    }
    return items;
}

//-------------------------------------------------------------------------------------------------

/// Active screen inside the Examples Workspace tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceScreen
{
    Gallery,
    Detail(usize),
}

/// Active renderer mode inside an example's Detail screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailViewMode
{
    Schematic,
    Scene3d,
    MathView,
    Plot2d,
    Plot3d,
    ResultTree,
    Waveform,
}

/// Lifecycle state of an example run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExampleRunStatus
{
    Running,
    Passed,
    Failed(String),
}

/// Owned tree node stored inside a completed `ExampleRunRecord`.
pub struct RunTreeNode
{
    pub xplr:     Box<dyn Xplr>,
    pub children: Option<Result<Vec<u32>, String>>,
    pub expanded: bool,
    pub preview:  Option<String>,
}

/// Complete state and cached derived views for one example execution.
pub struct ExampleRunRecord
{
    pub run_id:          u64,
    pub catalog_index:   usize,
    pub uri:             String,
    pub title:           String,
    pub job_id:          JobId,
    pub status:          ExampleRunStatus,
    pub nodes:           Vec<RunTreeNode>,
    pub selected_node:   Option<u32>,
    pub inspector_open:  bool,
    pub view_mode:       DetailViewMode,
    pub schematic:       Option<Cask2dLayout>,
    pub scene_3d:        Option<GeometryViewerState>,
    pub fresco:          Option<FrescoViewerState>,
    pub plot_2d:         Option<Plot2dState>,
    pub plot_3d:         Option<GeometryViewerState>,
    pub waveform:        Option<WaveformState>,
    pub waveform_busy:   bool,
    pub waveform_error:  Option<String>,
}

/// Actions emitted by the Examples Workspace UI.
#[derive(Debug, Clone)]
pub enum ExamplesWorkspaceAction
{
    SearchChanged(String),
    SelectSuite(Option<String>),
    ClearFilters,
    OpenDetail(usize),
    BackToGallery,
    RunExample(usize),
    SelectRun(u64),
    SwitchView(u64, DetailViewMode),
    ToggleNode(u64, u32),
    SelectNode(u64, u32),
    CloseInspector(u64),
    ViewRunDetails(JobId),
    RecordDaddaWaveform(u64),
    Schematic(u64, Cask2dAction),
    Scene3d(u64, GeometryAction),
    MathView(u64, FrescoAction),
    Plot2d(u64, Plot2dAction),
    Plot3d(u64, GeometryAction),
    Waveform(u64, WaveformAction),
}

/// Persistent workspace state owned by `AppState` (survives tab close/reopen).
pub struct ExamplesWorkspaceState
{
    _Examples:          Vec<&'static XplrExample>,
    _Catalog:           Vec<CatalogItem>,
    pub screen:         WorkspaceScreen,
    pub search_query:   String,
    pub suite_filter:   Option<String>,
    pub active_run_id:  Option<u64>,
    pub runs:           HashMap<u64, ExampleRunRecord>,
    pub latest_by_uri:  HashMap<String, u64>,
    pub run_order:      Vec<u64>,
    _NextRunId:         u64,
}

impl Default for ExamplesWorkspaceState
{
    fn default() -> Self { return Self::New(); }
}

impl ExamplesWorkspaceState
{
    pub fn New() -> Self
    {
        let reg = XplrExample::Registered();
        let mut examples = Vec::with_capacity(reg.Len() as usize);
        for i in 0..reg.Len() {
            examples.push(reg[i]);
        }
        let catalog = build_catalog(&examples);
        return Self {
            _Examples:     examples,
            _Catalog:      catalog,
            screen:        WorkspaceScreen::Gallery,
            search_query:  String::new(),
            suite_filter:  None,
            active_run_id: None,
            runs:          HashMap::new(),
            latest_by_uri: HashMap::new(),
            run_order:     Vec::new(),
            _NextRunId:    1,
        };
    }

    pub fn Catalog(&self) -> &[CatalogItem] { return &self._Catalog; }

    pub fn ExampleAt(&self, catalog_idx: usize) -> Option<&'static XplrExample>
    {
        return self._Examples.get(catalog_idx).copied();
    }

    pub fn IsBusy(&self) -> bool { return self.active_run_id.is_some(); }

    pub fn AllocateRun(&mut self, catalog_idx: usize, job_id: JobId) -> Option<(u64, &'static XplrExample)>
    {
        if self.active_run_id.is_some() {
            return None;
        }
        let ex = *self._Examples.get(catalog_idx)?;
        let item = self._Catalog.get(catalog_idx)?;
        let run_id = self._NextRunId;
        self._NextRunId += 1;
        let record = ExampleRunRecord {
            run_id,
            catalog_index: catalog_idx,
            uri: item.uri.clone(),
            title: item.title.clone(),
            job_id,
            status: ExampleRunStatus::Running,
            nodes: Vec::new(),
            selected_node: None,
            inspector_open: false,
            view_mode: DetailViewMode::ResultTree,
            schematic: None,
            scene_3d: None,
            fresco: None,
            plot_2d: None,
            plot_3d: None,
            waveform: None,
            waveform_busy: false,
            waveform_error: None,
        };
        self.runs.insert(run_id, record);
        self.run_order.push(run_id);
        self.active_run_id = Some(run_id);
        return Some((run_id, ex));
    }

    pub fn CompleteRun(&mut self, run_id: u64, completion: ExampleCompletion, palette: &ThemePalette)
    {
        if self.active_run_id != Some(run_id) {
            return;
        }
        let Some(result) = completion.Take() else {
            return;
        };
        self.active_run_id = None;
        let Some(rec) = self.runs.get_mut(&run_id) else {
            return;
        };
        if !matches!(rec.status, ExampleRunStatus::Running) {
            return;
        }
        match result {
            Ok(root) => {
                let root_dyn: Box<dyn Xplr> = root;
                let has_circuit = root_dyn.Circuit().is_some();
                let fresco_doc = root_dyn.BuildFrescoDocument();
                if let Some(circuit) = root_dyn.Circuit() {
                    rec.schematic = Some(CircuitSchematic(circuit, palette));
                    rec.view_mode = DetailViewMode::Schematic;
                } else if let Some(doc) = fresco_doc {
                    rec.fresco = Some(FrescoViewerState::New(doc));
                    rec.view_mode = DetailViewMode::MathView;
                } else {
                    rec.view_mode = DetailViewMode::ResultTree;
                }
                let preview = read_node_preview(root_dyn.as_ref());
                rec.nodes.push(RunTreeNode {
                    xplr: root_dyn,
                    children: None,
                    expanded: !has_circuit,
                    preview,
                });
                rec.selected_node = Some(0);
                rec.status = ExampleRunStatus::Passed;
                self.latest_by_uri.insert(rec.uri.clone(), run_id);
            }
            Err(err) => {
                rec.status = ExampleRunStatus::Failed(err);
            }
        }
    }

    pub fn CompleteWaveform(&mut self, run_id: u64, result: Result<String, String>)
    {
        let Some(rec) = self.runs.get_mut(&run_id) else {
            return;
        };
        rec.waveform_busy = false;
        match result {
            Ok(vcd) => {
                let wf = ParseWaveform("vcd", &vcd)
                    .map(|m| WaveformState::New(m.into()))
                    .unwrap_or_else(WaveformState::FromError);
                rec.waveform = Some(wf);
                rec.waveform_error = None;
                rec.view_mode = DetailViewMode::Waveform;
            }
            Err(err) => {
                rec.waveform_error = Some(err);
            }
        }
    }

    pub fn LatestRunFor(&self, catalog_idx: usize) -> Option<&ExampleRunRecord>
    {
        let item = self._Catalog.get(catalog_idx)?;
        let latest_id = self.latest_by_uri.get(&item.uri).copied();
        if let Some(active_id) = self.active_run_id
            && let Some(active_rec) = self.runs.get(&active_id)
            && active_rec.catalog_index == catalog_idx
        {
            return Some(active_rec);
        }
        return latest_id.and_then(|id| self.runs.get(&id));
    }

    pub fn ToggleOrExpandNode(&mut self, run_id: u64, node_id: u32)
    {
        let Some(rec) = self.runs.get_mut(&run_id) else { return; };
        let Some(node) = rec.nodes.get(node_id as usize) else { return; };
        if node.expanded {
            if let Some(n) = rec.nodes.get_mut(node_id as usize) {
                n.expanded = false;
            }
            return;
        }
        if node.children.is_none() {
            let children_res = node.xplr.Branch().map(|b| b.Children());
            match children_res {
                Some(Ok(buff)) => {
                    let unpacked = unpack_xplr_children(buff);
                    let mut child_ids = Vec::with_capacity(unpacked.len());
                    for child in unpacked {
                        let id = rec.nodes.len() as u32;
                        let preview = read_node_preview(child.as_ref());
                        rec.nodes.push(RunTreeNode {
                            xplr: child,
                            children: None,
                            expanded: false,
                            preview,
                        });
                        child_ids.push(id);
                    }
                    if let Some(n) = rec.nodes.get_mut(node_id as usize) {
                        n.children = Some(Ok(child_ids));
                        n.expanded = true;
                    }
                }
                Some(Err(err)) => {
                    if let Some(n) = rec.nodes.get_mut(node_id as usize) {
                        n.children = Some(Err(err));
                        n.expanded = true;
                    }
                }
                None => {}
            }
        } else if let Some(n) = rec.nodes.get_mut(node_id as usize) {
            n.expanded = true;
        }
    }

    pub fn SelectNode(&mut self, run_id: u64, node_id: u32)
    {
        let Some(rec) = self.runs.get_mut(&run_id) else { return; };
        if (node_id as usize) < rec.nodes.len() {
            rec.selected_node = Some(node_id);
            rec.inspector_open = true;
        }
    }

    pub fn EnsureViewPrepared(&mut self, run_id: u64, mode: DetailViewMode)
    {
        let Some(rec) = self.runs.get_mut(&run_id) else { return; };
        match mode {
            DetailViewMode::Plot2d if rec.plot_2d.is_none() => {
                if let Some(ref mv) = rec.fresco {
                    if let Ok(series) = SampleFrescoPlot2d(mv.Document(), -5.0, 5.0, 240) {
                        rec.plot_2d = Some(Plot2dState::New(
                            format!("{} · 2D Plot", rec.title),
                            series,
                            -5.0,
                            5.0,
                        ));
                    }
                }
            }
            DetailViewMode::Plot3d if rec.plot_3d.is_none() => {
                if let Some(ref mv) = rec.fresco {
                    let root_idx = mv.SelectedRoot();
                    if let Ok(asset) = SampleFrescoSurface3d(mv.Document(), root_idx, (-3.0, 3.0), (-3.0, 3.0), 40) {
                        let mut gs = GeometryViewerState::default();
                        gs.Complete(Ok(Arc::new(asset)));
                        rec.plot_3d = Some(gs);
                    }
                }
            }
            DetailViewMode::Scene3d if rec.scene_3d.is_none() => {
                if let Some(root_node) = rec.nodes.first()
                    && let Some(circuit) = root_node.xplr.Circuit()
                {
                    let cancelled = AtomicBool::new(false);
                    if let Ok(asset) = CircuitBuild(circuit, &cancelled) {
                        let mut gs = GeometryViewerState::default();
                        gs.Complete(Ok(Arc::new(asset)));
                        rec.scene_3d = Some(gs);
                    }
                }
            }
            _ => {}
        }
        rec.view_mode = mode;
    }
}

fn read_node_preview(node: &dyn Xplr) -> Option<String>
{
    let leaf = node.Leaf()?;
    match leaf.ReadChunk(0, 4096) {
        Ok(chunk) => {
            let mut s = format!("{} ({} bytes)\n{}", node.Path(), leaf.Size(), chunk.Content());
            if !chunk.IsEof() {
                s.push_str("\n[Preview truncated at 4096 bytes]");
            }
            return Some(s);
        }
        Err(err) => return Some(format!("Read error: {}", err)),
    }
}

fn unpack_xplr_children(children: Buff<Box<dyn Xplr>>) -> Vec<Box<dyn Xplr>>
{
    let (ptr, len) = children.IntoRawParts();
    let mut out = Vec::with_capacity(len as usize);
    for idx in 0..len {
        let item = unsafe { std::ptr::read(ptr.add(idx as usize)) };
        out.push(item);
    }
    if len > 0 {
        let layout = std::alloc::Layout::array::<Box<dyn Xplr>>(len as usize).unwrap();
        unsafe { std::alloc::dealloc(ptr as *mut u8, layout); }
    }
    return out;
}

//-------------------------------------------------------------------------------------------------

/// Renders the Examples Workspace (Gallery or Detail screen).
pub fn ViewExamplesWorkspace<'a, Message: 'static + Clone>(
    state: &'a ExamplesWorkspaceState,
    palette: ThemePalette,
    on_action: impl Fn(ExamplesWorkspaceAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    match state.screen {
        WorkspaceScreen::Gallery => view_gallery(state, palette, on_action),
        WorkspaceScreen::Detail(catalog_idx) => view_detail(state, catalog_idx, palette, on_action),
    }
}

fn view_gallery<'a, Message: 'static + Clone>(
    state: &'a ExamplesWorkspaceState,
    palette: ThemePalette,
    on_action: impl Fn(ExamplesWorkspaceAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let header = row![
        ViewIcon(IconId::ExampleBolt, 16.0, palette.accent),
        Space::new().width(8),
        text("Examples").size(18).style(move |_| text::Style { color: Some(palette.text_primary) }),
        Space::new().width(Length::Fill),
        text_input("Search examples...", &state.search_query)
            .on_input(move |s| on_action(ExamplesWorkspaceAction::SearchChanged(s)))
            .width(Length::Fixed(240.0))
            .padding([6, 10])
            .size(12),
    ]
    .align_y(Alignment::Center);

    let catalog = state.Catalog();
    let mut suite_counts: Vec<(&'static str, usize)> = Vec::new();
    for item in catalog {
        if let Some(entry) = suite_counts.iter_mut().find(|(s, _)| *s == item.suite) {
            entry.1 += 1;
        } else {
            suite_counts.push((item.suite, 1));
        }
    }

    let all_active = state.suite_filter.is_none();
    let mut pills = row![
        button(text(format!("All {}", catalog.len())).size(12).style(move |_| text::Style {
            color: Some(if all_active { palette.accent } else { palette.text_secondary }),
        }))
        .padding([4, 10])
        .style(move |_, st| FasciaStyle::tree_row_button(palette, all_active, st))
        .on_press(on_action(ExamplesWorkspaceAction::SelectSuite(None))),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    for (suite, count) in suite_counts {
        let is_active = state.suite_filter.as_deref() == Some(suite);
        let suite_owned = suite.to_string();
        pills = pills.push(
            button(text(format!("{} {}", suite, count)).size(12).style(move |_| text::Style {
                color: Some(if is_active { palette.accent } else { palette.text_secondary }),
            }))
            .padding([4, 10])
            .style(move |_, st| FasciaStyle::tree_row_button(palette, is_active, st))
            .on_press(on_action(ExamplesWorkspaceAction::SelectSuite(Some(suite_owned)))),
        );
    }

    let q = state.search_query.trim().to_lowercase();
    let mut filtered: Vec<&CatalogItem> = catalog
        .iter()
        .filter(|item| {
            let suite_ok = state
                .suite_filter
                .as_deref()
                .is_none_or(|s| s == item.suite);
            let query_ok = q.is_empty()
                || item.title.to_lowercase().contains(&q)
                || item.name.to_lowercase().contains(&q)
                || item.suite.to_lowercase().contains(&q)
                || item.summary.to_lowercase().contains(&q);
            suite_ok && query_ok
        })
        .collect();

    let is_default_all = q.is_empty() && state.suite_filter.is_none();
    if is_default_all {
        filtered.sort_by_key(|item| (item.featured_rank.unwrap_or(u8::MAX), item.catalog_index));
    } else {
        filtered.sort_by(|a, b| a.title.cmp(&b.title));
    }

    let mut body = column![].spacing(12).width(Length::Fill);
    if catalog.is_empty() {
        body = body.push(
            container(
                text("This build contains no registered XplrExamples.")
                    .size(13)
                    .style(move |_| text::Style { color: Some(palette.text_muted) }),
            )
            .padding(24),
        );
    } else if filtered.is_empty() {
        body = body.push(
            container(
                column![
                    text("No examples match the current filter.").size(13).style(move |_| text::Style {
                        color: Some(palette.text_secondary),
                    }),
                    button(text("Clear filters").size(12))
                        .padding([5, 12])
                        .style(move |_, st| FasciaStyle::toolbar_button(palette, st))
                        .on_press(on_action(ExamplesWorkspaceAction::ClearFilters)),
                ]
                .spacing(10),
            )
            .padding(20),
        );
    } else {
        if is_default_all {
            let featured: Vec<&CatalogItem> = filtered
                .iter()
                .copied()
                .filter(|item| item.featured_rank.is_some())
                .collect();
            for pair in featured.chunks(2) {
                let mut r = row![].spacing(12).width(Length::Fill);
                for item in pair {
                    r = r.push(view_featured_card(item, state.LatestRunFor(item.catalog_index), palette, on_action));
                }
                if pair.len() == 1 {
                    r = r.push(Space::new().width(Length::Fill));
                }
                body = body.push(r);
            }
            for item in filtered.into_iter().filter(|item| item.featured_rank.is_none()) {
                body = body.push(view_catalog_row(item, state.LatestRunFor(item.catalog_index), palette, on_action));
            }
        } else {
            for item in filtered {
                body = body.push(view_catalog_row(item, state.LatestRunFor(item.catalog_index), palette, on_action));
            }
        }
    }

    return container(
        column![
            header,
            Space::new().height(12),
            scrollable(pills).direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default().width(4.0),
            )),
            Space::new().height(12),
            scrollable(body).width(Length::Fill).height(Length::Fill),
        ]
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into();
}

fn view_featured_card<'a, Message: 'static + Clone>(
    item: &CatalogItem,
    latest_run: Option<&ExampleRunRecord>,
    palette: ThemePalette,
    on_action: impl Fn(ExamplesWorkspaceAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let status_badge = match latest_run.map(|r| &r.status) {
        Some(ExampleRunStatus::Running) => "Running…",
        Some(ExampleRunStatus::Passed) => "Passed",
        Some(ExampleRunStatus::Failed(_)) => "Failed",
        None => item.suite,
    };
    let card = container(
        column![
            row![
                text(item.title.clone()).size(14).style(move |_| text::Style {
                    color: Some(palette.text_primary),
                }),
                Space::new().width(Length::Fill),
                text(status_badge.to_string()).size(11).style(move |_| text::Style {
                    color: Some(palette.accent),
                }),
            ]
            .align_y(Alignment::Center),
            text(item.summary.clone()).size(12).style(move |_| text::Style {
                color: Some(palette.text_secondary),
            }),
            text(format!("{} · {}", item.suite, item.name))
                .size(11)
                .font(default_code_font(crate::fascia::FasciaTheme::default()))
                .style(move |_| text::Style { color: Some(palette.text_muted) }),
        ]
        .spacing(6),
    )
    .padding(14)
    .width(Length::Fill)
    .style(move |_| FasciaStyle::card_container(palette));

    return mouse_area(card)
        .on_press(on_action(ExamplesWorkspaceAction::OpenDetail(item.catalog_index)))
        .into();
}

fn view_catalog_row<'a, Message: 'static + Clone>(
    item: &CatalogItem,
    latest_run: Option<&ExampleRunRecord>,
    palette: ThemePalette,
    on_action: impl Fn(ExamplesWorkspaceAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let status_txt = match latest_run.map(|r| &r.status) {
        Some(ExampleRunStatus::Running) => "Running",
        Some(ExampleRunStatus::Passed) => "Passed",
        Some(ExampleRunStatus::Failed(_)) => "Failed",
        None => "",
    };
    let row_content = row![
        column![
            text(item.title.clone()).size(13).style(move |_| text::Style {
                color: Some(palette.text_primary),
            }),
            text(item.summary.clone()).size(11).style(move |_| text::Style {
                color: Some(palette.text_secondary),
            }),
        ]
        .spacing(2)
        .width(Length::Fill),
        text(item.suite).size(11).style(move |_| text::Style { color: Some(palette.text_muted) }),
        Space::new().width(12),
        text(status_txt.to_string()).size(11).style(move |_| text::Style {
            color: Some(palette.accent),
        }),
    ]
    .align_y(Alignment::Center);

    return button(row_content)
        .width(Length::Fill)
        .padding([8, 12])
        .style(move |_, st| FasciaStyle::tree_row_button(palette, false, st))
        .on_press(on_action(ExamplesWorkspaceAction::OpenDetail(item.catalog_index)))
        .into();
}

fn view_detail<'a, Message: 'static + Clone>(
    state: &'a ExamplesWorkspaceState,
    catalog_idx: usize,
    palette: ThemePalette,
    on_action: impl Fn(ExamplesWorkspaceAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let Some(item) = state.Catalog().get(catalog_idx) else {
        return view_gallery(state, palette, on_action);
    };
    let run = state.LatestRunFor(catalog_idx);
    let is_busy = state.IsBusy();

    let run_label = match run.map(|r| &r.status) {
        Some(ExampleRunStatus::Running) => "Running…",
        Some(ExampleRunStatus::Passed | ExampleRunStatus::Failed(_)) => "Run again",
        None => "Run example",
    };

    let header = row![
        button(text("< Examples").size(12))
            .padding([4, 10])
            .style(move |_, st| FasciaStyle::toolbar_button(palette, st))
            .on_press(on_action(ExamplesWorkspaceAction::BackToGallery)),
        Space::new().width(10),
        text(format!("{} / {}", item.suite, item.title)).size(15).style(move |_| text::Style {
            color: Some(palette.text_primary),
        }),
        Space::new().width(Length::Fill),
        button(
            row![
                ViewIcon(IconId::Play, 11.0, palette.accent),
                Space::new().width(6),
                text(run_label).size(12),
            ]
            .align_y(Alignment::Center),
        )
        .padding([5, 12])
        .style(move |_, st| FasciaStyle::tab_button(palette, true, st))
        .on_press_maybe((!is_busy).then(|| on_action(ExamplesWorkspaceAction::RunExample(catalog_idx)))),
    ]
    .align_y(Alignment::Center);

    let Some(rec) = run else {
        return container(
            column![
                header,
                Space::new().height(24),
                container(
                    column![
                        text(item.title.clone()).size(18).style(move |_| text::Style {
                            color: Some(palette.text_primary),
                        }),
                        text(item.summary.clone()).size(13).style(move |_| text::Style {
                            color: Some(palette.text_secondary),
                        }),
                        text(item.uri.clone()).size(11).style(move |_| text::Style {
                            color: Some(palette.text_muted),
                        }),
                    ]
                    .spacing(8),
                )
                .padding(20)
                .width(Length::Fill)
                .style(move |_| FasciaStyle::card_container(palette)),
            ]
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    };

    let rid = rec.run_id;
    let mut tabs_row = row![].spacing(6).align_y(Alignment::Center);
    if rec.schematic.is_some() {
        tabs_row = tabs_row.push(mode_tab("Schematic", DetailViewMode::Schematic, rec.view_mode, rid, palette, on_action));
        tabs_row = tabs_row.push(mode_tab("3D Hierarchy", DetailViewMode::Scene3d, rec.view_mode, rid, palette, on_action));
    }
    if rec.fresco.is_some() {
        tabs_row = tabs_row.push(mode_tab("MathView", DetailViewMode::MathView, rec.view_mode, rid, palette, on_action));
        tabs_row = tabs_row.push(mode_tab("2D Plot", DetailViewMode::Plot2d, rec.view_mode, rid, palette, on_action));
        tabs_row = tabs_row.push(mode_tab("3D Surface", DetailViewMode::Plot3d, rec.view_mode, rid, palette, on_action));
    }
    if rec.waveform.is_some() {
        tabs_row = tabs_row.push(mode_tab("Waveform", DetailViewMode::Waveform, rec.view_mode, rid, palette, on_action));
    }
    tabs_row = tabs_row.push(mode_tab("Result tree", DetailViewMode::ResultTree, rec.view_mode, rid, palette, on_action));
    tabs_row = tabs_row.push(Space::new().width(Length::Fill));

    if item.uri == DADDA_EXAMPLE_URI && matches!(rec.status, ExampleRunStatus::Passed) {
        let wf_label = if rec.waveform_busy { "Recording…" } else { "Record waveform" };
        tabs_row = tabs_row.push(
            button(text(wf_label).size(11))
                .padding([4, 10])
                .style(move |_, st| FasciaStyle::toolbar_button(palette, st))
                .on_press_maybe((!rec.waveform_busy).then(|| on_action(ExamplesWorkspaceAction::RecordDaddaWaveform(rid)))),
        );
    }

    let main_pane: Element<'a, Message> = match rec.status {
        ExampleRunStatus::Running => container(
            text("Executing example on Heist worker...")
                .size(13)
                .style(move |_| text::Style { color: Some(palette.text_secondary) }),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
        ExampleRunStatus::Failed(ref err) => container(
            column![
                text("Execution Failed").size(15).style(move |_| text::Style {
                    color: Some(palette.status_error()),
                }),
                text(err.clone()).size(12).style(move |_| text::Style {
                    color: Some(palette.text_primary),
                }),
            ]
            .spacing(8),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
        ExampleRunStatus::Passed => match rec.view_mode {
            DetailViewMode::Schematic => {
                if let Some(ref layout) = rec.schematic {
                    view_cask2d(layout, Length::Fill, Length::Fill)
                } else {
                    view_result_tree(rec, palette, on_action)
                }
            }
            DetailViewMode::Scene3d => {
                if let Some(ref gs) = rec.scene_3d {
                    ViewGeometry(rid, gs, palette, move |a| on_action(ExamplesWorkspaceAction::Scene3d(rid, a)))
                } else {
                    view_result_tree(rec, palette, on_action)
                }
            }
            DetailViewMode::MathView => {
                if let Some(ref mv) = rec.fresco {
                    ViewMathTab(mv, move |a| on_action(ExamplesWorkspaceAction::MathView(rid, a)), palette)
                } else {
                    view_result_tree(rec, palette, on_action)
                }
            }
            DetailViewMode::Plot2d => {
                if let Some(ref p2) = rec.plot_2d {
                    ViewPlot2d(p2, palette, move |a| on_action(ExamplesWorkspaceAction::Plot2d(rid, a)))
                } else {
                    view_result_tree(rec, palette, on_action)
                }
            }
            DetailViewMode::Plot3d => {
                if let Some(ref p3) = rec.plot_3d {
                    ViewGeometry(rid, p3, palette, move |a| on_action(ExamplesWorkspaceAction::Plot3d(rid, a)))
                } else {
                    view_result_tree(rec, palette, on_action)
                }
            }
            DetailViewMode::Waveform => {
                if let Some(ref wf) = rec.waveform {
                    view_waveform(wf, palette, move |a| on_action(ExamplesWorkspaceAction::Waveform(rid, a)))
                } else {
                    view_result_tree(rec, palette, on_action)
                }
            }
            DetailViewMode::ResultTree => view_result_tree(rec, palette, on_action),
        },
    };

    let mut content_row = row![
        container(main_pane)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| FasciaStyle::card_container(palette)),
    ]
    .spacing(10)
    .width(Length::Fill)
    .height(Length::Fill);

    if rec.inspector_open && let Some(sel) = rec.selected_node && let Some(node) = rec.nodes.get(sel as usize) {
        let mut insp = column![
            row![
                text("Inspector").size(12).style(move |_| text::Style { color: Some(palette.text_primary) }),
                Space::new().width(Length::Fill),
                button(ViewIcon(IconId::Close, 10.0, palette.text_muted))
                    .padding(3)
                    .style(move |_, st| FasciaStyle::tab_close_button(palette, st))
                    .on_press(on_action(ExamplesWorkspaceAction::CloseInspector(rid))),
            ]
            .align_y(Alignment::Center),
            text(node.xplr.Name().to_string()).size(13).style(move |_| text::Style {
                color: Some(palette.accent),
            }),
            text(node.xplr.Path().to_string()).size(11).style(move |_| text::Style {
                color: Some(palette.text_muted),
            }),
        ]
        .spacing(6);

        if let Some(ref prev) = node.preview {
            insp = insp.push(
                scrollable(
                    text(prev.clone())
                        .size(11)
                        .font(default_code_font(crate::fascia::FasciaTheme::default()))
                        .style(move |_| text::Style { color: Some(palette.text_secondary) }),
                )
                .height(Length::Fill),
            );
        }
        content_row = content_row.push(
            container(insp)
                .width(Length::Fixed(300.0))
                .height(Length::Fill)
                .padding(12)
                .style(move |_| FasciaStyle::card_container(palette)),
        );
    }

    let status_text = match &rec.status {
        ExampleRunStatus::Running => format!("Run {} · Running", rec.run_id),
        ExampleRunStatus::Passed => format!("Run {} · Passed", rec.run_id),
        ExampleRunStatus::Failed(_) => format!("Run {} · Failed", rec.run_id),
    };
    let footer = row![
        text(status_text).size(11).style(move |_| text::Style { color: Some(palette.text_secondary) }),
        Space::new().width(Length::Fill),
        button(text("View run details").size(11).style(move |_| text::Style { color: Some(palette.accent) }))
            .padding([2, 8])
            .style(move |_, st| FasciaStyle::toolbar_button(palette, st))
            .on_press(on_action(ExamplesWorkspaceAction::ViewRunDetails(rec.job_id))),
    ]
    .align_y(Alignment::Center);

    return container(
        column![
            header,
            Space::new().height(10),
            tabs_row,
            Space::new().height(8),
            content_row,
            Space::new().height(8),
            footer,
        ]
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into();
}

fn mode_tab<'a, Message: 'static + Clone>(
    label: &'static str,
    mode: DetailViewMode,
    active: DetailViewMode,
    run_id: u64,
    palette: ThemePalette,
    on_action: impl Fn(ExamplesWorkspaceAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let is_active = mode == active;
    let underline = container(Space::new().width(Length::Fill).height(Length::Fixed(2.0)))
        .style(move |_| container::Style {
            background: is_active.then_some(Background::Color(palette.accent)),
            ..Default::default()
        });
    return button(
        column![
            text(label).size(12).style(move |_| text::Style {
                color: Some(if is_active { palette.text_primary } else { palette.text_secondary }),
            }),
            underline,
        ]
        .spacing(2),
    )
    .padding([4, 10])
    .style(move |_, st| FasciaStyle::tree_row_button(palette, is_active, st))
    .on_press(on_action(ExamplesWorkspaceAction::SwitchView(run_id, mode)))
    .into();
}

fn view_result_tree<'a, Message: 'static + Clone>(
    rec: &'a ExampleRunRecord,
    palette: ThemePalette,
    on_action: impl Fn(ExamplesWorkspaceAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    if rec.nodes.is_empty() {
        return container(text("No result nodes.").size(12)).padding(16).into();
    }
    let mut rows_col = column![].spacing(2).width(Length::Fill);
    let mut stack: Vec<(u32, u32)> = vec![(0, 0)];
    while let Some((nid, depth)) = stack.pop() {
        let Some(node) = rec.nodes.get(nid as usize) else { continue; };
        let is_selected = rec.selected_node == Some(nid);
        let is_branch = node.xplr.Branch().is_some();
        let disclosure: Element<'a, Message> = if is_branch {
            button(ViewIcon(
                if node.expanded { IconId::ChevronDown } else { IconId::ChevronRight },
                11.0,
                palette.text_secondary,
            ))
            .padding(2)
            .style(move |_, st| FasciaStyle::tab_close_button(palette, st))
            .on_press(on_action(ExamplesWorkspaceAction::ToggleNode(rec.run_id, nid)))
            .into()
        } else {
            Space::new().width(Length::Fixed(15.0)).into()
        };
        let label_btn = button(
            text(node.xplr.Name().to_string()).size(12).style(move |_| text::Style {
                color: Some(if is_selected { palette.text_primary } else { palette.text_secondary }),
            }),
        )
        .width(Length::Fill)
        .padding([3, 6])
        .style(move |_, st| FasciaStyle::tree_row_button(palette, is_selected, st))
        .on_press(on_action(ExamplesWorkspaceAction::SelectNode(rec.run_id, nid)));

        rows_col = rows_col.push(
            container(row![disclosure, label_btn].spacing(4).align_y(Alignment::Center))
                .padding(iced::Padding::default().left((depth.min(16) as f32) * 14.0)),
        );

        if node.expanded {
            if let Some(Ok(ref children)) = node.children {
                for &cid in children.iter().rev() {
                    stack.push((cid, depth + 1));
                }
            } else if let Some(Err(ref err)) = node.children {
                rows_col = rows_col.push(
                    container(text(format!("Error: {}", err)).size(11).style(move |_| text::Style {
                        color: Some(palette.status_error()),
                    }))
                    .padding(iced::Padding::default().left(((depth + 1).min(16) as f32) * 14.0)),
                );
            }
        }
    }
    return scrollable(container(rows_col).padding(10).width(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
}
