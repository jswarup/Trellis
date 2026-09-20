// compute.rs ------------------------------------------------------------------------------------------------------
use	crate::symph::StandardOp;

//-------------------------------------------------------------------------------------------------
// SPIR-V compute entry points. The shader crate must export these exact names.
pub fn	ComputeEntryPoint( op: StandardOp) -> &'static str
{
    match op {
        StandardOp::Double => "double_cs",
        StandardOp::VectorAdd => "vecadd_cs",
        StandardOp::Collatz => "collatz_cs",
        StandardOp::PointCloud => "pts_pointcloud_cs",
        StandardOp::CameraTransform => "camera_transform_cs",
    }
}

//-------------------------------------------------------------------------------------------------
