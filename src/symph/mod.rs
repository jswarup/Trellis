// mod.rs ---------------------------------------------------------------------------------------------------------
#[cfg(feature = "tests")]
pub mod _test;
pub mod compshade;
pub mod vertshade;
pub use compshade::{
    Collatz, CollatzElem, DoubleElem, HashToFloat, PointCloudElem, VectorAddElem, WangHash,
};
pub use vertshade::{CameraUniforms, Vec2, Vec3, Vec4, VertexTransformPos, VertexTransformResult};
