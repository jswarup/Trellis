//-- mod.rs --------------------------------------------------------------------------------------------------------
#[cfg( feature = "tests")]
pub mod _tests;
pub mod adder;
pub mod coro_kernel;
pub mod engine;
pub mod gates;
pub mod latches;
pub mod layout;
pub mod module;
pub mod netlist;
pub mod port;
pub mod trigger;
pub mod vcd;
pub mod vcd_model;
pub mod vcdio;
// Re-exports matching Trellis rube.h
pub use	adder::{ Adder, FullAdder, HalfAdder };
pub use	coro_kernel::{ CORO_MAX_PORTS, CoroCell, CoroInstance, CoroKernelFactory, CoroPorts, CoroWarp };
pub use	engine::{ SimEngine, SimEngineMode };
pub use	gates::{ AndGate, CreateGate2, NandGate, NorGate, NotGate, OrGate, XnorGate, XorGate };
pub use	latches::{ CRSLatch, DLatch, RSLatch };
pub use	layout::Layout;
pub use	module::{ Eval4Result, Eval4State, EvalRaw, FastWarp, KernelKind, KernelOp, Module };
pub use	netlist::Netlist;
pub use	port::{ IPort, ModuleId, PortDesc, PortDir, PortId, PortType, PortTypeKind };
pub use	trigger::{ CURR_I, CURR_MASK, CURR_X, FUTR_I, FUTR_MASK, FUTR_X, PAST_I, PAST_MASK, PAST_X, TriggerId, TriggerWad };
pub use	vcd::VcdWriter;
pub use	vcd_model::{ VcdDisplayModel, VcdSignal };
pub use	vcdio::{ ParseVcd, SerializeVcd, VcdModel, VcdScope, VcdShard, VcdTimeStep, VcdValue, VcdVar };
