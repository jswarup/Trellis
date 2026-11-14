//-- sketch_view.rs -------------------------------------------------------------------------------------------------
use crate::fascia::sketch_canvas::{SketchCanvasAction, SketchCanvasProgram, SketchCanvasState};
use crate::fascia::sketch_tool::{SketchToolController, SketchToolKind};
use crate::fascia::tabs::TabId;
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::sketch::{SketchDocument, SketchPoint};
use crate::fenst::sketch_edit::SketchHistory;
use crate::fleck::sketch_flux::SketchFlux;
use crate::fleck::sketch_io::SketchIo;
use crate::flux::{FluxFile, FluxFileFormat, FluxValue};
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Point};
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
    pub zoom:           f32,
    pub pan:            [f32; 2],
    pub palette:        Option<ThemePalette>,
    pub dark_theme:     bool,
    pub size:           [f32; 2],
    pub status_message: Option<String>,
    _PathInput: String,
    _FileMetadata: Option<FluxValue>,
    _OverwritePath: Option<PathBuf>,
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

        let pathInput = path.as_ref()
                            .map(|p| p.with_extension("flx").to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Sketch.flx".into());
        Self {
            path,
            title,
            doc,
            history,
            tool_ctrl,
            canvas_state: SketchCanvasState::New(),
            zoom: 1.0,
            pan: [400.0, 300.0],
            palette: Some(palette),
            dark_theme: is_dark,
            size: [800.0, 600.0],
            status_message: None,
            _PathInput: pathInput,
            _FileMetadata: None,
            _OverwritePath: None,
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
        }
    }

    pub fn Redo(&mut self)
    {
        if let Ok(Some(_)) = self.history.Redo(&mut self.doc) {
            self.canvas_state.InvalidateGeometry();
            self.tool_ctrl.ResetDraft();
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

    pub fn ProcessAction(&mut self, action: SketchViewAction)
    {
        match action {
            SketchViewAction::PathChanged(path) => { self._PathInput = path; self._OverwritePath = None; }
            SketchViewAction::OpenPath => { self.status_message = Some("Use Open to restore the path in a new tab".into()); }
            SketchViewAction::Format(format) => {
                self._PathInput = Path::new(&self._PathInput).with_extension(if format == FluxFileFormat::Binary { "flx" } else { "tflx" }).to_string_lossy().into_owned();
            }
            SketchViewAction::SaveAs => { self.RequestSave(PathBuf::from(self._PathInput.trim())); }
            SketchViewAction::SaveFormat(format) => {
                let p = Path::new(&self._PathInput).with_extension(if format == FluxFileFormat::Binary { "flx" } else { "tflx" });
                self._PathInput = p.to_string_lossy().into_owned();
                self.RequestSave(p);
            }
            SketchViewAction::Restore => {
                if let Some(path) = self.path.clone() {
                    let palette = self.palette.unwrap_or_else(|| crate::fascia::theme::FasciaTheme::WindowsDark.palette());
                    match Self::FromFile(path.clone(), palette) {
                        Ok(reloaded) => {
                            self.doc = reloaded.doc;
                            self.history = reloaded.history;
                            self._FileMetadata = reloaded._FileMetadata;
                            self.canvas_state.InvalidateAll();
                            self.tool_ctrl.ResetDraft();
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
                if id != u32::MAX
                    && let Some(entity) = self.doc.FindEntity(id)
                {
                    let mut builder = crate::fenst::sketch_edit::SketchPatchBuilder::New();
                    builder.RemoveEntity(entity.clone());
                    let patch = builder.Build();
                    let _ = self.history.CommitPatch(&mut self.doc, patch);
                    self.canvas_state.InvalidateGeometry();
                    self.tool_ctrl.SetSelectedEntityId(u32::MAX);
                }
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
                self.pan[0] += dx;
                self.pan[1] += dy;
                self.canvas_state.InvalidateAll();
            }
            SketchViewAction::Zoom(factor, focal) => {
                let old_zoom = self.zoom;
                let new_zoom = (old_zoom * factor).clamp(0.005, 500.0);

                if let Some([fx, fy]) = focal {
                    let world_x = (fx - self.pan[0]) / old_zoom;
                    let world_y = (fy - self.pan[1]) / old_zoom;
                    self.pan[0] = fx - world_x * new_zoom;
                    self.pan[1] = fy - world_y * new_zoom;
                }

                self.zoom = new_zoom;
                self.tool_ctrl.SetZoom(new_zoom as f64);
                self.canvas_state.InvalidateAll();
            }
            SketchViewAction::CanvasAction(canvas_act) => match canvas_act {
                SketchCanvasAction::Pan(dx, dy) => {
                    self.pan[0] += dx;
                    self.pan[1] += dy;
                    self.canvas_state.InvalidateAll();
                }
                SketchCanvasAction::Undo => self.Undo(),
                SketchCanvasAction::Redo => self.Redo(),
                SketchCanvasAction::Save => self.ProcessAction(SketchViewAction::Save),
                SketchCanvasAction::Tool(tool) => self.tool_ctrl.SetTool(tool),
                SketchCanvasAction::Zoom(factor, focal) => {
                    self.ProcessAction(SketchViewAction::Zoom(factor, focal));
                }
                SketchCanvasAction::Resize(w, h) => {
                    self.size = [w, h];
                    self.canvas_state.last_size = [w, h];
                    self.canvas_state.InvalidateAll();
                }
                SketchCanvasAction::Input(event) => {
                    let oldSelection = self.tool_ctrl.SelectedEntityId();
                    self.tool_ctrl.ProcessEvent(&self.doc, event);
                    if let Some(patch) = self.tool_ctrl.TakeCommittedPatch() {
                        if let Err(error) = self.history.CommitPatch(&mut self.doc, patch) { self.status_message = Some(error); }
                        self.canvas_state.InvalidateGeometry();
                    }
                    if self.tool_ctrl.SelectedEntityId() != oldSelection && let Some(entity) = self.doc.FindEntity(self.tool_ctrl.SelectedEntityId()) {
                        let angles = match entity.Primitive() {
                            crate::fenst::sketch::SketchPrimitive::Circle(_) => Some((0.0, std::f64::consts::TAU)),
                            crate::fenst::sketch::SketchPrimitive::Arc(arc) => Some((arc._StartAngle, arc._SweepAngle)),
                            crate::fenst::sketch::SketchPrimitive::Ellipse(ellipse) => Some((ellipse._StartAngle, ellipse._SweepAngle)),
                            _ => None,
                        };
                        if let Some((start, sweep)) = angles { self._ArcStart = start.to_degrees().to_string(); self._ArcSweep = sweep.to_degrees().to_string(); }
                    }
                }
            },
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
            crate::fenst::sketch::SketchPrimitive::Ellipse(ellipse) => {
                crate::fenst::sketch::SketchPrimitive::Ellipse(ellipse.WithArc(start.to_radians(),
                                                                               sweep.to_radians()))
            }
            crate::fenst::sketch::SketchPrimitive::Circle(circle) => {
                if sweep.abs() == 360.0 {
                    crate::fenst::sketch::SketchPrimitive::Circle(circle)
                } else {
                    crate::fenst::sketch::SketchPrimitive::Arc(crate::fenst::sketch::CircularArc::New(circle._Center, circle._Radius, start.to_radians(), sweep.to_radians()))
                }
            }
            crate::fenst::sketch::SketchPrimitive::Arc(arc) => {
                if sweep.abs() == 360.0 {
                    crate::fenst::sketch::SketchPrimitive::Circle(crate::fenst::sketch::Circle::New(arc._Center, arc._Radius))
                } else {
                    crate::fenst::sketch::SketchPrimitive::Arc(crate::fenst::sketch::CircularArc::New(arc._Center, arc._Radius, start.to_radians(), sweep.to_radians()))
                }
            }
            _ => return Err("Arc angles apply to a circle, arc, or ellipse".into()),
        };
        if primitive == *entity.Primitive() {
            return Ok(());
        }
        let mut updated = entity.clone();
        updated.SetPrimitive(primitive);
        let mut builder = crate::fenst::sketch_edit::SketchPatchBuilder::New();
        builder.ReplaceEntity(entity, updated);
        self.history.CommitPatch(&mut self.doc, builder.Build())?;
        self.tool_ctrl.ResetDraft();
        self.canvas_state.InvalidateGeometry();
        self.status_message = Some("Arc angles applied (degrees)".into());
        return Ok(());
    }
}

//-------------------------------------------------------------------------------------------------

/// Constructs the complete interactive Sketch Editor tab UI.
pub fn view_sketch_tab<'a, Message: 'static + Clone>(_tab_id: TabId, state: &'a SketchViewerState,
                                                     palette: ThemePalette,
                                                     map_action: impl Fn(SketchViewAction) -> Message
                                                     + Copy
                                                     + 'static)
                                                     -> Element<'a, Message>
{
    let active_tool = state.tool_ctrl.Tool();

    // Helper to build toolbar tool buttons
    let tool_btn = |name: &'static str, tool: SketchToolKind| {
        let is_selected = active_tool == tool;
        let mut btn = button(text(name).size(12)).padding([4, 8]);
        if is_selected {
            btn = btn.style(move |_, status| FasciaStyle::tab_button(palette, true, status));
        } else {
            btn = btn.style(move |_, status| FasciaStyle::toolbar_button(palette, status));
        }
        btn.on_press(map_action(SketchViewAction::SetTool(tool)))
    };

    let select_btn = tool_btn("Select / Move", SketchToolKind::Select);
    let line_btn = tool_btn("📏 Line", SketchToolKind::Line);
    let poly_btn = tool_btn("📐 Polyline", SketchToolKind::Polyline);
    let circle_btn = tool_btn("⭕ Circle", SketchToolKind::Circle);
    let arc_btn = tool_btn("⌒ Arc", SketchToolKind::Arc);
    let ellipse_btn = tool_btn("⬯ Ellipse", SketchToolKind::Ellipse);
    let elliptic_btn = tool_btn("Elliptic arc", SketchToolKind::EllipticalArc);

    // History and view buttons
    let undo_btn = button(text("↩ Undo").size(12))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status));
    let undo_btn = if state.CanUndo() {
        undo_btn.on_press(map_action(SketchViewAction::Undo))
    } else {
        undo_btn
    };

    let redo_btn = button(text("↪ Redo").size(12))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status));
    let redo_btn = if state.CanRedo() {
        redo_btn.on_press(map_action(SketchViewAction::Redo))
    } else {
        redo_btn
    };

    let fit_btn = button(text("⛶ Fit").size(12))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchViewAction::FitAll));

    let reset_btn = button(text("1:1").size(12))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchViewAction::ResetView));

    let save_btn = button(text("💾 Save").size(12))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchViewAction::Save));

    let save_flx_btn = button(text("💾 .flx").size(12))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchViewAction::SaveFormat(FluxFileFormat::Binary)));

    let save_tflx_btn = button(text("📝 .tflx").size(12))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchViewAction::SaveFormat(FluxFileFormat::Text)));

    let restore_btn = button(text("↺ Restore").size(12))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchViewAction::Restore));

    let export_btn = button(text("SVG").size(12))
        .padding([4, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchViewAction::ExportSvg));

    let toolbar = row![
        text("SKETCH").size(12).style(move |_| text::Style {
            color: Some(palette.accent),
        }),
        Space::new().width(Length::Fixed(8.0)),
        select_btn,
        line_btn,
        poly_btn,
        circle_btn,
        arc_btn,
        ellipse_btn,
        elliptic_btn,
        Space::new().width(Length::Fixed(12.0)),
        undo_btn,
        redo_btn,
        Space::new().width(Length::Fixed(12.0)),
        fit_btn,
        reset_btn,
        Space::new().width(Length::Fill),
        save_btn,
        save_flx_btn,
        save_tflx_btn,
        restore_btn,
        export_btn,
    ]
    .padding([5, 10])
    .spacing(4)
    .align_y(Alignment::Center);

    let toolbar_container = container(scrollable(toolbar).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::default())))
        .width(Length::Fill)
        .style(move |_| FasciaStyle::menubar_container(palette));

    // Center Canvas
    let canvas_element = iced::widget::canvas(SketchCanvasProgram {
        doc:          &state.doc,
        ctrl:         &state.tool_ctrl,
        canvas_state: &state.canvas_state,
        zoom:         state.zoom,
        pan:          state.pan,
        palette,
        map_action:   move |ca| map_action(SketchViewAction::CanvasAction(ca)),
        _phantom:     std::marker::PhantomData,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    // Bottom Status Bar
    let tool_name = match active_tool {
        SketchToolKind::Select => "Select",
        SketchToolKind::Line => "Line",
        SketchToolKind::Polyline => "Polyline (Enter/Click start to finish)",
        SketchToolKind::Circle => "Circle",
        SketchToolKind::Arc => "Arc (Center -> Radius -> Sweep)",
        SketchToolKind::Ellipse => "Ellipse (Center -> Major -> Minor)",
        SketchToolKind::EllipticalArc => "Elliptic arc (Center -> Major -> Minor; edit sweep below)",
        SketchToolKind::Move => "Move",
    };

    let zoom_pct = (state.zoom * 100.0).round() as i32;
    let ent_count = state.doc.EntityCount();
    let dirty_indicator = if state.IsDirty() {
        "● Unsaved"
    } else if state.path.is_none() {
        "New sketch"
    } else {
        "Saved"
    };

    let bottom_bar = row![
        text(format!("Tool: {}", tool_name)).size(11).style(move |_| text::Style {
            color: Some(palette.accent),
        }),
        Space::new().width(Length::Fixed(16.0)),
        text(format!("Zoom: {}%", zoom_pct)).size(11).style(move |_| text::Style {
            color: Some(palette.text_secondary),
        }),
        Space::new().width(Length::Fixed(16.0)),
        text(format!("Entities: {}", ent_count)).size(11).style(move |_| text::Style {
            color: Some(palette.text_muted),
        }),
        Space::new().width(Length::Fill),
        text(dirty_indicator).size(11).style(move |_| text::Style {
            color: Some(if state.IsDirty() { palette.accent } else { palette.text_muted }),
        }),
    ]
    .padding([3, 10])
    .align_y(Alignment::Center);

    let bottom_container = container(bottom_bar)
        .width(Length::Fill)
        .style(move |_| FasciaStyle::status_bar_container(palette));

    let filebar = row![
        text_input("Sketch.flx or Sketch.tflx", &state._PathInput)
            .on_input(move |value| map_action(SketchViewAction::PathChanged(value)))
            .on_submit(map_action(SketchViewAction::OpenPath)).size(13),
        button("Open").on_press(map_action(SketchViewAction::OpenPath)),
        button("Binary .flx").on_press(map_action(SketchViewAction::Format(FluxFileFormat::Binary))),
        button("Text .tflx").on_press(map_action(SketchViewAction::Format(FluxFileFormat::Text))),
        button("Save As").on_press(map_action(SketchViewAction::SaveAs)),
        button("Save .flx").on_press(map_action(SketchViewAction::SaveFormat(FluxFileFormat::Binary))),
        button("Save .tflx").on_press(map_action(SketchViewAction::SaveFormat(FluxFileFormat::Text))),
        button("Restore").on_press(map_action(SketchViewAction::Restore)),
    ].spacing(6).padding([6, 10]).align_y(Alignment::Center);
    let angles = row![text("Arc start / sweep (degrees)").size(12),
        text_input("Start", &state._ArcStart).width(90).on_input(move |v| map_action(SketchViewAction::ArcStart(v))),
        text_input("Sweep", &state._ArcSweep).width(90).on_input(move |v| map_action(SketchViewAction::ArcSweep(v))),
        button("Apply to selection").on_press(map_action(SketchViewAction::ApplyArc)),
        button("Delete").on_press(map_action(SketchViewAction::DeleteSelected)),
        button("Cancel tool").on_press(map_action(SketchViewAction::CancelDraft)),
    ].spacing(6).padding([4, 10]).align_y(Alignment::Center);
    let hint = state.status_message.as_deref().unwrap_or("Click to place points; line/circle also drag. Enter finishes a polyline. Middle-drag pans; wheel zooms. Esc cancels.");
    let mut editor = column![filebar,
                             toolbar_container,
                             angles,
                             text(hint).size(12),
                             canvas_element,
                             bottom_container];
    if let Some(path) = &state._OverwritePath {
        editor = editor.push(
                             row![
            text(format!("Replace {}?", path.display())),
            button("Replace file").on_press(map_action(SketchViewAction::ConfirmOverwrite)),
            button("Cancel").on_press(map_action(SketchViewAction::CancelOverwrite)),
        ].spacing(10)
                             .padding(8),
        );
    }
    editor
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

//-------------------------------------------------------------------------------------------------
