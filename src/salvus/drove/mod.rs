// mod.rs ---------------------------------------------------------------------------------------------------------
//! Rust GPU program contracts and generated SPIR-V artifacts.
pub mod composite;
pub mod compute;
pub mod geometry;
pub use	composite::CompositeEntryPoint;
pub use	compute::{ ComputeEntryPoint, ComputeSpirV };
pub use	geometry::{ GeometryEntryPoint, GeometryFragmentEntryPoint };

//-------------------------------------------------------------------------------------------------
