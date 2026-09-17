// src/fascia/tabs.rs
use crate::fascia::theme::{FasciaStyle, ThemePalette};
use iced::widget::{Space, button, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use std::path::PathBuf;

//-------------------------------------------------------------------------------------------------

/// Unique ID for a tab in the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(pub u64);
/// Represents the nature of content displayed in the tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabKind {
    Welcome,
    FileEditor,
    Settings,
}
/// Metadata and state for an open tab.
#[derive(Debug, Clone)]
pub struct TabItem {
    pub id: TabId,
    pub title: String,
    pub path: Option<PathBuf>,
    pub kind: TabKind,
    pub is_dirty: bool,
    pub icon: &'static str,
}
impl TabItem {
    pub fn new_welcome(id: TabId) -> Self {
        Self {
            id,
            title: "Welcome".to_string(),
            path: None,
            kind: TabKind::Welcome,
            is_dirty: false,
            icon: "👋",
        }
    }
    pub fn new_settings(id: TabId) -> Self {
        Self {
            id,
            title: "Settings".to_string(),
            path: None,
            kind: TabKind::Settings,
            is_dirty: false,
            icon: "⚙",
        }
    }
    pub fn new_file(id: TabId, path: PathBuf) -> Self {
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let icon = match ext.as_str() {
            "rs" => "🦀",
            "toml" | "json" | "yaml" | "yml" => "⚙",
            "md" | "txt" => "📝",
            "png" | "jpg" | "svg" => "🖼",
            "html" | "css" | "js" | "ts" => "🌐",
            _ => "📄",
        };
        Self {
            id,
            title,
            path: Some(path),
            kind: TabKind::FileEditor,
            is_dirty: false,
            icon,
        }
    }
}

//-------------------------------------------------------------------------------------------------

/// Actions emitted by the tab bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabBarAction {
    SelectTab(usize),
    CloseTab(usize),
    NewTab,
    CloseAll,
}
/// Constructs the VS Code-style horizontal tab bar.
pub fn view_tab_bar<'a, Message: 'static + Clone>(
    tabs: &'a [TabItem],
    active_index: Option<usize>,
    palette: ThemePalette,
    map_action: impl Fn(TabBarAction) -> Message + Copy + 'static,
) -> Element<'a, Message> {
    let mut tab_elements = row![].spacing(1).align_y(Alignment::Center);
    for (index, tab) in tabs.iter().enumerate() {
        let is_active = active_index == Some(index);
        let dirty_or_spacer: Element<'a, Message> = if tab.is_dirty {
            text("●")
                .size(10)
                .style(move |_| text::Style {
                    color: Some(palette.accent),
                })
                .into()
        } else {
            Space::new().width(Length::Fixed(10.0)).into()
        };
        let close_btn = button(text("✕").size(11).style(move |_| text::Style {
            color: Some(palette.text_secondary),
        }))
        .padding([2, 5])
        .style(move |_, status| FasciaStyle::tab_close_button(palette, status))
        .on_press(map_action(TabBarAction::CloseTab(index)));
        let tab_inner = row![
            text(tab.icon).size(13),
            Space::new().width(Length::Fixed(6.0)),
            text(&tab.title).size(13).style(move |_| text::Style {
                color: if is_active {
                    Some(palette.text_primary)
                } else {
                    Some(palette.text_secondary)
                },
            }),
            Space::new().width(Length::Fixed(6.0)),
            dirty_or_spacer,
            close_btn,
        ]
        .align_y(Alignment::Center);
        let tab_btn = button(tab_inner)
            .padding([5, 10])
            .style(move |_, status| FasciaStyle::tab_button(palette, is_active, status))
            .on_press(map_action(TabBarAction::SelectTab(index)));
        tab_elements = tab_elements.push(tab_btn);
    }
    let add_btn = button(text("+").size(14).style(move |_| text::Style {
        color: Some(palette.text_secondary),
    }))
    .padding([3, 8])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
    .on_press(map_action(TabBarAction::NewTab));
    tab_elements = tab_elements
        .push(Space::new().width(Length::Fixed(4.0)))
        .push(add_btn);
    let scrollable_tabs = scrollable(tab_elements)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default().width(4.0),
        ))
        .width(Length::Fill);
    let bar = row![scrollable_tabs]
        .align_y(Alignment::Center)
        .width(Length::Fill);
    container(bar)
        .width(Length::Fill)
        .style(move |_| FasciaStyle::tab_bar_container(palette))
        .into()
}

