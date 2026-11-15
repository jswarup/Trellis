//-- sketch_view.rs -------------------------------------------------------------------------------------------------
use crate::fascia::sketch_canvas::{SketchCanvasAction, SketchCanvasProgram, SketchCanvasState};
use crate::fascia::sketch_inspector::{
    SketchInspectorAction, SketchInspectorState, view_sketch_inspector,
};
use crate::fascia::sketch_tool::{SketchToolController, SketchToolKind};
use crate::fascia::tabs::TabId;
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::sketch::{
    Circle, CircularArc, Ellipse, LineSegment, PolylineHeader, SketchDocument, SketchEntity,
    SketchPlane, SketchPoint, SketchPrimitive,
};
use crate::fenst::sketch_edit::{SketchHistory, SketchPatchBuilder};
use crate::fleck::sketch_flux::SketchFlux;
use crate::fleck::sketch_io::SketchIo;
use crate::flux::{FluxFile, FluxFileFormat, FluxValue};
use crate::silo::IArr;
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Point};
use iced_aw::menu::{Item, Menu, MenuBar};
use std::fs;
use std::path::{Path, PathBuf};

//-------------------------------------------------------------------------------------------------

/// Action emitted by the sketch viewer tab and its contextual controls.
#[derive(Clone, Debug, PartialEq)]
pub enum SketchViewAction
{
    SetTool(SketchToolKind),
    Undo,
    Redo,
    Save,
    ExportSvg,
    PathChanged(String),
    OpenPath,
    SaveAs,
    SaveFormat(FluxFileFormat),
    Restore,
    ConfirmRestore,
    CancelRestore,
    Format(FluxFileFormat),
    ConfirmOverwrite,
    CancelOverwrite,
    ArcStart(String),
    ArcSweep(String),
    ApplyArc,
    CancelDraft,
    Zoom(f32, Option<[f32; 2]>),
    Pan(f32, f32),
    ResetView,
    FitAll,
    DeleteSelected,
    TogglePanel,
    SetWorkplane(SketchPlane),
    OffsetWorkplane(f64),
    ExtrudeDepthInput(String),
    Extrude3d(f64),
    View3dWireframe,
    OpenContextMenu([f32; 2]),
    DismissContextMenu,
    InspectorAction(SketchInspectorAction),
    CanvasAction(SketchCanvasAction),
}

//-------------------------------------------------------------------------------------------------

/// State and transactional document session for an interactive 2D sketch viewer tab.
pub struct SketchViewerState
{
    pub path:           Option<PathBuf>,
    pub title:          String,
    pub doc:            SketchDocument,
    pub history:        SketchHistory,
    pub tool_ctrl:      SketchToolController,
    pub canvas_state:   SketchCanvasState,
    pub inspector:      SketchInspectorState,
    pub zoom:           f32,
    pub pan:            [f32; 2],
    pub palette:        Option<ThemePalette>,
    pub dark_theme:     bool,
    pub size:           [f32; 2],
    pub status_message: Option<String>,
    pub context_menu:   Option<[f32; 2]>,
    _PathInput: String,
    _FileMetadata: Option<FluxValue>,
    _OverwritePath: Option<PathBuf>,
    _ConfirmRestore: bool,
    _ArcStart: String,
    _ArcSweep: String,
}

impl SketchViewerState
{
    pub fn New(path: Option<PathBuf>, title: String, palette: ThemePalette) -> Self
    {
        let doc = SketchDocument::New();
        Self::FromDocument(doc, path, title, palette)
    }

    pub fn FromDocument(doc: SketchDocument, path: Option<PathBuf>, title: String,
                        palette: ThemePalette)
                        -> Self
    {
        let is_dark = palette.is_dark();
        let history = SketchHistory::New(128);
        let mut tool_ctrl = SketchToolController::New();
        tool_ctrl.SetZoom(1.0);

        let mut inspector = SketchInspectorState::New();
        inspector.SyncFromDocument(&doc, tool_ctrl.SelectedEntityId());

        let pathInput = path.as_ref()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Sketch.flx".into());
        Self {
            path,
            title,
            doc,
            history,
            tool_ctrl,
            canvas_state: SketchCanvasState::New(),
            inspector,
            zoom: 1.0,
            pan: [400.0, 300.0],
            palette: Some(palette),
            dark_theme: is_dark,
            size: [800.0, 600.0],
            status_message: None,
            context_menu: None,
            _PathInput: pathInput,
            _FileMetadata: None,
            _OverwritePath: None,
            _ConfirmRestore: false,
            _ArcStart: "0".into(),
            _ArcSweep: "90".into(),
        }
    }

    pub fn FromFile(path: PathBuf, palette: ThemePalette) -> Result<Self, String>
    {
        let legacy = path.extension()
                         .is_some_and(|e| e.eq_ignore_ascii_case("tsketch"));
        let file = if legacy {
            None
        } else {
            Some(FluxFile::Read(&path)?)
        };
        let doc = if let Some(file) = &file {
            SketchFlux::Document(file)?
        } else {
            SketchIo::ParseSketch(&fs::read_to_string(&path).map_err(|e| e.to_string())?)?
        };
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled.flx")
            .to_string();

        let mut state = Self::FromDocument(doc, Some(path), title, palette);
        if let Some(file) = file {
            if !file.IntegrityVerified() { state.status_message = Some("Loaded editable text without an integrity checksum".into()); }
            state._FileMetadata = Some(file.Metadata().clone());
        }
        if !legacy {
            state._PathInput = state.path.as_ref().unwrap().to_string_lossy().into_owned();
        } else {
            state._PathInput = state.path.as_ref().unwrap().with_extension("flx").to_string_lossy().into_owned();
            state.path = None;
            state.status_message = Some("Legacy sketch loaded; save as .flx or .tflx".into());
        }
        state.history.MarkSaved(state.doc.Revision());
        state.FitAll();
        Ok(state)
    }

    pub fn SaveToFile(&mut self, target_path: &Path) -> Result<(), String>
    {
        let file = SketchFlux::File(&self.doc, self._FileMetadata.as_ref())?;
        file.Save(target_path)?;
        self._FileMetadata = Some(file.Metadata().clone());

        self.history.MarkSaved(self.doc.Revision());
        self.path = Some(target_path.to_path_buf());
        self._PathInput = target_path.to_string_lossy().into_owned();
        self.title = target_path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned();
        self.status_message = Some(format!("Saved {}", target_path.display()));
        Ok(())
    }

    pub fn ExportSvgString(&self) -> String
    {
        SketchIo::ExportSvg(&self.doc)
    }

    pub fn CanUndo(&self) -> bool
    {
        self.history.CanUndo()
    }

    pub fn CanRedo(&self) -> bool
    {
        self.history.CanRedo()
    }

    pub fn IsDirty(&self) -> bool
    {
        self.history.IsDirty(self.doc.Revision())
    }

    pub fn Undo(&mut self)
    {
        if let Ok(Some(_)) = self.history.Undo(&mut self.doc) {
            self.canvas_state.InvalidateGeometry();
            self.tool_ctrl.ResetDraft();
            self.ReconcileSelection();
            self.inspector.ResetSync();
            self.inspector.SyncFromDocument(&self.doc, self.tool_ctrl.SelectedEntityId());
        }
    }

    pub fn Redo(&mut self)
    {
        if let Ok(Some(_)) = self.history.Redo(&mut self.doc) {
            self.canvas_state.InvalidateGeometry();
            self.tool_ctrl.ResetDraft();
            self.ReconcileSelection();
            self.inspector.ResetSync();
            self.inspector.SyncFromDocument(&self.doc, self.tool_ctrl.SelectedEntityId());
        }
    }

    pub fn ResetView(&mut self)
    {
        self.zoom = 1.0;
        self.pan = [self.size[0] * 0.5, self.size[1] * 0.5];
        self.tool_ctrl.SetZoom(1.0);
        self.canvas_state.InvalidateAll();
    }

