// circuit_xplr.rs ---------------------------------------------------------------------------------
//! Implements `fenst::xplr::CircuitXplr` and `fenst::xplr::Xplr` for Karst multi-die SoC fabrics,
//! dies, hosts, crossbar NoCs, memory channels, and near-memory VPUs.
//!
//! Architectural Topology:
//! - Top-Level Fascia / Karst Xplr Node (`KarstSystemCircuit`):
//!   Instantiates 8 Host processors (`Host 0..7`), 1 Karst Fabric Module (`KarstModule`),
//!   and 8 DDR5 memory channels (`MemChan 0..7`) at its first level of submodules.
//!   The Karst Fabric Module connects the Hosts to the DDR5 memory channels.
//! - Karst Fabric Module (`KarstFabricCircuit` / `KarstFabricXplr`):
//!   Contains the multi-die interconnect crossbar spanning 2 KarstHind dies (`Die 0` / `Die1Circuit`,
//!   `Die 1` / `Die2Circuit`) interconnected by 64-bit bidirectional inter-die links.
//! - KarstHind Die (`KarstDieCircuit` / `KarstDieXplr`):
//!   Houses the on-die crossbar NoC router connecting the die's host ports, memory channel ports,
//!   and inter-die links.
//! - DDR5 Memory Channel (`KarstMemChanCircuit`):
//!   Houses the DDR5 memory controller connected to its dedicated near-memory Vector Processing Unit (`VPU`).

use crate::fenst::xplr::{
    BranchXplr, CircuitXplr, PortDescriptor, PortDirection, PortSide, WireDescriptor, Xplr,
};
use crate::karst::fabric::KarstFabric;
use crate::silo::{Buff, Stash, USeg};

//-------------------------------------------------------------------------------------------------
// System-Level Ports and Wires (Hosts <-> Karst Fabric Module <-> DDR5 Memory Channels)

pub fn SystemPorts() -> Vec<PortDescriptor>
{
    vec![PortDescriptor::New("SysClk", PortDirection::In).WithSide(PortSide::West),
         PortDescriptor::New("Reset_n", PortDirection::In).WithSide(PortSide::West),
         PortDescriptor::New("Host_IO", PortDirection::In).WithSide(PortSide::West),
         PortDescriptor::New("Status", PortDirection::Out).WithSide(PortSide::East),
         PortDescriptor::New("Interrupt", PortDirection::Out).WithSide(PortSide::East),]
}

pub fn SystemWires(root_name: &str, module_name: &str) -> Vec<WireDescriptor>
{
    let mut wires = Vec::new();

    USeg::FromLen(8).Traverse(|i| {
        let host_name = format!("Host {}", i);
        let chan_name = format!("MemChan {}", i);

        // Host <-> Karst Fabric Module
        wires.push(WireDescriptor::New(host_name.clone(), "Req", module_name, format!("Host_In_{}", i))
            .WithName(format!("HostReq_{}", i)));
        wires.push(WireDescriptor::New(module_name, format!("Host_Out_{}", i), host_name, "Resp")
            .WithName(format!("HostResp_{}", i))
            .AsFeedback());

        // Karst Fabric Module <-> DDR5 Memory Channel
        wires.push(WireDescriptor::New(module_name, format!("MC_Out_{}", i), chan_name.clone(), "Req")
            .WithName(format!("McReq_{}", i)));
        wires.push(WireDescriptor::New(chan_name, "Resp", module_name, format!("MC_In_{}", i))
            .WithName(format!("McResp_{}", i))
            .AsFeedback());
    });

    // Boundary connections
    wires.push(WireDescriptor::New(root_name, "SysClk", module_name, "SysClk"));
    wires.push(WireDescriptor::New(root_name, "Reset_n", module_name, "Reset_n"));
    wires.push(WireDescriptor::New(root_name, "Host_IO", module_name, "Host_IO"));
    wires.push(WireDescriptor::New(module_name, "Status", root_name, "Status"));
    wires.push(WireDescriptor::New(module_name, "Interrupt", root_name, "Interrupt"));

    wires
}

