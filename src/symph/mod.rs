// mod.rs ---------------------------------------------------------------------------------------------------------

pub mod compshade;
pub mod vertshade;

#[cfg(feature = "tests")]
pub mod _test;

pub use compshade::{
    Collatz, CollatzElem, DoubleElem, HashToFloat, PointCloudElem, VectorAddElem, WangHash,
};
pub use vertshade::{CameraUniforms, Vec2, Vec3, Vec4, VertexTransformPos, VertexTransformResult};