    pub fn FitAll(&mut self)
    {
        let bounds = self.doc.ComputeBounds();
        let span_x = (bounds._Max._X - bounds._Min._X).abs();
        let span_y = (bounds._Max._Y - bounds._Min._Y).abs();

        if self.doc.EntityCount() > 0 && self.size[0] > 50.0 && self.size[1] > 50.0 {
            let margin = 40.0f32;
            let avail_w = (self.size[0] - margin * 2.0).max(50.0);
            let avail_h = (self.size[1] - margin * 2.0).max(50.0);

            let scale_x = avail_w / span_x.max(1.0) as f32;
            let scale_y = avail_h / span_y.max(1.0) as f32;
            let fit_zoom = scale_x.min(scale_y).clamp(0.01, 100.0);

            let center_x = (bounds._Min._X + bounds._Max._X) * 0.5;
            let center_y = (bounds._Min._Y + bounds._Max._Y) * 0.5;

            self.zoom = fit_zoom;
            self.pan = [
                self.size[0] * 0.5 - (center_x as f32) * fit_zoom,
                self.size[1] * 0.5 - (center_y as f32) * fit_zoom,
            ];
            self.tool_ctrl.SetZoom(fit_zoom as f64);
            self.canvas_state.InvalidateAll();
        } else {
            self.ResetView();
        }
    }

    pub fn ScreenToModel(&self, pt: Point) -> SketchPoint
    {
        SketchPoint::New(
            (pt.x as f64 - self.pan[0] as f64) / self.zoom as f64,
            (pt.y as f64 - self.pan[1] as f64) / self.zoom as f64,
        )
    }

    pub fn ModelToScreen(&self, p: SketchPoint) -> Point
    {
        Point::new(
            (p._X * self.zoom as f64 + self.pan[0] as f64) as f32,
            (p._Y * self.zoom as f64 + self.pan[1] as f64) as f32,
        )
    }

    pub fn Generate3dWireframe(&self) -> Result<crate::fleck::GeometryAsset, String>
    {
        let options = crate::fleck::Sketch3dProjectionOptions::default();
        crate::fleck::Sketch3dProjection::ProjectDocument(&self.doc, options)
    }

    pub fn Generate3dExtrude(&self, depth: f64) -> Result<crate::fleck::GeometryAsset, String>
    {
        crate::fleck::SketchExtrude::ExtrudeDocument(&self.doc, depth)
    }

    pub fn ProcessAction(&mut self, action: SketchViewAction)
    {
        match action {
            SketchViewAction::PathChanged(path) => { self._PathInput = path; self._OverwritePath = None; }
            SketchViewAction::OpenPath => { self.status_message = Some("Use Open to restore the path in a new tab".into()); }
            SketchViewAction::Format(format) => {
                self._OverwritePath = None;
                self._PathInput = Path::new(&self._PathInput).with_extension(if format == FluxFileFormat::Binary { "flx" } else { "tflx" }).to_string_lossy().into_owned();
            }
            SketchViewAction::SaveAs => { self.RequestSave(PathBuf::from(self._PathInput.trim())); }
            SketchViewAction::SaveFormat(format) => {
                let p = Path::new(&self._PathInput).with_extension(if format == FluxFileFormat::Binary { "flx" } else { "tflx" });
                self._PathInput = p.to_string_lossy().into_owned();
                self.RequestSave(p);
            }
            SketchViewAction::Restore if self.IsDirty() && self.path.is_some() => {
                self._ConfirmRestore = true;
            }
            SketchViewAction::CancelRestore => self._ConfirmRestore = false,
            SketchViewAction::Restore | SketchViewAction::ConfirmRestore => {
                self._ConfirmRestore = false;
                if let Some(path) = self.path.clone() {
                    let palette = self.palette.unwrap_or_else(|| crate::fascia::theme::FasciaTheme::WindowsDark.palette());
                    match Self::FromFile(path.clone(), palette) {
                        Ok(reloaded) => {
                            self.doc = reloaded.doc;
                            self.history = reloaded.history;
                            self._FileMetadata = reloaded._FileMetadata;
                            self.canvas_state.InvalidateAll();
                            self.tool_ctrl.SetSelectedEntityId(u32::MAX);
                            self.tool_ctrl.SetActiveLayerId(0);
                            self.inspector.ResetSync();
                            self.inspector.SyncFromDocument(&self.doc, self.tool_ctrl.SelectedEntityId());
                            self.status_message = Some(format!("Restored {}", path.display()));
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Restore failed: {e}"));
                        }
                    }
                } else {
                    self.status_message = Some("No saved file to restore from".into());
                }
            }
            SketchViewAction::ConfirmOverwrite => {
                if let Some(path) = self._OverwritePath.take() { self.SaveReporting(path); }
            }
            SketchViewAction::CancelOverwrite => { self._OverwritePath = None; }
            SketchViewAction::ArcStart(value) => self._ArcStart = value,
            SketchViewAction::ArcSweep(value) => self._ArcSweep = value,
            SketchViewAction::ApplyArc => {
                if let Err(error) = self.ApplyArc() { self.status_message = Some(error); }
            }
            SketchViewAction::CancelDraft => self.tool_ctrl.ResetDraft(),
            SketchViewAction::SetTool(tool) => {
                self.tool_ctrl.SetTool(tool);
            }
            SketchViewAction::Undo => {
                self.Undo();
            }
            SketchViewAction::Redo => {
                self.Redo();
            }
            SketchViewAction::ResetView => {
                self.ResetView();
            }
            SketchViewAction::FitAll => {
                self.FitAll();
            }
            SketchViewAction::DeleteSelected => {
                let id = self.tool_ctrl.SelectedEntityId();
                self.ProcessInspectorAction(SketchInspectorAction::DeleteEntity(id));
            }
            SketchViewAction::TogglePanel => {
                self.inspector.ToggleShowPanel();
            }
            SketchViewAction::InspectorAction(act) => {
                self.ProcessInspectorAction(act);
            }
            SketchViewAction::Save => {
                if let Some(path) = self.path.clone() { self.SaveReporting(path); }
                else { self.RequestSave(PathBuf::from(self._PathInput.trim())); }
            }
            SketchViewAction::ExportSvg => {
                if let Some(path) = &self.path {
                    let svg_path = path.with_extension("svg");
                    let svg_text = self.ExportSvgString();
                    self.status_message = Some(match fs::write(&svg_path, svg_text) {
                        Ok(()) => format!("Exported SVG: {}", svg_path.display()), Err(error) => format!("SVG export failed: {error}"),
                    });
                } else { self.status_message = Some("Save the sketch before exporting SVG".into()); }
            }
            SketchViewAction::Pan(dx, dy) => {
                if !dx.is_finite() || !dy.is_finite() { return; }
                self.pan[0] += dx;
                self.pan[1] += dy;
                self.canvas_state.InvalidateAll();
            }
            SketchViewAction::Zoom(factor, focal) => {
                if !factor.is_finite() || factor <= 0.0
                    || focal.is_some_and(|p| !p[0].is_finite() || !p[1].is_finite()) { return; }
                let old_zoom = self.zoom;
                let new_zoom = (old_zoom * factor).clamp(0.005, 500.0);

                let [fx, fy] = focal.unwrap_or([self.size[0] * 0.5, self.size[1] * 0.5]);
                let world_x = (fx - self.pan[0]) / old_zoom;
                let world_y = (fy - self.pan[1]) / old_zoom;
                self.pan[0] = fx - world_x * new_zoom;
                self.pan[1] = fy - world_y * new_zoom;

                self.zoom = new_zoom;
                self.tool_ctrl.SetZoom(new_zoom as f64);
                self.canvas_state.InvalidateAll();
            }
            SketchViewAction::CanvasAction(canvas_act) => match canvas_act {
                SketchCanvasAction::Pan(dx, dy) => {
                    self.ProcessAction(SketchViewAction::Pan(dx, dy));
                }
                SketchCanvasAction::Undo => self.Undo(),
                SketchCanvasAction::Redo => self.Redo(),
                SketchCanvasAction::Save => self.ProcessAction(SketchViewAction::Save),
                SketchCanvasAction::Tool(tool) => self.tool_ctrl.SetTool(tool),
                SketchCanvasAction::Zoom(factor, focal) => {
                    self.ProcessAction(SketchViewAction::Zoom(factor, focal));
                }
                SketchCanvasAction::Resize(w, h) => {
                    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 { return; }
                    self.pan[0] += (w - self.size[0]) * 0.5;
                    self.pan[1] += (h - self.size[1]) * 0.5;
                    self.size = [w, h];
                    self.canvas_state.last_size = [w, h];
                    self.canvas_state.InvalidateAll();
                }
                SketchCanvasAction::Input(event) => {
                    self.context_menu = None;
                    let oldSelection = self.tool_ctrl.SelectedEntityId();
                    let oldRevision = self.doc.Revision();
                    self.tool_ctrl.ProcessEvent(&self.doc, event);
                    if let Some(patch) = self.tool_ctrl.TakeCommittedPatch() {
                        if let Err(error) = self.history.CommitPatch(&mut self.doc, patch) { self.status_message = Some(error); }
                        self.canvas_state.InvalidateGeometry();
                        self.inspector.ResetSync();
                        self.inspector.SyncFromDocument(&self.doc, self.tool_ctrl.SelectedEntityId());
                    }
                    if self.tool_ctrl.SelectedEntityId() != oldSelection || self.doc.Revision() != oldRevision {
                        self.inspector.ResetSync();
                        self.ReconcileSelection();
                    }
                }
                SketchCanvasAction::ContextMenu(pos) => {
                    self.context_menu = Some(pos);
                }
            },
            SketchViewAction::OpenContextMenu(pos) => {
                self.context_menu = Some(pos);
            }
            SketchViewAction::DismissContextMenu => {
                self.context_menu = None;
            }
            SketchViewAction::OffsetWorkplane(distance) => {
                let offset_plane = self.doc.Plane().WithOffset(distance);
                self.doc.SetPlane(offset_plane);
                self.canvas_state.InvalidateAll();
                self.inspector.ResetSync();
                self.inspector.SyncFromDocument(&self.doc, self.tool_ctrl.SelectedEntityId());
                self.status_message = Some(format!("Workplane offset by {:+.1} mm", distance));
            }
            SketchViewAction::SetWorkplane(plane) => {
                self.context_menu = None;
                if *self.doc.Plane() == plane { return; }
                self.doc.SetPlane(plane);
                self.canvas_state.InvalidateAll();
                self.inspector.ResetSync();
                self.inspector.SyncFromDocument(&self.doc, self.tool_ctrl.SelectedEntityId());
                self.status_message = Some(format!("Workplane set to {}", plane.PresetName()));
            }
            SketchViewAction::ExtrudeDepthInput(val) => {
                self.inspector.SetExtrudeDepth(val);
            }
            SketchViewAction::Extrude3d(_) | SketchViewAction::View3dWireframe => {}
        }
    }

