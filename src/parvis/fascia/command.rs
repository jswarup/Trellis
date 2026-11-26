// command.rs -------------------------------------------------------------------------------------
//! Centralized workbench command registry, shortcuts, context enablement, and dispatch.

//-------------------------------------------------------------------------------------------------

/// Stable identifier for a workbench command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandId(pub &'static str);

impl CommandId
{
    // --- File Operations ---
    pub const FILE_NEW: Self = Self("workbench.action.files.newFile");
    pub const FILE_NEW_SKETCH: Self = Self("workbench.action.files.newSketch");
    pub const FILE_NEW_MATH: Self = Self("workbench.action.files.newMathView");
    pub const MATH_EXPORT_TEXT: Self = Self("mathView.exportDisplayText");
    pub const MATH_EXPORT_SVG: Self = Self("mathView.exportSvg");
    pub const MATH_PLOT_2D: Self = Self("mathView.plot2d");
    pub const MATH_PLOT_3D: Self = Self("mathView.plot3d");
    pub const FILE_OPEN: Self = Self("workbench.action.files.openFile");
    pub const FOLDER_OPEN: Self = Self("workbench.action.files.openFolder");
    pub const FILE_SAVE: Self = Self("workbench.action.files.save");
    pub const TAB_CLOSE: Self = Self("workbench.action.closeActiveTab");
    pub const TAB_CLOSE_ALL: Self = Self("workbench.action.closeAllTabs");
    pub const APP_EXIT: Self = Self("workbench.action.quit");

    // --- Edit Operations ---
    pub const EDIT_UNDO: Self = Self("workbench.action.undo");
    pub const EDIT_REDO: Self = Self("workbench.action.redo");
    pub const EDIT_CUT: Self = Self("workbench.action.cut");
    pub const EDIT_COPY: Self = Self("workbench.action.copy");
    pub const EDIT_PASTE: Self = Self("workbench.action.paste");
    pub const EDIT_SELECT_ALL: Self = Self("workbench.action.selectAll");

    // --- View Operations ---
    pub const VIEW_TOGGLE_SIDEBAR: Self = Self("workbench.action.toggleSidebar");
    pub const VIEW_TOGGLE_STATUS_BAR: Self = Self("workbench.action.toggleStatusBar");
    pub const VIEW_TOGGLE_BOTTOM_PANEL: Self = Self("workbench.action.toggleBottomPanel");
    pub const VIEW_TOGGLE_PROPERTIES: Self = Self("workbench.action.toggleProperties");
    pub const VIEW_TOGGLE_EXPLORER_FS: Self = Self("workbench.action.toggleExplorerFilesystem");
    pub const VIEW_TOGGLE_EXPLORER_SETTINGS: Self = Self("workbench.action.toggleExplorerSettings");
    pub const VIEW_TOGGLE_EXPLORER_EXAMPLES: Self = Self("workbench.action.toggleExplorerExamples");
    pub const VIEW_OPEN_EXPLORER: Self = Self("workbench.view.explorer");
    pub const VIEW_OPEN_SEARCH: Self = Self("workbench.view.search");
    pub const VIEW_QUICK_OPEN: Self = Self("workbench.action.quickOpen");
    pub const VIEW_COMMAND_PALETTE: Self = Self("workbench.action.showCommands");
    pub const VIEW_OPEN_SETTINGS: Self = Self("workbench.action.openSettings");
    pub const VIEW_OPEN_PYTHON_CONSOLE: Self = Self("workbench.action.openPythonConsole");
    pub const VIEW_OPEN_OUTPUT: Self = Self("workbench.action.openOutput");

    // --- Theme Operations ---
    pub const THEME_SELECT_WINDOWS_DARK: Self = Self("workbench.action.theme.windowsDark");
    pub const THEME_SELECT_WINDOWS_LIGHT: Self = Self("workbench.action.theme.windowsLight");
    pub const THEME_SELECT_LINUX_DARK: Self = Self("workbench.action.theme.linuxDark");
    pub const THEME_SELECT_LINUX_LIGHT: Self = Self("workbench.action.theme.linuxLight");
    pub const THEME_SELECT_VSCODE_DARK: Self = Self("workbench.action.theme.vsCodeDark");
    pub const THEME_TOGGLE: Self = Self("workbench.action.toggleTheme");

    // --- Run / Job Operations ---
    pub const RUN_EXAMPLE: Self = Self("workbench.action.runExample");
    pub const RUN_STOP: Self = Self("workbench.action.stopJob");

