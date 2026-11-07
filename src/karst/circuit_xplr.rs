// circuit_xplr.rs ---------------------------------------------------------------------------------
//! Implements `fenst::xplr::CircuitXplr` and `fenst::xplr::Xplr` for Karst multi-die SoC fabrics,
//! dies, hosts, crossbar NoCs, memory channels, and near-memory VPUs.

use crate::fenst::xplr::{BranchXplr, CircuitXplr, PortDescriptor, PortDirection, WireDescriptor, Xplr};
use crate::karst::fabric::KarstFabric;
use crate::silo::{Buff, Stash};
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------
// Top-Level KarstFabric Circuit Explorer

pub struct KarstFabricXplr<'a>
{
    _fabric: &'a KarstFabric,
}

impl<'a> KarstFabricXplr<'a>
{
    pub fn New(fabric: &'a KarstFabric) -> Self
    {
        Self { _fabric: fabric }
    }

    pub fn Fabric(&self) -> &'a KarstFabric
    {
        self._fabric
    }
}

impl Xplr for KarstFabricXplr<'_>
{
    fn Name(&self) -> &str
    {
        "KarstFabric"
    }

    fn Path(&self) -> &str
    {
        "karst://fabric"
    }

    fn Circuit(&self) -> Option<&dyn CircuitXplr>
    {
        Some(self)
    }
}

fn FabricPorts() -> Vec<PortDescriptor>
{
    vec![
        PortDescriptor::New("SysClk", PortDirection::In),
        PortDescriptor::New("Reset_n", PortDirection::In),
        PortDescriptor::New("Status", PortDirection::Out),
        PortDescriptor::New("Interrupt", PortDirection::Out),
    ]
}

fn FabricWires() -> Vec<WireDescriptor>
{
    vec![
        WireDescriptor::New("Die 0", "InterDie_Tx", "Die 1", "InterDie_Rx")
            .WithName("InterDie_Link_0_1")
            .WithWidth(64),
        WireDescriptor::New("Die 1", "InterDie_Tx", "Die 0", "InterDie_Rx")
            .WithName("InterDie_Link_1_0")
            .WithWidth(64),
    ]
}

fn DiePorts() -> Vec<PortDescriptor>
{
    vec![
        PortDescriptor::New("InterDie_Rx", PortDirection::In).WithType(64, "KarstLink"),
        PortDescriptor::New("InterDie_Tx", PortDirection::Out).WithType(64, "KarstLink"),
    ]
}

fn DieWires(die_id: u32) -> Vec<WireDescriptor>
{
    let die_name = format!("Die {}", die_id);
    let base_host = die_id * 4;
    let base_chan = die_id * 4;
    let noc_name = format!("NoC {}", die_id);
    let mut wires = Vec::new();

    for i in 0..4
    {
        let host_name = format!("Host {}", base_host + i);
        let chan_name = format!("MemChan {}", base_chan + i);
        let vpu_name = format!("VPU {}", base_chan + i);

        // Host <-> NoC
        wires.push(WireDescriptor::New(host_name.clone(), "Req", noc_name.clone(), format!("Host_In_{}", i))
            .WithName(format!("HostReq_{}", base_host + i)));
        wires.push(WireDescriptor::New(noc_name.clone(), format!("Host_Out_{}", i), host_name, "Resp")
            .WithName(format!("HostResp_{}", base_host + i)));

        // NoC <-> MemChan
        wires.push(WireDescriptor::New(noc_name.clone(), format!("MC_Out_{}", i), chan_name.clone(), "Req")
            .WithName(format!("McReq_{}", base_chan + i)));
        wires.push(WireDescriptor::New(chan_name.clone(), "Resp", noc_name.clone(), format!("MC_In_{}", i))
            .WithName(format!("McResp_{}", base_chan + i)));

        // MemChan <-> VPU
        wires.push(WireDescriptor::New(chan_name.clone(), "Vpu_Out", vpu_name.clone(), "Stream_In")
            .WithName(format!("VpuStream_{}", base_chan + i)));
        wires.push(WireDescriptor::New(vpu_name, "Writeback_Out", chan_name, "Vpu_In")
            .WithName(format!("VpuWriteback_{}", base_chan + i)));
    }

    // Inter-die link boundary connections
    wires.push(WireDescriptor::New(die_name.clone(), "InterDie_Rx", noc_name.clone(), "InterDie_In"));
    wires.push(WireDescriptor::New(noc_name, "InterDie_Out", die_name, "InterDie_Tx"));

    wires
}

fn DieSubmodules(die_id: u32) -> Vec<Box<dyn CircuitXplr + 'static>>
{
    let base_host = die_id * 4;
    let base_chan = die_id * 4;
    let mut subs: Vec<Box<dyn CircuitXplr + 'static>> = Vec::new();

    // 4 Hosts
    for i in 0..4
    {
        subs.push(Box::new(KarstComponentXplr::NewHost(base_host + i)));
    }

    // 1 NoC Crossbar
    subs.push(Box::new(KarstComponentXplr::NewNoc(die_id)));

    // 4 DDR5 Memory Controllers
    for i in 0..4
    {
        subs.push(Box::new(KarstComponentXplr::NewMemChan(base_chan + i)));
    }

    // 4 Near-Memory VPUs
    for i in 0..4
    {
        subs.push(Box::new(KarstComponentXplr::NewVpu(base_chan + i)));
    }

    subs
}

impl CircuitXplr for KarstFabricXplr<'_>
{
    fn ComponentType(&self) -> &str
    {
        "SoC Fabric (Multi-Die)"
    }

