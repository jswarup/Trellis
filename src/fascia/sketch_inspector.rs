//-- sketch_inspector.rs -------------------------------------------------------------------------------------------------
//! Interactive property inspector and hierarchical explorer tree projection for the 2D sketcher.

use crate::fascia::sketch_tool::SketchToolController;
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::sketch::{SketchDocument, SketchPlane, SketchPrimitive};
use crate::silo::IArr;
use iced::widget::{Column, Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length};

//-------------------------------------------------------------------------------------------------

/// Active panel tab within the sketch side panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectorPanelTab
{
    #[default]
    Tree,
    Inspector,
}

//-------------------------------------------------------------------------------------------------

/// Action emitted by the sketch inspector and explorer tree controls.
#[derive(Clone, Debug, PartialEq)]
pub enum SketchInspectorAction
{
    SelectTab(InspectorPanelTab),
    ToggleLayersExpanded,
    ToggleGroupsExpanded,
    ToggleEntitiesExpanded,
    SelectEntity(u32),
    DeleteEntity(u32),
    DuplicateEntity(u32),
    ToggleEntityConstruction(u32),
    SetEntityLayer { entity_id: u32, layer_id: u32 },
    SetActiveLayer(u32),
    ToggleLayerVisible(u32),
    ToggleLayerLocked(u32),
    ToggleLayerConstruction(u32),
    FieldInput { index: usize, value: String },
    ApplyFieldEdits,
    SetPlane(SketchPlane),
    ExtrudeDepthInput(String),
    Extrude3d,
    View3dWireframe,
    FitAll,
    ResetView,
    ClosePanel,
}

//-------------------------------------------------------------------------------------------------

/// Non-record state holding navigation, expansion, and edit buffers for the inspector panel.
#[derive(Clone, Debug, PartialEq)]
pub struct SketchInspectorState
{
    _ActiveTab:        InspectorPanelTab,
    _LayersExpanded:   bool,
    _GroupsExpanded:   bool,
    _EntitiesExpanded: bool,
    _ShowPanel:        bool,
    _Fields:           [String; 8],
    _ExtrudeDepth:     String,
    _LastEntityId:     u32,
    _LastRevision:     u64,
}

impl Default for SketchInspectorState
{
    fn default() -> Self
    {
        Self::New()
    }
}

impl SketchInspectorState
{
    pub fn New() -> Self
    {
        Self {
            _ActiveTab:        InspectorPanelTab::Tree,
            _LayersExpanded:   true,
            _GroupsExpanded:   true,
            _EntitiesExpanded: true,
            _ShowPanel:        true,
            _Fields:           [
                String::new(), String::new(), String::new(), String::new(),
                String::new(), String::new(), String::new(), String::new(),
            ],
            _ExtrudeDepth:     "10.0".to_string(),
            _LastEntityId:     u32::MAX,
            _LastRevision:     u64::MAX,
        }
    }

    pub fn ActiveTab(&self) -> InspectorPanelTab
    {
        self._ActiveTab
    }

    pub fn SetActiveTab(&mut self, tab: InspectorPanelTab)
    {
        self._ActiveTab = tab;
    }

    pub fn ShowPanel(&self) -> bool
    {
        self._ShowPanel
    }

    pub fn SetShowPanel(&mut self, show: bool)
    {
        self._ShowPanel = show;
    }

    pub fn ToggleShowPanel(&mut self)
    {
        self._ShowPanel = !self._ShowPanel;
    }

    pub fn LayersExpanded(&self) -> bool
    {
        self._LayersExpanded
    }

    pub fn ToggleLayers(&mut self)
    {
        self._LayersExpanded = !self._LayersExpanded;
    }

    pub fn GroupsExpanded(&self) -> bool
    {
        self._GroupsExpanded
    }

    pub fn ToggleGroups(&mut self)
    {
        self._GroupsExpanded = !self._GroupsExpanded;
    }

    pub fn EntitiesExpanded(&self) -> bool
    {
        self._EntitiesExpanded
    }

    pub fn ToggleEntities(&mut self)
    {
        self._EntitiesExpanded = !self._EntitiesExpanded;
    }

