//-- sketch_view.rs -------------------------------------------------------------------------------------------------
use crate::fascia::sketch_canvas::{
    SketchCanvasAction, SketchCanvasProgram, SketchCanvasState,
};
use crate::fascia::sketch_tool::{SketchToolController, SketchToolKind};
use crate::fascia::tabs::TabId;
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::sketch::{SketchDocument, SketchPoint};
use crate::fenst::sketch_edit::SketchHistory;
use crate::fleck::sketch_io::SketchIo;
use iced::widget::{Space, button, column, container, row, text};
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
}

impl SketchViewerState
{
    pub fn New(path: Option<PathBuf>, title: String, palette: ThemePalette) -> Self
    {
        let doc = SketchDocument::New();
        Self::FromDocument(doc, path, title, palette)
    }

    pub fn FromDocument(
        doc: SketchDocument,
        path: Option<PathBuf>,
        title: String,
        palette: ThemePalette,
    ) -> Self
    {
        let is_dark = palette.is_dark();
        let history = SketchHistory::New(128);
        let mut tool_ctrl = SketchToolController::New();
        tool_ctrl.SetZoom(1.0);

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
        }
    }

    pub fn FromFile(path: PathBuf, palette: ThemePalette) -> Result<Self, String>
    {
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read sketch file: {}", e))?;
        let doc = SketchIo::ParseSketch(&content)?;
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled.tsketch")
            .to_string();

        let mut state = Self::FromDocument(doc, Some(path), title, palette);
        state.history.MarkSaved(state.doc.Revision());
        state.FitAll();
        Ok(state)
    }

    pub fn SaveToFile(&mut self, target_path: &Path) -> Result<(), String>
    {
        let text = SketchIo::SerializeSketch(&self.doc);
        let tmp_path = target_path.with_extension("tsketch.tmp");

        fs::write(&tmp_path, text)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        fs::rename(&tmp_path, target_path)
            .map_err(|e| format!("Failed to atomic rename: {}", e))?;

        self.history.MarkSaved(self.doc.Revision());
        self.path = Some(target_path.to_path_buf());
        self.status_message = Some("Saved sketch successfully".to_string());
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

        if span_x > 1e-4 && span_y > 1e-4 && self.size[0] > 50.0 && self.size[1] > 50.0 {
            let margin = 40.0f32;
            let avail_w = (self.size[0] - margin * 2.0).max(50.0);
            let avail_h = (self.size[1] - margin * 2.0).max(50.0);

            let scale_x = avail_w / span_x as f32;
            let scale_y = avail_h / span_y as f32;
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
                if let Some(path) = self.path.clone() {
                    let _ = self.SaveToFile(&path);
                }
            }
            SketchViewAction::ExportSvg => {
                if let Some(path) = &self.path {
                    let svg_path = path.with_extension("svg");
                    let svg_text = self.ExportSvgString();
                    let _ = fs::write(&svg_path, svg_text);
                    self.status_message = Some(format!("Exported SVG: {}", svg_path.display()));
                }
            }
            SketchViewAction::Pan(dx, dy) => {
                self.pan[0] += dx;
                self.pan[1] += dy;
                self.canvas_state.InvalidateGrid();
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
                    self.canvas_state.InvalidateGrid();
                }
                SketchCanvasAction::Zoom(factor, focal) => {
                    self.ProcessAction(SketchViewAction::Zoom(factor, focal));
                }
                SketchCanvasAction::Resize(w, h) => {
                    self.size = [w, h];
                    self.canvas_state.last_size = [w, h];
                    self.canvas_state.InvalidateGrid();
                }
                SketchCanvasAction::Input(event) => {
                    self.tool_ctrl.ProcessEvent(&self.doc, event);
                    if let Some(patch) = self.tool_ctrl.TakeCommittedPatch() {
                        let _ = self.history.CommitPatch(&mut self.doc, patch);
                        self.canvas_state.InvalidateGeometry();
                    }
                }
            },
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Constructs the complete interactive Sketch Editor tab UI.
pub fn view_sketch_tab<'a, Message: 'static + Clone>(
    _tab_id:     TabId,
    state:       &'a SketchViewerState,
    palette:     ThemePalette,
    map_action:  impl Fn(SketchViewAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
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

    let select_btn = tool_btn("↖ Select", SketchToolKind::Select);
    let line_btn = tool_btn("📏 Line", SketchToolKind::Line);
    let poly_btn = tool_btn("📐 Polyline", SketchToolKind::Polyline);
    let circle_btn = tool_btn("⭕ Circle", SketchToolKind::Circle);
    let arc_btn = tool_btn("⌒ Arc", SketchToolKind::Arc);
    let ellipse_btn = tool_btn("⬯ Ellipse", SketchToolKind::Ellipse);

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
        Space::new().width(Length::Fixed(12.0)),
        undo_btn,
        redo_btn,
        Space::new().width(Length::Fixed(12.0)),
        fit_btn,
        reset_btn,
        Space::new().width(Length::Fill),
        save_btn,
        export_btn,
    ]
    .padding([5, 10])
    .spacing(4)
    .align_y(Alignment::Center);

    let toolbar_container = container(toolbar)
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
        SketchToolKind::Move => "Move",
    };

    let zoom_pct = (state.zoom * 100.0).round() as i32;
    let ent_count = state.doc.EntityCount();
    let dirty_indicator = if state.IsDirty() { "● Unsaved" } else { "Saved" };

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

    column![toolbar_container, canvas_element, bottom_container]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

//-------------------------------------------------------------------------------------------------