    fn Ports(&self) -> Vec<PortDescriptor>
    {
        FabricPorts()
    }

    fn Wires(&self) -> Vec<WireDescriptor>
    {
        FabricWires()
    }

    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        vec![
            Box::new(KarstDieXplr::New(self._fabric, 0)),
            Box::new(KarstDieXplr::New(self._fabric, 1)),
        ]
    }
}

//-------------------------------------------------------------------------------------------------
// Karst Die (KarstHind Memory Fabric Die) Circuit Explorer

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
        Self {
            _fabric: fabric,
            _die_id: die_id,
            _name:   format!("Die {}", die_id),
            _path:   format!("karst://fabric/die{}", die_id),
        }
    }

    pub fn DieId(&self) -> u32
    {
        self._die_id
    }
}

impl Xplr for KarstDieXplr<'_>
{
    fn Name(&self) -> &str
    {
        &self._name
    }

    fn Path(&self) -> &str
    {
        &self._path
    }

    fn Circuit(&self) -> Option<&dyn CircuitXplr>
    {
        Some(self)
    }
}

impl CircuitXplr for KarstDieXplr<'_>
{
    fn ComponentType(&self) -> &str
    {
        "KarstHind Die"
    }

    fn Ports(&self) -> Vec<PortDescriptor>
    {
        DiePorts()
    }

    fn Wires(&self) -> Vec<WireDescriptor>
    {
        DieWires(self._die_id)
    }

    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        DieSubmodules(self._die_id)
            .into_iter()
            .map(|s| s as Box<dyn CircuitXplr + '_>)
            .collect()
    }
}

//-------------------------------------------------------------------------------------------------
// Karst Leaf Component (Host, NoC, MemChan, VPU)

pub struct KarstComponentXplr
{
    _name:      String,
    _path:      String,
    _kind:      &'static str,
    _ports:     Vec<PortDescriptor>,
}

impl KarstComponentXplr
{
    pub fn NewHost(id: u32) -> Self
    {
        Self {
            _name:  format!("Host {}", id),
            _path:  format!("karst://fabric/host/{}", id),
            _kind:  "KarstFore Host",
            _ports: vec![
                PortDescriptor::New("Req", PortDirection::Out),
                PortDescriptor::New("Resp", PortDirection::In),
            ],
        }
    }

    pub fn NewNoc(die_id: u32) -> Self
    {
        let mut ports = Vec::new();
        for i in 0..4
        {
            ports.push(PortDescriptor::New(format!("Host_In_{}", i), PortDirection::In));
            ports.push(PortDescriptor::New(format!("Host_Out_{}", i), PortDirection::Out));
            ports.push(PortDescriptor::New(format!("MC_In_{}", i), PortDirection::In));
            ports.push(PortDescriptor::New(format!("MC_Out_{}", i), PortDirection::Out));
        }
        ports.push(PortDescriptor::New("InterDie_In", PortDirection::In));
        ports.push(PortDescriptor::New("InterDie_Out", PortDirection::Out));

        Self {
            _name:  format!("NoC {}", die_id),
            _path:  format!("karst://fabric/noc/{}", die_id),
            _kind:  "MemFabric Crossbar",
            _ports: ports,
        }
    }

    pub fn NewMemChan(id: u32) -> Self
    {
        Self {
            _name:  format!("MemChan {}", id),
            _path:  format!("karst://fabric/memchan/{}", id),
            _kind:  "DDR5 Controller",
            _ports: vec![
                PortDescriptor::New("Req", PortDirection::In),
                PortDescriptor::New("Resp", PortDirection::Out),
                PortDescriptor::New("Vpu_Out", PortDirection::Out),
                PortDescriptor::New("Vpu_In", PortDirection::In),
            ],
        }
    }