    pub fn ProcessInspectorAction(&mut self, act: SketchInspectorAction)
    {
        let target = match &act {
            SketchInspectorAction::DeleteEntity(id)
            | SketchInspectorAction::DuplicateEntity(id)
            | SketchInspectorAction::ToggleEntityConstruction(id) => *id,
            SketchInspectorAction::SetEntityLayer { entity_id, .. } => *entity_id,
            _ => u32::MAX,
        };
        if target != u32::MAX && !self.CanEditEntity(target) {
            return;
        }
        match act {
            SketchInspectorAction::SelectTab(tab) => {
                self.inspector.SetActiveTab(tab);
                self.inspector.SyncFromDocument(&self.doc, self.tool_ctrl.SelectedEntityId());
            }
            SketchInspectorAction::ToggleLayersExpanded => {
                self.inspector.ToggleLayers();
            }
            SketchInspectorAction::ToggleGroupsExpanded => {
                self.inspector.ToggleGroups();
            }
            SketchInspectorAction::ToggleEntitiesExpanded => {
                self.inspector.ToggleEntities();
            }
            SketchInspectorAction::SelectEntity(id) => {
                if !self.CanEditEntity(id) { return; }
                self.tool_ctrl.SetSelectedEntityId(id);
                self.inspector.ResetSync();
                self.inspector.SyncFromDocument(&self.doc, id);
                self.canvas_state.InvalidateAll();

            }
            SketchInspectorAction::DeleteEntity(id) => {
                if let Some(entity) = self.doc.FindEntity(id) {
                    let mut builder = SketchPatchBuilder::New();
                    builder.RemoveEntity(entity.clone());
                    let patch = builder.Build();
                    if let Err(e) = self.history.CommitPatch(&mut self.doc, patch) {
                        self.status_message = Some(e);
                    } else {
                        self.status_message = Some(format!("Deleted entity #{}", id));
                    }
                    if self.tool_ctrl.SelectedEntityId() == id {
                        self.tool_ctrl.SetSelectedEntityId(u32::MAX);
                    }
                    self.inspector.ResetSync();
                    self.inspector.SyncFromDocument(&self.doc, self.tool_ctrl.SelectedEntityId());
                    self.canvas_state.InvalidateGeometry();
                }
            }
            SketchInspectorAction::DuplicateEntity(id) => {
                self.DuplicateEntity(id);
            }
            SketchInspectorAction::ToggleEntityConstruction(id) => {
                if let Some(entity) = self.doc.FindEntity(id) {
                    let mut updated = entity.clone();
                    let is_const = !entity.IsConstruction();
                    updated = updated.WithConstruction(is_const);
                    let mut builder = SketchPatchBuilder::New();
                    builder.ReplaceEntity(entity.clone(), updated);
                    let patch = builder.Build();
                    if let Err(e) = self.history.CommitPatch(&mut self.doc, patch) {
                        self.status_message = Some(e);
                    } else {
                        self.status_message = Some(format!("Entity #{} construction: {}", id, is_const));
                    }
                    self.canvas_state.InvalidateGeometry();
                    self.inspector.ResetSync();
                    self.inspector.SyncFromDocument(&self.doc, id);
                }
            }
            SketchInspectorAction::SetEntityLayer { entity_id, layer_id } => {
                if !SketchToolController::IsLayerEditable(&self.doc, layer_id) {
                    self.status_message = Some("Choose a visible, unlocked layer".into());
                    return;
                }
                if let Some(entity) = self.doc.FindEntity(entity_id) {
                    let updated = entity.clone().WithLayer(layer_id);
                    let mut builder = SketchPatchBuilder::New();
                    builder.ReplaceEntity(entity.clone(), updated);
                    let patch = builder.Build();
                    if let Err(e) = self.history.CommitPatch(&mut self.doc, patch) {
                        self.status_message = Some(e);
                    }
                    self.canvas_state.InvalidateGeometry();
                    self.inspector.ResetSync();
                    self.inspector.SyncFromDocument(&self.doc, entity_id);
                }
            }
            SketchInspectorAction::SetActiveLayer(id) => {
                self.tool_ctrl.SetActiveLayerId(id);
            }
            SketchInspectorAction::ToggleLayerVisible(id) => {
                self.tool_ctrl.ResetDraft();
                if let Some(layer) = self.doc.FindLayerMut(id) {
                    let vis = !layer.IsVisible();
                    layer.SetVisible(vis);
                    self.doc.Touch();
                    self.canvas_state.InvalidateAll();
                    self.status_message = Some(format!("Layer #{} visibility: {}", id, vis));
                }
            }
            SketchInspectorAction::ToggleLayerLocked(id) => {
                self.tool_ctrl.ResetDraft();
                if let Some(layer) = self.doc.FindLayerMut(id) {
                    let locked = !layer.IsLocked();
                    layer.SetLocked(locked);
                    self.doc.Touch();
                    self.canvas_state.InvalidateAll();
                    self.status_message = Some(format!("Layer #{} locked: {}", id, locked));
                }
            }
            SketchInspectorAction::ToggleLayerConstruction(id) => {
                self.tool_ctrl.ResetDraft();
                if let Some(layer) = self.doc.FindLayerMut(id) {
                    let c = !layer.IsConstruction();
                    layer.SetConstruction(c);
                    self.doc.Touch();
                    self.canvas_state.InvalidateAll();
                    self.status_message = Some(format!("Layer #{} construction: {}", id, c));
                }
            }
            SketchInspectorAction::FieldInput { index, value } => {
                self.inspector.SetField(index, value);
            }
            SketchInspectorAction::ApplyFieldEdits => {
                self.ApplyInspectorEdits();
            }
            SketchInspectorAction::FitAll => {
                self.FitAll();
            }
            SketchInspectorAction::ResetView => {
                self.ResetView();
            }
            SketchInspectorAction::SetPlane(plane) => {
                self.ProcessAction(SketchViewAction::SetWorkplane(plane));
            }
            SketchInspectorAction::ExtrudeDepthInput(val) => {
                self.ProcessAction(SketchViewAction::ExtrudeDepthInput(val));
            }
            SketchInspectorAction::Extrude3d => {
                if let Ok(depth) = self.ExtrudeDepth() {
                    self.ProcessAction(SketchViewAction::Extrude3d(depth));
                } else {
                    self.status_message = Some("Enter a finite, non-zero extrusion depth".into());
                }
            }
            SketchInspectorAction::View3dWireframe => {
                self.ProcessAction(SketchViewAction::View3dWireframe);
            }
            SketchInspectorAction::ClosePanel => {
                self.inspector.SetShowPanel(false);
            }
        }
        self.ReconcileSelection();
    }

