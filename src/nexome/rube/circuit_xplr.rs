// circuit_xplr.rs ---------------------------------------------------------------------------------
//! Implements `fenst::xplr::CircuitXplr` and `fenst::xplr::Xplr` for Rube digital logic modules,
//! exposing their boundary ports, child submodules, and netlist wire connections.

use crate::fenst::xplr::{BranchXplr, CircuitXplr, PortDescriptor, PortDirection, WireDescriptor, Xplr};
use crate::rube::module::{KernelKind, KernelOp};
use crate::rube::{Layout, ModuleId};
use crate::silo::{Buff, Stash};
use std::sync::Arc;

/// Explorer adapter for a Rube module in a `Layout` hierarchy.
pub struct RubeModuleXplr<'a>
{
    _layout: &'a Layout,
    _id:     ModuleId,
}

impl<'a> RubeModuleXplr<'a>
{
    pub fn New(layout: &'a Layout, id: ModuleId) -> Self
    {
        Self { _layout: layout, _id: id }
    }

    pub fn Layout(&self) -> &'a Layout
    {
        self._layout
    }

    pub fn Id(&self) -> ModuleId
    {
        self._id
    }
}

impl Xplr for RubeModuleXplr<'_>
{
    fn Name(&self) -> &str
    {
        self._layout.LocalName(self._id)
    }

    fn Path(&self) -> &str
    {
        self._layout.Module(self._id).Name()
    }

    fn Circuit(&self) -> Option<&dyn CircuitXplr>
    {
        Some(self)
    }
}

fn ModuleKernelType(layout: &Layout, id: ModuleId) -> &'static str
{
    match layout.Module(id).Kernel()
    {
        KernelKind::None => "Module",
        KernelKind::Fast(op) => match op {
            KernelOp::Nand => "NAND",
            KernelOp::And => "AND",
            KernelOp::Or => "OR",
            KernelOp::Not => "NOT",
            KernelOp::Xor => "XOR",
            KernelOp::Nor => "NOR",
            KernelOp::Xnor => "XNOR",
            KernelOp::Add => "ADD",
            KernelOp::Sub => "SUB",
            KernelOp::Shl => "SHL",
            KernelOp::Shr => "SHR",
        },
        KernelKind::Coro(_) => "Coroutine",
    }
}

impl CircuitXplr for RubeModuleXplr<'_>
{
    fn ComponentType(&self) -> &str
    {
        ModuleKernelType(self._layout, self._id)
    }

    fn Ports(&self) -> Vec<PortDescriptor>
    {
        let module = self._layout.Module(self._id);
        let mut ports = Vec::new();

        for i in 0..module.InPorts().Size()
        {
            let port_id = self._layout.InPort(self._id, i);
            let name = self._layout.LocalPortName(port_id);
            ports.push(PortDescriptor::New(name, PortDirection::In));
        }

        for j in 0..module.OutPorts().Size()
        {
            let port_id = self._layout.OutPort(self._id, j);
            let name = self._layout.LocalPortName(port_id);
            ports.push(PortDescriptor::New(name, PortDirection::Out));
        }

        ports
    }

