// popups.rs --------------------------------------------------------------------------------------
//! Unified overlay popups: Quick Open, Command Palette, cursor-anchored Context Menus, and Toasts.

use crate::fascia::command::{CommandContext, CommandId, CommandRegistry};
use crate::fascia::icons::{IconId, ViewIcon};
use crate::fascia::output::JobId;
use crate::fascia::tabs::TabItem;
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use crate::silo::Arr;
use iced::widget::{Space, button, column, container, mouse_area, opaque, row, scrollable, stack, text, text_input};
use iced::{Alignment, Background, Element, Length, Point};
use std::path::PathBuf;
use std::time::Instant;

//-------------------------------------------------------------------------------------------------

/// Active modal popup mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupMode
{
    None,
    QuickOpen,
    CommandPalette,
}

/// Actions emitted by context menu items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextAction
{
    OpenFile(PathBuf),
    InspectPath(PathBuf),
    RunExample(String),
    CopyText(String),
    SelectTab(usize),
    CloseTab(usize),
    CloseOtherTabs(usize),
    CloseAllTabs,
    CancelJob(JobId),
    ExecuteCommand(CommandId),
}

/// Single entry inside a floating context menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMenuItem
{
    pub label:    String,
    pub shortcut: Option<&'static str>,
    pub icon:     Option<IconId>,
    pub action:   ContextAction,
    pub enabled:  bool,
}

impl ContextMenuItem
{
    pub fn New(label: impl Into<String>, action: ContextAction) -> Self
    {
        return Self {
            label:    label.into(),
            shortcut: None,
            icon:     None,
            action,
            enabled:  true,
        };
    }

    pub fn WithIcon(mut self, icon: IconId) -> Self
    {
        self.icon = Some(icon);
        return self;
    }

    pub fn WithShortcut(mut self, shortcut: &'static str) -> Self
    {
        self.shortcut = Some(shortcut);
        return self;
    }
}

/// Cursor-anchored floating context menu state.
#[derive(Debug, Clone)]
pub struct ContextMenuState
{
    pub position: Point,
    pub items:    Vec<ContextMenuItem>,
}

/// Visual severity for transient toast notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel
{
    Info,
    Success,
    Warning,
    Error,
}

/// Single transient toast notification.
#[derive(Debug, Clone)]
pub struct ToastItem
{
    pub id:         u64,
    pub level:      ToastLevel,
    pub title:      String,
    pub detail:     Option<String>,
    pub created_at: Instant,
}

/// Actions emitted by the popup & overlay subsystem.
#[derive(Debug, Clone)]
pub enum PopupAction
{
    OpenQuickOpen,
    OpenCommandPalette,
    CloseModal,
    QueryChanged(String),
    MoveSelection(i32),
    SubmitSelection,
    ExecuteCommand(CommandId),
    SelectTab(usize),
    OpenFile(PathBuf),
    DismissContextMenu,
    TriggerContextAction(ContextAction),
    DismissToast(u64),
}

/// Central state for popups, context menus, and toast notifications.
#[derive(Debug, Clone)]
pub struct PopupState
{
    pub mode:           PopupMode,
    pub query:          String,
    pub selected_index: usize,
    pub context_menu:   Option<ContextMenuState>,
    pub toasts:         Vec<ToastItem>,
    _NextToastId:       u64,
}

impl Default for PopupState
{
    fn default() -> Self { return Self::New(); }
}

impl PopupState
{
    pub fn New() -> Self
    {
        return Self {
            mode:           PopupMode::None,
            query:          String::new(),
            selected_index: 0,
            context_menu:   None,
            toasts:         Vec::new(),
            _NextToastId:   1,
        };
    }

    pub fn IsModalOpen(&self) -> bool { return self.mode != PopupMode::None; }

    pub fn Open(&mut self, mode: PopupMode)
    {
        self.mode = mode;
        self.query.clear();
        self.selected_index = 0;
        self.context_menu = None;
    }

    pub fn CloseModal(&mut self)
    {
        self.mode = PopupMode::None;
        self.query.clear();
        self.selected_index = 0;
    }

    pub fn ShowContextMenu(&mut self, position: Point, items: Vec<ContextMenuItem>)
    {
        if items.is_empty() {
            return;
        }
        self.context_menu = Some(ContextMenuState { position, items });
    }

    pub fn DismissContextMenu(&mut self)
    {
        self.context_menu = None;
    }

    pub fn PushToast(
        &mut self,
        level: ToastLevel,
        title: impl Into<String>,
        detail: Option<String>,
    )
    {
        let id = self._NextToastId;
        self._NextToastId += 1;
        self.toasts.push(ToastItem {
            id,
            level,
            title: title.into(),
            detail,
            created_at: Instant::now(),
        });
        while self.toasts.len() > 3 {
            self.toasts.remove(0);
        }
    }

    pub fn DismissToast(&mut self, id: u64)
    {
        self.toasts.retain(|t| t.id != id);
    }

