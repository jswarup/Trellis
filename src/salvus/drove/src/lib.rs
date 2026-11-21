#![cfg_attr( target_arch = "spirv", no_std)]
#![allow( unexpected_cfgs, non_snake_case)]
//! Rust GPU entry points compiled to SPIR-V by the root build script.
pub mod composite;
pub mod compute;
pub mod geometry;