    // --- Help Operations ---
    pub const HELP_ABOUT: Self = Self("workbench.action.showAbout");
    pub const HELP_DOCUMENTATION: Self = Self("workbench.action.showDocumentation");

    pub fn AsStr(&self) -> &'static str { self.0 }
}

impl core::fmt::Display for CommandId
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result
    {
        return write!(f, "{}", self.0);
    }
}

//-------------------------------------------------------------------------------------------------

/// Keyboard shortcut representation with modifier matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut
{
    pub ctrl:    bool,
    pub shift:   bool,
    pub alt:     bool,
    pub key:     &'static str,
    pub display: &'static str,
}

impl Shortcut
{
    pub const fn Ctrl(key: &'static str, display: &'static str) -> Self
    {
        return Self { ctrl: true, shift: false, alt: false, key, display };
    }

    pub const fn CtrlShift(key: &'static str, display: &'static str) -> Self
    {
        return Self { ctrl: true, shift: true, alt: false, key, display };
    }

    pub const fn Alt(key: &'static str, display: &'static str) -> Self
    {
        return Self { ctrl: false, shift: false, alt: true, key, display };
    }

    pub const fn CtrlAlt(key: &'static str, display: &'static str) -> Self
    {
        return Self { ctrl: true, shift: false, alt: true, key, display };
    }

    pub fn Matches(&self, key: &str, modifiers: iced::keyboard::Modifiers) -> bool
    {
        return self.ctrl == modifiers.control()
            && self.shift == modifiers.shift()
            && self.alt == modifiers.alt()
            && self.key.eq_ignore_ascii_case(key);
    }
}

//-------------------------------------------------------------------------------------------------

/// Snapshot of UI and document context evaluated for command enablement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommandContext
{
    pub has_active_tab:        bool,
    pub is_active_tab_dirty:   bool,
    pub tab_count:             u32,
    pub can_save:              bool,
    pub can_undo:              bool,
    pub can_redo:              bool,
    pub is_sidebar_visible:    bool,
    pub is_status_bar_visible: bool,
    pub has_visible_roots:     bool,
    pub has_runnable_example:  bool,
    pub has_active_job:        bool,
}

//-------------------------------------------------------------------------------------------------

pub type AvailabilityPredicate = fn(&CommandContext) -> bool;

pub fn Always(_: &CommandContext) -> bool { true }
pub fn RequiresActiveTab(ctx: &CommandContext) -> bool { ctx.has_active_tab }
pub fn RequiresTabs(ctx: &CommandContext) -> bool { ctx.tab_count > 0 }
pub fn CanSave(ctx: &CommandContext) -> bool { ctx.can_save }
pub fn CanUndo(ctx: &CommandContext) -> bool { ctx.can_undo }
pub fn CanRedo(ctx: &CommandContext) -> bool { ctx.can_redo }
pub fn RequiresExplorer(ctx: &CommandContext) -> bool { ctx.has_visible_roots }
pub fn CanRunExample(ctx: &CommandContext) -> bool { ctx.has_runnable_example }
pub fn CanStop(ctx: &CommandContext) -> bool { ctx.has_active_job }
pub fn Never(_: &CommandContext) -> bool { false }

//-------------------------------------------------------------------------------------------------

/// Definitive metadata and availability rules for a workbench command.
#[derive(Debug, Clone)]
pub struct CommandDef
{
    pub id:                 CommandId,
    pub label:              &'static str,
    pub category:           &'static str,
    pub shortcut:           Option<Shortcut>,
    pub secondary_shortcut: Option<Shortcut>,
    pub is_available:       AvailabilityPredicate,
}

impl CommandDef
{
    pub const fn New(
        id: CommandId,
        label: &'static str,
        category: &'static str,
        shortcut: Option<Shortcut>,
        is_available: AvailabilityPredicate,
    ) -> Self
    {
        return Self {
            id,
            label,
            category,
            shortcut,
            secondary_shortcut: None,
            is_available,
        };
    }

    pub const fn WithSecondaryShortcut(mut self, secondary: Shortcut) -> Self
    {
        self.secondary_shortcut = Some(secondary);
        return self;
    }

    pub fn Id(&self) -> CommandId { self.id }
    pub fn Label(&self) -> &'static str { self.label }
    pub fn Category(&self) -> &'static str { self.category }

    pub fn ShortcutDisplay(&self) -> Option<&'static str>
    {
        return self.shortcut.map(|s| s.display);
    }

