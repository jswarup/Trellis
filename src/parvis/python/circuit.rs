//-- circuit.rs ------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::rube::engine::SimEngine;
use	crate::rube::gates::AndGate;
use	crate::rube::layout::Layout;
use	crate::rube::port::{ ModuleId, PortId };
use	pyo3::prelude::*;

//--------------------------------------------------------------------------------------------------
/// Python wrapper around digital circuit Layout.
#[pyclass( name = "Layout")]
pub struct PyLayout
{
    pub(crate) _Inner: Layout,
}

impl Default for PyLayout {
    fn	default() -> Self
    {
        Self::new()
    }
}

#[pymethods]
impl PyLayout
{
    #[new]
    pub fn	new() -> Self
    {
        Self {
            _Inner: Layout::New(),
        }
    }

    pub fn	freeze( &mut self)
    {
        self._Inner.Freeze();
    }

    #[getter]
    pub fn	module_count( &self) -> u32
    {
        self._Inner.Modules().Size()
    }

    pub fn	add_and_gate( &mut self, name: &str) -> ( u32, u32, u32)
    {
        let  	gate = AndGate::New( &mut self._Inner, name);
        (
            gate.In1().Index(),
            gate.In2().Index(),
            gate.Out().Index(),
        )
    }

    pub fn	to_verilog( &self, module_id: u32) -> PyResult< String>
    {
        Ok( self._Inner.ToVerilog( ModuleId::New( module_id)))
    }
}

//--------------------------------------------------------------------------------------------------
/// Python wrapper around digital circuit simulation engine.
#[pyclass( name = "SimEngine")]
pub struct PySimEngine
{
    pub(crate) _Inner: SimEngine,
}

#[pymethods]
impl PySimEngine
{
    #[new]
    pub fn	new( layout: &PyLayout) -> Self
    {
        Self {
            _Inner: SimEngine::Create( &layout._Inner),
        }
    }

    pub fn	step_cycle( &mut self) -> usize
    {
        self._Inner.StepCycle()
    }

    pub fn	advance( &mut self, ticks: u32) -> u32
    {
        self._Inner.AdvanceTicks( ticks)
    }

    #[getter]
    pub fn	cycle_count( &self) -> usize
    {
        self._Inner.CycleCount()
    }

    pub fn	get_bool( &self, port_id: u32) -> bool
    {
        self._Inner.GetBool( PortId::New( port_id))
    }

    pub fn	set_bool( &mut self, port_id: u32, val: bool) -> bool
    {
        self._Inner.SetBool( PortId::New( port_id), val)
    }

    pub fn	get_u32( &self, port_id: u32) -> u32
    {
        self._Inner.GetU32( PortId::New( port_id))
    }

    pub fn	set_u32( &mut self, port_id: u32, val: u32) -> bool
    {
        self._Inner.SetU32( PortId::New( port_id), val)
    }
}
