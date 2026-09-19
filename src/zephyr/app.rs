// src/zephyr/app.rs
use	crate::crew::hub::CrewHub;
use	crate::crew::node::CrewNode;
use	crate::zephyr::config::{ ZephyrFlavor, ZephyrVmConfig };
use	crate::zephyr::driver::ZephyrCrewDriver;
use	crate::zephyr::runtime::{ IZephyrRuntime, LibRuntime, StepBudget };
use	std::sync::Arc;

//-------------------------------------------------------------------------------------------------
// ZephyrVm — simulates a guest Zephyr RTOS VM running the crew driver and cooperative tasks.
pub struct ZephyrVm
{
    _config: ZephyrVmConfig,
    _runtime: Box< dyn IZephyrRuntime>,
}
impl ZephyrVm
{
    pub fn	new( config: ZephyrVmConfig, hub: Arc< CrewHub>, node: Arc< CrewNode>) -> Self
    {
        let  	runtime: Box< dyn IZephyrRuntime> = match config.flavor {
            ZephyrFlavor::Lib => {
                Box::new( LibRuntime::new( hub, node, config.machine.crew_base_addr))
            }
            ZephyrFlavor::Renode => Box::new( crate::zephyr::runtime::RenodeRuntime::new( 
                config.clone(),
                hub,
                node,
            )),
            _ => unimplemented!( "Flavor {:?} not yet implemented", config.flavor),
        };
        Self {
            _config: config,
            _runtime: runtime,
        }
    }
    #[inline]
    pub fn	node_id( &self) -> u32
    {
        self._config.node_id
    }
    pub fn	runtime( &mut self) -> &mut dyn IZephyrRuntime
    {
        self._runtime.as_mut()
    }

    //---------------------------------------------------------------------------------------------
    // Convenience Helpers (Lib Flavor Only)
    fn	lib_driver( &self) -> &ZephyrCrewDriver
    {
        let  	lib_runtime = self
            ._runtime
            .as_any()
            .downcast_ref::< LibRuntime>()
            .expect( "Convenience helpers only available on LibFlavor");
        lib_runtime.driver()
    }
    pub fn	tick_heartbeat( &mut self) -> u32
    {
        let  	budget = StepBudget {
            instruction_limit: self._config.execution.step_instruction_limit,
            time_ns: self._config.execution.step_time_ns,
        };
        // Ensure the runtime is started
        let  	_ = self._runtime.start();
        let  	_ = self._runtime.step( budget);
        let  	lib_runtime = self
            ._runtime
            .as_any()
            .downcast_ref::< LibRuntime>()
            .expect( "Convenience helpers only available on LibFlavor");
        ( lib_runtime.diagnostics().uptime_ns / 1000) as u32
    }
    pub fn	heartbeat_ticks( &self) -> u32
    {
        let  	lib_runtime = self
            ._runtime
            .as_any()
            .downcast_ref::< LibRuntime>()
            .expect( "Convenience helpers only available on LibFlavor");
        ( lib_runtime.diagnostics().uptime_ns / 1000) as u32
    }
    pub fn	send_message( &self, msg: &str) -> usize
    {
        self.lib_driver().send( msg.as_bytes())
    }
    pub fn	recv_message( &self, max_bytes: usize) -> String
    {
        let  	mut buf = crate::silo::buff::Buff::FromDispenser( max_bytes as u32, |_| 0u8);
        let  	slice = unsafe { std::slice::from_raw_parts_mut( buf.AsMutPtr(), max_bytes) };
        let  	n = self.lib_driver().recv( slice);
        String::from_utf8_lossy( &slice[..n]).to_string()
    }
}
