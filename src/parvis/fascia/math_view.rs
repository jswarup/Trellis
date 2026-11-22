// math_view.rs ------------------------------------------------------------------------------------
//! Owned read-only Fresco document, layout cache and native math canvas.

use crate::fascia::math_inspector::ViewMathProperties;
use crate::fascia::math_text::CosmicMathFontMetrics;
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::fenst::fresco_document::FrescoDocument;
use crate::fenst::fresco_properties::FrescoProperties;
use crate::fenst::math_layout::{
    DelimiterKind, MathColor, MathDrawCommand, MathLayout, MathLayoutBuilder, MathOccurrence,
};
use crate::silo::IArr;
use iced::widget::canvas::{Frame, Geometry, Path, Program, Stroke};
use iced::widget::{Space, button, column, container, row, sensor, text};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Size, mouse};

//-------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum FrescoAction
{
    SelectRoot(u32),
    SelectOccurrence(u32),
    Zoom(f32, Option<[f32; 2]>),
    Pan(f32, f32),
    ResetView,
    FitView,
    NextRoot,
    PrevRoot,
    Resize(f32, f32),
}

//-------------------------------------------------------------------------------------------------

pub const MIN_ZOOM: f32 = 0.000001;
pub const MAX_ZOOM: f32 = 10.0;

pub struct FrescoViewerState
{
    _Document:           FrescoDocument,
    _SelectedRoot:       u32,
    _SelectedOccurrence: u32,
    _Zoom:               f32,
    _Pan:                [f32; 2],
    _ViewportSize:       [f32; 2],
    _FitToViewport:      bool,
    _Layout:             Option<MathLayout>,
    _LayoutError:        Option<String>,
}

impl FrescoViewerState
{
    pub fn New(doc: FrescoDocument) -> Self
    {
        let mut state = Self { _Document:           doc,
                               _SelectedRoot:       0,
                               _SelectedOccurrence: u32::MAX,
                               _Zoom:               1.0,
                               _Pan:                [40.0, 60.0],
                               _ViewportSize:       [0.0, 0.0],
                               _FitToViewport:      true,
                               _Layout:             None,
                               _LayoutError:        None, };
        state.RebuildLayout();
        return state;
    }

    pub fn Document(&self) -> &FrescoDocument { return &self._Document; }
    pub fn Layout(&self) -> Option<&MathLayout> { return self._Layout.as_ref(); }
    pub fn LayoutError(&self) -> Option<&str> { return self._LayoutError.as_deref(); }
    pub fn RootCount(&self) -> u32 { return self._Document.RootCount(); }
    pub fn SelectedRoot(&self) -> u32 { return self._SelectedRoot; }
    pub fn SelectedOccurrenceId(&self) -> u32 { return self._SelectedOccurrence; }
    pub fn Zoom(&self) -> f32 { return self._Zoom; }
    pub fn Pan(&self) -> [f32; 2] { return self._Pan; }
    pub fn ViewportSize(&self) -> [f32; 2] { return self._ViewportSize; }

    pub fn SelectedRootName(&self) -> &str
    {
        return self._Document
                   .RootAt(self._SelectedRoot)
                   .map(|r| r.Name())
                   .unwrap_or("Unknown");
    }

    pub fn SelectedOccurrence(&self) -> Option<&MathOccurrence>
    {
        return self.Layout()?.OccurrenceAt(self._SelectedOccurrence);
    }

    pub fn HitTestDoc(&self, x: f32, y: f32) -> u32
    {
        return self.Layout()
                   .map(|layout| layout.HitTest(x, y))
                   .unwrap_or(u32::MAX);
    }

    pub fn RebuildLayout(&mut self)
    {
        self._SelectedOccurrence = u32::MAX;
        match MathLayoutBuilder::New(&self._Document,
                                     &CosmicMathFontMetrics::New(),
                                     self._SelectedRoot,
                                     20.0).Build()
        {
            Ok(layout) => {
                self._Layout = Some(layout);
                self._LayoutError = None;
            }
            Err(error) => {
                self._Layout = None;
                self._LayoutError = Some(error);
            }
        }
    }

