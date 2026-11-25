// src/fascia/shell.rs
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use iced::widget::{Space, column, container, mouse_area, row};
use iced::{Element, Length, Point, mouse};

//-------------------------------------------------------------------------------------------------

/// Logical workbench regions for focus tracking and keyboard routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneId
{
    Explorer,
    Content,
    Properties,
    BottomTray,
    Popup,
}

/// Draggable splitter boundaries in the workbench shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitTarget
{
    Sidebar,
    Properties,
    BottomTray,
}

/// Actions emitted by interactive shell splitters and pane focus regions.
#[derive(Debug, Clone, PartialEq)]
pub enum ShellAction
{
    SplitterPressed(SplitTarget),
    SplitterDoubleClicked(SplitTarget),
    SplitterHovered(Option<SplitTarget>),
    FocusPane(PaneId),
}

/// Persistent pane sizes and active splitter drag state.
#[derive(Debug, Clone)]
pub struct WorkbenchLayout
{
    pub sidebar_width:      f32,
    pub properties_width:   f32,
    pub bottom_tray_height: f32,
    pub focused_pane:       PaneId,
    pub hovered_splitter:   Option<SplitTarget>,
    _ActiveDrag:            Option<(SplitTarget, Point, f32)>,
}

impl Default for WorkbenchLayout
{
    fn default() -> Self
    {
        return Self {
            sidebar_width:      260.0,
            properties_width:   290.0,
            bottom_tray_height: 220.0,
            focused_pane:       PaneId::Content,
            hovered_splitter:   None,
            _ActiveDrag:        None,
        };
    }
}

impl WorkbenchLayout
{
    pub fn IsDragging(&self) -> bool { return self._ActiveDrag.is_some(); }

    pub fn BeginDrag(&mut self, target: SplitTarget, cursor: Point)
    {
        let start_val = match target {
            SplitTarget::Sidebar => self.sidebar_width,
            SplitTarget::Properties => self.properties_width,
            SplitTarget::BottomTray => self.bottom_tray_height,
        };
        self._ActiveDrag = Some((target, cursor, start_val));
    }

    pub fn UpdateDrag(&mut self, cursor: Point)
    {
        let Some((target, origin, start_val)) = self._ActiveDrag else { return; };
        match target {
            SplitTarget::Sidebar => {
                self.sidebar_width = (start_val + (cursor.x - origin.x)).clamp(180.0, 520.0);
            }
            SplitTarget::Properties => {
                self.properties_width = (start_val - (cursor.x - origin.x)).clamp(220.0, 520.0);
            }
            SplitTarget::BottomTray => {
                self.bottom_tray_height = (start_val - (cursor.y - origin.y)).clamp(120.0, 500.0);
            }
        }
    }

    pub fn EndDrag(&mut self) { self._ActiveDrag = None; }

    pub fn ResetSplit(&mut self, target: SplitTarget)
    {
        match target {
            SplitTarget::Sidebar => self.sidebar_width = 260.0,
            SplitTarget::Properties => self.properties_width = 290.0,
            SplitTarget::BottomTray => self.bottom_tray_height = 220.0,
        }
    }
}

//-------------------------------------------------------------------------------------------------