//-------------------------------------------------------------------------------------------------
// Fabric Module Ports and Wires (Multi-Die Interconnect)

pub fn FabricPorts() -> Vec<PortDescriptor>
{
    KarstModulePorts()
}

pub fn FabricWires() -> Vec<WireDescriptor>
{
    KarstModuleWires("KarstFabric", "Die 0", "Die 1")
}

pub fn KarstModulePorts() -> Vec<PortDescriptor>
{
    let mut ports = Vec::new();
    USeg::FromLen(8).Traverse(|i| {
                        ports.push(PortDescriptor::New(format!("Host_In_{}", i),
                                                       PortDirection::In)
                            .WithSide(PortSide::West));
                        ports.push(PortDescriptor::New(format!("Host_Out_{}", i),
                                                       PortDirection::Out)
                            .WithSide(PortSide::West));
                        ports.push(PortDescriptor::New(format!("MC_In_{}", i), PortDirection::In)
                            .WithSide(PortSide::East));
                        ports.push(PortDescriptor::New(format!("MC_Out_{}", i),
                                                       PortDirection::Out)
                            .WithSide(PortSide::East));
                    });
    ports.push(PortDescriptor::New("SysClk", PortDirection::In).WithSide(PortSide::West));
    ports.push(PortDescriptor::New("Reset_n", PortDirection::In).WithSide(PortSide::West));
    ports.push(PortDescriptor::New("Host_IO", PortDirection::In).WithSide(PortSide::West));
    ports.push(PortDescriptor::New("Status", PortDirection::Out).WithSide(PortSide::East));
    ports.push(PortDescriptor::New("Interrupt", PortDirection::Out).WithSide(PortSide::East));
    ports
}

pub fn KarstModuleWires(root_name: &str, die1_name: &str, die2_name: &str) -> Vec<WireDescriptor>
{
    let mut wires = Vec::new();

    // Die 1 connects to KarstModule Host ports 0..3 and MC ports 0..3
    USeg::FromLen(4).Traverse(|i| {
        wires.push(WireDescriptor::New(root_name, format!("Host_In_{}", i), die1_name, format!("Host_In_{}", i))
            .WithName(format!("ModHostIn_{}", i)));
        wires.push(WireDescriptor::New(die1_name, format!("Host_Out_{}", i), root_name, format!("Host_Out_{}", i))
            .WithName(format!("ModHostOut_{}", i))
            .AsFeedback());
        wires.push(WireDescriptor::New(die1_name, format!("MC_Out_{}", i), root_name, format!("MC_Out_{}", i))
            .WithName(format!("ModMcOut_{}", i)));
        wires.push(WireDescriptor::New(root_name, format!("MC_In_{}", i), die1_name, format!("MC_In_{}", i))
            .WithName(format!("ModMcIn_{}", i))
            .AsFeedback());
    });

    // Die 2 connects to KarstModule Host ports 4..7 and MC ports 4..7
    USeg::FromLen(4).Traverse(|i| {
        wires.push(WireDescriptor::New(root_name, format!("Host_In_{}", 4 + i), die2_name, format!("Host_In_{}", i))
            .WithName(format!("ModHostIn_{}", 4 + i)));
        wires.push(WireDescriptor::New(die2_name, format!("Host_Out_{}", i), root_name, format!("Host_Out_{}", 4 + i))
            .WithName(format!("ModHostOut_{}", 4 + i))
            .AsFeedback());
        wires.push(WireDescriptor::New(die2_name, format!("MC_Out_{}", i), root_name, format!("MC_Out_{}", 4 + i))
            .WithName(format!("ModMcOut_{}", 4 + i)));
        wires.push(WireDescriptor::New(root_name, format!("MC_In_{}", 4 + i), die2_name, format!("MC_In_{}", i))
            .WithName(format!("ModMcIn_{}", 4 + i))
            .AsFeedback());
    });

    // Inter-die bidirectional physical links between Die 1 and Die 2
    wires.push(WireDescriptor::New(die1_name, "InterDie_Tx", die2_name, "InterDie_Rx")
        .WithName("InterDie_Link_1_2")
        .WithWidth(64));
    wires.push(WireDescriptor::New(die2_name, "InterDie_Tx", die1_name, "InterDie_Rx")
        .WithName("InterDie_Link_2_1")
        .WithWidth(64));

    wires
}