    pub fn PruneExpiredToasts(&mut self)
    {
        let now = Instant::now();
        self.toasts.retain(|t| now.duration_since(t.created_at).as_secs_f32() < 4.5);
    }
}

//-------------------------------------------------------------------------------------------------

/// Filtered Quick Open entry.
pub enum QuickOpenEntry
{
    Tab(usize, String),
    Command(CommandId, &'static str, &'static str, Option<&'static str>, bool),
}

pub fn CollectPaletteEntries(
    mode: PopupMode,
    query: &str,
    tabs: Arr<'_, TabItem>,
    ctx: &CommandContext,
) -> Vec<QuickOpenEntry>
{
    let q = query.trim().to_lowercase();
    let show_commands = mode == PopupMode::CommandPalette || q.starts_with('>');
    let needle = q.strip_prefix('>').unwrap_or(&q).trim();
    let mut out = Vec::new();

    if show_commands {
        for cmd in CommandRegistry::All() {
            let matches = needle.is_empty()
                || cmd.label.to_lowercase().contains(needle)
                || cmd.category.to_lowercase().contains(needle);
            if matches {
                out.push(QuickOpenEntry::Command(
                    cmd.id,
                    cmd.category,
                    cmd.label,
                    cmd.ShortcutDisplay(),
                    cmd.IsAvailable(ctx),
                ));
            }
        }
    } else {
        for idx in 0..tabs.Len() {
            if let Some(tab) = tabs.Get(idx) {
                if needle.is_empty() || tab.title.to_lowercase().contains(needle) {
                    out.push(QuickOpenEntry::Tab(idx as usize, tab.title.clone()));
                }
            }
        }
        for cmd in CommandRegistry::All() {
            if !needle.is_empty()
                && (cmd.label.to_lowercase().contains(needle)
                    || cmd.category.to_lowercase().contains(needle))
            {
                out.push(QuickOpenEntry::Command(
                    cmd.id,
                    cmd.category,
                    cmd.label,
                    cmd.ShortcutDisplay(),
                    cmd.IsAvailable(ctx),
                ));
            }
        }
    }
    return out;
}

/// Renders the full overlay stack (backdrop + Quick Open / Command Palette + Context Menu + Toasts).
pub fn ViewPopupOverlay<'a, Message: 'static + Clone>(
    base: Element<'a, Message>,
    state: &'a PopupState,
    tabs: Arr<'a, TabItem>,
    ctx: &CommandContext,
    palette: ThemePalette,
    on_action: impl Fn(PopupAction) -> Message + Copy + 'static,
) -> Element<'a, Message>
{
    let mut layers = vec![base];

    if state.mode != PopupMode::None {
        let placeholder = if state.mode == PopupMode::CommandPalette {
            "> Type a command to run..."
        } else {
            "Search open tabs or type > for commands..."
        };
        let entries = CollectPaletteEntries(state.mode, &state.query, tabs, ctx);
        let selected_idx = state.selected_index.min(entries.len().saturating_sub(1));

        let input = text_input(placeholder, &state.query)
            .on_input(move |s| on_action(PopupAction::QueryChanged(s)))
            .on_submit(on_action(PopupAction::SubmitSelection))
            .padding([8, 12])
            .size(13);

        let mut list_col = column![].spacing(2);
        if entries.is_empty() {
            list_col = list_col.push(
                container(
                    text("No matching items")
                        .size(12)
                        .style(move |_| text::Style { color: Some(palette.text_muted) }),
                )
                .padding([10, 12]),
            );
        } else {
            for (idx, entry) in entries.into_iter().take(12).enumerate() {
                let is_sel = idx == selected_idx;
                let row_btn = match entry {
                    QuickOpenEntry::Tab(tab_idx, title) => {
                        let content = row![
                            ViewIcon(IconId::FileText, 13.0, palette.accent),
                            Space::new().width(8),
                            text(title).size(12).style(move |_| text::Style {
                                color: Some(palette.text_primary),
                            }),
                            Space::new().width(Length::Fill),
                            text("Open Tab").size(10).style(move |_| text::Style {
                                color: Some(palette.text_muted),
                            }),
                        ]
                        .align_y(Alignment::Center);
                        button(content)
                            .width(Length::Fill)
                            .padding([6, 10])
                            .style(move |_, status| FasciaStyle::tree_row_button(palette, is_sel, status))
                            .on_press(on_action(PopupAction::SelectTab(tab_idx)))
                    }
                    QuickOpenEntry::Command(cmd_id, category, label, shortcut, enabled) => {
                        let label_color = if enabled { palette.text_primary } else { palette.text_muted };
                        let mut content = row![
                            ViewIcon(IconId::ExampleBolt, 12.0, if enabled { palette.accent } else { palette.text_muted }),
                            Space::new().width(8),
                            text(format!("{}: {}", category, label)).size(12).style(move |_| text::Style {
                                color: Some(label_color),
                            }),
                            Space::new().width(Length::Fill),
                        ]
                        .align_y(Alignment::Center);
                        if let Some(sc) = shortcut {
                            content = content.push(text(sc).size(10).style(move |_| text::Style {
                                color: Some(palette.text_muted),
                            }));
                        }
                        let mut btn = button(content)
                            .width(Length::Fill)
                            .padding([6, 10])
                            .style(move |_, status| FasciaStyle::tree_row_button(palette, is_sel, status));
                        if enabled {
                            btn = btn.on_press(on_action(PopupAction::ExecuteCommand(cmd_id)));
                        }
                        btn
                    }
                };
                list_col = list_col.push(row_btn);
            }
        }

        let modal = container(
            column![
                row![
                    ViewIcon(IconId::Search, 14.0, palette.accent),
                    Space::new().width(8),
                    text(if state.mode == PopupMode::CommandPalette {
                        "Command Palette"
                    } else {
                        "Quick Open"
                    })
                    .size(12)
                    .style(move |_| text::Style { color: Some(palette.text_secondary) }),
                    Space::new().width(Length::Fill),
                    text("Esc to close").size(10).style(move |_| text::Style {
                        color: Some(palette.text_muted),
                    }),
                ]
                .align_y(Alignment::Center),
                Space::new().height(8),
                input,
                Space::new().height(8),
                container(scrollable(list_col)).max_height(320.0),
            ]
            .padding(12),
        )
        .width(Length::Fixed(560.0))
        .style(move |_| FasciaStyle::overlay_container(palette));

        let scrim = mouse_area(
            container(
                column![
                    Space::new().height(Length::Fixed(56.0)),
                    mouse_area(modal).on_press(on_action(PopupAction::MoveSelection(0))),
                ]
                .width(Length::Fill)
                .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(palette.backdrop())),
                ..Default::default()
            }),
        )
        .on_press(on_action(PopupAction::CloseModal));

