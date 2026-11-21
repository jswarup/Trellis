// composite.rs ----------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------
// SPIR-V composition entry points for placing a viewport in the application target.
pub enum CompositeEntryPoint
{
    Vertex,
    SrgbFragment,
    EncodedFragment,
}
impl CompositeEntryPoint
{
    pub const fn	Str( &self) -> &'static str
    {
        match self {
            Self::Vertex => "vs_quad",
            Self::SrgbFragment => "fs_quad",
            Self::EncodedFragment => "fs_encoded",
        }
    }
}

//-------------------------------------------------------------------------------------------------
