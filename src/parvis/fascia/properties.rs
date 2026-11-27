// properties.rs ----------------------------------------------------------------------------------
//! Unified workbench Properties inspector pane showing target breadcrumb, overview,
//! and type-specific inspection for files, examples, circuits, and expressions.

use crate::fascia::icons::{IconId, ViewIcon};
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use crate::fenst::fresco_resource::{FRESCO_EXAMPLE_DESCRIPTION, FRESCO_EXAMPLE_TITLE, FRESCO_EXAMPLE_URI};
use crate::fenst::math_layout::MathCoordinates;
use iced::{Alignment, Element, Length};
use std::path::{Path, PathBuf};

//-------------------------------------------------------------------------------------------------

/// Represents the entity currently inspected by the Properties pane.
#[derive(Debug, Clone, PartialEq)]
pub enum InspectedTarget
{
    None,
    File
    {
        path:      PathBuf,
        name:      String,
        size:      u64,
        is_dir:    bool,
        extension: String,
    },
    Example
    {
        uri:         String,
        suite:       String,
        name:        String,
        status:      String,
        description: String,
    },
    FrescoExpr
    {
        name:          String,
        path:          String,
        formula:       String,
        tensor_shape:  Option<String>,
        selected_term: Option<String>,
        resource_selection: Option<(String, Option<MathCoordinates>)>,
    },
    Circuit
    {
        name:       String,
        path:       String,
        ports:      u32,
        submodules: u32,
    },
    Custom
    {
        title:      String,
        breadcrumb: String,
        fields:     Vec<(String, String)>,
    },
}

//-------------------------------------------------------------------------------------------------

/// Actions emitted from the Properties pane.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertiesAction
{
    TogglePin,
    Close,
    OpenTarget(String),
    RunTarget(String),
    UseInPython(String),
}

//-------------------------------------------------------------------------------------------------

/// State of the workbench Properties inspector.
#[derive(Debug, Clone)]
pub struct PropertiesState
{
    _Target:   InspectedTarget,
    _IsPinned: bool,
}

impl Default for PropertiesState
{
    fn default() -> Self { Self::New() }
}

impl PropertiesState
{
    pub fn New() -> Self
    {
        return Self {
            _Target:   InspectedTarget::None,
            _IsPinned: false,
        };
    }

    pub fn Target(&self) -> &InspectedTarget { &self._Target }
    pub fn IsPinned(&self) -> bool { self._IsPinned }

    pub fn SetPinned(&mut self, pinned: bool)
    {
        self._IsPinned = pinned;
    }

    pub fn TogglePinned(&mut self)
    {
        self._IsPinned = !self._IsPinned;
    }

    pub fn Clear(&mut self)
    {
        if !self._IsPinned {
            self._Target = InspectedTarget::None;
        }
    }

    pub fn Breadcrumb(&self) -> String
    {
        match &self._Target {
            InspectedTarget::None => "No selection".to_string(),
            InspectedTarget::File { path, .. } => path.to_string_lossy().to_string(),
            InspectedTarget::Example { uri, .. } => uri.clone(),
            InspectedTarget::FrescoExpr { path, .. } => path.clone(),
            InspectedTarget::Circuit { path, .. } => path.clone(),
            InspectedTarget::Custom { breadcrumb, .. } => breadcrumb.clone(),
        }
    }

