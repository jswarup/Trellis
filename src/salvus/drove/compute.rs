// compute.rs ------------------------------------------------------------------------------------------------------
use	crate::silo::Arr;
use	crate::symph::StandardOp;

//-------------------------------------------------------------------------------------------------
// SPIR-V compute entry points compiled from the Drove workspace member.
pub fn	ComputeEntryPoint( op: StandardOp) -> &'static str
{
    match op {
        StandardOp::Double => "compute::double_cs",
        StandardOp::VectorAdd => "compute::vecadd_cs",
        StandardOp::Collatz => "compute::collatz_cs",
        StandardOp::PointCloud => "compute::pts_pointcloud_cs",
        StandardOp::CameraTransform => "compute::camera_transform_cs",
    }
}

pub fn	ComputeSpirV( op: StandardOp) -> Option< Arr<'static, u8>>
{
    const DOUBLE_SPIRV: &[u8] = include_bytes!( env!( "DROVE_SPV_PATH"));
    match op {
        StandardOp::Double => Some( DOUBLE_SPIRV.into()),
        StandardOp::VectorAdd
        | StandardOp::Collatz
        | StandardOp::PointCloud
        | StandardOp::CameraTransform => None,
    }
}

//-------------------------------------------------------------------------------------------------