    pub fn Field(&self, idx: usize) -> &str
    {
        if idx < self._Fields.len() {
            &self._Fields[idx]
        } else {
            ""
        }
    }

    pub fn SetField(&mut self, idx: usize, val: String)
    {
        if idx < self._Fields.len() {
            self._Fields[idx] = val;
        }
    }

    pub fn ExtrudeDepth(&self) -> &str
    {
        &self._ExtrudeDepth
    }

    pub fn SetExtrudeDepth(&mut self, val: String)
    {
        self._ExtrudeDepth = val;
    }

    pub fn ResetSync(&mut self)
    {
        self._LastEntityId = u32::MAX;
        self._LastRevision = u64::MAX;
    }

    pub fn SyncFromDocument(&mut self, doc: &SketchDocument, selected_id: u32)
    {
        if selected_id == self._LastEntityId && doc.Revision() == self._LastRevision {
            return;
        }
        self._LastEntityId = selected_id;
        self._LastRevision = doc.Revision();

        if let Some(entity) = doc.FindEntity(selected_id) {
            match entity.Primitive() {
                SketchPrimitive::Line(l) => {
                    self._Fields[0] = l._Start._X.to_string();
                    self._Fields[1] = l._Start._Y.to_string();
                    self._Fields[2] = l._End._X.to_string();
                    self._Fields[3] = l._End._Y.to_string();
                }
                SketchPrimitive::Circle(c) => {
                    self._Fields[0] = c._Center._X.to_string();
                    self._Fields[1] = c._Center._Y.to_string();
                    self._Fields[2] = c._Radius.to_string();
                }
                SketchPrimitive::Arc(a) => {
                    self._Fields[0] = a._Center._X.to_string();
                    self._Fields[1] = a._Center._Y.to_string();
                    self._Fields[2] = a._Radius.to_string();
                    self._Fields[3] = a._StartAngle.to_degrees().to_string();
                    self._Fields[4] = a._SweepAngle.to_degrees().to_string();
                }
                SketchPrimitive::Ellipse(e) => {
                    self._Fields[0] = e._Center._X.to_string();
                    self._Fields[1] = e._Center._Y.to_string();
                    self._Fields[2] = e._SemiMajor.to_string();
                    self._Fields[3] = e._SemiMinor.to_string();
                    self._Fields[4] = e._Rotation.to_degrees().to_string();
                    self._Fields[5] = e._StartAngle.to_degrees().to_string();
                    self._Fields[6] = e._SweepAngle.to_degrees().to_string();
                }
                SketchPrimitive::Polyline(p) => {
                    self._Fields[0] = p._PointCount.to_string();
                    self._Fields[1] = p._IsClosed.to_string();
                }
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Constructs the complete interactive side panel (Explorer Tree & Property Inspector).
pub fn view_sketch_inspector<'a, Message: 'static + Clone>(doc: &'a SketchDocument,
                                                           ctrl: &'a SketchToolController,
                                                           state: &'a SketchInspectorState,
                                                           palette: ThemePalette,
                                                           map_action: impl Fn(SketchInspectorAction)
                                                              -> Message
                                                           + Copy
                                                           + 'static)
                                                           -> Element<'a, Message>
{
    let tab = state.ActiveTab();

    // Top Header with Tab Switcher
    let is_tree = tab == InspectorPanelTab::Tree;
    let is_insp = tab == InspectorPanelTab::Inspector;

    let tree_tab_btn = button(
        row![
            text("🌲").size(12),
            Space::new().width(Length::Fixed(4.0)),
            text("Tree").size(11),
        ]
        .align_y(Alignment::Center),
    )
    .padding([4, 8])
    .style(move |_, status| FasciaStyle::tab_button(palette, is_tree, status))
    .on_press(map_action(SketchInspectorAction::SelectTab(InspectorPanelTab::Tree)));

    let insp_tab_btn = button(
        row![
            text("⚙").size(12),
            Space::new().width(Length::Fixed(4.0)),
            text("Properties").size(11),
        ]
        .align_y(Alignment::Center),
    )
    .padding([4, 8])
    .style(move |_, status| FasciaStyle::tab_button(palette, is_insp, status))
    .on_press(map_action(SketchInspectorAction::SelectTab(InspectorPanelTab::Inspector)));

    let close_btn = button(text("✕").size(11))
        .padding([2, 6])
        .style(move |_, status| FasciaStyle::tab_close_button(palette, status))
        .on_press(map_action(SketchInspectorAction::ClosePanel));

    let header_bar = row![
        tree_tab_btn,
        insp_tab_btn,
        Space::new().width(Length::Fill),
        close_btn,
    ]
    .padding([8, 10])
    .spacing(4)
    .align_y(Alignment::Center);

    let content: Element<'a, Message> = match tab {
        InspectorPanelTab::Tree => view_tree_content(doc, ctrl, state, palette, map_action),
        InspectorPanelTab::Inspector => view_inspector_content(doc, ctrl, state, palette, map_action),
    };

    container(column![header_bar, content].spacing(2))
        .width(Length::Fixed(320.0))
        .height(Length::Fill)
        .style(move |_| FasciaStyle::sidebar_container(palette))
        .into()
}

//-------------------------------------------------------------------------------------------------

fn view_tree_content<'a, Message: 'static + Clone>(doc: &'a SketchDocument,
                                                   ctrl: &'a SketchToolController,
                                                   state: &'a SketchInspectorState,
                                                   palette: ThemePalette,
                                                   map_action: impl Fn(SketchInspectorAction)
                                                      -> Message
                                                   + Copy
                                                   + 'static)
                                                   -> Element<'a, Message>
{
    let mut items: Vec<Element<'a, Message>> = Vec::new();

    // Section 1: LAYERS
    let layers_expanded = state.LayersExpanded();
    let layers_arrow = if layers_expanded { "▾" } else { "▸" };
    let layers_header_text = format!("{} Layers ({})", layers_arrow, doc.LayerCount());
    let layers_toggle_btn = button(
        text(layers_header_text)
            .size(12)
            .style(move |_| text::Style { color: Some(palette.accent) }),
    )
    .padding([2, 4])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(map_action(SketchInspectorAction::ToggleLayersExpanded));

    items.push(layers_toggle_btn.into());

    if layers_expanded {
        let active_layer_id = ctrl.ActiveLayerId();
        doc.Layers().Traverse(|layer| {
            let lid = layer.Id();
            let is_active = lid == active_layer_id;
            let bullet = if is_active { "●" } else { "○" };
            let vis_icon = if layer.IsVisible() { "👁" } else { "🚫" };
            let lock_icon = if layer.IsLocked() { "🔒" } else { "🔓" };
            let const_icon = if layer.IsConstruction() { "📐" } else { "━" };

            let name_btn = button(text(format!("{} {}", bullet, layer.Name())).size(12))
                .padding([2, 4])
                .style(move |_, status| FasciaStyle::tree_row_button(palette, is_active, status))
                .on_press(map_action(SketchInspectorAction::SetActiveLayer(lid)));

            let vis_btn = button(text(vis_icon).size(11))
                .padding([1, 4])
                .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                .on_press(map_action(SketchInspectorAction::ToggleLayerVisible(lid)));

            let lock_btn = button(text(lock_icon).size(11))
                .padding([1, 4])
                .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                .on_press(map_action(SketchInspectorAction::ToggleLayerLocked(lid)));

            let const_btn = button(text(const_icon).size(11))
                .padding([1, 4])
                .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                .on_press(map_action(SketchInspectorAction::ToggleLayerConstruction(lid)));

            let layer_row = row![
                name_btn,
                Space::new().width(Length::Fill),
                const_btn,
                vis_btn,
                lock_btn,
            ]
            .spacing(2)
            .align_y(Alignment::Center);

            items.push(layer_row.into());
        });
    }

    // Section 2: GROUPS
    let groups_expanded = state.GroupsExpanded();
    let groups_arrow = if groups_expanded { "▾" } else { "▸" };
    let groups_header_text = format!("{} Groups ({})", groups_arrow, doc.GroupCount());
    let groups_toggle_btn = button(
        text(groups_header_text)
            .size(12)
            .style(move |_| text::Style { color: Some(palette.accent) }),
    )
    .padding([2, 4])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(map_action(SketchInspectorAction::ToggleGroupsExpanded));

    items.push(groups_toggle_btn.into());

    if groups_expanded {
        if doc.GroupCount() == 0 {
            items.push(
                text("  (No groups defined)")
                    .size(12)
                    .style(move |_| text::Style { color: Some(palette.text_muted) })
                    .into(),
            );
        } else {
            doc.Groups().Traverse(|group| {
                let lbl = format!("  Group #{}: \"{}\" ({} members)", group.Id(), group.Name(), group.MemberIds().Len());
                items.push(
                    text(lbl)
                        .size(12)
                        .style(move |_| text::Style { color: Some(palette.text_secondary) })
                        .into(),
                );
            });
        }
    }

    // Section 3: ENTITIES
    let entities_expanded = state.EntitiesExpanded();
    let entities_arrow = if entities_expanded { "▾" } else { "▸" };
    let entities_header_text = format!("{} Entities ({})", entities_arrow, doc.EntityCount());
    let entities_toggle_btn = button(
        text(entities_header_text)
            .size(12)
            .style(move |_| text::Style { color: Some(palette.accent) }),
    )
    .padding([2, 4])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(map_action(SketchInspectorAction::ToggleEntitiesExpanded));

    items.push(entities_toggle_btn.into());

    if entities_expanded {
        let sel_id = ctrl.SelectedEntityId();
        if doc.EntityCount() == 0 {
            items.push(
                text("  (No entities drawn)")
                    .size(12)
                    .style(move |_| text::Style { color: Some(palette.text_muted) })
                    .into(),
            );
        } else {
            doc.Entities().Traverse(|entity| {
                let eid = entity.Id();
                let is_selected = eid == sel_id;

                let (icon, detail) = match entity.Primitive() {
                    SketchPrimitive::Line(l) => {
                        ("╱", format!("Line #{} (L={:.1})", eid, l.Length()))
                    }
                    SketchPrimitive::Circle(c) => {
                        ("◯", format!("Circle #{} (R={:.1})", eid, c._Radius))
                    }
                    SketchPrimitive::Arc(a) => {
                        ("⌒", format!("Arc #{} (R={:.1}, {:.0}°)", eid, a._Radius, a._SweepAngle.to_degrees()))
                    }
                    SketchPrimitive::Ellipse(e) => {
                        ("⬭", format!("Ellipse #{} ({:.1}×{:.1})", eid, e._SemiMajor, e._SemiMinor))
                    }
                    SketchPrimitive::Polyline(p) => {
                        ("⬡", format!("Polyline #{} ({} pts)", eid, p._PointCount))
                    }
                };
                let const_badge = if entity.IsConstruction() { " 📐" } else { "" };
                let label = format!("{} {}{}", icon, detail, const_badge);

                let item_btn = button(text(label).size(11))
                    .padding([2, 4])
                    .width(Length::Fill)
                    .style(move |_, status| FasciaStyle::tree_row_button(palette, is_selected, status))
                    .on_press(map_action(SketchInspectorAction::SelectEntity(eid)));

                let del_btn = button(text("✕").size(10))
                    .padding([1, 4])
                    .style(move |_, status| FasciaStyle::tab_close_button(palette, status))
                    .on_press(map_action(SketchInspectorAction::DeleteEntity(eid)));

                let entity_row = row![
                    item_btn,
                    del_btn,
                ]
                .spacing(2)
                .align_y(Alignment::Center);

                items.push(entity_row.into());
            });
        }
    }

    let tree_col = Column::with_children(items).spacing(4).padding([6, 8]);
    scrollable(tree_col).height(Length::Fill).into()
}

//-------------------------------------------------------------------------------------------------

fn view_inspector_content<'a, Message: 'static + Clone>(doc: &'a SketchDocument,
                                                        ctrl: &'a SketchToolController,
                                                        state: &'a SketchInspectorState,
                                                        palette: ThemePalette,
                                                        map_action: impl Fn(SketchInspectorAction)
                                                           -> Message
                                                        + Copy
                                                        + 'static)
                                                        -> Element<'a, Message>
{
    let sel_id = ctrl.SelectedEntityId();

    if let Some(entity) = doc.FindEntity(sel_id) {
        let mut insp_col = column![].spacing(10).padding(16);

        // Entity Header
        let kind_name = entity.Primitive().KindName();
        let header_title = format!("{} (ID #{})", kind_name, entity.Id());
        insp_col = insp_col.push(
            text(header_title)
                .size(13)
                .style(move |_| text::Style { color: Some(palette.accent) }),
        );

        // Layer and Construction flags
        let mut layer_name = format!("Layer #{}", entity.LayerId());
        doc.Layers().Traverse(|l| {
            if l.Id() == entity.LayerId() {
                layer_name = l.Name().to_string();
            }
        });

        let const_btn_text = if entity.IsConstruction() { "Construction" } else { "Solid" };
        let const_btn = button(text(const_btn_text).size(12))
            .padding([2, 6])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(SketchInspectorAction::ToggleEntityConstruction(entity.Id())));

        let flags_row = row![
            text(format!("Layer: {}", layer_name)).size(12).style(move |_| text::Style { color: Some(palette.text_secondary) }),
            Space::new().width(Length::Fill),
            const_btn,
        ]
        .align_y(Alignment::Center);
        insp_col = insp_col.push(flags_row);

        // Parameters inputs
        match entity.Primitive() {
            SketchPrimitive::Line(l) => {
                insp_col = insp_col.push(make_input_field("Start X", state.Field(0), 0, map_action));
                insp_col = insp_col.push(make_input_field("Start Y", state.Field(1), 1, map_action));
                insp_col = insp_col.push(make_input_field("End X", state.Field(2), 2, map_action));
                insp_col = insp_col.push(make_input_field("End Y", state.Field(3), 3, map_action));

                let len = l.Length();
                let dx = l._End._X - l._Start._X;
                let dy = l._End._Y - l._Start._Y;
                let ang_deg = dy.atan2(dx).to_degrees();
                insp_col = insp_col.push(
                    text(format!("Length: {:.3} {}  Angle: {:.1}°", len, doc.Units(), ang_deg))
                        .size(12)
                        .style(move |_| text::Style { color: Some(palette.text_muted) }),
                );
            }
            SketchPrimitive::Circle(c) => {
                insp_col = insp_col.push(make_input_field("Center X", state.Field(0), 0, map_action));
                insp_col = insp_col.push(make_input_field("Center Y", state.Field(1), 1, map_action));
                insp_col = insp_col.push(make_input_field("Radius", state.Field(2), 2, map_action));

                let diam = c._Radius * 2.0;
                let circum = c.Circumference();
                let area = c.Area();
                insp_col = insp_col.push(
                    text(format!("Diameter: {:.3} {}\nCircumference: {:.3} {}\nArea: {:.3} {}²", diam, doc.Units(), circum, doc.Units(), area, doc.Units()))
                        .size(12)
                        .style(move |_| text::Style { color: Some(palette.text_muted) }),
                );
            }
            SketchPrimitive::Arc(a) => {
                insp_col = insp_col.push(make_input_field("Center X", state.Field(0), 0, map_action));
                insp_col = insp_col.push(make_input_field("Center Y", state.Field(1), 1, map_action));
                insp_col = insp_col.push(make_input_field("Radius", state.Field(2), 2, map_action));
                insp_col = insp_col.push(make_input_field("Start Angle (°)", state.Field(3), 3, map_action));
                insp_col = insp_col.push(make_input_field("Sweep Angle (°)", state.Field(4), 4, map_action));

                let arc_len = a.ArcLength();
                let chord = 2.0 * a._Radius * (a._SweepAngle * 0.5).sin().abs();
                insp_col = insp_col.push(
                    text(format!("Arc length: {:.3} {}\nChord: {:.3} {}", arc_len, doc.Units(), chord, doc.Units()))
                        .size(12)
                        .style(move |_| text::Style { color: Some(palette.text_muted) }),
                );
            }
            SketchPrimitive::Ellipse(e) => {
                insp_col = insp_col.push(make_input_field("Center X", state.Field(0), 0, map_action));
                insp_col = insp_col.push(make_input_field("Center Y", state.Field(1), 1, map_action));
                insp_col = insp_col.push(make_input_field("Semi-Major (A)", state.Field(2), 2, map_action));
                insp_col = insp_col.push(make_input_field("Semi-Minor (B)", state.Field(3), 3, map_action));
                insp_col = insp_col.push(make_input_field("Rotation (°)", state.Field(4), 4, map_action));
                insp_col = insp_col.push(make_input_field("Start Angle (°)", state.Field(5), 5, map_action));
                insp_col = insp_col.push(make_input_field("Sweep Angle (°)", state.Field(6), 6, map_action));

                let ecc = if e._SemiMajor > 0.0 {
                    (1.0 - (e._SemiMinor * e._SemiMinor) / (e._SemiMajor * e._SemiMajor)).max(0.0).sqrt()
                } else {
                    0.0
                };
                insp_col = insp_col.push(
                    text(format!("Eccentricity: {:.3}\nClosed: {}", ecc, e.IsClosed()))
                        .size(12)
                        .style(move |_| text::Style { color: Some(palette.text_muted) }),
                );
            }
            SketchPrimitive::Polyline(p) => {
                insp_col = insp_col.push(
                    text(format!("Vertices: {}\nClosed: {}", p._PointCount, p._IsClosed))
                        .size(12)
                        .style(move |_| text::Style { color: Some(palette.text_secondary) }),
                );
            }
        }

        // Apply and action buttons
        let apply_btn = button(text("Apply changes").size(12))
            .padding([3, 8])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(SketchInspectorAction::ApplyFieldEdits));

        let dup_btn = button(text("Duplicate").size(12))
            .padding([3, 8])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(map_action(SketchInspectorAction::DuplicateEntity(entity.Id())));

        let del_btn = button(text("Delete").size(12))
            .padding([3, 8])
            .style(move |_, status| FasciaStyle::tab_close_button(palette, status))
            .on_press(map_action(SketchInspectorAction::DeleteEntity(entity.Id())));

        let actions_row = row![
            dup_btn,
            del_btn,
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        insp_col = insp_col.push(apply_btn).push(actions_row);

        scrollable(insp_col).height(Length::Fill).into()
    } else {
        view_document_inspector(doc, state, palette, map_action)
    }
}

//-------------------------------------------------------------------------------------------------

fn view_document_inspector<'a, Message: 'static + Clone>(doc: &'a SketchDocument,
                                                         state: &'a SketchInspectorState,
                                                         palette: ThemePalette,
                                                         map_action: impl Fn(SketchInspectorAction)
                                                            -> Message
                                                         + Copy
                                                         + 'static)
                                                         -> Element<'a, Message>
{
    let mut doc_col = column![].spacing(10).padding(16);

    doc_col = doc_col.push(
        text("Document Properties")
            .size(13)
            .style(move |_| text::Style { color: Some(palette.accent) }),
    );

    doc_col = doc_col.push(
        text(format!("Units: {}", doc.Units()))
            .size(12)
            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
    );

    doc_col = doc_col.push(
        text(format!("Total Entities: {}", doc.EntityCount()))
            .size(12)
            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
    );

    doc_col = doc_col.push(
        text(format!("Layers: {}", doc.LayerCount()))
            .size(12)
            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
    );

    doc_col = doc_col.push(
        text(format!("Groups: {}", doc.GroupCount()))
            .size(12)
            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
    );

    doc_col = doc_col.push(
        text(format!("Revision: {}", doc.Revision()))
            .size(12)
            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
    );

    let bounds = doc.ComputeBounds();
    doc_col = doc_col.push(
        text(format!(
            "Bounding Box:\nMin: ({:.2}, {:.2})\nMax: ({:.2}, {:.2})\nSize: {:.2} × {:.2} {}",
            bounds._Min._X, bounds._Min._Y, bounds._Max._X, bounds._Max._Y,
            bounds.Width(), bounds.Height(), doc.Units()
        ))
        .size(12)
        .style(move |_| text::Style { color: Some(palette.text_muted) }),
    );

    let fit_btn = button(text("Fit sketch").size(12))
        .padding([3, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchInspectorAction::FitAll));

    let reset_btn = button(text("1:1 Reset").size(12))
        .padding([3, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchInspectorAction::ResetView));

    let view_row = row![fit_btn, reset_btn].spacing(6);
    doc_col = doc_col.push(view_row);

    // 3D Workplane Selection
    doc_col = doc_col.push(
        text("3D Workplane")
            .size(12)
            .style(move |_| text::Style { color: Some(palette.accent) }),
    );
    let current_preset = doc.Plane().PresetName();
    let normal = doc.Plane().Normal();
    doc_col = doc_col.push(
        text(format!("Active: {} [{:.0}, {:.0}, {:.0}]", current_preset, normal[0], normal[1], normal[2]))
            .size(12)
            .style(move |_| text::Style { color: Some(palette.text_secondary) }),
    );

    let xy_active = current_preset == "XY (Front)";
    let xz_active = current_preset == "XZ (Top)";
    let yz_active = current_preset == "YZ (Right)";

    let xy_btn = button(text("XY").size(11))
        .padding([2, 6])
        .style(move |_, status| {
            if xy_active {
                FasciaStyle::tab_button(palette, true, status)
            } else {
                FasciaStyle::toolbar_button(palette, status)
            }
        })
        .on_press(map_action(SketchInspectorAction::SetPlane(SketchPlane::Xy())));

    let xz_btn = button(text("XZ").size(11))
        .padding([2, 6])
        .style(move |_, status| {
            if xz_active {
                FasciaStyle::tab_button(palette, true, status)
            } else {
                FasciaStyle::toolbar_button(palette, status)
            }
        })
        .on_press(map_action(SketchInspectorAction::SetPlane(SketchPlane::Xz())));

    let yz_btn = button(text("YZ").size(11))
        .padding([2, 6])
        .style(move |_, status| {
            if yz_active {
                FasciaStyle::tab_button(palette, true, status)
            } else {
                FasciaStyle::toolbar_button(palette, status)
            }
        })
        .on_press(map_action(SketchInspectorAction::SetPlane(SketchPlane::Yz())));

    let planes_row = row![xy_btn, xz_btn, yz_btn].spacing(4);
    doc_col = doc_col.push(planes_row);

    // 3D Operations
    doc_col = doc_col.push(
        text("3D Operations")
            .size(12)
            .style(move |_| text::Style { color: Some(palette.accent) }),
    );

    let depth_row = row![
        text(format!("Depth ({})", doc.Units())).size(12).width(Length::Fixed(75.0)),
        text_input("", state.ExtrudeDepth())
            .size(12)
            .width(Length::Fill)
            .on_input(move |v| map_action(SketchInspectorAction::ExtrudeDepthInput(v)))
            .on_submit(map_action(SketchInspectorAction::Extrude3d)),
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    doc_col = doc_col.push(depth_row);

    let extrude_btn = button(text("Extrude").size(12))
        .padding([3, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchInspectorAction::Extrude3d));

    let wire_btn = button(text("3D wireframe").size(12))
        .padding([3, 8])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(map_action(SketchInspectorAction::View3dWireframe));

    let ops_row = row![extrude_btn, wire_btn].spacing(6);
    doc_col = doc_col.push(ops_row);

    scrollable(doc_col).height(Length::Fill).into()
}

//-------------------------------------------------------------------------------------------------

fn make_input_field<'a, Message: 'static + Clone>(label: &'static str, val: &'a str, idx: usize,
                                                  map_action: impl Fn(SketchInspectorAction) -> Message
                                                  + Copy
                                                  + 'static)
                                                  -> Element<'a, Message>
{
    row![
        text(label).size(12).width(Length::Fixed(108.0)),
        text_input("", val)
            .size(12)
            .width(Length::Fill)
            .on_input(move |v| map_action(SketchInspectorAction::FieldInput { index: idx, value: v }))
            .on_submit(map_action(SketchInspectorAction::ApplyFieldEdits)),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

//-------------------------------------------------------------------------------------------------
