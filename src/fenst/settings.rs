// settings.rs ---------------------------------------------------------------------------------------------------

use crate::fenst::{BranchXplr, LeafXplr, StreamChunk, Xplr, XplrProvider};
use crate::silo::{Buff, Stash};

//-------------------------------------------------------------------------------------------------
/// A leaf representing a concrete setting item or value in the Xplr hierarchy.
#[derive(Clone, Debug)]
pub struct SettingsLeaf
{
    _Name:      String,
    _Path:      String,
    _Extension: String,
    _Content:   String,
}

impl SettingsLeaf
{
    pub fn New(
        name: impl Into<String>,
        path: impl Into<String>,
        extension: impl Into<String>,
        content: impl Into<String>,
    ) -> Self
    {
        Self {
            _Name:      name.into(),
            _Path:      path.into(),
            _Extension: extension.into(),
            _Content:   content.into(),
        }
    }
}

impl Xplr for SettingsLeaf
{
    fn Name(&self) -> &str { &self._Name }
    fn Path(&self) -> &str { &self._Path }
    fn IsLeaf(&self) -> bool { true }
    fn Leaf(&self) -> Option<&dyn LeafXplr> { Some(self) }
}

impl LeafXplr for SettingsLeaf
{
    fn Size(&self) -> u64 { self._Content.len() as u64 }
    fn Extension(&self) -> &str { &self._Extension }
    fn ReadChunk(&self, offset: u64, length: u32) -> Result<StreamChunk, String>
    {
        let start = usize::try_from(offset).unwrap_or(usize::MAX).min(self._Content.len());
        let end = start.saturating_add(length as usize).min(self._Content.len());
        Ok(StreamChunk::New(
            self._Path.clone(),
            offset,
            self.Size(),
            self._Content[start..end].to_string(),
        ))
    }
}

//-------------------------------------------------------------------------------------------------
/// A branch node representing a settings category or the settings root in the Xplr hierarchy.
pub struct SettingsBranch
{
    _Name:     String,
    _Path:     String,
    _Children: fn() -> Buff<Box<dyn Xplr>>,
}

impl SettingsBranch
{
    pub fn New(
        name: impl Into<String>,
        path: impl Into<String>,
        children: fn() -> Buff<Box<dyn Xplr>>,
    ) -> Self
    {
        Self {
            _Name:     name.into(),
            _Path:     path.into(),
            _Children: children,
        }
    }

    pub fn Root() -> Self
    {
        Self::New("Settings", "settings://", Self::RootChildren)
    }

    fn RootChildren() -> Buff<Box<dyn Xplr>>
    {
        let mut stash = Stash::New();
        stash.Push(Box::new(Self::Theme()) as Box<dyn Xplr>);
        stash.Push(Box::new(Self::Layout()) as Box<dyn Xplr>);
        stash.Push(Box::new(Self::System()) as Box<dyn Xplr>);
        stash.IntoBuff()
    }

    pub fn Theme() -> Self
    {
        Self::New("Theme", "settings://theme", Self::ThemeChildren)
    }

    fn ThemeChildren() -> Buff<Box<dyn Xplr>>
    {
        let mut stash = Stash::New();
        stash.Push(Box::new(SettingsLeaf::New("WindowsDark", "settings://theme/WindowsDark", "theme", "Windows Dark theme")) as Box<dyn Xplr>);
        stash.Push(Box::new(SettingsLeaf::New("WindowsLight", "settings://theme/WindowsLight", "theme", "Windows Light theme")) as Box<dyn Xplr>);
        stash.Push(Box::new(SettingsLeaf::New("LinuxDark", "settings://theme/LinuxDark", "theme", "Linux Dark theme")) as Box<dyn Xplr>);
        stash.Push(Box::new(SettingsLeaf::New("LinuxLight", "settings://theme/LinuxLight", "theme", "Linux Light theme")) as Box<dyn Xplr>);
        stash.Push(Box::new(SettingsLeaf::New("VsCodeDark", "settings://theme/VsCodeDark", "theme", "VS Code Dark theme")) as Box<dyn Xplr>);
        stash.IntoBuff()
    }

    pub fn Layout() -> Self
    {
        Self::New("Layout", "settings://layout", Self::LayoutChildren)
    }

    fn LayoutChildren() -> Buff<Box<dyn Xplr>>
    {
        let mut stash = Stash::New();
        stash.Push(Box::new(SettingsLeaf::New("Sidebar", "settings://layout/Sidebar", "cfg", "sidebar: visible")) as Box<dyn Xplr>);
        stash.Push(Box::new(SettingsLeaf::New("StatusBar", "settings://layout/StatusBar", "cfg", "statusbar: visible")) as Box<dyn Xplr>);
        stash.IntoBuff()
    }

    pub fn System() -> Self
    {
        Self::New("System", "settings://system", Self::SystemChildren)
    }

    fn SystemChildren() -> Buff<Box<dyn Xplr>>
    {
        let mut stash = Stash::New();
        stash.Push(Box::new(SettingsLeaf::New("Version", "settings://system/Version", "txt", "Trellis 0.1.0")) as Box<dyn Xplr>);
        stash.Push(Box::new(SettingsLeaf::New("Runtime", "settings://system/Runtime", "txt", "wgpu / Iced / PyO3")) as Box<dyn Xplr>);
        stash.IntoBuff()
    }
}

impl Xplr for SettingsBranch
{
    fn Name(&self) -> &str { &self._Name }
    fn Path(&self) -> &str { &self._Path }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
}

impl BranchXplr for SettingsBranch
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        Ok((self._Children)())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok((self._Children)().Len())
    }
}

//-------------------------------------------------------------------------------------------------
/// Settings provider for `settings://` URIs.
pub struct SettingsProvider;

impl SettingsProvider
{
    pub fn New() -> Self { Self }
}

impl Default for SettingsProvider
{
    fn default() -> Self { Self::New() }
}

impl XplrProvider for SettingsProvider
{
    fn Scheme(&self) -> &str { "settings" }

    fn OpenRoot(&self, uri: &str) -> Result<Box<dyn BranchXplr>, String>
    {
        let clean = uri.trim_end_matches('/');
        match clean {
            "settings:" | "settings://" | "" => Ok(Box::new(SettingsBranch::Root())),
            "settings://theme" => Ok(Box::new(SettingsBranch::Theme())),
            "settings://layout" => Ok(Box::new(SettingsBranch::Layout())),
            "settings://system" => Ok(Box::new(SettingsBranch::System())),
            _ => Ok(Box::new(SettingsBranch::Root())),
        }
    }
}