    pub fn InspectPath(&mut self, path: &Path)
    {
        if self._IsPinned {
            return;
        }

        let path_str = path.to_string_lossy();
        if path_str.starts_with("example://") {
            let stripped = path_str.strip_prefix("example://").unwrap_or(&path_str);
            let trimmed = stripped.trim_matches('/');
            let (suite, rest) = trimmed.split_once('/').unwrap_or((trimmed, ""));
            let (name, _sub) = rest.split_once('/').unwrap_or((rest, ""));

            if path_str == FRESCO_EXAMPLE_URI {
                self.InspectFresco(FRESCO_EXAMPLE_TITLE, FRESCO_EXAMPLE_URI,
                                   FRESCO_EXAMPLE_DESCRIPTION, None, None);
                return;
            }

            if suite == "Rube" && name == "DaddaMultiplier4x4" {
                self._Target = InspectedTarget::Circuit {
                    name:       "DaddaMultiplier4x4".to_string(),
                    path:       path_str.to_string(),
                    ports:      16,
                    submodules: 28,
                };
                return;
            }

            let all = crate::fenst::example::XplrExample::Registered();
            let mut found = None;
            all.Traverse(|ex| {
                if ex.suite == suite && ex.name == name {
                    found = Some(*ex);
                }
            });

            if let Some(ex) = found {
                self._Target = InspectedTarget::Example {
                    uri:         path_str.to_string(),
                    suite:       ex.suite.to_string(),
                    name:        ex.name.to_string(),
                    status:      "Ready to run".to_string(),
                    description: format!("Example {}::{}", ex.suite, ex.name),
                };
            } else {
                self._Target = InspectedTarget::Example {
                    uri:         path_str.to_string(),
                    suite:       suite.to_string(),
                    name:        name.to_string(),
                    status:      "Available".to_string(),
                    description: format!("Workbench example {}", path_str),
                };
            }
            return;
        }

        if path_str.starts_with("fresco://") {
            self._Target = InspectedTarget::FrescoExpr {
                name:          path.file_name().and_then(|n| n.to_str()).unwrap_or("Expr").to_string(),
                path:          path_str.to_string(),
                formula:       "Fresco symbolic tensor expression".to_string(),
                tensor_shape:  None,
                selected_term: None,
                resource_selection: None,
            };
            return;
        }

        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Untitled").to_string();
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let is_dir = path.is_dir();
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

        self._Target = InspectedTarget::File {
            path: path.to_path_buf(),
            name,
            size,
            is_dir,
            extension,
        };
    }

    pub fn InspectExample(
        &mut self,
        uri: impl Into<String>,
        suite: impl Into<String>,
        name: impl Into<String>,
        status: impl Into<String>,
        desc: impl Into<String>,
    )
    {
        if self._IsPinned {
            return;
        }
        self._Target = InspectedTarget::Example {
            uri:         uri.into(),
            suite:       suite.into(),
            name:        name.into(),
            status:      status.into(),
            description: desc.into(),
        };
    }