//-------------------------------------------------------------------------------------------------
// Intra-Die Ports, Wires, and Submodules

pub fn DiePorts() -> Vec<PortDescriptor>
{
    let mut ports = Vec::new();
    USeg::FromLen(4).Traverse(|i| {
        ports.push(PortDescriptor::New(format!("Host_In_{}", i), PortDirection::In)
            .WithSide(PortSide::West));
        ports.push(PortDescriptor::New(format!("Host_Out_{}", i), PortDirection::Out)
            .WithSide(PortSide::West));
        ports.push(PortDescriptor::New(format!("MC_In_{}", i), PortDirection::In)
            .WithSide(PortSide::East));
        ports.push(PortDescriptor::New(format!("MC_Out_{}", i), PortDirection::Out)
            .WithSide(PortSide::East));
    });
    ports.push(PortDescriptor::New("InterDie_Rx", PortDirection::In)
        .WithType(64, "KarstLink")
        .WithSide(PortSide::East));
    ports.push(PortDescriptor::New("InterDie_Tx", PortDirection::Out)
        .WithType(64, "KarstLink")
        .WithSide(PortSide::East));
    ports
}

pub fn DieWires(die_id: u32, die_name: &str) -> Vec<WireDescriptor>
{
    let noc_name = format!("NoC {}", die_id);
    let mut wires = Vec::new();

    USeg::FromLen(4).Traverse(|i| {
        // Boundary Host ports <-> on-die NoC crossbar
        wires.push(WireDescriptor::New(die_name, format!("Host_In_{}", i), noc_name.clone(), format!("Host_In_{}", i))
            .WithName(format!("DieHostIn_{}", i)));
        wires.push(WireDescriptor::New(noc_name.clone(), format!("Host_Out_{}", i), die_name, format!("Host_Out_{}", i))
            .WithName(format!("DieHostOut_{}", i))
            .AsFeedback());

        // on-die NoC crossbar <-> Boundary MC ports
        wires.push(WireDescriptor::New(noc_name.clone(), format!("MC_Out_{}", i), die_name, format!("MC_Out_{}", i))
            .WithName(format!("DieMcOut_{}", i)));
        wires.push(WireDescriptor::New(die_name, format!("MC_In_{}", i), noc_name.clone(), format!("MC_In_{}", i))
            .WithName(format!("DieMcIn_{}", i))
            .AsFeedback());
    });

    // Boundary InterDie ports <-> on-die NoC crossbar
    wires.push(WireDescriptor::New(die_name, "InterDie_Rx", noc_name.clone(), "InterDie_In")
        .WithName("DieInterDie_Rx")
        .WithWidth(64));
    wires.push(WireDescriptor::New(noc_name, "InterDie_Out", die_name, "InterDie_Tx")
        .WithName("DieInterDie_Tx")
        .WithWidth(64));

    wires
}

pub fn DieSubmodules(die_id: u32) -> Vec<Box<dyn CircuitXplr + 'static>>
{
    vec![Box::new(KarstComponentXplr::NewNoc(die_id))]
}

//-------------------------------------------------------------------------------------------------
// Top-Level KarstFabric Circuit Explorer (Multi-Die Fabric View)

pub struct KarstFabricXplr<'a>
{
    _fabric: &'a KarstFabric,
}

impl<'a> KarstFabricXplr<'a>
{
    pub fn New(fabric: &'a KarstFabric) -> Self { Self { _fabric: fabric } }

    pub fn Fabric(&self) -> &'a KarstFabric { self._fabric }
}

impl Xplr for KarstFabricXplr<'_>
{
    fn Name(&self) -> &str { "KarstFabric" }