    fn Fit(&mut self)
    {
        let Some(layout) = &self._Layout else {
            return;
        };
        let bounds = layout.Bounds();
        let [width, height] = self._ViewportSize;
        if width <= 0.0 || height <= 0.0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return;
        }
        let margin = 40.0f32.min(width * 0.1).min(height * 0.1);
        let fit_zoom = ((width - 2.0 * margin) / bounds.width).min((height - 2.0 * margin)
                                                                   / bounds.height)
                                                              .clamp(MIN_ZOOM, 2.5);
        self._Zoom = fit_zoom;
        self._Pan = [(width - bounds.width * self._Zoom) * 0.5 - bounds.x * self._Zoom,
                     (height - bounds.height * self._Zoom) * 0.5 - bounds.y * self._Zoom];
    }

    pub fn Update(&mut self, action: FrescoAction)
    {
        match action {
            FrescoAction::SelectRoot(index) => {
                if index < self.RootCount() && index != self._SelectedRoot {
                    self._SelectedRoot = index;
                    self.RebuildLayout();
                    self._FitToViewport = true;
                    self.Fit();
                }
            }
            FrescoAction::SelectOccurrence(id) => {
                self._SelectedOccurrence =
                    if self.Layout().and_then(|l| l.OccurrenceAt(id)).is_some() {
                        id
                    } else {
                        u32::MAX
                    };
            }
            FrescoAction::Zoom(factor, anchor) => {
                if !factor.is_finite() || factor <= 0.0 {
                    return;
                }
                let old = self._Zoom;
                let zoom = (old * factor).clamp(MIN_ZOOM, MAX_ZOOM);
                if let Some([x, y]) = anchor
                   && x.is_finite()
                   && y.is_finite()
                {
                    self._Pan[0] = x - (x - self._Pan[0]) / old * zoom;
                    self._Pan[1] = y - (y - self._Pan[1]) / old * zoom;
                }
                self._Zoom = zoom;
                self._FitToViewport = false;
            }
            FrescoAction::Pan(x, y) => {
                if x.is_finite() && y.is_finite() {
                    self._Pan[0] += x;
                    self._Pan[1] += y;
                    self._FitToViewport = false;
                }
            }
            FrescoAction::ResetView => {
                self._Zoom = 1.0;
                self._Pan = [40.0, 60.0];
                self._FitToViewport = false;
            }
            FrescoAction::FitView => {
                self._FitToViewport = true;
                self.Fit();
            }
            FrescoAction::NextRoot => {
                self.Update(FrescoAction::SelectRoot((self._SelectedRoot + 1) % self.RootCount()));
            }
            FrescoAction::PrevRoot => {
                self.Update(FrescoAction::SelectRoot(if self._SelectedRoot == 0 {
                                                         self.RootCount() - 1
                                                     } else {
                                                         self._SelectedRoot - 1
                                                     }));
            }
            FrescoAction::Resize(width, height) => {
                if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
                    self._ViewportSize = [width, height];
                    if self._FitToViewport {
                        self.Fit();
                    }
                }
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Resolve presentation roles without baking a particular desktop theme into layout.
pub fn MathColorForPalette(role: MathColor, palette: ThemePalette) -> Color
{
    return match role {
        MathColor::Text => palette.text_primary,
        MathColor::Muted => palette.text_secondary,
        MathColor::Accent | MathColor::Operator => palette.accent,
        MathColor::Selection => {
            Color::from_rgba(palette.accent.r, palette.accent.g, palette.accent.b, 0.2)
        }
    };
}

//-------------------------------------------------------------------------------------------------

/// Canvas Program for painting and interacting with the math layout.
pub struct FrescoCanvasProgram<'a, Message, F>
{
    _State:   &'a FrescoViewerState,
    _Palette: ThemePalette,
    _Map:     F,
    _Phantom: std::marker::PhantomData<Message>,
}

impl<'a, Message, F> FrescoCanvasProgram<'a, Message, F>
{
    pub fn New(state: &'a FrescoViewerState, palette: ThemePalette, map: F) -> Self
    {
        Self { _State:   state,
               _Palette: palette,
               _Map:     map,
               _Phantom: std::marker::PhantomData, }
    }
}

impl<'a, Message: 'static + Clone, F: Fn(FrescoAction) -> Message + Copy + 'static> Program<Message>
    for FrescoCanvasProgram<'a, Message, F>
{
    type State = FrescoCanvasInteraction;

    fn update(&self, interaction: &mut Self::State, event: &iced::Event, bounds: Rectangle,
              cursor: mouse::Cursor)
              -> Option<iced::widget::Action<Message>>
    {
        let isOver = cursor.is_over(bounds);
        let publish =
            |action| Some(iced::widget::Action::publish((self._Map)(action)).and_capture());

        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if isOver => {
                if let Some(pos) = cursor.position_in(bounds) {
                    interaction._IsDragging = true;
                    interaction._DragOrigin = Some([pos.x, pos.y]);
                    interaction._LastCursor = Some([pos.x, pos.y]);
                    interaction._PannedBeyondThreshold = false;
                    Some(iced::widget::Action::capture())
                } else {
                    None
                }
            }

            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if interaction._IsDragging
                   && let Some(pos) = cursor.position_in(bounds)
                {
                    let cursorPos = [pos.x, pos.y];
                    if let Some(origin) = interaction._DragOrigin {
                        let dxOrig = cursorPos[0] - origin[0];
                        let dyOrig = cursorPos[1] - origin[1];
                        // 4 logical pixel drag threshold
                        if dxOrig * dxOrig + dyOrig * dyOrig >= 16.0 {
                            interaction._PannedBeyondThreshold = true;
                        }
                    }

                    if interaction._PannedBeyondThreshold
                       && let Some(last) = interaction._LastCursor
                    {
                        let dx = cursorPos[0] - last[0];
                        let dy = cursorPos[1] - last[1];
                        interaction._LastCursor = Some(cursorPos);
                        return publish(FrescoAction::Pan(dx, dy));
                    }
                    interaction._LastCursor = Some(cursorPos);
                }
                None
            }

            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if interaction._IsDragging {
                    let wasPanning = interaction._PannedBeyondThreshold;
                    interaction._IsDragging = false;
                    let orig = interaction._DragOrigin.take();
                    interaction._LastCursor = None;

                    if !wasPanning
                       && let (Some(_orig), Some(pos)) = (orig, cursor.position_in(bounds))
                    {
                        // Click selection! Inverse transform to doc space
                        let docX = (pos.x - self._State._Pan[0]) / self._State._Zoom;
                        let docY = (pos.y - self._State._Pan[1]) / self._State._Zoom;
                        let hit = self._State.HitTestDoc(docX, docY);
                        return publish(FrescoAction::SelectOccurrence(hit));
                    }
                }
                None
            }

            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) if isOver => {
                let scrollY = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y * 0.02,
                };
                if scrollY == 0.0 || !scrollY.is_finite() {
                    return None;
                }
                let factor = if scrollY > 0.0 { 1.15 } else { 0.87 };
                let anchor = cursor.position_in(bounds).map(|p| [p.x, p.y]);
                publish(FrescoAction::Zoom(factor, anchor))
            }

            _ => None,
        }
    }

    fn draw(&self, _interaction: &Self::State, renderer: &iced::Renderer, _theme: &iced::Theme,
            bounds: Rectangle, _cursor: mouse::Cursor)
            -> Vec<Geometry>
    {
        let mut frame = Frame::new(renderer, bounds.size());
        let zoom = self._State._Zoom;
        let pan = self._State._Pan;

        // 1. Draw subtle background grid
        DrawBackgroundGrid(&mut frame, bounds, zoom, pan, &self._Palette);

        // 2. Draw selection highlight if active
        if let Some(occ) = self._State.SelectedOccurrence() {
            let sx = occ._Bounds.x * zoom + pan[0] - 4.0;
            let sy = occ._Bounds.y * zoom + pan[1] - 4.0;
            let sw = occ._Bounds.width * zoom + 8.0;
            let sh = occ._Bounds.height * zoom + 8.0;

            let fillCol = Color::from_rgba(self._Palette.accent.r,
                                           self._Palette.accent.g,
                                           self._Palette.accent.b,
                                           0.20);
            let strokeCol = Color::from_rgba(self._Palette.accent.r,
                                             self._Palette.accent.g,
                                             self._Palette.accent.b,
                                             0.75);

            frame.fill_rectangle(Point::new(sx, sy), Size::new(sw, sh), fillCol);
            frame.stroke(&Path::rectangle(Point::new(sx, sy), Size::new(sw, sh)),
                         Stroke::default().with_color(strokeCol).with_width(1.5));
        }

        if let Some(layout) = self._State.Layout() {
            PaintMathLayout(&mut frame, layout, bounds, zoom, pan, self._Palette);
        }

        vec![frame.into_geometry()]
    }
}

