// fresco_resource.rs ------------------------------------------------------------------------------
//! Opening and inspecting Fresco examples across workbench hosts.

use crate::fenst::example::XplrExample;
use crate::fenst::fresco_document::FrescoDocument;
use crate::fenst::xplr::{BranchXplr, LeafXplr, StreamChunk, Xplr};
use crate::silo::{Buff, Stash};

//-------------------------------------------------------------------------------------------------

pub const FRESCO_EXAMPLE_URI: &str = "example://Fresco/MathView";
pub const FRESCO_EXAMPLE_TITLE: &str = "Math View";
pub const FRESCO_EXAMPLE_DESCRIPTION: &str = "Inspect symbolic scalar, vector, and matrix expressions.";

//-------------------------------------------------------------------------------------------------

/// Inspectable text/JSON leaf node attached to a Fresco `XplrExample` result tree.
pub struct FrescoTextLeaf
{
    _Name:      String,
    _Path:      String,
    _Extension: String,
    _Content:   String,
}

impl FrescoTextLeaf
{
    pub fn New(
        name: impl Into<String>,
        path: impl Into<String>,
        extension: impl Into<String>,
        content: impl Into<String>,
    ) -> Self
    {
        return Self {
            _Name:      name.into(),
            _Path:      path.into(),
            _Extension: extension.into(),
            _Content:   content.into(),
        };
    }
}

impl Xplr for FrescoTextLeaf
{
    fn Name(&self) -> &str { &self._Name }
    fn Path(&self) -> &str { &self._Path }
    fn IsLeaf(&self) -> bool { true }
    fn Leaf(&self) -> Option<&dyn LeafXplr> { Some(self) }
}

impl LeafXplr for FrescoTextLeaf
{
    fn Size(&self) -> u64 { self._Content.len() as u64 }
    fn Extension(&self) -> &str { &self._Extension }

    fn ReadChunk(&self, offset: u64, length: u32) -> Result<StreamChunk, String>
    {
        let total = self._Content.len() as u64;
        let start = offset.min(total) as usize;
        let end = start.saturating_add(length as usize).min(self._Content.len());
        return Ok(StreamChunk::New(
            self._Path.clone(),
            offset,
            total,
            self._Content[start..end].to_string(),
        ));
    }
}

//-------------------------------------------------------------------------------------------------

/// `Xplr` branch returned by `Fresco` `XplrExample`s, bundling a `FrescoDocument` builder
/// for `MathView` alongside inspectable text/JSON artifact leaves.
pub struct FrescoExampleXplr
{
    _Title:       String,
    _Uri:         String,
    _Description: String,
    _Builder:     fn() -> Result<FrescoDocument, String>,
    _Artifacts:   Vec<FrescoTextLeaf>,
}

impl FrescoExampleXplr
{
    pub fn New(
        title: impl Into<String>,
        uri: impl Into<String>,
        description: impl Into<String>,
        root_count: u32,
        builder: fn() -> Result<FrescoDocument, String>,
        artifacts: Vec<FrescoTextLeaf>,
    ) -> Self
    {
        let title_str = title.into();
        let uri_str = uri.into();
        let desc_str = description.into();
        let mut all_artifacts = Vec::with_capacity(artifacts.len() + 1);
        all_artifacts.push(FrescoTextLeaf::New(
            "Overview.txt",
            format!("{}/Overview.txt", uri_str),
            "txt",
            format!("{}\nURI: {}\nRoots: {}\n{}", title_str, uri_str, root_count, desc_str),
        ));
        all_artifacts.extend(artifacts);
        return Self {
            _Title:       title_str,
            _Uri:         uri_str,
            _Description: desc_str,
            _Builder:     builder,
            _Artifacts:   all_artifacts,
        };
    }

    pub fn Description(&self) -> &str { &self._Description }
}

impl Xplr for FrescoExampleXplr
{
    fn Name(&self) -> &str { &self._Title }
    fn Path(&self) -> &str { &self._Uri }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }

    fn BuildFrescoDocument(&self) -> Option<FrescoDocument> { (self._Builder)().ok() }
}

impl BranchXplr for FrescoExampleXplr
{
    fn ChildCount(&self) -> Result<u32, String> { Ok(self._Artifacts.len() as u32) }

    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut out = Stash::<Box<dyn Xplr>>::WithCapacity(self._Artifacts.len() as u32);
        for leaf in &self._Artifacts {
            out.Push(Box::new(FrescoTextLeaf::New(
                leaf._Name.clone(),
                leaf._Path.clone(),
                leaf._Extension.clone(),
                leaf._Content.clone(),
            )));
        }
        return Ok(out.IntoBuff());
    }
}

//-------------------------------------------------------------------------------------------------

/// Opens the built-in `FRESCO_EXAMPLE_URI` or executes a registered `example://Fresco/*`
/// `XplrExample` and builds its `FrescoDocument`.
pub fn OpenFrescoExample(uri: &str) -> Option<FrescoDocument>
{
    if uri == FRESCO_EXAMPLE_URI {
        return Some(FrescoDocument::CreateExample());
    }
    if uri.starts_with("example://Fresco/")
        && let Some(example) = XplrExample::FindByUri(uri)
        && let Ok(root) = example.Run()
    {
        return root.BuildFrescoDocument();
    }
    return None;
}
