// geometry.rs -----------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------
// SPIR-V geometry program entry points for mesh and point viewport passes.
pub enum GeometryEntryPoint
{
    Mesh,
    Point,
}
impl GeometryEntryPoint
{
    pub const fn	Str( &self) -> &'static str
    {
        match self {
            Self::Mesh => "vs_mesh",
            Self::Point => "vs_point",
        }
    }
}
pub enum GeometryFragmentEntryPoint
{
    Mesh,
    Wire,
    Point,
}
impl GeometryFragmentEntryPoint
{
    pub const fn	Str( &self) -> &'static str
    {
        match self {
            Self::Mesh => "fs_mesh",
            Self::Wire => "fs_wire",
            Self::Point => "fs_point",
        }
    }
}

//-------------------------------------------------------------------------------------------------