    pub fn IsAvailable(&self, ctx: &CommandContext) -> bool
    {
        return (self.is_available)(ctx);
    }

    pub fn MatchesShortcut(&self, key: &str, modifiers: iced::keyboard::Modifiers) -> bool
    {
        if let Some(ref sc) = self.shortcut
            && sc.Matches(key, modifiers)
        {
            return true;
        }
        if let Some(ref sc) = self.secondary_shortcut
            && sc.Matches(key, modifiers)
        {
            return true;
        }
        return false;
    }
}

//-------------------------------------------------------------------------------------------------

/// Static registry of all registered workbench commands.
pub struct CommandRegistry;

static COMMANDS: &[CommandDef] = &[
    // --- File Menu ---
    CommandDef::New(CommandId::FILE_NEW, "New File", "File", Some(Shortcut::Ctrl("n", "Ctrl+N")), Always),
    CommandDef::New(CommandId::FILE_NEW_SKETCH, "New Sketch", "File", Some(Shortcut::CtrlShift("n", "Ctrl+Shift+N")), Always),
    CommandDef::New(CommandId::FILE_NEW_MATH, "New Math View", "File", None, Always),
    CommandDef::New(CommandId::MATH_EXPORT_TEXT, "Math View: Export Display Text", "Math View", None, RequiresActiveTab),
    CommandDef::New(CommandId::MATH_EXPORT_SVG, "Math View: Export SVG", "Math View", None, RequiresActiveTab),
    CommandDef::New(CommandId::MATH_PLOT_2D, "Math View: Plot 2D Curve", "Math View", None, RequiresActiveTab),
    CommandDef::New(CommandId::MATH_PLOT_3D, "Math View: Plot 3D Surface", "Math View", None, RequiresActiveTab),
    CommandDef::New(CommandId::FILE_OPEN, "Open File...", "File", Some(Shortcut::Ctrl("o", "Ctrl+O")), Always),
    CommandDef::New(CommandId::FOLDER_OPEN, "Open Folder...", "File", None, Always),
    CommandDef::New(CommandId::FILE_SAVE, "Save", "File", Some(Shortcut::Ctrl("s", "Ctrl+S")), CanSave),
    CommandDef::New(CommandId::TAB_CLOSE, "Close Tab", "File", Some(Shortcut::Ctrl("w", "Ctrl+W")), RequiresActiveTab),
    CommandDef::New(CommandId::TAB_CLOSE_ALL, "Close All Tabs", "File", None, RequiresTabs),
    CommandDef::New(CommandId::APP_EXIT, "Exit", "File", Some(Shortcut::Alt("f4", "Alt+F4")), Always),

    // --- Edit Menu ---
    CommandDef::New(CommandId::EDIT_UNDO, "Undo", "Edit", Some(Shortcut::Ctrl("z", "Ctrl+Z")), CanUndo),
    CommandDef::New(CommandId::EDIT_REDO, "Redo", "Edit", Some(Shortcut::Ctrl("y", "Ctrl+Y")), CanRedo)
        .WithSecondaryShortcut(Shortcut::CtrlShift("z", "Ctrl+Shift+Z")),
    CommandDef::New(CommandId::EDIT_CUT, "Cut", "Edit", Some(Shortcut::Ctrl("x", "Ctrl+X")), RequiresActiveTab),
    CommandDef::New(CommandId::EDIT_COPY, "Copy", "Edit", Some(Shortcut::Ctrl("c", "Ctrl+C")), RequiresActiveTab),
    CommandDef::New(CommandId::EDIT_PASTE, "Paste", "Edit", Some(Shortcut::Ctrl("v", "Ctrl+V")), RequiresActiveTab),
    CommandDef::New(CommandId::EDIT_SELECT_ALL, "Select All", "Edit", Some(Shortcut::Ctrl("a", "Ctrl+A")), RequiresActiveTab),

    // --- View Menu ---
    CommandDef::New(CommandId::VIEW_TOGGLE_SIDEBAR, "Toggle Primary Sidebar", "View", Some(Shortcut::Ctrl("b", "Ctrl+B")), Always),
    CommandDef::New(CommandId::VIEW_TOGGLE_STATUS_BAR, "Toggle Status Bar", "View", None, Always),
    CommandDef::New(CommandId::VIEW_TOGGLE_BOTTOM_PANEL, "Toggle Bottom Panel", "View", Some(Shortcut::Ctrl("j", "Ctrl+J")), Always),
    CommandDef::New(CommandId::VIEW_TOGGLE_PROPERTIES, "Toggle Properties Panel", "View", Some(Shortcut::CtrlAlt("p", "Ctrl+Alt+P")), Always),
    CommandDef::New(CommandId::VIEW_TOGGLE_EXPLORER_FS, "Filesystem", "View", None, Always),
    CommandDef::New(CommandId::VIEW_TOGGLE_EXPLORER_SETTINGS, "Settings", "View", None, Always),
    CommandDef::New(CommandId::VIEW_TOGGLE_EXPLORER_EXAMPLES, "Examples", "View", None, Always),
    CommandDef::New(CommandId::VIEW_OPEN_EXPLORER, "Explorer View", "View", Some(Shortcut::CtrlShift("e", "Ctrl+Shift+E")), RequiresExplorer),
    CommandDef::New(CommandId::VIEW_OPEN_SEARCH, "Search View", "View", Some(Shortcut::CtrlShift("f", "Ctrl+Shift+F")), Always),
    CommandDef::New(CommandId::VIEW_QUICK_OPEN, "Quick Open", "View", Some(Shortcut::Ctrl("p", "Ctrl+P")), Always),
    CommandDef::New(CommandId::VIEW_COMMAND_PALETTE, "Command Palette", "View", Some(Shortcut::CtrlShift("p", "Ctrl+Shift+P")), Always),
    CommandDef::New(CommandId::VIEW_OPEN_SETTINGS, "Settings Tab", "View", Some(Shortcut::Ctrl(",", "Ctrl+,")), Always),
    CommandDef::New(CommandId::VIEW_OPEN_PYTHON_CONSOLE, "Python Console", "View", Some(Shortcut::Ctrl("`", "Ctrl+`")), Always),
    CommandDef::New(CommandId::VIEW_OPEN_OUTPUT, "Output", "View", Some(Shortcut::CtrlShift("u", "Ctrl+Shift+U")), Always),

    // --- Theme Menu ---
    CommandDef::New(CommandId::THEME_SELECT_WINDOWS_DARK, "Windows 11 Dark (Fluent)", "Theme", None, Always),
    CommandDef::New(CommandId::THEME_SELECT_WINDOWS_LIGHT, "Windows 11 Light (Fluent)", "Theme", None, Always),
    CommandDef::New(CommandId::THEME_SELECT_LINUX_DARK, "Linux Dark (GNOME Adwaita)", "Theme", None, Always),
    CommandDef::New(CommandId::THEME_SELECT_LINUX_LIGHT, "Linux Light (GNOME Adwaita)", "Theme", None, Always),
    CommandDef::New(CommandId::THEME_SELECT_VSCODE_DARK, "VS Code Dark", "Theme", None, Always),
    CommandDef::New(CommandId::THEME_TOGGLE, "Toggle Theme", "Theme", None, Always),

    // --- Run Menu ---
    CommandDef::New(CommandId::RUN_EXAMPLE, "Run Example", "Run", Some(Shortcut::Ctrl("r", "Ctrl+R")), CanRunExample),
    CommandDef::New(CommandId::RUN_STOP, "Stop Job", "Run", Some(Shortcut::CtrlShift("c", "Ctrl+Shift+C")), CanStop),

    // --- Help Menu ---
    CommandDef::New(CommandId::HELP_ABOUT, "About Fascia", "Help", None, Always),
    CommandDef::New(CommandId::HELP_DOCUMENTATION, "Documentation", "Help", None, Always),
];

impl CommandRegistry
{
    pub fn All() -> &'static [CommandDef] { COMMANDS }

    pub fn Get(id: CommandId) -> Option<&'static CommandDef>
    {
        return COMMANDS.iter().find(|cmd| cmd.id == id);
    }

    pub fn IsAvailable(id: CommandId, ctx: &CommandContext) -> bool
    {
        return Self::Get(id).map(|cmd| cmd.IsAvailable(ctx)).unwrap_or(false);
    }

    pub fn ShortcutFor(id: CommandId) -> Option<&'static str>
    {
        return Self::Get(id).and_then(|cmd| cmd.ShortcutDisplay());
    }

    pub fn FindByShortcut(key: &str, modifiers: iced::keyboard::Modifiers) -> Option<CommandId>
    {
        return COMMANDS
            .iter()
            .find(|cmd| cmd.MatchesShortcut(key, modifiers))
            .map(|cmd| cmd.id);
    }
}

//-------------------------------------------------------------------------------------------------

/// Outcome of executing a workbench command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOutcome
{
    Handled,
    Disabled,
    NotFound,
}