    fn Path(&self) -> &str { "karst://fabric" }

    fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
}

impl CircuitXplr for KarstFabricXplr<'_>
{
    fn ComponentType(&self) -> &str { "SoC Fabric (Multi-Die)" }

    fn Ports(&self) -> Vec<PortDescriptor> { FabricPorts() }

    fn Wires(&self) -> Vec<WireDescriptor> { FabricWires() }

    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        vec![Box::new(KarstDieXplr::New(self._fabric, 0)),
             Box::new(KarstDieXplr::New(self._fabric, 1)),]
    }
}

//-------------------------------------------------------------------------------------------------
// Karst Die (KarstHind Memory Fabric Die) Circuit Explorer (Borrowed Reference)

pub struct KarstDieXplr<'a>
{
    _fabric: &'a KarstFabric,
    _die_id: u32,
    _name:   String,
    _path:   String,
}

impl<'a> KarstDieXplr<'a>
{
    pub fn New(fabric: &'a KarstFabric, die_id: u32) -> Self
    {
        Self { _fabric: fabric,
               _die_id: die_id,
               _name:   format!("Die {}", die_id),
               _path:   format!("karst://fabric/die{}", die_id), }
    }

    pub fn DieId(&self) -> u32 { self._die_id }
}

impl Xplr for KarstDieXplr<'_>
{
    fn Name(&self) -> &str { &self._name }

    fn Path(&self) -> &str { &self._path }

    fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
}

impl CircuitXplr for KarstDieXplr<'_>
{
    fn ComponentType(&self) -> &str { "KarstHind Die" }

    fn Ports(&self) -> Vec<PortDescriptor> { DiePorts() }

    fn Wires(&self) -> Vec<WireDescriptor> { DieWires(self._die_id, self.Name()) }

    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        DieSubmodules(self._die_id).into_iter()
                                   .map(|s| s as Box<dyn CircuitXplr + '_>)
                                   .collect()
    }
}

//-------------------------------------------------------------------------------------------------
// Karst Leaf Component (Host, NoC, VPU)

pub struct KarstComponentXplr
{
    _name:  String,
    _path:  String,
    _kind:  &'static str,
    _ports: Vec<PortDescriptor>,
}

impl KarstComponentXplr
{
    pub fn NewHost(id: u32) -> Self
    {
        Self::NewHostWithPath(id, format!("karst://fabric/host/{}", id))
    }

    pub fn NewHostWithPath(id: u32, path: String) -> Self
    {
        Self { _name:  format!("Host {}", id),
               _path:  path,
               _kind:  "KarstFore Host",
               _ports: vec![PortDescriptor::New("Req", PortDirection::Out).WithSide(PortSide::East),
                            PortDescriptor::New("Resp", PortDirection::In).WithSide(PortSide::East),], }
    }

    pub fn NewNoc(die_id: u32) -> Self
    {
        Self::NewNocWithPath(die_id, format!("karst://fabric/noc/{}", die_id))
    }

    pub fn NewNocWithPath(die_id: u32, path: String) -> Self
    {
        let mut ports = Vec::new();
        USeg::FromLen(4).Traverse(|i| {
                            ports.push(PortDescriptor::New(format!("Host_In_{}", i),
                                                           PortDirection::In)
                                .WithSide(PortSide::West));
                            ports.push(PortDescriptor::New(format!("Host_Out_{}", i),
                                                           PortDirection::Out)
                                .WithSide(PortSide::West));
                            ports.push(PortDescriptor::New(format!("MC_In_{}", i),
                                                           PortDirection::In)
                                .WithSide(PortSide::East));
                            ports.push(PortDescriptor::New(format!("MC_Out_{}", i),
                                                           PortDirection::Out)
                                .WithSide(PortSide::East));
                        });
        ports.push(PortDescriptor::New("InterDie_In", PortDirection::In).WithSide(PortSide::East));
        ports.push(PortDescriptor::New("InterDie_Out", PortDirection::Out).WithSide(PortSide::East));
        ports.push(PortDescriptor::New("Fabric_In", PortDirection::In).WithSide(PortSide::West));
        ports.push(PortDescriptor::New("Fabric_Out", PortDirection::Out).WithSide(PortSide::East));

        Self { _name:  format!("NoC {}", die_id),
               _path:  path,
               _kind:  "MemFabric Crossbar",
               _ports: ports, }
    }

