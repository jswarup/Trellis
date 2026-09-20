// compute.rs ------------------------------------------------------------------------------------------------------

//-------------------------------------------------------------------------------------------------
// Operations shared by the CPU fallback and GPU shader entry points.
#[derive( Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardOp
{
    Double,
    VectorAdd,
    Collatz,
    PointCloud,
    CameraTransform,
}
pub fn	StandardOpLabel( op: StandardOp) -> &'static str
{
    match op {
        StandardOp::Double => "double_kernel",
        StandardOp::VectorAdd => "vecadd_kernel",
        StandardOp::Collatz => "collatz_kernel",
        StandardOp::PointCloud => "pointcloud_kernel",
        StandardOp::CameraTransform => "camera_transform_kernel",
    }
}

//-------------------------------------------------------------------------------------------------