    pub fn NewVpu(id: u32) -> Self
    {
        Self {
            _name:  format!("VPU {}", id),
            _path:  format!("karst://fabric/vpu/{}", id),
            _kind:  "Near-Memory VPU",
            _ports: vec![
                PortDescriptor::New("Stream_In", PortDirection::In),
                PortDescriptor::New("Writeback_Out", PortDirection::Out),
            ],
        }
    }
}

impl Xplr for KarstComponentXplr
{
    fn Name(&self) -> &str
    {
        &self._name
    }

    fn Path(&self) -> &str
    {
        &self._path
    }

    fn Circuit(&self) -> Option<&dyn CircuitXplr>
    {
        Some(self)
    }
}

impl CircuitXplr for KarstComponentXplr
{
    fn ComponentType(&self) -> &str
    {
        self._kind
    }

    fn Ports(&self) -> Vec<PortDescriptor>
    {
        self._ports.clone()
    }

    fn Wires(&self) -> Vec<WireDescriptor>
    {
        Vec::new()
    }

    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        Vec::new()
    }
}

//-------------------------------------------------------------------------------------------------
/// Owned, self-contained `Xplr` and `CircuitXplr` implementation for Karst multi-die SoC fabric.
#[derive(Clone)]
pub struct KarstFabricCircuit
{
    _fabric: Arc<KarstFabric>,
}

impl KarstFabricCircuit
{
    pub fn New(fabric: Arc<KarstFabric>) -> Self
    {
        Self { _fabric: fabric }
    }
}

impl Xplr for KarstFabricCircuit
{
    fn Name(&self) -> &str { "KarstFabric" }
    fn Path(&self) -> &str { "example://Karst/FabricExplorer" }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn BranchXplr> { Some(self) }
    fn Circuit(&self) -> Option<&dyn CircuitXplr> { Some(self) }
}

impl BranchXplr for KarstFabricCircuit
{
    fn Children(&self) -> Result<Buff<Box<dyn Xplr>>, String>
    {
        let mut stash = Stash::New();
        stash.Push(Box::new(KarstDieCircuit::New(self._fabric.clone(), 0)) as Box<dyn Xplr>);
        stash.Push(Box::new(KarstDieCircuit::New(self._fabric.clone(), 1)) as Box<dyn Xplr>);
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(2)
    }
}

impl CircuitXplr for KarstFabricCircuit
{
    fn ComponentType(&self) -> &str { "SoC Fabric (Multi-Die)" }
    fn Ports(&self) -> Vec<PortDescriptor> { FabricPorts() }
    fn Wires(&self) -> Vec<WireDescriptor> { FabricWires() }
    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        vec![
            Box::new(KarstDieCircuit::New(self._fabric.clone(), 0)),
            Box::new(KarstDieCircuit::New(self._fabric.clone(), 1)),
        ]
    }
}

//-------------------------------------------------------------------------------------------------
/// Owned, self-contained `Xplr` and `CircuitXplr` implementation for an individual Karst die.
#[derive(Clone)]
pub struct KarstDieCircuit
{
    _fabric: Arc<KarstFabric>,
    _die_id: u32,
    _name:   String,
    _path:   String,
}

impl KarstDieCircuit
{
    pub fn New(fabric: Arc<KarstFabric>, die_id: u32) -> Self
    {
        Self {
            _fabric: fabric,
            _die_id: die_id,
            _name:   format!("Die {}", die_id),
            _path:   if die_id == 0 {
                "example://Karst/DieCircuit".to_string()
            } else {
                format!("example://Karst/Die{}Circuit", die_id)
            },
        }
    }
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
        let base_host = self._die_id * 4;
        let base_chan = self._die_id * 4;
        let mut stash = Stash::New();
        for i in 0..4 {
            stash.Push(Box::new(KarstComponentXplr::NewHost(base_host + i)) as Box<dyn Xplr>);
        }
        stash.Push(Box::new(KarstComponentXplr::NewNoc(self._die_id)) as Box<dyn Xplr>);
        for i in 0..4 {
            stash.Push(Box::new(KarstComponentXplr::NewMemChan(base_chan + i)) as Box<dyn Xplr>);
        }
        for i in 0..4 {
            stash.Push(Box::new(KarstComponentXplr::NewVpu(base_chan + i)) as Box<dyn Xplr>);
        }
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(13)
    }
}

impl CircuitXplr for KarstDieCircuit
{
    fn ComponentType(&self) -> &str { "KarstHind Die" }
    fn Ports(&self) -> Vec<PortDescriptor> { DiePorts() }
    fn Wires(&self) -> Vec<WireDescriptor> { DieWires(self._die_id) }
    fn Submodules(&self) -> Vec<Box<dyn CircuitXplr + '_>>
    {
        DieSubmodules(self._die_id)
            .into_iter()
            .map(|s| s as Box<dyn CircuitXplr + '_>)
            .collect()
    }
}
