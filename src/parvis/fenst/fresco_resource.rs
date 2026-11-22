// fresco_resource.rs ------------------------------------------------------------------------------
//! Opening and inspecting the immutable Fresco example across workbench hosts.

use crate::fenst::fresco_document::FrescoDocument;

//-------------------------------------------------------------------------------------------------

pub const FRESCO_EXAMPLE_URI: &str = "example://Fresco/MathView";
pub const FRESCO_EXAMPLE_TITLE: &str = "Math View";
pub const FRESCO_EXAMPLE_DESCRIPTION: &str = "Inspect symbolic scalar, vector, and matrix expressions.";

pub fn OpenFrescoExample(uri: &str) -> Option<FrescoDocument>
{
    return (uri == FRESCO_EXAMPLE_URI).then(FrescoDocument::CreateExample);
}

//-------------------------------------------------------------------------------------------------
