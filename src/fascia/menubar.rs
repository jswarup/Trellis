// src/fascia/menubar.rs
use crate::fascia::theme::{FasciaStyle, FasciaTheme, ThemePalette};
use iced::widget::{Space, button, container, row, text};
use iced::{Alignment, Element, Length};
use iced_aw::menu::{Item, Menu, MenuBar};
/// Actions emitted by the menubar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    NewFile,
    OpenFile,
    OpenFolder,
    SaveFile,
    CloseTab,
    CloseAllTabs,
    Exit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    ToggleSidebar,
    ToggleStatusBar,
    OpenExplorer,
    OpenSearch,
    OpenSettings,
    SelectTheme(FasciaTheme),
    About,
    Documentation,
}
/// Helper to render an item button with label and shortcut.
fn menu_button<'a, Message: 'static + Clone>(
    label: &'static str,
    shortcut: Option<&'static str>,
    action: Option<Message>,
    palette: ThemePalette,
) -> button::Button<'a, Message> {
    let mut content = row![text(label).size(13).style(move |_| text::Style {
        color: Some(palette.text_primary),
    })]
    .align_y(Alignment::Center)
    .width(Length::Fill);
    if let Some(sc) = shortcut {
        content = content
            .push(Space::new().width(Length::Fill))
            .push(text(sc).size(11).style(move |_| text::Style {
                color: Some(palette.text_muted),
            }));
    }
    let mut btn = button(content)
        .padding([4, 10])
        .width(Length::Fill)
        .style(move |_, status| FasciaStyle::menu_item_button(palette, status));
    if let Some(msg) = action {
        btn = btn.on_press(msg);
    }
    btn
}
fn root_button<'a, Message: 'static>(
    title: &'static str,
    palette: ThemePalette,
) -> button::Button<'a, Message> {
    button(text(title).size(13).style(move |_| text::Style {
        color: Some(palette.text_primary),
    }))
    .padding([4, 8])
    .style(move |_, status| FasciaStyle::toolbar_button(palette, status))
}
/// Constructs the full Fascia Menubar.
pub fn view_menubar<'a, Message: 'static + Clone>(
    palette: ThemePalette,
    active_theme: FasciaTheme,
    map_action: impl Fn(MenuAction) -> Message + Copy + 'static,
) -> Element<'a, Message> {
    let sub_template = |items: Vec<_>| Menu::new(items).max_width(220.0).offset(2.0);
    // --- File Menu ---
    let file_items = vec![
        Item::new(menu_button(
            "New File",
            Some("Ctrl+N"),
            Some(map_action(MenuAction::NewFile)),
            palette,
        )),
        Item::new(menu_button(
            "Open File...",
            Some("Ctrl+O"),
            Some(map_action(MenuAction::OpenFile)),
            palette,
        )),
        Item::new(menu_button(
            "Open Folder...",
            None,
            Some(map_action(MenuAction::OpenFolder)),
            palette,
        )),
        Item::new(menu_button(
            "Save",
            Some("Ctrl+S"),
            Some(map_action(MenuAction::SaveFile)),
            palette,
        )),
        Item::new(menu_button(
            "Close Tab",
            Some("Ctrl+W"),
            Some(map_action(MenuAction::CloseTab)),
            palette,
        )),
        Item::new(menu_button(
            "Close All Tabs",
            None,
            Some(map_action(MenuAction::CloseAllTabs)),
            palette,
        )),
        Item::new(menu_button(
            "Exit",
            Some("Alt+F4"),
            Some(map_action(MenuAction::Exit)),
            palette,
        )),
    ];
    let file_root = Item::with_menu(root_button("File", palette), sub_template(file_items));
    // --- Edit Menu ---
    let edit_items = vec![
        Item::new(menu_button(
            "Undo",
            Some("Ctrl+Z"),
            Some(map_action(MenuAction::Undo)),
            palette,
        )),
        Item::new(menu_button(
            "Redo",
            Some("Ctrl+Y"),
            Some(map_action(MenuAction::Redo)),
            palette,
        )),
        Item::new(menu_button(
            "Cut",
            Some("Ctrl+X"),
            Some(map_action(MenuAction::Cut)),
            palette,
        )),
        Item::new(menu_button(
            "Copy",
            Some("Ctrl+C"),
            Some(map_action(MenuAction::Copy)),
            palette,
        )),
        Item::new(menu_button(
            "Paste",
            Some("Ctrl+V"),
            Some(map_action(MenuAction::Paste)),
            palette,
        )),
        Item::new(menu_button(
            "Select All",
            Some("Ctrl+A"),
            Some(map_action(MenuAction::SelectAll)),
            palette,
        )),
    ];
    let edit_root = Item::with_menu(root_button("Edit", palette), sub_template(edit_items));
    // --- View Menu ---
    let view_items = vec![
        Item::new(menu_button(
            "Toggle Primary Sidebar",
            Some("Ctrl+B"),
            Some(map_action(MenuAction::ToggleSidebar)),
            palette,
        )),
        Item::new(menu_button(
            "Toggle Status Bar",
            None,
            Some(map_action(MenuAction::ToggleStatusBar)),
            palette,
        )),
        Item::new(menu_button(
            "Explorer View",
            Some("Ctrl+Shift+E"),
            Some(map_action(MenuAction::OpenExplorer)),
            palette,
        )),
        Item::new(menu_button(
            "Search View",
            Some("Ctrl+Shift+F"),
            Some(map_action(MenuAction::OpenSearch)),
            palette,
        )),
        Item::new(menu_button(
            "Settings View",
            Some("Ctrl+,"),
            Some(map_action(MenuAction::OpenSettings)),
            palette,
        )),
    ];
    let view_root = Item::with_menu(root_button("View", palette), sub_template(view_items));
    // --- Theme Menu ---
    let mut theme_items = Vec::new();
    for &theme_variant in FasciaTheme::ALL {
        let prefix = if theme_variant == active_theme {
            "✓ "
        } else {
            "   "
        };
        let label_owned = format!("{}{}", prefix, theme_variant.display_name());
        let label: &'static str = Box::leak(label_owned.into_boxed_str());
        theme_items.push(Item::new(menu_button(
            label,
            None,
            Some(map_action(MenuAction::SelectTheme(theme_variant))),
            palette,
        )));
    }
    let theme_root = Item::with_menu(root_button("Theme", palette), sub_template(theme_items));
    // --- Help Menu ---
    let help_items = vec![
        Item::new(menu_button(
            "About Fascia",
            None,
            Some(map_action(MenuAction::About)),
            palette,
        )),
        Item::new(menu_button(
            "Documentation",
            None,
            Some(map_action(MenuAction::Documentation)),
            palette,
        )),
    ];
    let help_root = Item::with_menu(root_button("Help", palette), sub_template(help_items));
    let roots = vec![file_root, edit_root, view_root, theme_root, help_root];
    let menu_bar = MenuBar::new(roots);
    container(menu_bar)
        .padding([2, 6])
        .width(Length::Fill)
        .style(move |_| FasciaStyle::menubar_container(palette))
        .into()
}