    pub fn ExtrudeDepth(&self) -> Result<f64, String>
    {
        let depth = self.inspector
                        .ExtrudeDepth()
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| "Enter a numeric extrusion depth")?;
        if !depth.is_finite() || depth.abs() < 1e-6 {
            return Err("Enter a finite, non-zero extrusion depth".into());
        }
        return Ok(depth);
    }

    fn CanEditEntity(&mut self, id: u32) -> bool
    {
        let editable =
            self.doc
                .FindEntity(id)
                .is_some_and(|entity| {
                    SketchToolController::IsLayerEditable(&self.doc, entity.LayerId())
                });
        if !editable {
            self.status_message = Some("Select an entity on a visible, unlocked layer".into());
        }
        return editable;
    }

    fn ReconcileSelection(&mut self)
    {
        let id = self.tool_ctrl.SelectedEntityId();
        if !self.doc
                .FindEntity(id)
                .is_some_and(|entity| {
                    SketchToolController::IsLayerEditable(&self.doc, entity.LayerId())
                })
        {
            self.tool_ctrl.SetSelectedEntityId(u32::MAX);
        }
        self.inspector
            .SyncFromDocument(&self.doc, self.tool_ctrl.SelectedEntityId());
        if let Some(entity) = self.doc.FindEntity(self.tool_ctrl.SelectedEntityId()) {
            let angles = match entity.Primitive() {
                SketchPrimitive::Circle(_) => Some((0.0, std::f64::consts::TAU)),
                SketchPrimitive::Arc(arc) => Some((arc._StartAngle, arc._SweepAngle)),
                SketchPrimitive::Ellipse(ellipse) => {
                    Some((ellipse._StartAngle, ellipse._SweepAngle))
                }
                _ => None,
            };
            if let Some((start, sweep)) = angles {
                self._ArcStart = start.to_degrees().to_string();
                self._ArcSweep = sweep.to_degrees().to_string();
            }
        }
    }

    pub fn DuplicateEntity(&mut self, id: u32)
    {
        if !self.CanEditEntity(id) {
            return;
        }
        if let Some(entity) = self.doc.FindEntity(id) {
            let mut next_id = self.doc.NextEntityId();
            self.doc.Entities().Traverse(|e| {
                next_id = next_id.max(e.Id().saturating_add(1));
            });
            let offset_x = 10.0;
            let offset_y = 10.0;
            let (prim, extra_points) = match entity.Primitive() {
                SketchPrimitive::Line(l) => (
                    SketchPrimitive::Line(LineSegment::New(
                        SketchPoint::New(l._Start._X + offset_x, l._Start._Y + offset_y),
                        SketchPoint::New(l._End._X + offset_x, l._End._Y + offset_y),
                    )),
                    None,
                ),
                SketchPrimitive::Circle(c) => (
                    SketchPrimitive::Circle(Circle::New(
                        SketchPoint::New(c._Center._X + offset_x, c._Center._Y + offset_y),
                        c._Radius,
                    )),
                    None,
                ),
                SketchPrimitive::Arc(a) => (
                    SketchPrimitive::Arc(CircularArc::New(
                        SketchPoint::New(a._Center._X + offset_x, a._Center._Y + offset_y),
                        a._Radius,
                        a._StartAngle,
                        a._SweepAngle,
                    )),
                    None,
                ),
                SketchPrimitive::Ellipse(e) => (
                    SketchPrimitive::Ellipse(
                        Ellipse::New(
                            SketchPoint::New(e._Center._X + offset_x, e._Center._Y + offset_y),
                            e._SemiMajor,
                            e._SemiMinor,
                            e._Rotation,
                        )
                        .WithArc(e._StartAngle, e._SweepAngle),
                    ),
                    None,
                ),
                SketchPrimitive::Polyline(p) => {
                    let pts = self.doc.PolylinePoints(p._PointOffset, p._PointCount);
                    let offset = self.doc.PointArena().Len();
                    let mut offset_pts = crate::silo::Stash::WithCapacity(pts.Len());
                    pts.Arr().Traverse(|pt| {
                        offset_pts.Push(SketchPoint::New(pt._X + offset_x, pt._Y + offset_y));
                    });
                    let new_header = PolylineHeader::New(offset, p._PointCount, p._IsClosed);
                    (SketchPrimitive::Polyline(new_header), Some((offset, offset_pts.IntoBuff())))
                }
            };

            let dup_ent = SketchEntity::New(next_id, prim)
                .WithLayer(entity.LayerId())
                .WithGroup(entity.GroupId())
                .WithConstruction(entity.IsConstruction());

            let mut builder = SketchPatchBuilder::New();
            if let Some((offset, pts_buff)) = extra_points {
                builder.AppendPoints(offset, pts_buff.Arr());
            }
            builder.AddEntity(dup_ent);
            let patch = builder.Build();
            if let Err(e) = self.history.CommitPatch(&mut self.doc, patch) {
                self.status_message = Some(e);
            } else {
                self.status_message = Some(format!("Duplicated entity #{} -> #{}", id, next_id));
                self.tool_ctrl.SetSelectedEntityId(next_id);
                self.inspector.ResetSync();
                self.inspector.SyncFromDocument(&self.doc, next_id);
                self.canvas_state.InvalidateGeometry();
            }
        }
    }

    fn ParsePrimitiveFromFields(&self, entity: &SketchEntity) -> Result<SketchPrimitive, String>
    {
        let parse_f64 = |val: &str, field_name: &str| -> Result<f64, String> {
            val.trim().parse::<f64>().map_err(|_| format!("Invalid numeric value for {}", field_name))
        };
        let preserveAngle = |degrees: f64, original: f64| {
            if degrees == original.to_degrees() {
                original
            } else {
                degrees.to_radians()
            }
        };

        match entity.Primitive() {
            SketchPrimitive::Line(_) => {
                let x0 = parse_f64(self.inspector.Field(0), "Start X")?;
                let y0 = parse_f64(self.inspector.Field(1), "Start Y")?;
                let x1 = parse_f64(self.inspector.Field(2), "End X")?;
                let y1 = parse_f64(self.inspector.Field(3), "End Y")?;
                let p0 = SketchPoint::New(x0, y0);
                let p1 = SketchPoint::New(x1, y1);
                if !p0.IsFinite() || !p1.IsFinite() {
                    return Err("Line endpoints must be finite numbers".into());
                }
                if p0.DistanceSqTo(p1) < 1e-12 {
                    return Err("Line segment length must be positive".into());
                }
                Ok(SketchPrimitive::Line(LineSegment::New(p0, p1)))
            }
            SketchPrimitive::Circle(_) => {
                let cx = parse_f64(self.inspector.Field(0), "Center X")?;
                let cy = parse_f64(self.inspector.Field(1), "Center Y")?;
                let r = parse_f64(self.inspector.Field(2), "Radius")?;
                let c = SketchPoint::New(cx, cy);
                if !c.IsFinite() || !r.is_finite() || r <= 1e-9 {
                    return Err("Circle radius must be strictly positive".into());
                }
                Ok(SketchPrimitive::Circle(Circle::New(c, r)))
            }
            SketchPrimitive::Arc(original) => {
                let cx = parse_f64(self.inspector.Field(0), "Center X")?;
                let cy = parse_f64(self.inspector.Field(1), "Center Y")?;
                let r = parse_f64(self.inspector.Field(2), "Radius")?;
                let start_deg = parse_f64(self.inspector.Field(3), "Start Angle")?;
                let sweep_deg = parse_f64(self.inspector.Field(4), "Sweep Angle")?;
                let c = SketchPoint::New(cx, cy);
                if !c.IsFinite() || !r.is_finite() || r <= 1e-9 {
                    return Err("Arc radius must be strictly positive".into());
                }
                if !start_deg.is_finite() || !sweep_deg.is_finite() || sweep_deg.abs() <= 1e-7 || sweep_deg.abs() > 360.0 {
                    return Err("Arc sweep must be between -360 and 360 degrees (non-zero)".into());
                }
                Ok(SketchPrimitive::Arc(CircularArc::New(
                    c,
                    r,
                    preserveAngle(start_deg, original._StartAngle),
                    preserveAngle(sweep_deg, original._SweepAngle),
                )))
            }
            SketchPrimitive::Ellipse(original) => {
                let cx = parse_f64(self.inspector.Field(0), "Center X")?;
                let cy = parse_f64(self.inspector.Field(1), "Center Y")?;
                let a = parse_f64(self.inspector.Field(2), "Semi-Major")?;
                let b = parse_f64(self.inspector.Field(3), "Semi-Minor")?;
                let rot_deg = parse_f64(self.inspector.Field(4), "Rotation")?;
                let start_deg = parse_f64(self.inspector.Field(5), "Start Angle")?;
                let sweep_deg = parse_f64(self.inspector.Field(6), "Sweep Angle")?;
                let c = SketchPoint::New(cx, cy);
                if !c.IsFinite() || !a.is_finite() || !b.is_finite() || a <= 1e-9 || b <= 1e-9 {
                    return Err("Ellipse axes must be strictly positive".into());
                }
                if !rot_deg.is_finite() || !start_deg.is_finite() || !sweep_deg.is_finite() || sweep_deg.abs() <= 1e-7 || sweep_deg.abs() > 360.0 {
                    return Err("Sweep angle must be between -360 and 360 degrees (non-zero)".into());
                }
                Ok(SketchPrimitive::Ellipse(
                    Ellipse::New(c, a, b, preserveAngle(rot_deg, original._Rotation))
                        .WithArc(preserveAngle(start_deg, original._StartAngle), preserveAngle(sweep_deg, original._SweepAngle)),
                ))
            }
            SketchPrimitive::Polyline(p) => Ok(SketchPrimitive::Polyline(*p)),
        }
    }

    pub fn ApplyInspectorEdits(&mut self)
    {
        let sel_id = self.tool_ctrl.SelectedEntityId();
        if !self.CanEditEntity(sel_id) {
            return;
        }
        let entity = match self.doc.FindEntity(sel_id) {
            Some(e) => e.clone(),
            None => {
                self.status_message = Some("No entity selected to apply edits to".into());
                return;
            }
        };

        match self.ParsePrimitiveFromFields(&entity) {
            Ok(prim) => {
                if prim == *entity.Primitive() {
                    self.status_message = Some("No property changes detected".into());
                    return;
                }
                let mut updated = entity.clone();
                updated.SetPrimitive(prim);
                let mut builder = SketchPatchBuilder::New();
                builder.ReplaceEntity(entity, updated);
                let patch = builder.Build();
                if let Err(e) = self.history.CommitPatch(&mut self.doc, patch) {
                    self.status_message = Some(e);
                } else {
                    self.status_message = Some("Properties updated successfully".into());
                }
                self.tool_ctrl.ResetDraft();
                self.canvas_state.InvalidateGeometry();
                self.inspector.ResetSync();
                self.inspector.SyncFromDocument(&self.doc, sel_id);
            }
            Err(e) => {
                self.status_message = Some(e);
            }
        }
    }

    pub fn RequestedPath(&self) -> PathBuf { return PathBuf::from(self._PathInput.trim()); }

    fn RequestSave(&mut self, path: PathBuf)
    {
        if path.exists() && self.path.as_ref() != Some(&path) {
            self._OverwritePath = Some(path);
            return;
        }
        self.SaveReporting(path);
    }

    fn SaveReporting(&mut self, path: PathBuf)
    {
        if let Err(error) = self.SaveToFile(&path) {
            self.status_message = Some(error);
        }
    }

    fn ApplyArc(&mut self) -> Result<(), String>
    {
        if !self.CanEditEntity(self.tool_ctrl.SelectedEntityId()) {
            return Err("Select a curve on a visible, unlocked layer".into());
        }
        let start: f64 = self._ArcStart
                             .trim()
                             .parse()
                             .map_err(|_| "Enter a numeric start angle")?;
        let sweep: f64 = self._ArcSweep
                             .trim()
                             .parse()
                             .map_err(|_| "Enter a numeric sweep angle")?;
        if !start.is_finite() || !sweep.is_finite() || sweep.abs() <= 1e-7 || sweep.abs() > 360.0 {
            return Err("Sweep must be between -360 and 360 degrees, excluding zero".into());
        }
        let entity = self.doc
                         .FindEntity(self.tool_ctrl.SelectedEntityId())
                         .ok_or("Select a circle, arc, or ellipse first")?
                         .clone();
        let primitive = match *entity.Primitive() {
            SketchPrimitive::Ellipse(ellipse) => {
                SketchPrimitive::Ellipse(ellipse.WithArc(start.to_radians(),
                                                        sweep.to_radians()))
            }
            SketchPrimitive::Circle(circle) => {
                if sweep.abs() == 360.0 {
                    SketchPrimitive::Circle(circle)
                } else {
                    SketchPrimitive::Arc(CircularArc::New(circle._Center, circle._Radius, start.to_radians(), sweep.to_radians()))
                }
            }
            SketchPrimitive::Arc(arc) => {
                if sweep.abs() == 360.0 {
                    SketchPrimitive::Circle(Circle::New(arc._Center, arc._Radius))
                } else {
                    SketchPrimitive::Arc(CircularArc::New(arc._Center, arc._Radius, start.to_radians(), sweep.to_radians()))
                }
            }
            _ => return Err("Arc angles apply to a circle, arc, or ellipse".into()),
        };
        if primitive == *entity.Primitive() {
            return Ok(());
        }
        let mut updated = entity.clone();
        updated.SetPrimitive(primitive);
        let mut builder = SketchPatchBuilder::New();
        builder.ReplaceEntity(entity, updated);
        self.history.CommitPatch(&mut self.doc, builder.Build())?;
        self.tool_ctrl.ResetDraft();
        self.canvas_state.InvalidateGeometry();
        self.inspector.ResetSync();
        self.inspector.SyncFromDocument(&self.doc, self.tool_ctrl.SelectedEntityId());
        self.status_message = Some("Arc angles applied (degrees)".into());
        return Ok(());
    }
}