//-------------------------------------------------------------------------------------------------

/// State tracked during canvas interaction.
#[derive(Debug, Clone, Default)]
pub struct FrescoCanvasInteraction
{
    _IsDragging:            bool,
    _DragOrigin:            Option<[f32; 2]>,
    _LastCursor:            Option<[f32; 2]>,
    _PannedBeyondThreshold: bool,
}

struct FrescoPreview<'a>
{
    _Layout:  &'a MathLayout,
    _Palette: ThemePalette,
}

impl<Message> Program<Message> for FrescoPreview<'_>
{
    type State = ();

    fn draw(&self, _state: &(), renderer: &iced::Renderer, _theme: &iced::Theme,
            bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry>
    {
        let mut frame = Frame::new(renderer, bounds.size());
        let rect = self._Layout.Bounds();
        let zoom = ((bounds.width - 12.0) / rect.width.max(1.0))
            .min((bounds.height - 12.0) / rect.height.max(1.0))
            .min(0.9);
        if zoom >= 0.6 {
            let pan = [(bounds.width - rect.width * zoom) * 0.5 - rect.x * zoom,
                       (bounds.height - rect.height * zoom) * 0.5 - rect.y * zoom];
            PaintMathLayout(&mut frame, self._Layout, bounds, zoom, pan, self._Palette);
        } else {
            let mut hint = CosmicMathFontMetrics::CanvasText("See the full Math View".to_string(),
                                                             12.0, false, false);
            hint.position = Point::new(6.0, (bounds.height - 12.0) * 0.5);
            hint.color = self._Palette.text_secondary;
            frame.fill_text(hint);
        }
        return vec![frame.into_geometry()];
    }
}

fn PaintMathLayout(frame: &mut Frame, layout: &MathLayout, bounds: Rectangle, zoom: f32,
                   pan: [f32; 2], palette: ThemePalette)
{
    layout.Commands().Arr().Traverse(|cmd| {
        match cmd {
            MathDrawCommand::Text { text, x, yBaseline, paragraphBaselineOffset, fontSize, color,
                                   isBold, isItalic } => {
                let mut atom = CosmicMathFontMetrics::CanvasText(text.clone(), *fontSize * zoom,
                                                                 *isBold, *isItalic);
                atom.position = Point::new(*x * zoom + pan[0],
                    (*yBaseline - *paragraphBaselineOffset) * zoom + pan[1]);
                atom.color = MathColorForPalette(*color, palette);
                frame.fill_text(atom);
            }
            MathDrawCommand::Rule { bounds: r, color } => {
                let x = r.x * zoom + pan[0];
                let y = r.y * zoom + pan[1];
                let width = r.width * zoom;
                let height = (r.height * zoom).max(1.0);
                if x + width >= 0.0 && x <= bounds.width && y + height >= 0.0 && y <= bounds.height {
                    frame.fill_rectangle(Point::new(x, y), Size::new(width, height),
                                         MathColorForPalette(*color, palette));
                }
            }
            MathDrawCommand::Delimiter { kind, bounds: r, thickness, color } => {
                let x = r.x * zoom + pan[0];
                let y = r.y * zoom + pan[1];
                let width = r.width * zoom;
                let height = r.height * zoom;
                if x + width < 0.0 || x > bounds.width || y + height < 0.0 || y > bounds.height {
                    return;
                }
                let stroke = Stroke::default().with_color(MathColorForPalette(*color, palette))
                                              .with_width((*thickness * zoom).max(1.2));
                let path = match kind {
                    DelimiterKind::BracketLeft | DelimiterKind::BracketRight => {
                        let (cap, spine) = if *kind == DelimiterKind::BracketLeft {
                            (x + width, x)
                        } else {
                            (x, x + width)
                        };
                        Path::new(|builder| {
                            builder.move_to(Point::new(cap, y));
                            builder.line_to(Point::new(spine, y));
                            builder.line_to(Point::new(spine, y + height));
                            builder.line_to(Point::new(cap, y + height));
                        })
                    }
                    DelimiterKind::ParenthesisLeft | DelimiterKind::ParenthesisRight => {
                        let (cap, bend) = if *kind == DelimiterKind::ParenthesisLeft {
                            (x + width, x)
                        } else {
                            (x, x + width)
                        };
                        Path::new(|builder| {
                            builder.move_to(Point::new(cap, y));
                            builder.bezier_curve_to(Point::new(bend, y + height * 0.25),
                                                    Point::new(bend, y + height * 0.75),
                                                    Point::new(cap, y + height));
                        })
                    }
                    _ => Path::line(Point::new(x, y), Point::new(x, y + height)),
                };
                frame.stroke(&path, stroke);
            }
        }
    });
}

fn DrawBackgroundGrid(frame: &mut Frame, bounds: Rectangle, zoom: f32, pan: [f32; 2],
                      palette: &ThemePalette)
{
    let gridSpacing = 30.0 * zoom;
    if !(10.0..=120.0).contains(&gridSpacing) {
        return;
    }
    let offsetX = (pan[0]).rem_euclid(gridSpacing);
    let offsetY = (pan[1]).rem_euclid(gridSpacing);
    let dotColor = Color::from_rgba(palette.border.r, palette.border.g, palette.border.b, 0.18);

    let mut x = offsetX;
    while x < bounds.width {
        let mut y = offsetY;
        while y < bounds.height {
            frame.fill_rectangle(Point::new(x, y), Size::new(1.2, 1.2), dotColor);
            y += gridSpacing;
        }
        x += gridSpacing;
    }
}

//-------------------------------------------------------------------------------------------------

/// Renders the complete Math View tab.
pub fn ViewMathTab<'a, Message: 'static + Clone, F: Fn(FrescoAction) -> Message + Copy + 'static>(
    state: &'a FrescoViewerState, map: F, palette: ThemePalette)
    -> Element<'a, Message>
{
    let doc = state.Document();
    // 1. Top Toolbar
    let toolbar = ViewMathToolbar(state, map, palette);

    // 2. Center Math Canvas
    let canvasWidget =
        iced::widget::canvas(FrescoCanvasProgram::New(state, palette, map)).width(Length::Fill)
                                                                           .height(Length::Fill);

    let observedCanvas =
        sensor(canvasWidget).key(doc.Id())
                            .on_show(move |size| map(FrescoAction::Resize(size.width, size.height)))
                            .on_resize(move |size| {
                                map(FrescoAction::Resize(size.width, size.height))
                            });
    let canvasContainer =
        container(observedCanvas).width(Length::Fill)
                                 .height(Length::Fill)
                                 .style(move |_| FasciaStyle::content_container(palette));

    // 3. Right Inspector Sidebar
    let properties = FrescoProperties::Read(doc, state.SelectedRoot(), state.SelectedOccurrence());
    let preview = state.Layout().map(|layout| {
        iced::widget::canvas(FrescoPreview { _Layout: layout, _Palette: palette })
            .width(Length::Fill).height(Length::Fixed(72.0)).into()
    });
    let inspector = ViewMathProperties(properties, preview, palette);

    // 4. Main Body: Canvas + Inspector
    let mainBody = row![canvasContainer, inspector].width(Length::Fill)
                                                   .height(Length::Fill);

    // 5. Status Footer
    let statusStr = format!("Root {} of {}  \u{2022}  Zoom {:.0}%  \u{2022}  Selected: {}",
                            state._SelectedRoot + 1,
                            state.RootCount(),
                            state._Zoom * 100.0,
                            state.SelectedOccurrence()
                                 .map(|occ| format!("Occ #{}", occ._Id))
                                 .unwrap_or_else(|| "None".to_string()));

    let statusStr = if let Some(error) = state.LayoutError() {
        format!("{} | Layout error: {}", statusStr, error)
    } else {
        statusStr
    };
    let statusBar = container(row![
            text(statusStr)
                .size(11)
                .style(move |_| text::Style { color: Some(palette.text_secondary) }),
            Space::new().width(Length::Fill),
            text("Pan: Click+Drag  \u{2022}  Zoom: Scroll")
                .size(11)
                .style(move |_| text::Style { color: Some(palette.text_secondary) }),
        ].padding([4, 12])).width(Length::Fill)
                           .style(move |_| FasciaStyle::status_bar_container(palette));

    column![toolbar, mainBody, statusBar].width(Length::Fill)
                                         .height(Length::Fill)
                                         .into()
}

fn ViewMathToolbar<'a, Message: 'static + Clone, F: Fn(FrescoAction) -> Message + Copy + 'static>(
    state: &'a FrescoViewerState, map: F, palette: ThemePalette)
    -> Element<'a, Message>
{
    let prevBtn =
        button(text("<").size(12)).on_press(map(FrescoAction::PrevRoot))
                                  .style(move |_, status| {
                                      FasciaStyle::toolbar_button(palette, status)
                                  });

    let nextBtn =
        button(text(">").size(12)).on_press(map(FrescoAction::NextRoot))
                                  .style(move |_, status| {
                                      FasciaStyle::toolbar_button(palette, status)
                                  });

    let rootLabel =
        text(format!("Root: {} ({}/{})",
                     state.SelectedRootName(),
                     state._SelectedRoot + 1,
                     state.RootCount())).size(13)
                                        .style(move |_| {
                                            text::Style { color: Some(palette.text_primary), }
                                        });

    let zoomOutBtn =
        button(text("-").size(12)).on_press(map(FrescoAction::Zoom(0.85, None)))
                                  .style(move |_, status| {
                                      FasciaStyle::toolbar_button(palette, status)
                                  });

    let zoomInBtn =
        button(text("+").size(12)).on_press(map(FrescoAction::Zoom(1.15, None)))
                                  .style(move |_, status| {
                                      FasciaStyle::toolbar_button(palette, status)
                                  });

    let zoomPct = text(format!("{:.0}%", state._Zoom * 100.0))
        .size(12)
        .style(move |_| text::Style { color: Some(palette.text_secondary) });

    let fitBtn = button(text("Fit").size(12)).on_press(map(FrescoAction::FitView))
                                             .style(move |_, status| {
                                                 FasciaStyle::toolbar_button(palette, status)
                                             });

    let resetBtn = button(text("Reset").size(12)).on_press(map(FrescoAction::ResetView))
                                                 .style(move |_, status| {
                                                     FasciaStyle::toolbar_button(palette, status)
                                                 });

    container(row![prevBtn,
                   nextBtn,
                   Space::new().width(Length::Fixed(6.0)),
                   rootLabel,
                   Space::new().width(Length::Fill),
                   zoomOutBtn,
                   zoomPct,
                   zoomInBtn,
                   Space::new().width(Length::Fixed(8.0)),
                   fitBtn,
                   resetBtn,].align_y(Alignment::Center)
                             .spacing(6)
                             .padding([6, 12])).width(Length::Fill)
                                               .style(move |_| {
                                                   FasciaStyle::toolbar_container(palette)
                                               })
                                               .into()
}

//-------------------------------------------------------------------------------------------------