    fn Wires(&self) -> Vec<WireDescriptor>
    {
        let children = self._layout.Children(self._id);
        if children.Size() == 0
        {
            return Vec::new();
        }

        let is_child = |m: ModuleId| -> bool {
            let mut found = false;
            children.USeg().Span(|idx| {
                if children[idx] == m {
                    found = true;
                    return false;
                }
                true
            });
            found
        };

        let mut wires = Vec::new();
        let connections = self._layout.Connections();

        connections.USeg().Span(|index| {
            let (driver, sink) = connections[index];
            let d_owner = self._layout.Port(driver).Owner();
            let s_owner = self._layout.Port(sink).Owner();

            let d_valid = d_owner == self._id || is_child(d_owner);
            let s_valid = s_owner == self._id || is_child(s_owner);

            if d_valid && s_valid && (d_owner != s_owner || d_owner == self._id)
            {
                let d_mod_name = if d_owner == self._id {
                    self.Name().to_string()
                } else {
                    self._layout.LocalName(d_owner).to_string()
                };
                let d_port_name = self._layout.LocalPortName(driver).to_string();

                let s_mod_name = if s_owner == self._id {
                    self.Name().to_string()
                } else {
                    self._layout.LocalName(s_owner).to_string()
                };
                let s_port_name = self._layout.LocalPortName(sink).to_string();

                wires.push(WireDescriptor::New(d_mod_name, d_port_name, s_mod_name, s_port_name));
            }
            true
        });

        wires
    }

    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        let children = self._layout.Children(self._id);
        let mut list: Vec<Box<dyn CircuitXplr + '_>> = Vec::with_capacity(children.Size() as usize);
        children.USeg().Span(|idx| {
            let child_id = children[idx];
            list.push(Box::new(RubeModuleXplr {
                _layout: self._layout,
                _id:     child_id,
            }));
            true
        });
        list
    }
}

//-------------------------------------------------------------------------------------------------
/// Owned, self-contained `Xplr` and `CircuitXplr` implementation for an entire Rube module hierarchy.
#[derive(Clone)]
pub struct RubeCircuit
{
    _layout: Arc<Layout>,
    _id:     ModuleId,
    _path:   String,
}

impl RubeCircuit
{
    pub fn New(layout: Arc<Layout>, id: ModuleId) -> Self
    {
        let name = layout.Module(id).Name().to_string();
        Self { _layout: layout, _id: id, _path: name }
    }

    pub fn NewWithPath(layout: Arc<Layout>, id: ModuleId, path: String) -> Self
    {
        Self { _layout: layout, _id: id, _path: path }
    }

    fn AsBorrow(&self) -> RubeModuleXplr<'_>
    {
        RubeModuleXplr::New(&self._layout, self._id)
    }
}

impl Xplr for RubeCircuit
{
    fn Name(&self) -> &str { self._layout.LocalName(self._id) }
    fn Path(&self) -> &str { &self._path }
    fn IsLeaf(&self) -> bool { self._layout.Children(self._id).IsEmpty() }
    fn Branch(&self) -> Option<&dyn BranchXplr> {
        if self.IsLeaf() { None } else { Some(self) }
    }
    fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
}

impl BranchXplr for RubeCircuit
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let children = self._layout.Children(self._id);
        let mut stash = Stash::New();
        children.USeg().Traverse(|idx| {
            let child_id = children[idx];
            let child_name = self._layout.LocalName(child_id);
            let child_path = if self._path.starts_with("example://") {
                format!("{}/{}", self._path, child_name)
            } else {
                format!("{}.{}", self._path, child_name)
            };
            stash.Push(Box::new(RubeCircuit::NewWithPath(self._layout.clone(), child_id, child_path)) as Box<dyn Xplr>);
        });
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(self._layout.Children(self._id).Size())
    }
}

impl CircuitXplr for RubeCircuit
{
    fn ComponentType(&self) -> &str { ModuleKernelType(&self._layout, self._id) }
    fn Ports(&self) -> Vec<PortDescriptor> { self.AsBorrow().Ports() }
    fn Wires(&self) -> Vec<WireDescriptor> { self.AsBorrow().Wires() }
    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        let children = self._layout.Children(self._id);
        let mut list: Vec<Box<dyn CircuitXplr + '_>> = Vec::with_capacity(children.Size() as usize);
        children.USeg().Span(|idx| {
            let child_id = children[idx];
            let child_name = self._layout.LocalName(child_id);
            let child_path = if self._path.starts_with("example://") {
                format!("{}/{}", self._path, child_name)
            } else {
                format!("{}.{}", self._path, child_name)
            };
            list.push(Box::new(RubeCircuit::NewWithPath(self._layout.clone(), child_id, child_path)));
            true
        });
        list
    }
}

//-------------------------------------------------------------------------------------------------