    pub fn NewMemChan(id: u32) -> Self
    {
        Self::NewMemChanWithPath(id, format!("karst://fabric/memchan/{}", id))
    }

    pub fn NewMemChanWithPath(id: u32, path: String) -> Self
    {
        Self { _name:  format!("MemChan {}", id),
               _path:  path,
               _kind:  "DDR5 Controller",
               _ports: vec![PortDescriptor::New("Req", PortDirection::In).WithSide(PortSide::West),
                            PortDescriptor::New("Resp", PortDirection::Out).WithSide(PortSide::West),
                            PortDescriptor::New("Vpu_Out", PortDirection::Out).WithSide(PortSide::East),
                            PortDescriptor::New("Vpu_In", PortDirection::In).WithSide(PortSide::East),], }
    }

    pub fn NewVpu(id: u32) -> Self
    {
        Self::NewVpuWithPath(id, format!("karst://fabric/vpu/{}", id))
    }

    pub fn NewVpuWithPath(id: u32, path: String) -> Self
    {
        Self { _name:  format!("VPU {}", id),
               _path:  path,
               _kind:  "Near-Memory VPU",
               _ports: vec![PortDescriptor::New("Stream_In", PortDirection::In).WithSide(PortSide::West),
                            PortDescriptor::New("Writeback_Out", PortDirection::Out).WithSide(PortSide::West),], }
    }
}

impl Xplr for KarstComponentXplr
{
    fn Name(&self) -> &str { &self._name }

    fn Path(&self) -> &str { &self._path }

    fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
}

impl CircuitXplr for KarstComponentXplr
{
    fn ComponentType(&self) -> &str { self._kind }

    fn Ports(&self) -> Vec<PortDescriptor> { self._ports.clone() }

    fn Wires(&self) -> Vec<WireDescriptor> { Vec::new() }

    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>> { Vec::new() }
}

//-------------------------------------------------------------------------------------------------
// DDR5 Memory Channel Circuit Explorer (Contains near-memory VPU)

#[derive(Clone)]
pub struct KarstMemChanCircuit
{
    _id:   u32,
    _name: String,
    _path: String,
}

impl KarstMemChanCircuit
{
    pub fn New(id: u32) -> Self { Self::NewWithPath(id, format!("karst://fabric/memchan/{}", id)) }

    pub fn NewWithPath(id: u32, path: impl Into<String>) -> Self
    {
        Self { _id:   id,
               _name: format!("MemChan {}", id),
               _path: path.into(), }
    }

    pub fn Id(&self) -> u32 { self._id }
}

impl Xplr for KarstMemChanCircuit
{
    fn Name(&self) -> &str { &self._name }
    fn Path(&self) -> &str { &self._path }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
    fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
}

impl BranchXplr for KarstMemChanCircuit
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut stash = Stash::New();
        let vpu_path = format!("{}/VPU {}", self._path, self._id);
        stash.Push(Box::new(KarstComponentXplr::NewVpuWithPath(self._id, vpu_path))
                   as Box<dyn Xplr>);
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String> { Ok(1) }
}

impl CircuitXplr for KarstMemChanCircuit
{
    fn ComponentType(&self) -> &str { "DDR5 Memory Channel" }

    fn Ports(&self) -> Vec<PortDescriptor>
    {
        vec![PortDescriptor::New("Req", PortDirection::In).WithSide(PortSide::West),
             PortDescriptor::New("Resp", PortDirection::Out).WithSide(PortSide::West),
             PortDescriptor::New("Vpu_Out", PortDirection::Out).WithSide(PortSide::East),
             PortDescriptor::New("Vpu_In", PortDirection::In).WithSide(PortSide::East),]
    }