fn vertical_splitter<'a, Message: 'static + Clone>(
    target: SplitTarget,
    layout: &WorkbenchLayout,
    palette: ThemePalette,
    on_action: impl Fn(ShellAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let is_hovered = layout.hovered_splitter == Some(target);
    let is_dragging = layout._ActiveDrag.is_some_and(|(t, _, _)| t == target);
    let handle = container(Space::new().width(Length::Fixed(4.0)).height(Length::Fill))
        .style(move |_| FasciaStyle::splitter_handle(palette, is_hovered, is_dragging));
    return mouse_area(handle)
        .interaction(mouse::Interaction::ResizingHorizontally)
        .on_press(on_action(ShellAction::SplitterPressed(target)))
        .on_double_click(on_action(ShellAction::SplitterDoubleClicked(target)))
        .on_enter(on_action(ShellAction::SplitterHovered(Some(target))))
        .on_exit(on_action(ShellAction::SplitterHovered(None)))
        .into();
}

fn horizontal_splitter<'a, Message: 'static + Clone>(
    target: SplitTarget,
    layout: &WorkbenchLayout,
    palette: ThemePalette,
    on_action: impl Fn(ShellAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let is_hovered = layout.hovered_splitter == Some(target);
    let is_dragging = layout._ActiveDrag.is_some_and(|(t, _, _)| t == target);
    let handle = container(Space::new().width(Length::Fill).height(Length::Fixed(4.0)))
        .style(move |_| FasciaStyle::splitter_handle(palette, is_hovered, is_dragging));
    return mouse_area(handle)
        .interaction(mouse::Interaction::ResizingVertically)
        .on_press(on_action(ShellAction::SplitterPressed(target)))
        .on_double_click(on_action(ShellAction::SplitterDoubleClicked(target)))
        .on_enter(on_action(ShellAction::SplitterHovered(Some(target))))
        .on_exit(on_action(ShellAction::SplitterHovered(None)))
        .into();
}

/// Builds the complete workbench shell layout with interactive resize splitters.
#[allow(clippy::too_many_arguments)]
pub fn view_shell_with_layout<'a, Message: 'static + Clone>(
    menubar: Element<'a, Message>,
    toolbar: Option<Element<'a, Message>>,
    activity_bar: Element<'a, Message>,
    sidebar: Option<Element<'a, Message>>,
    content: Element<'a, Message>,
    bottom_tray: Option<Element<'a, Message>>,
    properties: Option<Element<'a, Message>>,
    status_bar: Option<Element<'a, Message>>,
    layout_state: &WorkbenchLayout,
    palette: ThemePalette,
    on_action: impl Fn(ShellAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let mut center_col = column![container(content).width(Length::Fill).height(Length::Fill)]
        .width(Length::Fill)
        .height(Length::Fill);
    if let Some(tray) = bottom_tray {
        center_col = center_col
            .push(horizontal_splitter(SplitTarget::BottomTray, layout_state, palette, on_action))
            .push(
                container(tray)
                    .width(Length::Fill)
                    .height(Length::Fixed(layout_state.bottom_tray_height)),
            );
    }

    let mut middle_row = row![activity_bar].height(Length::Fill);
    if let Some(sb) = sidebar {
        middle_row = middle_row
            .push(
                container(sb)
                    .width(Length::Fixed(layout_state.sidebar_width))
                    .height(Length::Fill),
            )
            .push(vertical_splitter(SplitTarget::Sidebar, layout_state, palette, on_action));
    }
    middle_row = middle_row.push(
        container(center_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| FasciaStyle::content_container(palette)),
    );
    if let Some(props) = properties {
        middle_row = middle_row
            .push(vertical_splitter(SplitTarget::Properties, layout_state, palette, on_action))
            .push(
                container(props)
                    .width(Length::Fixed(layout_state.properties_width))
                    .height(Length::Fill),
            );
    }

    let mut layout = column![menubar];
    if let Some(tb) = toolbar {
        layout = layout.push(tb);
    }
    layout = layout.push(middle_row);
    if let Some(sb) = status_bar {
        layout = layout.push(sb);
    }
    return container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
}

/// Builds the complete workbench shell layout containing Top Bar (or Menubar + Toolbar),
/// Activity Bar, Sidebar, Content Area, Bottom Tray, Properties Panel, and Status Bar.
#[allow(clippy::too_many_arguments)]
pub fn view_shell<'a, Message: 'static>(
    menubar: Element<'a, Message>,
    toolbar: Option<Element<'a, Message>>,
    activity_bar: Element<'a, Message>,
    sidebar: Option<Element<'a, Message>>,
    content: Element<'a, Message>,
    bottom_tray: Option<Element<'a, Message>>,
    properties: Option<Element<'a, Message>>,
    status_bar: Option<Element<'a, Message>>,
    palette: ThemePalette,
) -> Element<'a, Message>
{
    let mut center_col = column![content].width(Length::Fill).height(Length::Fill);
    if let Some(tray) = bottom_tray {
        center_col = center_col.push(tray);
    }

    let mut middle_row = row![activity_bar].height(Length::Fill);
    if let Some(sb) = sidebar {
        middle_row = middle_row.push(sb);
    }
    middle_row = middle_row.push(
        container(center_col)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| FasciaStyle::content_container(palette)),
    );
    if let Some(props) = properties {
        middle_row = middle_row.push(props);
    }

    let mut layout = column![menubar];
    if let Some(tb) = toolbar {
        layout = layout.push(tb);
    }
    layout = layout.push(middle_row);
    if let Some(sb) = status_bar {
        layout = layout.push(sb);
    }
    return container(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
}