//-------------------------------------------------------------------------------------------------

fn sketch_menu_root_button<'a, Message: 'static>(title: String, palette: ThemePalette)
    -> button::Button<'a, Message>
{
    button(text(title).size(12).style(move |_| text::Style {
        color: Some(palette.text_primary),
    }))
    .padding([4, 8])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
}

fn sketch_menu_item_button<'a, Message: 'static + Clone>(
    label: &'static str,
    shortcut: Option<&'static str>,
    action: Option<Message>,
    palette: ThemePalette,
) -> button::Button<'a, Message>
{
    let mut content = row![
        text(label).size(12).style(move |_| text::Style {
            color: Some(palette.text_primary),
        })
    ]
    .align_y(Alignment::Center)
    .width(Length::Fill);

    if let Some(sc) = shortcut {
        content = content
            .push(Space::new().width(Length::Fill))
            .push(text(sc).size(10).style(move |_| {
                text::Style { color: Some(palette.text_muted) }
            }));
    }
    let mut btn = button(content)
        .padding([4, 8])
        .width(Length::Fill)
        .style(move |_, status| FasciaStyle::menu_item_button(palette, status));

    if let Some(msg) = action {
        btn = btn.on_press(msg);
    }
    btn
}

fn render_sketch_ribbon<'a, Message: 'static + Clone>(
    state: &'a SketchViewerState,
    palette: ThemePalette,
    map_action: impl Fn(SketchViewAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let active_tool = state.tool_ctrl.Tool();

    let tool_btn = |icon: &'static str, name: &'static str, sc: &'static str, tool: SketchToolKind| {
        let is_active = active_tool == tool;
        let btn = button(text(icon).size(14).style(move |_| text::Style {
            color: Some(if is_active { palette.accent } else { palette.text_primary }),
        }))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::tab_button(palette, is_active, status))
        .on_press(map_action(SketchViewAction::SetTool(tool)));

        let tip = format!("{} ({})", name, sc);
        iced::widget::tooltip(btn, text(tip).size(11), iced::widget::tooltip::Position::Bottom)
            .gap(4).padding(6).style(move |_| FasciaStyle::sidebar_container(palette))
    };

    let sub_template = |items: Vec<_>| Menu::new(items).max_width(210.0).offset(2.0);

    // File Menu Dropdown
    let file_items = vec![
        Item::new(sketch_menu_item_button("💾 Save", Some("Ctrl+S"), Some(map_action(SketchViewAction::Save)), palette)),
        Item::new(sketch_menu_item_button("📁 Save As...", None, Some(map_action(SketchViewAction::SaveAs)), palette)),
        Item::new(sketch_menu_item_button("📦 Save Binary (.flx)", None, Some(map_action(SketchViewAction::SaveFormat(FluxFileFormat::Binary))), palette)),
        Item::new(sketch_menu_item_button("📝 Save Text (.tflx)", None, Some(map_action(SketchViewAction::SaveFormat(FluxFileFormat::Text))), palette)),
        Item::new(sketch_menu_item_button("⤓ Export SVG", None, Some(map_action(SketchViewAction::ExportSvg)), palette)),
        Item::new(sketch_menu_item_button("↺ Revert / Restore", None, Some(map_action(SketchViewAction::Restore)), palette)),
    ];
    let file_root = Item::with_menu(sketch_menu_root_button("💾 File ▾".to_string(), palette), sub_template(file_items));
    let file_menu = MenuBar::new(vec![file_root]);

    // Workplane Dropdown
    let plane_preset = state.doc.Plane().PresetName();
    let plane_label = match plane_preset {
        "XY (Front)" => "◫ Plane: XY ▾",
        "XZ (Top)" => "◫ Plane: XZ ▾",
        "YZ (Right)" => "◫ Plane: YZ ▾",
        _ => "◫ Plane: 3D ▾",
    };
    let plane_items = vec![
        Item::new(sketch_menu_item_button("◫ XY (Front)   [Z=0]", None, Some(map_action(SketchViewAction::SetWorkplane(SketchPlane::Xy()))), palette)),
        Item::new(sketch_menu_item_button("◫ XZ (Top)     [Y=0]", None, Some(map_action(SketchViewAction::SetWorkplane(SketchPlane::Xz()))), palette)),
        Item::new(sketch_menu_item_button("◫ YZ (Right)   [X=0]", None, Some(map_action(SketchViewAction::SetWorkplane(SketchPlane::Yz()))), palette)),
        Item::new(sketch_menu_item_button("↕ Offset +10 mm", None, Some(map_action(SketchViewAction::OffsetWorkplane(10.0))), palette)),
        Item::new(sketch_menu_item_button("↕ Offset -10 mm", None, Some(map_action(SketchViewAction::OffsetWorkplane(-10.0))), palette)),
    ];
    let plane_root = Item::with_menu(sketch_menu_root_button(plane_label.to_string(), palette), sub_template(plane_items));
    let plane_menu = MenuBar::new(vec![plane_root]);

    // 3D Extrude & Wire Buttons
    let depth = state.inspector.ExtrudeDepth().parse::<f64>().unwrap_or(10.0);
    let extrude_btn = {
        let btn = button(
            row![
                text("🧊").size(13),
                Space::new().width(Length::Fixed(4.0)),
                text(format!("Extrude ({:.0}mm)", depth)).size(11),
            ].align_y(Alignment::Center)
        )
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchViewAction::Extrude3d(depth)));
        iced::widget::tooltip(btn, text("Extrude closed 2D profiles into watertight 3D solid").size(11), iced::widget::tooltip::Position::Bottom)
            .gap(4).padding(6).style(move |_| FasciaStyle::sidebar_container(palette))
    };

    let wire_btn = {
        let btn = button(
            row![
                text("🌐").size(13),
                Space::new().width(Length::Fixed(4.0)),
                text("3D Wire").size(11),
            ].align_y(Alignment::Center)
        )
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchViewAction::View3dWireframe));
        iced::widget::tooltip(btn, text("Project 2D sketch into 3D world wireframe").size(11), iced::widget::tooltip::Position::Bottom)
            .gap(4).padding(6).style(move |_| FasciaStyle::sidebar_container(palette))
    };

    let icon_action_btn = |icon: &'static str, tip_text: &'static str, action: SketchViewAction, enabled: bool| {
        let btn = button(text(icon).size(13).style(move |_| text::Style {
            color: Some(if enabled { palette.text_primary } else { palette.text_muted }),
        }))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press_maybe(enabled.then(|| map_action(action)));

        iced::widget::tooltip(btn, text(tip_text).size(11), iced::widget::tooltip::Position::Bottom)
            .gap(4).padding(6).style(move |_| FasciaStyle::sidebar_container(palette))
    };

    let divider = || {
        container(Space::new().width(Length::Fixed(1.0)).height(Length::Fixed(18.0)))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(palette.border_subtle)),
                ..Default::default()
            })
    };

    let ribbon_content = row![
        text("SKETCH").size(11).style(move |_| text::Style { color: Some(palette.accent) }),
        text("·").size(11).style(move |_| text::Style { color: Some(palette.text_muted) }),
        text(&state.title).size(12).style(move |_| text::Style { color: Some(palette.text_primary) }),
        text(if state.IsDirty() { "●" } else { "○" }).size(9).style(move |_| text::Style {
            color: Some(if state.IsDirty() { palette.accent } else { palette.text_muted }),
        }),
        Space::new().width(Length::Fixed(6.0)),
        file_menu,
        Space::new().width(Length::Fixed(6.0)),
        divider(),
        Space::new().width(Length::Fixed(6.0)),

        tool_btn("↖", "Select / Transform", "V", SketchToolKind::Select),
        tool_btn("╱", "Line", "L", SketchToolKind::Line),
        tool_btn("⬡", "Polyline", "P", SketchToolKind::Polyline),
        tool_btn("◯", "Circle", "C", SketchToolKind::Circle),
        tool_btn("⌒", "Circular Arc", "A", SketchToolKind::Arc),
        tool_btn("⬭", "Ellipse", "E", SketchToolKind::Ellipse),
        tool_btn("◜", "Elliptic Arc", "Shift+E", SketchToolKind::EllipticalArc),

        Space::new().width(Length::Fixed(6.0)),
        divider(),
        Space::new().width(Length::Fixed(6.0)),

        plane_menu,
        Space::new().width(Length::Fixed(4.0)),
        extrude_btn,
        wire_btn,

        Space::new().width(Length::Fixed(6.0)),
        divider(),
        Space::new().width(Length::Fixed(6.0)),

        icon_action_btn("↩", "Undo (Ctrl+Z)", SketchViewAction::Undo, state.CanUndo()),
        icon_action_btn("↪", "Redo (Ctrl+Y)", SketchViewAction::Redo, state.CanRedo()),
        Space::new().width(Length::Fixed(4.0)),
        icon_action_btn("⛶", "Fit to Screen (F)", SketchViewAction::FitAll, true),
        icon_action_btn("1:1", "Reset 100% Zoom", SketchViewAction::ResetView, true),
        icon_action_btn("◧", "Toggle Side Panel", SketchViewAction::TogglePanel, true),
    ]
    .spacing(4)
    .padding([3, 10])
    .align_y(Alignment::Center);

    container(
        scrollable(ribbon_content).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default()))
    )
    .width(Length::Fill)
    .style(move |_| FasciaStyle::menubar_container(palette))
    .into()
}