    fn Wires(&self) -> Vec<WireDescriptor>
    {
        let vpu_name = format!("VPU {}", self._id);
        vec![
            WireDescriptor::New(&self._name, "Vpu_Out", vpu_name.clone(), "Stream_In")
                .WithName(format!("VpuStream_{}", self._id)),
            WireDescriptor::New(vpu_name, "Writeback_Out", &self._name, "Vpu_In")
                .WithName(format!("VpuWriteback_{}", self._id)),
        ]
    }

    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        let vpu_path = format!("{}/VPU {}", self._path, self._id);
        vec![Box::new(KarstComponentXplr::NewVpuWithPath(self._id, vpu_path))]
    }
}

//-------------------------------------------------------------------------------------------------
// Karst Fabric Module (`KarstModule`) Circuit Explorer
//
// The central multi-die fabric module interconnecting Hosts to DDR5 Memory Channels.
// Contains 2 KarstHind dies connected by 64-bit inter-die links.

#[derive(Clone)]
pub struct KarstFabricCircuit
{
    _name: String,
    _path: String,
}

impl KarstFabricCircuit
{
    pub fn New() -> Self { Self::NewNamed("KarstModule", "example://Karst/System/KarstModule") }

    pub fn NewNamed(name: impl Into<String>, path: impl Into<String>) -> Self
    {
        Self { _name: name.into(),
               _path: path.into(), }
    }
}

impl Default for KarstFabricCircuit
{
    fn default() -> Self { Self::New() }
}

impl Xplr for KarstFabricCircuit
{
    fn Name(&self) -> &str { &self._name }
    fn Path(&self) -> &str { &self._path }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
    fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
}

impl BranchXplr for KarstFabricCircuit
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut stash = Stash::New();
        let die1_path = format!("{}/Die1Circuit", self._path);
        let die2_path = format!("{}/Die2Circuit", self._path);
        stash.Push(Box::new(KarstDieCircuit::NewWithLabel(0, "Die1Circuit", die1_path))
                   as Box<dyn Xplr>);
        stash.Push(Box::new(KarstDieCircuit::NewWithLabel(1, "Die2Circuit", die2_path))
                   as Box<dyn Xplr>);
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String> { Ok(2) }
}

impl CircuitXplr for KarstFabricCircuit
{
    fn ComponentType(&self) -> &str { "Karst Fabric Module" }

    fn Ports(&self) -> Vec<PortDescriptor> { KarstModulePorts() }

    fn Wires(&self) -> Vec<WireDescriptor> { KarstModuleWires(&self._name, "Die1Circuit", "Die2Circuit") }

    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        let die1_path = format!("{}/Die1Circuit", self._path);
        let die2_path = format!("{}/Die2Circuit", self._path);
        vec![Box::new(KarstDieCircuit::NewWithLabel(0, "Die1Circuit", die1_path)),
             Box::new(KarstDieCircuit::NewWithLabel(1, "Die2Circuit", die2_path)),]
    }
}

//-------------------------------------------------------------------------------------------------
// Karst Die (KarstHind Memory Fabric Die) Circuit Explorer

#[derive(Clone)]
pub struct KarstDieCircuit
{
    _die_id: u32,
    _name:   String,
    _path:   String,
}

impl KarstDieCircuit
{
    pub fn New(die_id: u32) -> Self
    {
        let name = if die_id == 0 {
            "Die1Circuit"
        } else {
            "Die2Circuit"
        };
        let path = if die_id == 0 {
            "example://Karst/System/KarstModule/Die1Circuit".to_string()
        } else {
            "example://Karst/System/KarstModule/Die2Circuit".to_string()
        };
        Self::NewWithLabel(die_id, name, path)
    }

    pub fn NewWithLabel(die_id: u32, name: impl Into<String>, path: impl Into<String>) -> Self
    {
        Self { _die_id: die_id,
               _name:   name.into(),
               _path:   path.into(), }
    }

    pub fn DieId(&self) -> u32 { self._die_id }
}

