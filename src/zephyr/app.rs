// src/zephyr/app.rs
use crate::crew::hub::CrewHub;
use crate::crew::node::CrewNode;
use crate::zephyr::config::{ZephyrFlavor, ZephyrVmConfig};
use crate::zephyr::driver::ZephyrCrewDriver;
use crate::zephyr::runtime::{IZephyrRuntime, LibRuntime, StepBudget};
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------

// ZephyrVm — simulates a guest Zephyr RTOS VM running the crew driver and cooperative tasks.
pub struct ZephyrVm {
    _config: ZephyrVmConfig,
    _runtime: Box<dyn IZephyrRuntime>,
pub struct ZephyrVm
{
    _Config: ZephyrVmConfig,
    _Runtime: Box<dyn IZephyrRuntime>,
}
impl ZephyrVm {
    pub fn new(config: ZephyrVmConfig, hub: Arc<CrewHub>, node: Arc<CrewNode>) -> Self {

impl ZephyrVm
{
    pub fn New(config: ZephyrVmConfig, hub: Arc<CrewHub>, node: Arc<CrewNode>) -> Self
    {
        let runtime: Box<dyn IZephyrRuntime> = match config.flavor {
            ZephyrFlavor::Lib => {
                Box::new(LibRuntime::new(hub, node, config.machine.crew_base_addr))
            }
            ZephyrFlavor::Renode => Box::new(crate::zephyr::runtime::RenodeRuntime::new(
                config.clone(),
                hub,
                node,
            )),
            _ => unimplemented!("Flavor {:?} not yet implemented", config.flavor),
        };
        Self {
            _config: config,
            _runtime: runtime,
            _Config: config,
            _Runtime: runtime,
        }
    }

    #[inline]
    pub fn node_id(&self) -> u32 {
        self._config.node_id
    pub fn new(config: ZephyrVmConfig, hub: Arc<CrewHub>, node: Arc<CrewNode>) -> Self
    {
        Self::New(config, hub, node)
    }
    pub fn runtime(&mut self) -> &mut dyn IZephyrRuntime {
        self._runtime.as_mut()

    #[inline]
    pub fn NodeId(&self) -> u32
    {
        self._Config.node_id
    }

    #[inline]
    pub fn node_id(&self) -> u32
    {
        self.NodeId()
    }

    #[inline]
    pub fn Runtime(&mut self) -> &mut dyn IZephyrRuntime
    {
        self._Runtime.as_mut()
    }

    #[inline]
    pub fn runtime(&mut self) -> &mut dyn IZephyrRuntime
    {
        self.Runtime()
    }

    //---------------------------------------------------------------------------------------------

    // Convenience Helpers (Lib Flavor Only)
    fn lib_driver(&self) -> &ZephyrCrewDriver {
        let lib_runtime = self
            ._runtime
    fn LibDriver(&self) -> &ZephyrCrewDriver
    {
        let libRuntime = self
            ._Runtime
            .as_any()
            .downcast_ref::<LibRuntime>()
            .expect("Convenience helpers only available on LibFlavor");
        lib_runtime.driver()
        libRuntime.driver()
    }
    pub fn tick_heartbeat(&mut self) -> u32 {

    pub fn TickHeartbeat(&mut self) -> u32
    {
        let budget = StepBudget {
            instruction_limit: self._config.execution.step_instruction_limit,
            time_ns: self._config.execution.step_time_ns,
            instruction_limit: self._Config.execution.step_instruction_limit,
            time_ns: self._Config.execution.step_time_ns,
        };
        // Ensure the runtime is started
        let _ = self._runtime.start();
        let _ = self._runtime.step(budget);
        let lib_runtime = self
            ._runtime
        let _ = self._Runtime.start();
        let _ = self._Runtime.step(budget);
        let libRuntime = self
            ._Runtime
            .as_any()
            .downcast_ref::<LibRuntime>()
            .expect("Convenience helpers only available on LibFlavor");
        (lib_runtime.diagnostics().uptime_ns / 1000) as u32
        (libRuntime.diagnostics().uptime_ns / 1000) as u32
    }
    pub fn heartbeat_ticks(&self) -> u32 {
        let lib_runtime = self
            ._runtime

    #[inline]
    pub fn tick_heartbeat(&mut self) -> u32
    {
        self.TickHeartbeat()
    }

    #[inline]
    pub fn HeartbeatTicks(&self) -> u32
    {
        let libRuntime = self
            ._Runtime
            .as_any()
            .downcast_ref::<LibRuntime>()
            .expect("Convenience helpers only available on LibFlavor");
        (lib_runtime.diagnostics().uptime_ns / 1000) as u32
        (libRuntime.diagnostics().uptime_ns / 1000) as u32
    }
    pub fn send_message(&self, msg: &str) -> usize {
        self.lib_driver().send(msg.as_bytes())

    #[inline]
    pub fn heartbeat_ticks(&self) -> u32
    {
        self.HeartbeatTicks()
    }
    pub fn recv_message(&self, max_bytes: usize) -> String {
        let mut buf = crate::silo::buff::Buff::FromDispenser(max_bytes as u32, |_| 0u8);
        let slice = unsafe { std::slice::from_raw_parts_mut(buf.AsMutPtr(), max_bytes) };
        let n = self.lib_driver().recv(slice);

    #[inline]
    pub fn SendMessage(&self, msg: &str) -> usize
    {
        self.LibDriver().Send(msg.as_bytes())
    }

    #[inline]
    pub fn send_message(&self, msg: &str) -> usize
    {
        self.SendMessage(msg)
    }

    pub fn RecvMessage(&self, maxBytes: usize) -> String
    {
        let mut buf = crate::silo::buff::Buff::FromDispenser(maxBytes as u32, |_| 0u8);
        let slice = unsafe { std::slice::from_raw_parts_mut(buf.AsMutPtr(), maxBytes) };
        let n = self.LibDriver().Recv(slice);
        String::from_utf8_lossy(&slice[..n]).to_string()
    }

    #[inline]
    pub fn recv_message(&self, maxBytes: usize) -> String
    {
        self.RecvMessage(maxBytes)
    }
}