fn render_contextual_strip<'a, Message: 'static + Clone>(
    state: &'a SketchViewerState,
    palette: ThemePalette,
    map_action: impl Fn(SketchViewAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let selected = state.doc.FindEntity(state.tool_ctrl.SelectedEntityId());
    if let Some(entity) = selected {
        let kind_icon = match entity.Primitive() {
            SketchPrimitive::Line(_) => "╱",
            SketchPrimitive::Circle(_) => "◯",
            SketchPrimitive::Arc(_) => "⌒",
            SketchPrimitive::Ellipse(_) => "⬭",
            SketchPrimitive::Polyline(_) => "⬡",
        };
        let const_btn_text = if entity.IsConstruction() { "📐 Construction" } else { "━ Solid" };
        let const_btn = button(text(const_btn_text).size(11))
            .padding([2, 6])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(SketchViewAction::InspectorAction(
                SketchInspectorAction::ToggleEntityConstruction(entity.Id())
            )));

        let dup_btn = button(text("⧉ Duplicate").size(11))
            .padding([2, 6])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(SketchViewAction::InspectorAction(
                SketchInspectorAction::DuplicateEntity(entity.Id())
            )));

        let del_btn = button(text("🗑 Delete").size(11))
            .padding([2, 6])
            .style(move |_, status| FasciaStyle::tab_close_button(palette, status))
            .on_press(map_action(SketchViewAction::DeleteSelected));

        let depth = state.inspector.ExtrudeDepth().parse::<f64>().unwrap_or(10.0);
        let extrude_btn = button(text("🧊 Extrude").size(11))
            .padding([2, 6])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(SketchViewAction::Extrude3d(depth)));

        let mut row = row![
            text(format!("{} #{}", kind_icon, entity.Id())).size(11).style(move |_| text::Style { color: Some(palette.accent) }),
            Space::new().width(Length::Fixed(8.0)),
            const_btn,
            dup_btn,
            del_btn,
            extrude_btn,
        ].spacing(6).align_y(Alignment::Center);

        if matches!(entity.Primitive(), SketchPrimitive::Circle(_) | SketchPrimitive::Arc(_) | SketchPrimitive::Ellipse(_)) {
            row = row.push(Space::new().width(Length::Fixed(12.0)))
                .push(text("Start:").size(11).style(move |_| text::Style { color: Some(palette.text_secondary) }))
                .push(text_input("", &state._ArcStart).size(11).padding([2, 6]).width(55)
                    .on_input(move |v| map_action(SketchViewAction::ArcStart(v)))
                    .on_submit(map_action(SketchViewAction::ApplyArc)))
                .push(text("Sweep:").size(11).style(move |_| text::Style { color: Some(palette.text_secondary) }))
                .push(text_input("", &state._ArcSweep).size(11).padding([2, 6]).width(55)
                    .on_input(move |v| map_action(SketchViewAction::ArcSweep(v)))
                    .on_submit(map_action(SketchViewAction::ApplyArc)))
                .push(button(text("✓ Apply").size(11)).padding([2, 6])
                    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                    .on_press(map_action(SketchViewAction::ApplyArc)));
        }

        container(row)
            .padding([3, 12])
            .width(Length::Fill)
            .style(move |_| FasciaStyle::sidebar_container(palette))
            .into()
    } else {
        Space::new().height(Length::Fixed(0.0)).into()
    }
}

