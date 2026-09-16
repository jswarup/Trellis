// src/zephyr/config.rs

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub enum ZephyrFlavor {
    Lib,
    Renode,
    Hypervisor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZephyrBoard {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZephyrMemoryConfig {
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZephyrMachineConfig {
    pub crew_base_addr: u64,
    pub firmware_elf: Option<PathBuf>,
    pub board: ZephyrBoard,
    pub memory: ZephyrMemoryConfig,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZephyrExecutionConfig {
    pub step_instruction_limit: u64,
    pub step_time_ns: u64,
    pub reset_on_start: bool,
    pub rx_interrupt: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ZephyrObservabilityConfig {
    pub trace_enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZephyrVmConfig {
    pub flavor: ZephyrFlavor,
    pub node_id: u32,
    pub machine: ZephyrMachineConfig,
    pub execution: ZephyrExecutionConfig,
    pub observability: ZephyrObservabilityConfig,
}

impl Default for ZephyrVmConfig {
    fn default() -> Self {
        Self {
            flavor: ZephyrFlavor::Lib,
            node_id: 0,
            machine: ZephyrMachineConfig {
                crew_base_addr: 0x50000000,
                firmware_elf: None,
                board: ZephyrBoard {
                    name: "default".to_string(),
                },
                memory: ZephyrMemoryConfig {
                    size_bytes: 1024 * 1024, // 1MB default
                },
            },
            execution: ZephyrExecutionConfig {
                step_instruction_limit: 1000,
                step_time_ns: 1000,
                reset_on_start: true,
                rx_interrupt: false,
            },
            observability: ZephyrObservabilityConfig::default(),
        }
    }
}