        layers.push(opaque(scrim));
    }

    if let Some(menu) = &state.context_menu {
        let mut items_col = column![].spacing(2);
        for item in &menu.items {
            let fg = if item.enabled { palette.text_primary } else { palette.text_muted };
            let mut r = row![].align_y(Alignment::Center);
            if let Some(icon) = item.icon {
                r = r.push(ViewIcon(icon, 12.0, if item.enabled { palette.text_secondary } else { palette.text_muted }))
                     .push(Space::new().width(8));
            }
            r = r.push(text(item.label.clone()).size(12).style(move |_| text::Style { color: Some(fg) }))
                 .push(Space::new().width(Length::Fill));
            if let Some(sc) = item.shortcut {
                r = r.push(text(sc).size(10).style(move |_| text::Style { color: Some(palette.text_muted) }));
            }
            let mut btn = button(r)
                .width(Length::Fill)
                .padding([5, 10])
                .style(move |_, status| FasciaStyle::menu_item_button(palette, status));
            if item.enabled {
                btn = btn.on_press(on_action(PopupAction::TriggerContextAction(item.action.clone())));
            }
            items_col = items_col.push(btn);
        }

        let card = container(items_col)
            .width(Length::Fixed(210.0))
            .padding(4)
            .style(move |_| FasciaStyle::overlay_container(palette));

        let anchored = mouse_area(
            container(
                column![
                    Space::new().height(Length::Fixed(menu.position.y.max(4.0))),
                    row![
                        Space::new().width(Length::Fixed(menu.position.x.max(4.0))),
                        card,
                    ],
                ]
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .on_press(on_action(PopupAction::DismissContextMenu));

        layers.push(anchored.into());
    }

    if !state.toasts.is_empty() {
        let mut toast_col = column![].spacing(6).align_x(Alignment::End);
        for toast in &state.toasts {
            let (icon, accent_color) = match toast.level {
                ToastLevel::Info => (IconId::OutputLog, palette.accent),
                ToastLevel::Success => (IconId::Check, palette.status_success()),
                ToastLevel::Warning => (IconId::AlertWarn, palette.status_warning()),
                ToastLevel::Error => (IconId::AlertError, palette.status_error()),
            };
            let id = toast.id;
            let close_btn = button(ViewIcon(IconId::Close, 10.0, palette.text_muted))
                .padding(2)
                .style(move |_, status| FasciaStyle::tab_close_button(palette, status))
                .on_press(on_action(PopupAction::DismissToast(id)));
            let mut body = column![
                row![
                    ViewIcon(icon, 13.0, accent_color),
                    Space::new().width(6),
                    text(toast.title.clone()).size(12).style(move |_| text::Style {
                        color: Some(palette.text_primary),
                    }),
                    Space::new().width(Length::Fill),
                    close_btn,
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(3);
            if let Some(detail) = &toast.detail {
                body = body.push(text(detail.clone()).size(11).style(move |_| text::Style {
                    color: Some(palette.text_secondary),
                }));
            }
            let card = container(body)
                .width(Length::Fixed(300.0))
                .padding([8, 10])
                .style(move |_| FasciaStyle::toast_container(palette, accent_color));
            toast_col = toast_col.push(card);
        }

        let host = container(
            column![
                Space::new().height(Length::Fill),
                row![Space::new().width(Length::Fill), toast_col],
                Space::new().height(Length::Fixed(30.0)),
            ]
            .padding([0, 14]),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        layers.push(host.into());
    }

    return stack(layers).into();
}
