// mod.rs ---------------------------------------------------------------------------------------------------------
//! Rust GPU program contracts. Shader implementations will compile from this boundary to SPIR-V.
pub mod composite;
pub mod compute;
pub mod geometry;
pub use	composite::CompositeEntryPoint;
pub use	compute::ComputeEntryPoint;
pub use	geometry::{ GeometryEntryPoint, GeometryFragmentEntryPoint };

//-------------------------------------------------------------------------------------------------