fn render_sketch_context_menu<'a, Message: 'static + Clone>(
    state: &'a SketchViewerState,
    x: f32,
    y: f32,
    palette: ThemePalette,
    map_action: impl Fn(SketchViewAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let selected = state.doc.FindEntity(state.tool_ctrl.SelectedEntityId());
    let mut items = column![].spacing(3);

    let menu_btn = |icon: &'static str, label: &'static str, action: SketchViewAction| {
        button(
            row![
                text(icon).size(13),
                Space::new().width(Length::Fixed(6.0)),
                text(label).size(12).style(move |_| text::Style {
                    color: Some(palette.text_primary),
                }),
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fixed(190.0))
        .padding([5, 8])
        .style(move |_, status| FasciaStyle::menu_item_button(palette, status))
        .on_press(map_action(action))
    };

    if let Some(entity) = selected {
        let title = format!("{} #{}", entity.Primitive().KindName(), entity.Id());
        items = items.push(
            text(title).size(11).style(move |_| text::Style { color: Some(palette.accent) })
        );
        let const_label = if entity.IsConstruction() { "━ Solid Line" } else { "📐 Construction" };
        items = items.push(menu_btn("📐", const_label, SketchViewAction::InspectorAction(
            SketchInspectorAction::ToggleEntityConstruction(entity.Id())
        )));
        items = items.push(menu_btn("⧉", "Duplicate", SketchViewAction::InspectorAction(
            SketchInspectorAction::DuplicateEntity(entity.Id())
        )));
        items = items.push(menu_btn("🗑", "Delete", SketchViewAction::DeleteSelected));
        let depth = state.inspector.ExtrudeDepth().parse::<f64>().unwrap_or(10.0);
        items = items.push(menu_btn("🧊", "Extrude Profile (3D)", SketchViewAction::Extrude3d(depth)));
    } else {
        items = items.push(
            text("Canvas Actions").size(11).style(move |_| text::Style { color: Some(palette.accent) })
        );
        items = items.push(menu_btn("⛶", "Fit All (F)", SketchViewAction::FitAll));
        items = items.push(menu_btn("1:1", "Reset 100% Zoom", SketchViewAction::ResetView));
        items = items.push(menu_btn("◫", "Workplane: XY (Front)", SketchViewAction::SetWorkplane(SketchPlane::Xy())));
        items = items.push(menu_btn("◫", "Workplane: XZ (Top)", SketchViewAction::SetWorkplane(SketchPlane::Xz())));
        items = items.push(menu_btn("◫", "Workplane: YZ (Right)", SketchViewAction::SetWorkplane(SketchPlane::Yz())));
        let depth = state.inspector.ExtrudeDepth().parse::<f64>().unwrap_or(10.0);
        items = items.push(menu_btn("🧊", "Extrude Document (3D)", SketchViewAction::Extrude3d(depth)));
        items = items.push(menu_btn("🌐", "3D Wireframe Preview", SketchViewAction::View3dWireframe));
    }

    let dismiss_btn = button(
        row![
            text("✕").size(11),
            Space::new().width(Length::Fixed(6.0)),
            text("Dismiss").size(11).style(move |_| text::Style {
                color: Some(palette.text_muted),
            }),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fixed(190.0))
    .padding([3, 8])
    .style(move |_, status| FasciaStyle::menu_item_button(palette, status))
    .on_press(map_action(SketchViewAction::DismissContextMenu));

    items = items.push(dismiss_btn);

    let popup_card = container(items)
        .padding([6, 8])
        .style(move |_| FasciaStyle::sidebar_container(palette));

    let pad_left = x.clamp(8.0, (state.size[0] - 210.0).max(8.0));
    let pad_top = y.clamp(8.0, (state.size[1] - 220.0).max(8.0));

    container(
        row![
            Space::new().width(Length::Fixed(pad_left)),
            column![
                Space::new().height(Length::Fixed(pad_top)),
                popup_card,
            ]
        ]
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn render_sketch_statusbar<'a, Message: 'static + Clone>(
    state: &'a SketchViewerState,
    palette: ThemePalette,
) -> Element<'a, Message>
{
    let tool_name = match state.tool_ctrl.Tool() {
        SketchToolKind::Select => "Select",
        SketchToolKind::Line => "Line",
        SketchToolKind::Polyline => "Polyline",
        SketchToolKind::Circle => "Circle",
        SketchToolKind::Arc => "Arc",
        SketchToolKind::Ellipse => "Ellipse",
        SketchToolKind::EllipticalArc => "Elliptic Arc",
        SketchToolKind::Move => "Move",
    };

    let plane_name = state.doc.Plane().PresetName();
    let zoom_pct = (state.zoom * 100.0).round() as i32;
    let ent_count = state.doc.EntityCount();
    let dirty_indicator = if state.IsDirty() {
        "● Unsaved"
    } else if state.path.is_none() {
        "New sketch"
    } else {
        "Saved"
    };

    let status_row = row![
        text(format!("Tool: {}", tool_name)).size(11).style(move |_| text::Style {
            color: Some(palette.accent),
        }),
        Space::new().width(Length::Fixed(14.0)),
        text(format!("Plane: {}", plane_name)).size(11).style(move |_| text::Style {
            color: Some(palette.text_secondary),
        }),
        Space::new().width(Length::Fixed(14.0)),
        text(format!("Entities: {}", ent_count)).size(11).style(move |_| text::Style {
            color: Some(palette.text_muted),
        }),
        Space::new().width(Length::Fixed(14.0)),
        text(format!("Zoom: {}%", zoom_pct)).size(11).style(move |_| text::Style {
            color: Some(palette.text_secondary),
        }),
        Space::new().width(Length::Fill),
        text("Middle-drag: Pan  ·  Scroll: Zoom  ·  Right-click: Context Menu").size(10).style(move |_| text::Style {
            color: Some(palette.text_muted),
        }),
        Space::new().width(Length::Fixed(14.0)),
        text(dirty_indicator).size(11).style(move |_| text::Style {
            color: Some(if state.IsDirty() { palette.accent } else { palette.text_muted }),
        }),
    ]
    .padding([3, 12])
    .align_y(Alignment::Center);

    container(status_row)
        .width(Length::Fill)
        .style(move |_| FasciaStyle::status_bar_container(palette))
        .into()
}

/// Constructs the complete interactive Sketch Editor tab UI.
pub fn view_sketch_tab<'a, Message: 'static + Clone>(
    _tab_id: TabId,
    state: &'a SketchViewerState,
    palette: ThemePalette,
    map_action: impl Fn(SketchViewAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let ribbon = render_sketch_ribbon(state, palette, map_action);
    let contextual = render_contextual_strip(state, palette, map_action);

    let canvas = iced::widget::canvas(SketchCanvasProgram {
        doc: &state.doc,
        ctrl: &state.tool_ctrl,
        canvas_state: &state.canvas_state,
        zoom: state.zoom,
        pan: state.pan,
        palette,
        map_action: move |ca| map_action(SketchViewAction::CanvasAction(ca)),
        _phantom: std::marker::PhantomData,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    let mut canvas_stack = iced::widget::stack![canvas];
    if state.doc.EntityCount() == 0 && state.tool_ctrl.DraftGeometry().is_none() {
        canvas_stack = canvas_stack.push(
            container(column![
                text("Start your sketch").size(20).color(palette.text_secondary),
                text("Choose a drawing tool above, or right-click canvas for quick tools.").size(12).color(palette.text_secondary),
                text("L Line  ·  P Polyline  ·  C Circle  ·  A Arc  ·  Esc Cancel").size(11).color(palette.text_muted),
            ].spacing(6)).padding(28),
        );
    }
    if let Some(pos) = state.context_menu {
        canvas_stack = canvas_stack.push(render_sketch_context_menu(state, pos[0], pos[1], palette, map_action));
    }
    let canvas_area: Element<'a, Message> = canvas_stack.into();

    let main_area: Element<'a, Message> = if state.inspector.ShowPanel() {
        row![
            canvas_area,
            view_sketch_inspector(
                &state.doc,
                &state.tool_ctrl,
                &state.inspector,
                palette,
                move |act| map_action(SketchViewAction::InspectorAction(act))
            )
        ]
        .height(Length::Fill)
        .into()
    } else {
        canvas_area
    };

    let statusbar = render_sketch_statusbar(state, palette);

    let mut editor = column![ribbon, contextual];
    if state._ConfirmRestore {
        let confirm_box = container(column![
            text("Restore saved version?").size(14),
            text("Unsaved changes will be discarded. This cannot be undone.").size(12),
            row![
                button(text("Restore saved version").size(12))
                    .padding([6, 12])
                    .on_press(map_action(SketchViewAction::ConfirmRestore)),
                button(text("Keep editing").size(12))
                    .padding([6, 12])
                    .on_press(map_action(SketchViewAction::CancelRestore)),
            ].spacing(8),
        ].spacing(8))
        .padding([12, 16])
        .width(Length::Fill)
        .style(move |_| FasciaStyle::sidebar_container(palette));
        editor = editor.push(confirm_box);
    }
    if let Some(path) = &state._OverwritePath {
        let overwrite_box = container(column![
            text("Replace existing file?").size(14),
            text(path.to_string_lossy().into_owned()).size(12),
            row![
                button(text("Replace file").size(12))
                    .padding([6, 12])
                    .on_press(map_action(SketchViewAction::ConfirmOverwrite)),
                button(text("Cancel").size(12))
                    .padding([6, 12])
                    .on_press(map_action(SketchViewAction::CancelOverwrite)),
            ].spacing(8),
        ].spacing(8))
        .padding([12, 16])
        .width(Length::Fill)
        .style(move |_| FasciaStyle::sidebar_container(palette));
        editor = editor.push(overwrite_box);
    }

    editor = editor.push(main_area);
    if let Some(message) = &state.status_message {
        editor = editor.push(
            container(text(message).size(12).color(palette.text_primary))
                .padding([6, 16])
                .width(Length::Fill)
                .style(move |_| FasciaStyle::menubar_container(palette))
        );
    }
    editor = editor.push(statusbar);

    container(editor.width(Length::Fill).height(Length::Fill))
        .style(move |_| FasciaStyle::content_container(palette))
        .into()
}

//-------------------------------------------------------------------------------------------------
