// src/zephyr/mod.rs
#[cfg(feature = "tests")]
pub mod _tests;
pub mod app;
pub mod config;
pub mod driver;
pub mod runtime;
pub mod shm;

pub use app::ZephyrVm;
pub use driver::ZephyrCrewDriver;
pub use runtime::{IZephyrRuntime, LibRuntime, RenodeRuntime, ZephyrRuntime};
pub use shm::{
    Fletcher32, ShmIvcb, ShmPacket, ShmRingBuffer, SHM_IVCB_MAGIC, SHM_IVCB_SIZE,
    SHM_PAYLOAD_CAPACITY, SHM_PKT_MAGIC, SHM_PKT_SIZE, SHM_RING_CAPACITY,
};