    pub fn InspectFile(&mut self, path: &Path, size: u64, is_dir: bool)
    {
        if self._IsPinned {
            return;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        self._Target = InspectedTarget::File {
            path: path.to_path_buf(),
            name,
            size,
            is_dir,
            extension,
        };
    }

    pub fn InspectFresco(
        &mut self,
        name: impl Into<String>,
        path: impl Into<String>,
        formula: impl Into<String>,
        shape: Option<String>,
        term: Option<String>,
    )
    {
        if self._IsPinned {
            return;
        }
        self._Target = InspectedTarget::FrescoExpr {
            name:          name.into(),
            path:          path.into(),
            formula:       formula.into(),
            tensor_shape:  shape,
            selected_term: term,
            resource_selection: None,
        };
    }

    pub fn InspectFrescoProperties(&mut self, props: &crate::fenst::fresco_properties::FrescoProperties<'_>)
    {
        self.InspectFrescoResource(props, None);
    }

    pub(super) fn InspectFrescoResource(
        &mut self, props: &crate::fenst::fresco_properties::FrescoProperties<'_>, uri: Option<&str>,
    )
    {
        if self._IsPinned {
            return;
        }
        let termSummary = props.Term.as_ref().map(|term| {
            let mut summary = format!("Occurrence #{} (Expr #{})", term.OccurrenceId, term.ExprId);
            if let Some(coords) = &term.Coordinates {
                summary.push_str(&format!(" at {}", coords));
            }
            summary.push_str(&format!(": {}", term.Summary()));
            return summary;
        });

        let formula = match &termSummary {
            Some(t) => format!("Root '{}' · {}", props.Name, t),
            None => format!("Root '{}' (no term selected)", props.Name),
        };

        self._Target = InspectedTarget::FrescoExpr {
            name: props.Name.to_string(),
            path: uri.map(str::to_string).unwrap_or_else(|| {
                format!("fresco://Doc#{}/{}", props.DocumentId, props.Name)
            }),
            formula,
            tensor_shape: props.ShapeLabel(),
            selected_term: termSummary,
            resource_selection: uri.filter(|uri| *uri == FRESCO_EXAMPLE_URI)
                .map(|_| (props.Name.to_string(), props.Term.as_ref().and_then(|term| term.Coordinates))),
        };
    }

    pub(super) fn FrescoPythonSnippet(&self) -> Option<String>
    {
        let InspectedTarget::FrescoExpr { path, resource_selection, .. } = &self._Target else {
            return None;
        };
        if path != FRESCO_EXAMPLE_URI {
            return None;
        }
        let mut call = format!("trellis.inspect({path:?}");
        if let Some((root, coords)) = resource_selection {
            call.push_str(&format!(", root={root:?}"));
            if let Some(coords) = coords {
                call.push_str(&format!(", coordinates={coords}"));
            }
        }
        call.push_str(")\n");
        return Some(call);
    }

    pub fn InspectCircuit(
        &mut self,
        name: impl Into<String>,
        path: impl Into<String>,
        ports: u32,
        submodules: u32,
    )
    {
        if self._IsPinned {
            return;
        }
        self._Target = InspectedTarget::Circuit {
            name: name.into(),
            path: path.into(),
            ports,
            submodules,
        };
    }

    pub fn InspectCustom(
        &mut self,
        title: impl Into<String>,
        breadcrumb: impl Into<String>,
        fields: Vec<(String, String)>,
    )
    {
        if self._IsPinned {
            return;
        }
        self._Target = InspectedTarget::Custom {
            title: title.into(),
            breadcrumb: breadcrumb.into(),
            fields,
        };
    }
}

//-------------------------------------------------------------------------------------------------

fn property_row<'a, Message: 'static>(
    label: &'static str,
    value: String,
    palette: ThemePalette,
) -> Element<'a, Message>
{
    return row![
        text(label)
            .size(11)
            .style(move |_| text::Style { color: Some(palette.text_muted) })
            .width(Length::Fixed(80.0)),
        text(value)
            .size(12)
            .style(move |_| text::Style { color: Some(palette.text_primary) })
            .width(Length::Fill),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into();
}

//-------------------------------------------------------------------------------------------------

/// Renders the Properties inspector pane widget.
pub fn view_properties<'a, Message: 'static + Clone>(
    state: &PropertiesState,
    palette: ThemePalette,
    on_action: impl Fn(PropertiesAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let pin_label = if state.IsPinned() { "Pinned" } else { "Pin" };
    let pin_btn = button(
        row![
            ViewIcon(IconId::Pin, 11.0, if state.IsPinned() { palette.accent } else { palette.text_secondary }),
            Space::new().width(4),
            text(pin_label).size(11),
        ]
        .align_y(Alignment::Center),
    )
    .padding([2, 6])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(on_action(PropertiesAction::TogglePin));

    let close_btn = button(ViewIcon(IconId::Close, 10.0, palette.text_secondary))
        .padding([3, 6])
        .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
        .on_press(on_action(PropertiesAction::Close));

    let header = row![
        text("PROPERTIES").size(12).style(move |_| text::Style { color: Some(palette.text_secondary) }),
        Space::new().width(Length::Fill),
        pin_btn,
        close_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(4);

    let breadcrumb_text = state.Breadcrumb();
    let breadcrumb_bar = container(
        text(breadcrumb_text.clone())
            .size(11)
            .style(move |_| text::Style { color: Some(palette.accent) }),
    )
    .padding([2, 6])
    .width(Length::Fill)
    .style(move |_| container::Style {
        background: Some(iced::Background::Color(palette.tab_bar_bg)),
        border: iced::Border {
            color: palette.border_subtle,
            width: 1.0,
            radius: palette.corner_radius.into(),
        },
        ..Default::default()
    });

    let mut body = column![].spacing(8).width(Length::Fill);

    match state.Target() {
        InspectedTarget::None => {
            body = body.push(
                text("No item selected.\nSelect an item in Explorer or Document to inspect.")
                    .size(12)
                    .style(move |_| text::Style { color: Some(palette.text_muted) }),
            );
        }
        InspectedTarget::File { path, name, size, is_dir, extension } => {
            let kind_str = if *is_dir { "Directory".to_string() } else { format!("File (.{extension})") };
            let size_str = if *is_dir { "-".to_string() } else { format!("{size} bytes") };

            body = body
                .push(property_row("Name", name.clone(), palette))
                .push(property_row("Kind", kind_str, palette))
                .push(property_row("Size", size_str, palette))
                .push(property_row("Path", path.to_string_lossy().to_string(), palette));

            let path_target = path.to_string_lossy().to_string();
            let open_btn = button(text("Open ↗").size(11))
                .padding([3, 8])
                .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                .on_press(on_action(PropertiesAction::OpenTarget(path_target.clone())));

            let py_snippet = format!("trellis.open('{}')", path_target);
            let py_btn = button(
                row![ViewIcon(IconId::Terminal, 11.0, palette.text_secondary), Space::new().width(4), text("Use in Python").size(11)]
                    .align_y(Alignment::Center),
            )
            .padding([3, 8])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(on_action(PropertiesAction::UseInPython(py_snippet)));

            body = body.push(row![open_btn, py_btn].spacing(6));
        }
        InspectedTarget::Example { uri, suite, name, status, description } => {
            body = body
                .push(property_row("Suite", suite.clone(), palette))
                .push(property_row("Example", name.clone(), palette))
                .push(property_row("Status", status.clone(), palette))
                .push(property_row("Desc", description.clone(), palette))
                .push(property_row("URI", uri.clone(), palette));

            let uri_target = uri.clone();
            let open_btn = button(text("Open ↗").size(11))
                .padding([3, 8])
                .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                .on_press(on_action(PropertiesAction::OpenTarget(uri_target.clone())));

            let run_btn = button(
                row![ViewIcon(IconId::Play, 10.0, palette.accent), Space::new().width(4), text("Run").size(11)]
                    .align_y(Alignment::Center),
            )
            .padding([3, 8])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(on_action(PropertiesAction::RunTarget(uri_target.clone())));

            let py_snippet = format!("trellis.open('{}')\n", uri_target);
            let py_btn = button(
                row![ViewIcon(IconId::Terminal, 11.0, palette.text_secondary), Space::new().width(4), text("Use in Python").size(11)]
                    .align_y(Alignment::Center),
            )
            .padding([3, 8])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(on_action(PropertiesAction::UseInPython(py_snippet)));

            body = body.push(row![open_btn, run_btn, py_btn].spacing(6));
        }
        InspectedTarget::FrescoExpr { name, path, formula, tensor_shape, selected_term, .. } => {
            body = body
                .push(property_row("Name", name.clone(), palette))
                .push(property_row("Path", path.clone(), palette))
                .push(property_row("Formula", formula.clone(), palette));

            if let Some(shape) = tensor_shape {
                body = body.push(property_row("Shape", shape.clone(), palette));
            }
            if let Some(term) = selected_term {
                body = body.push(property_row("Term", term.clone(), palette));
            }

            if let Some(snippet) = state.FrescoPythonSnippet() {
                let open_btn = button(text("Open Math View ↗").size(11))
                    .padding([3, 8])
                    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                    .on_press(on_action(PropertiesAction::OpenTarget(path.clone())));
                let py_btn = button(
                    row![ViewIcon(IconId::Terminal, 11.0, palette.text_secondary), Space::new().width(4), text("Use in Python").size(11)]
                        .align_y(Alignment::Center),
                )
                .padding([3, 8])
                .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                .on_press(on_action(PropertiesAction::UseInPython(snippet)));
                body = body.push(row![open_btn, py_btn].spacing(6));
            }
        }
        InspectedTarget::Circuit { name, path, ports, submodules } => {
            body = body
                .push(property_row("Module", name.clone(), palette))
                .push(property_row("Path", path.clone(), palette))
                .push(property_row("Ports", ports.to_string(), palette))
                .push(property_row("Submodules", submodules.to_string(), palette));

            let path_target = path.clone();
            let open_btn = button(text("Open Schematic ↗").size(11))
                .padding([3, 8])
                .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
                .on_press(on_action(PropertiesAction::OpenTarget(path_target.clone())));

            let run_btn = button(
                row![ViewIcon(IconId::Play, 10.0, palette.accent), Space::new().width(4), text("Simulate").size(11)]
                    .align_y(Alignment::Center),
            )
            .padding([3, 8])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(on_action(PropertiesAction::RunTarget(path_target.clone())));

            let py_snippet = format!("trellis.inspect('{}')\n", path_target);
            let py_btn = button(
                row![ViewIcon(IconId::Terminal, 11.0, palette.text_secondary), Space::new().width(4), text("Use in Python").size(11)]
                    .align_y(Alignment::Center),
            )
            .padding([3, 8])
            .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
            .on_press(on_action(PropertiesAction::UseInPython(py_snippet)));

            body = body.push(row![open_btn, run_btn, py_btn].spacing(6));
        }
        InspectedTarget::Custom { title, fields, .. } => {
            body = body.push(property_row("Entity", title.clone(), palette));
            for (k, v) in fields {
                body = body.push(property_row(Box::leak(k.clone().into_boxed_str()), v.clone(), palette));
            }
        }
    }

    let panel = column![header, breadcrumb_bar, scrollable(body).height(Length::Fill)]
        .spacing(8)
        .padding(8)
        .width(Length::Fixed(280.0))
        .height(Length::Fill);

    return container(panel)
        .width(Length::Fixed(280.0))
        .height(Length::Fill)
        .style(move |_| FasciaStyle::sidebar_container(palette))
        .into();
}