//-------------------------------------------------------------------------------------------------

/// Manages multi-tab state for the editor content area.
#[derive(Debug, Clone)]
pub struct TabManager {
    tabs: Vec<TabItem>,
    active_index: Option<usize>,
    next_id: u64,
}
impl Default for TabManager {
    fn default() -> Self {
        let mut manager = Self {
            tabs: Vec::new(),
            active_index: None,
            next_id: 1,
        };
        manager.open_welcome();
        manager
    }
}
impl TabManager {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn tabs(&self) -> &[TabItem] {
        &self.tabs
    }
    pub fn active_index(&self) -> Option<usize> {
        self.active_index
    }
    pub fn active_tab(&self) -> Option<&TabItem> {
        self.active_index.and_then(|i| self.tabs.get(i))
    }
    pub fn active_tab_mut(&mut self) -> Option<&mut TabItem> {
        if let Some(i) = self.active_index {
            self.tabs.get_mut(i)
        } else {
            None
        }
    }
    fn allocate_id(&mut self) -> TabId {
        let id = TabId(self.next_id);
        self.next_id += 1;
        id
    }
    pub fn select_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_index = Some(index);
        }
    }
    pub fn open_welcome(&mut self) {
        if let Some(pos) = self.tabs.iter().position(|t| t.kind == TabKind::Welcome) {
            self.active_index = Some(pos);
            return;
        }
        let id = self.allocate_id();
        self.tabs.push(TabItem::new_welcome(id));
        self.active_index = Some(self.tabs.len() - 1);
    }
    pub fn open_settings(&mut self) {
        if let Some(pos) = self.tabs.iter().position(|t| t.kind == TabKind::Settings) {
            self.active_index = Some(pos);
            return;
        }
        let id = self.allocate_id();
        self.tabs.push(TabItem::new_settings(id));
        self.active_index = Some(self.tabs.len() - 1);
    }
    pub fn open_file(&mut self, path: PathBuf) -> (usize, bool) {
        if let Some(pos) = self
            .tabs
            .iter()
            .position(|t| t.path.as_ref() == Some(&path))
        {
            self.active_index = Some(pos);
            return (pos, false);
        }
        let id = self.allocate_id();
        let tab = TabItem::new_file(id, path);
        self.tabs.push(tab);
        let new_idx = self.tabs.len() - 1;
        self.active_index = Some(new_idx);
        (new_idx, true)
    }
    pub fn open_new_file(&mut self) {
        let id = self.allocate_id();
        let mut tab = TabItem::new_file(id, PathBuf::from(format!("Untitled-{}", id.0)));
        tab.path = None;
        tab.is_dirty = true;
        self.tabs.push(tab);
        self.active_index = Some(self.tabs.len() - 1);
    }
    pub fn close_tab(&mut self, index: usize) -> Option<TabItem> {
        if index >= self.tabs.len() {
            return None;
        }
        let removed = self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.active_index = None;
        } else if let Some(current) = self.active_index {
            if current >= self.tabs.len() {
                self.active_index = Some(self.tabs.len() - 1);
            } else if current > index {
                self.active_index = Some(current - 1);
            }
        }
        Some(removed)
    }
    pub fn close_all(&mut self) {
        self.tabs.clear();
        self.active_index = None;
    }
    pub fn mark_dirty(&mut self, index: usize, dirty: bool) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.is_dirty = dirty;
        }
    }
    pub fn mark_active_dirty(&mut self, dirty: bool) {
        if let Some(idx) = self.active_index {
            self.mark_dirty(idx, dirty);
        }
    }
}
