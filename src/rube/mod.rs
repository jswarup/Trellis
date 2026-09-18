//-- mod.rs --------------------------------------------------------------------------------------------------------

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

#[cfg(feature = "tests")]
pub mod _tests;

// Re-exports matching Trellis rube.h
pub use adder::{Adder, FullAdder, HalfAdder};
pub use coro_kernel::{
    CoroCell, CoroInstance, CoroKernelFactory, CoroPorts, CoroWarp, CORO_MAX_PORTS,
};
pub use engine::{SimEngine, SimEngineMode};
pub use gates::{
    AndGate, CreateGate2, NandGate, NorGate, NotGate, OrGate, XnorGate, XorGate,
};
pub use latches::{CRSLatch, DLatch, RSLatch};
pub use layout::Layout;
pub use module::{
    Eval4Result, Eval4State, EvalRaw, FastWarp, KernelKind, KernelOp, Module,
};
pub use netlist::Netlist;
pub use port::{IPort, ModuleId, PortDesc, PortDir, PortId, PortType, PortTypeKind};
pub use trigger::{
    TriggerId, TriggerWad, CURR_I, CURR_MASK, CURR_X, FUTR_I, FUTR_MASK, FUTR_X, PAST_I,
    PAST_MASK, PAST_X,
};