impl Xplr for KarstDieCircuit
{
    fn Name(&self) -> &str { &self._name }
    fn Path(&self) -> &str { &self._path }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
    fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
}

impl BranchXplr for KarstDieCircuit
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut stash = Stash::New();
        let noc_name = format!("NoC {}", self._die_id);
        let noc_path = format!("{}/{}", self._path, noc_name);
        stash.Push(Box::new(KarstComponentXplr::NewNocWithPath(self._die_id, noc_path))
                   as Box<dyn Xplr>);
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String> { Ok(1) }
}

impl CircuitXplr for KarstDieCircuit
{
    fn ComponentType(&self) -> &str { "KarstHind Die" }
    fn Ports(&self) -> Vec<PortDescriptor> { DiePorts() }
    fn Wires(&self) -> Vec<WireDescriptor> { DieWires(self._die_id, &self._name) }
    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        DieSubmodules(self._die_id).into_iter()
                                   .map(|s| s as Box<dyn CircuitXplr + '_>)
                                   .collect()
    }
}

//-------------------------------------------------------------------------------------------------
// Top-Level Karst System Circuit Explorer (`example://Karst/System`)
//
// At its first level of submodules, instantiates:
// 1. Hosts 0..7 (`KarstFore Host`)
// 2. Karst Fabric Module (`KarstModule`)
// 3. DDR5 Memory Channels 0..7 (`MemChan 0..7`)
// The Karst Fabric Module connects Hosts to DDR5 Memory Channels.

#[derive(Clone, Default)]
pub struct KarstSystemCircuit;

impl KarstSystemCircuit
{
    pub fn New() -> Self { Self }
}

impl Xplr for KarstSystemCircuit
{
    fn Name(&self) -> &str { "Karst" }
    fn Path(&self) -> &str { "example://Karst/System" }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
    fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
}

impl BranchXplr for KarstSystemCircuit
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut stash = Stash::New();

        // 1. Hosts 0..7
        USeg::FromLen(8).Traverse(|i| {
                            let path = format!("{}/Host {}", self.Path(), i);
                            stash.Push(Box::new(KarstComponentXplr::NewHostWithPath(i, path))
                                       as Box<dyn Xplr>);
                        });

        // 2. Karst Fabric Module
        let km_path = format!("{}/KarstModule", self.Path());
        stash.Push(Box::new(KarstFabricCircuit::NewNamed("KarstModule", km_path)) as Box<dyn Xplr>);

        // 3. DDR5 Memory Channels 0..7
        USeg::FromLen(8).Traverse(|i| {
                            let path = format!("{}/MemChan {}", self.Path(), i);
                            stash.Push(Box::new(KarstMemChanCircuit::NewWithPath(i, path))
                                       as Box<dyn Xplr>);
                        });

        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String> { Ok(17) }
}

impl CircuitXplr for KarstSystemCircuit
{
    fn ComponentType(&self) -> &str { "Multi-Die SoC System" }

    fn Ports(&self) -> Vec<PortDescriptor> { SystemPorts() }

    fn Wires(&self) -> Vec<WireDescriptor> { SystemWires(self.Name(), "KarstModule") }

    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        let mut subs: Vec<Box<dyn CircuitXplr + '_>> = Vec::new();

        // 1. Hosts 0..7
        USeg::FromLen(8).Traverse(|i| {
                            let path = format!("{}/Host {}", self.Path(), i);
                            subs.push(Box::new(KarstComponentXplr::NewHostWithPath(i, path)));
                        });

        // 2. Karst Fabric Module
        let km_path = format!("{}/KarstModule", self.Path());
        subs.push(Box::new(KarstFabricCircuit::NewNamed("KarstModule", km_path)));

        // 3. DDR5 Memory Channels 0..7
        USeg::FromLen(8).Traverse(|i| {
                            let path = format!("{}/MemChan {}", self.Path(), i);
                            subs.push(Box::new(KarstMemChanCircuit::NewWithPath(i, path)));
                        });

        subs
    }
}

//-------------------------------------------------------------------------------------------------
