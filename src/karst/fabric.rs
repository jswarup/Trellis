// src/karst/fabric.rs
use crate::karst::config::{
    K_HIND_DIES_PER_FABRIC, K_HOSTS_PER_FABRIC, K_HOSTS_PER_HIND, K_INTERDIE_PORT_BASE,
    K_KL_PORTS_PER_HIND, K_MEM_CHANS_PER_FABRIC,
};
use crate::karst::fabric_node::KarstFabricNode;
use crate::karst::host_node::{HostResponse, KarstHostNode};
use crate::karst::memchan::MemChan;
use crate::karst::vpu::Vpu;
use crate::karst::memchan::KarstDChan;
use crate::karst::vpu::KarstVPU;
use crate::silo::USeg;
use crate::swarm::cpu::ComputeDevice;

//-------------------------------------------------------------------------------------------------

// Aggregate fabric metrics across all host ports, memory channels, and VPUs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct KarstStats {
pub struct KarstStats
{
    pub _TotalTxCount: u32,
    pub _TotalRxCount: u32,
    pub _TotalBytesWritten: u64,
    pub _TotalBytesRead: u64,
    pub _TotalVPUDispatches: u32,
    pub _CycleCount: u64,
}

//-------------------------------------------------------------------------------------------------

// Simulation engine representation for inspection and diagnostics.
#[derive(Copy, Clone, Debug, Default)]
pub struct KarstEngineInfo {
pub struct KarstEngineInfo
{
    pub _CycleCount: u64,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct DieInputSignals {
pub struct DieInputSignals
{
    pub valid: [bool; K_KL_PORTS_PER_HIND],
    pub data: [u64; K_KL_PORTS_PER_HIND],
    pub ready: [bool; K_KL_PORTS_PER_HIND],
}

//-------------------------------------------------------------------------------------------------

// KarstFabric — top-level system framework orchestrating the balanced Karst(8, 8) topology.
// Wires 8 KarstFore host nodes with 2 KarstHind Memory Fabric dies over KarstLinks.
pub struct KarstFabric {
    _compute_device: ComputeDevice,
    _hosts: [KarstHostNode; K_HOSTS_PER_FABRIC as usize],
    _fabrics: [KarstFabricNode; K_HIND_DIES_PER_FABRIC as usize],
    _cycle_count: u64,
    _workers: u32,
pub struct KarstFabric
{
    _ComputeDevice: ComputeDevice,
    _Hosts: [KarstHostNode; K_HOSTS_PER_FABRIC as usize],
    _Fabrics: [KarstFabricNode; K_HIND_DIES_PER_FABRIC as usize],
    _CycleCount: u64,
    _Workers: u32,
}
impl Default for KarstFabric {
    fn default() -> Self {
        Self::new()

impl Default for KarstFabric
{
    fn default() -> Self
    {
        Self::New()
    }
}
impl KarstFabric {
    pub fn new() -> Self {
        Self::with_workers(1)

impl KarstFabric
{
    pub fn New() -> Self
    {
        Self::WithWorkers(1)
    }
    pub fn with_workers(workers: u32) -> Self {
        let compute_device = ComputeDevice::WithWorkers(workers);

    pub fn new() -> Self
    {
        Self::New()
    }

    pub fn WithWorkers(workers: u32) -> Self
    {
        let computeDevice = ComputeDevice::WithWorkers(workers);
        let hosts = [
            KarstHostNode::new(0),
            KarstHostNode::new(1),
            KarstHostNode::new(2),
            KarstHostNode::new(3),
            KarstHostNode::new(4),
            KarstHostNode::new(5),
            KarstHostNode::new(6),
            KarstHostNode::new(7),
            KarstHostNode::New(0),
            KarstHostNode::New(1),
            KarstHostNode::New(2),
            KarstHostNode::New(3),
            KarstHostNode::New(4),
            KarstHostNode::New(5),
            KarstHostNode::New(6),
            KarstHostNode::New(7),
        ];
        let fabrics = [
            KarstFabricNode::new(&compute_device, 0),
            KarstFabricNode::new(&compute_device, 1),
            KarstFabricNode::New(&computeDevice, 0),
            KarstFabricNode::New(&computeDevice, 1),
        ];
        Self {
            _compute_device: compute_device,
            _hosts: hosts,
            _fabrics: fabrics,
            _cycle_count: 0,
            _workers: workers.max(1),
            _ComputeDevice: computeDevice,
            _Hosts: hosts,
            _Fabrics: fabrics,
            _CycleCount: 0,
            _Workers: workers.max(1),
        }
    }

    pub fn with_workers(workers: u32) -> Self
    {
        Self::WithWorkers(workers)
    }

    #[inline]
    pub fn cycle_count(&self) -> u64 {
        self._cycle_count
    pub fn CycleCount(&self) -> u64
    {
        self._CycleCount
    }

    #[inline]
    pub fn workers(&self) -> u32 {
        self._workers
    pub fn cycle_count(&self) -> u64
    {
        self.CycleCount()
    }

    #[inline]
    pub fn compute_device(&self) -> &ComputeDevice {
        &self._compute_device
    pub fn Workers(&self) -> u32
    {
        self._Workers
    }

    #[inline]
    pub fn compute_device_mut(&mut self) -> &mut ComputeDevice {
        &mut self._compute_device
    pub fn workers(&self) -> u32
    {
        self.Workers()
    }

    #[inline]
    pub fn engine(&self) -> KarstEngineInfo {
    pub fn ComputeDeviceRef(&self) -> &ComputeDevice
    {
        &self._ComputeDevice
    }

    #[inline]
    pub fn compute_device(&self) -> &ComputeDevice
    {
        self.ComputeDeviceRef()
    }

    #[inline]
    pub fn ComputeDeviceMut(&mut self) -> &mut ComputeDevice
    {
        &mut self._ComputeDevice
    }

    #[inline]
    pub fn compute_device_mut(&mut self) -> &mut ComputeDevice
    {
        self.ComputeDeviceMut()
    }

    #[inline]
    pub fn Engine(&self) -> KarstEngineInfo
    {
        KarstEngineInfo {
            _CycleCount: self._cycle_count,
            _CycleCount: self._CycleCount,
        }
    }

    #[inline]
    pub fn Engine(&self) -> KarstEngineInfo {
        self.engine()
    pub fn engine(&self) -> KarstEngineInfo
    {
        self.Engine()
    }

    #[inline]
    pub fn host(&self, id: u32) -> &KarstHostNode {
        &self._hosts[id as usize]
    pub fn Host(&self, hostId: u32) -> &KarstHostNode
    {
        &self._Hosts[hostId as usize]
    }

    #[inline]
    pub fn host_mut(&mut self, id: u32) -> &mut KarstHostNode {
        &mut self._hosts[id as usize]
    pub fn host(&self, hostId: u32) -> &KarstHostNode
    {
        self.Host(hostId)
    }

    #[inline]
    pub fn Host(&self, id: u32) -> &KarstHostNode {
        self.host(id)
    pub fn HostMut(&mut self, hostId: u32) -> &mut KarstHostNode
    {
        &mut self._Hosts[hostId as usize]
    }

    #[inline]
    pub fn HostMut(&mut self, id: u32) -> &mut KarstHostNode {
        self.host_mut(id)
    pub fn host_mut(&mut self, hostId: u32) -> &mut KarstHostNode
    {
        self.HostMut(hostId)
    }

    #[inline]
    pub fn fabric_node(&self, die_id: u32) -> &KarstFabricNode {
        &self._fabrics[die_id as usize]
    pub fn FabricNode(&self, dieId: u32) -> &KarstFabricNode
    {
        &self._Fabrics[dieId as usize]
    }

    #[inline]
    pub fn fabric_node_mut(&mut self, die_id: u32) -> &mut KarstFabricNode {
        &mut self._fabrics[die_id as usize]
    pub fn fabric_node(&self, dieId: u32) -> &KarstFabricNode
    {
        self.FabricNode(dieId)
    }

    #[inline]
    pub fn FabricNode(&self, die_id: u32) -> &KarstFabricNode {
        self.fabric_node(die_id)
    pub fn FabricNodeMut(&mut self, dieId: u32) -> &mut KarstFabricNode
    {
        &mut self._Fabrics[dieId as usize]
    }

    #[inline]
    pub fn FabricNodeMut(&mut self, die_id: u32) -> &mut KarstFabricNode {
        self.fabric_node_mut(die_id)
    pub fn fabric_node_mut(&mut self, dieId: u32) -> &mut KarstFabricNode
    {
        self.FabricNodeMut(dieId)
    }

    #[inline]
    pub fn mem_chan(&self, global_idx: u32) -> &MemChan {
        let die_id = (global_idx / 4) as usize;
        let mc_idx = (global_idx % 4) as usize;
        self._fabrics[die_id].mem_chan(mc_idx)
    pub fn DChan(&self, globalIdx: u32) -> &KarstDChan
    {
        let dieId = (globalIdx / 4) as usize;
        let mcIdx = (globalIdx % 4) as usize;
        self._Fabrics[dieId].DChan(mcIdx)
    }

    #[inline]
    pub fn mem_chan_mut(&mut self, global_idx: u32) -> &mut MemChan {
        let die_id = (global_idx / 4) as usize;
        let mc_idx = (global_idx % 4) as usize;
        self._fabrics[die_id].mem_chan_mut(mc_idx)
    pub fn dchan(&self, globalIdx: u32) -> &KarstDChan
    {
        self.DChan(globalIdx)
    }

    #[inline]
    pub fn MemChan(&self, global_idx: u32) -> &MemChan {
        self.mem_chan(global_idx)
    pub fn DChanMut(&mut self, globalIdx: u32) -> &mut KarstDChan
    {
        let dieId = (globalIdx / 4) as usize;
        let mcIdx = (globalIdx % 4) as usize;
        self._Fabrics[dieId].DChanMut(mcIdx)
    }

    #[inline]
    pub fn MemChanMut(&mut self, global_idx: u32) -> &mut MemChan {
        self.mem_chan_mut(global_idx)
    pub fn dchan_mut(&mut self, globalIdx: u32) -> &mut KarstDChan
    {
        self.DChanMut(globalIdx)
    }
    // Trellis alias DChan -> MemChan

    #[inline]
    pub fn DChan(&self, global_idx: u32) -> &MemChan {
        self.mem_chan(global_idx)
    pub fn MemChan(&self, globalIdx: u32) -> &KarstDChan
    {
        self.DChan(globalIdx)
    }

    #[inline]
    pub fn DChanMut(&mut self, global_idx: u32) -> &mut MemChan {
        self.mem_chan_mut(global_idx)
    pub fn mem_chan(&self, globalIdx: u32) -> &KarstDChan
    {
        self.DChan(globalIdx)
    }

    #[inline]
    pub fn vpu(&self, global_idx: u32) -> &Vpu {
        let die_id = (global_idx / 4) as usize;
        let mc_idx = (global_idx % 4) as usize;
        self._fabrics[die_id].vpu(mc_idx)
    pub fn MemChanMut(&mut self, globalIdx: u32) -> &mut KarstDChan
    {
        self.DChanMut(globalIdx)
    }

    #[inline]
    pub fn vpu_mut(&mut self, global_idx: u32) -> &mut Vpu {
        let die_id = (global_idx / 4) as usize;
        let mc_idx = (global_idx % 4) as usize;
        self._fabrics[die_id].vpu_mut(mc_idx)
    pub fn mem_chan_mut(&mut self, globalIdx: u32) -> &mut KarstDChan
    {
        self.DChanMut(globalIdx)
    }

    #[inline]
    pub fn VPU(&self, global_idx: u32) -> &Vpu {
        self.vpu(global_idx)
    pub fn VPU(&self, globalIdx: u32) -> &KarstVPU
    {
        let dieId = (globalIdx / 4) as usize;
        let mcIdx = (globalIdx % 4) as usize;
        self._Fabrics[dieId].VPU(mcIdx)
    }

    #[inline]
    pub fn VPUMut(&mut self, global_idx: u32) -> &mut Vpu {
        self.vpu_mut(global_idx)
    pub fn vpu(&self, globalIdx: u32) -> &KarstVPU
    {
        self.VPU(globalIdx)
    }

    #[inline]
    pub fn VPU_mut(&mut self, global_idx: u32) -> &mut Vpu {
        self.vpu_mut(global_idx)
    pub fn VPUMut(&mut self, globalIdx: u32) -> &mut KarstVPU
    {
        let dieId = (globalIdx / 4) as usize;
        let mcIdx = (globalIdx % 4) as usize;
        self._Fabrics[dieId].VPUMut(mcIdx)
    }

    #[inline]
    pub fn MemChan_mut(&mut self, global_idx: u32) -> &mut MemChan {
        self.mem_chan_mut(global_idx)
    pub fn vpu_mut(&mut self, globalIdx: u32) -> &mut KarstVPU
    {
        self.VPUMut(globalIdx)
    }

    pub fn DispatchVPU(
        &self,
        vpuIdx: u32,
        chanIdx: u32,
        dim: crate::swarm::traits::WorkgroupDim,
    ) -> Result<(), crate::swarm::traits::SwarmError>
    {
        let vpu = self.VPU(vpuIdx);
        let chan = self.DChan(chanIdx);
        vpu.Dispatch(&self._ComputeDevice, chan, dim)
    }

    #[inline]
    pub fn dispatch_vpu(
        &self,
        vpu_idx: u32,
        chan_idx: u32,
        vpuIdx: u32,
        chanIdx: u32,
        dim: crate::swarm::traits::WorkgroupDim,
    ) -> Result<(), crate::swarm::traits::SwarmError> {
        let vpu = self.vpu(vpu_idx);
        let chan = self.mem_chan(chan_idx);
        vpu.dispatch(&self._compute_device, chan, dim)
    ) -> Result<(), crate::swarm::traits::SwarmError>
    {
        self.DispatchVPU(vpuIdx, chanIdx, dim)
    }
    pub fn try_post_host_write(

    pub fn TryPostHostWrite(
        &mut self,
        host_id: u32,
        hostId: u32,
        addr: u32,
        data: u32,
    ) -> Result<(), &'static str> {
        if host_id < K_HOSTS_PER_FABRIC {
            self._hosts[host_id as usize].post_write(addr, data)
    ) -> Result<(), &'static str>
    {
        if hostId < K_HOSTS_PER_FABRIC {
            self._Hosts[hostId as usize].PostWrite(addr, data)
        } else {
            Err("Invalid host id")
        }
    }
    pub fn post_host_write(&mut self, host_id: u32, addr: u32, data: u32) {
        let _ = self.try_post_host_write(host_id, addr, data);

    #[inline]
    pub fn try_post_host_write(
        &mut self,
        hostId: u32,
        addr: u32,
        data: u32,
    ) -> Result<(), &'static str>
    {
        self.TryPostHostWrite(hostId, addr, data)
    }

    pub fn PostHostWrite(&mut self, hostId: u32, addr: u32, data: u32)
    {
        let _ = self.TryPostHostWrite(hostId, addr, data);
    }

    #[inline]
    pub fn PostHostWrite(&mut self, host_id: u32, addr: u32, data: u32) {
        self.post_host_write(host_id, addr, data);
    pub fn post_host_write(&mut self, hostId: u32, addr: u32, data: u32)
    {
        self.PostHostWrite(hostId, addr, data);
    }
    pub fn try_post_host_read(&mut self, host_id: u32, addr: u32) -> Result<(), &'static str> {
        if host_id < K_HOSTS_PER_FABRIC {
            self._hosts[host_id as usize].post_read(addr)

    pub fn TryPostHostRead(&mut self, hostId: u32, addr: u32) -> Result<(), &'static str>
    {
        if hostId < K_HOSTS_PER_FABRIC {
            self._Hosts[hostId as usize].PostRead(addr)
        } else {
            Err("Invalid host id")
        }
    }
    pub fn post_host_read(&mut self, host_id: u32, addr: u32) {
        let _ = self.try_post_host_read(host_id, addr);

    #[inline]
    pub fn try_post_host_read(&mut self, hostId: u32, addr: u32) -> Result<(), &'static str>
    {
        self.TryPostHostRead(hostId, addr)
    }

    pub fn PostHostRead(&mut self, hostId: u32, addr: u32)
    {
        let _ = self.TryPostHostRead(hostId, addr);
    }

    #[inline]
    pub fn PostHostRead(&mut self, host_id: u32, addr: u32) {
        self.post_host_read(host_id, addr);
    pub fn post_host_read(&mut self, hostId: u32, addr: u32)
    {
        self.PostHostRead(hostId, addr);
    }
    pub fn pop_host_response(&mut self, host_id: u32, out: &mut HostResponse) -> bool {
        if host_id < K_HOSTS_PER_FABRIC {
            self._hosts[host_id as usize].pop_response(out)

    pub fn PopHostResponse(&mut self, hostId: u32, out: &mut HostResponse) -> bool
    {
        if hostId < K_HOSTS_PER_FABRIC {
            self._Hosts[hostId as usize].PopResponse(out)
        } else {
            false
        }
    }

    #[inline]
    pub fn PopHostResponse(&mut self, host_id: u32, out: &mut HostResponse) -> bool {
        self.pop_host_response(host_id, out)
    pub fn pop_host_response(&mut self, hostId: u32, out: &mut HostResponse) -> bool
    {
        self.PopHostResponse(hostId, out)
    }
    pub fn stats(&self) -> KarstStats {

    pub fn Stats(&self) -> KarstStats
    {
        let mut total = KarstStats {
            _TotalTxCount: 0,
            _TotalRxCount: 0,
            _TotalBytesWritten: 0,
            _TotalBytesRead: 0,
            _TotalVPUDispatches: 0,
            _CycleCount: self._cycle_count,
            _CycleCount: self._CycleCount,
        };
        for h in 0..K_HOSTS_PER_FABRIC {
            let hs = self._hosts[h as usize].stats();
        USeg::FromLen(K_HOSTS_PER_FABRIC).Traverse(|h| {
            let hs = self._Hosts[h as usize].Stats();
            total._TotalTxCount += hs._TxCount;
            total._TotalRxCount += hs._RxCount;
        }
        for c in 0..K_MEM_CHANS_PER_FABRIC {
            let ds = self.mem_chan(c).stats();
        });
        USeg::FromLen(K_MEM_CHANS_PER_FABRIC).Traverse(|c| {
            let ds = self.DChan(c).Stats();
            total._TotalBytesWritten += ds._BytesWritten;
            total._TotalBytesRead += ds._BytesRead;
        }
        for v in 0..K_MEM_CHANS_PER_FABRIC {
            total._TotalVPUDispatches += self.vpu(v).dispatches();
        }
        });
        USeg::FromLen(K_MEM_CHANS_PER_FABRIC).Traverse(|v| {
            total._TotalVPUDispatches += self.VPU(v).Dispatches();
        });
        total
    }

    #[inline]
    pub fn Stats(&self) -> KarstStats {
        self.stats()
    pub fn stats(&self) -> KarstStats
    {
        self.Stats()
    }
    fn prepare_cycle_inputs(&mut self) -> (DieInputSignals, DieInputSignals) {

    fn PrepareCycleInputs(&mut self) -> (DieInputSignals, DieInputSignals)
    {
        // 1. Sample signals from Die 0 and Die 1 NoCs
        let d0_noc = self._fabrics[0].noc();
        let mut d0_kl_tx_valid = [false; K_KL_PORTS_PER_HIND];
        let mut d0_kl_tx_data = [0u64; K_KL_PORTS_PER_HIND];
        let mut d0_kl_rx_ready = [false; K_KL_PORTS_PER_HIND];
        for t in 0..K_KL_PORTS_PER_HIND {
            d0_kl_tx_valid[t] = d0_noc.kl_tx_valid(t);
            d0_kl_tx_data[t] = d0_noc.kl_tx_data(t);
            d0_kl_rx_ready[t] = d0_noc.kl_rx_ready(t);
        }
        let d1_noc = self._fabrics[1].noc();
        let mut d1_kl_tx_valid = [false; K_KL_PORTS_PER_HIND];
        let mut d1_kl_tx_data = [0u64; K_KL_PORTS_PER_HIND];
        let mut d1_kl_rx_ready = [false; K_KL_PORTS_PER_HIND];
        for t in 0..K_KL_PORTS_PER_HIND {
            d1_kl_tx_valid[t] = d1_noc.kl_tx_valid(t);
            d1_kl_tx_data[t] = d1_noc.kl_tx_data(t);
            d1_kl_rx_ready[t] = d1_noc.kl_rx_ready(t);
        }
        let d0Noc = self._Fabrics[0].Noc();
        let mut d0KlTxValid = [false; K_KL_PORTS_PER_HIND];
        let mut d0KlTxData = [0u64; K_KL_PORTS_PER_HIND];
        let mut d0KlRxReady = [false; K_KL_PORTS_PER_HIND];
        USeg::FromLen(K_KL_PORTS_PER_HIND as u32).Traverse(|t_u| {
            let t = t_u as usize;
            d0KlTxValid[t] = d0Noc.KlTxValid(t);
            d0KlTxData[t] = d0Noc.KlTxData(t);
            d0KlRxReady[t] = d0Noc.KlRxReady(t);
        });

        let d1Noc = self._Fabrics[1].Noc();
        let mut d1KlTxValid = [false; K_KL_PORTS_PER_HIND];
        let mut d1KlTxData = [0u64; K_KL_PORTS_PER_HIND];
        let mut d1KlRxReady = [false; K_KL_PORTS_PER_HIND];
        USeg::FromLen(K_KL_PORTS_PER_HIND as u32).Traverse(|t_u| {
            let t = t_u as usize;
            d1KlTxValid[t] = d1Noc.KlTxValid(t);
            d1KlTxData[t] = d1Noc.KlTxData(t);
            d1KlRxReady[t] = d1Noc.KlRxReady(t);
        });

        // 2. Sample signals from Hosts
        let mut h_l0_tx_valid = [false; K_HOSTS_PER_FABRIC as usize];
        let mut h_l0_tx_data = [0u64; K_HOSTS_PER_FABRIC as usize];
        let mut h_l0_rx_ready = [false; K_HOSTS_PER_FABRIC as usize];
        let mut h_l1_tx_valid = [false; K_HOSTS_PER_FABRIC as usize];
        let mut h_l1_tx_data = [0u64; K_HOSTS_PER_FABRIC as usize];
        let mut h_l1_rx_ready = [false; K_HOSTS_PER_FABRIC as usize];
        for h in 0..K_HOSTS_PER_FABRIC as usize {
            h_l0_tx_valid[h] = self._hosts[h].l0_tx_valid;
            h_l0_tx_data[h] = self._hosts[h].l0_tx_data;
            h_l0_rx_ready[h] = self._hosts[h].l0_rx_ready;
            h_l1_tx_valid[h] = self._hosts[h].l1_tx_valid;
            h_l1_tx_data[h] = self._hosts[h].l1_tx_data;
            h_l1_rx_ready[h] = self._hosts[h].l1_rx_ready;
        }
        let mut hL0TxValid = [false; K_HOSTS_PER_FABRIC as usize];
        let mut hL0TxData = [0u64; K_HOSTS_PER_FABRIC as usize];
        let mut hL0RxReady = [false; K_HOSTS_PER_FABRIC as usize];
        let mut hL1TxValid = [false; K_HOSTS_PER_FABRIC as usize];
        let mut hL1TxData = [0u64; K_HOSTS_PER_FABRIC as usize];
        let mut hL1RxReady = [false; K_HOSTS_PER_FABRIC as usize];
        USeg::FromLen(K_HOSTS_PER_FABRIC).Traverse(|h_u| {
            let h = h_u as usize;
            hL0TxValid[h] = self._Hosts[h].l0_tx_valid;
            hL0TxData[h] = self._Hosts[h].l0_tx_data;
            hL0RxReady[h] = self._Hosts[h].l0_rx_ready;
            hL1TxValid[h] = self._Hosts[h].l1_tx_valid;
            hL1TxData[h] = self._Hosts[h].l1_tx_data;
            hL1RxReady[h] = self._Hosts[h].l1_rx_ready;
        });

        // 3. Step Hosts
        for h in 0..K_HOSTS_PER_HIND {
            let l0_tx_ready = d0_kl_rx_ready[h];
            let l0_rx_valid = d0_kl_tx_valid[h];
            let l0_rx_data = d0_kl_tx_data[h];
            let l1_tx_ready = d1_kl_rx_ready[K_HOSTS_PER_HIND + h];
            let l1_rx_valid = d1_kl_tx_valid[K_HOSTS_PER_HIND + h];
            let l1_rx_data = d1_kl_tx_data[K_HOSTS_PER_HIND + h];
            self._hosts[h].step(
                l0_tx_ready,
                l0_rx_valid,
                l0_rx_data,
                l1_tx_ready,
                l1_rx_valid,
                l1_rx_data,
        USeg::FromLen(K_HOSTS_PER_HIND as u32).Traverse(|h_u| {
            let h = h_u as usize;
            let l0TxReady = d0KlRxReady[h];
            let l0RxValid = d0KlTxValid[h];
            let l0RxData = d0KlTxData[h];
            let l1TxReady = d1KlRxReady[K_HOSTS_PER_HIND + h];
            let l1RxValid = d1KlTxValid[K_HOSTS_PER_HIND + h];
            let l1RxData = d1KlTxData[K_HOSTS_PER_HIND + h];
            self._Hosts[h].Step(
                l0TxReady,
                l0RxValid,
                l0RxData,
                l1TxReady,
                l1RxValid,
                l1RxData,
            );
        }
        for h in K_HOSTS_PER_HIND..K_HOSTS_PER_FABRIC as usize {
            let l0_tx_ready = d1_kl_rx_ready[h - K_HOSTS_PER_HIND];
            let l0_rx_valid = d1_kl_tx_valid[h - K_HOSTS_PER_HIND];
            let l0_rx_data = d1_kl_tx_data[h - K_HOSTS_PER_HIND];
            let l1_tx_ready = d0_kl_rx_ready[h];
            let l1_rx_valid = d0_kl_tx_valid[h];
            let l1_rx_data = d0_kl_tx_data[h];
            self._hosts[h].step(
                l0_tx_ready,
                l0_rx_valid,
                l0_rx_data,
                l1_tx_ready,
                l1_rx_valid,
                l1_rx_data,
        });

        USeg::FromLen(K_HOSTS_PER_HIND as u32).Traverse(|idx| {
            let h = (K_HOSTS_PER_HIND as u32 + idx) as usize;
            let l0TxReady = d1KlRxReady[h - K_HOSTS_PER_HIND];
            let l0RxValid = d1KlTxValid[h - K_HOSTS_PER_HIND];
            let l0RxData = d1KlTxData[h - K_HOSTS_PER_HIND];
            let l1TxReady = d0KlRxReady[h];
            let l1RxValid = d0KlTxValid[h];
            let l1RxData = d0KlTxData[h];
            self._Hosts[h].Step(
                l0TxReady,
                l0RxValid,
                l0RxData,
                l1TxReady,
                l1RxValid,
                l1RxData,
            );
        }
        });

        // 4. Form inputs for Die 0 and Die 1
        let mut d0_in_valid = [false; K_KL_PORTS_PER_HIND];
        let mut d0_in_data = [0u64; K_KL_PORTS_PER_HIND];
        let mut d0_in_ready = [false; K_KL_PORTS_PER_HIND];
        let mut d0InValid = [false; K_KL_PORTS_PER_HIND];
        let mut d0InData = [0u64; K_KL_PORTS_PER_HIND];
        let mut d0InReady = [false; K_KL_PORTS_PER_HIND];
        // Ports 0..3: Hosts 0..3 Link0
        d0_in_valid[..K_HOSTS_PER_HIND].copy_from_slice(&h_l0_tx_valid[..K_HOSTS_PER_HIND]);
        d0_in_data[..K_HOSTS_PER_HIND].copy_from_slice(&h_l0_tx_data[..K_HOSTS_PER_HIND]);
        d0_in_ready[..K_HOSTS_PER_HIND].copy_from_slice(&h_l0_rx_ready[..K_HOSTS_PER_HIND]);
        d0InValid[..K_HOSTS_PER_HIND].copy_from_slice(&hL0TxValid[..K_HOSTS_PER_HIND]);
        d0InData[..K_HOSTS_PER_HIND].copy_from_slice(&hL0TxData[..K_HOSTS_PER_HIND]);
        d0InReady[..K_HOSTS_PER_HIND].copy_from_slice(&hL0RxReady[..K_HOSTS_PER_HIND]);
        // Ports 4..7: Hosts 4..7 Link1
        d0_in_valid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_tx_valid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d0_in_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_tx_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d0_in_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_rx_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d0InValid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&hL1TxValid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d0InData[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&hL1TxData[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d0InReady[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&hL1RxReady[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        // Ports 8..9: Inter-die links from Die 1
        d0_in_valid[K_INTERDIE_PORT_BASE] = d1_kl_tx_valid[K_INTERDIE_PORT_BASE];
        d0_in_data[K_INTERDIE_PORT_BASE] = d1_kl_tx_data[K_INTERDIE_PORT_BASE];
        d0_in_ready[K_INTERDIE_PORT_BASE] = d1_kl_rx_ready[K_INTERDIE_PORT_BASE];
        d0_in_valid[K_INTERDIE_PORT_BASE + 1] = d1_kl_tx_valid[K_INTERDIE_PORT_BASE + 1];
        d0_in_data[K_INTERDIE_PORT_BASE + 1] = d1_kl_tx_data[K_INTERDIE_PORT_BASE + 1];
        d0_in_ready[K_INTERDIE_PORT_BASE + 1] = d1_kl_rx_ready[K_INTERDIE_PORT_BASE + 1];
        let mut d1_in_valid = [false; K_KL_PORTS_PER_HIND];
        let mut d1_in_data = [0u64; K_KL_PORTS_PER_HIND];
        let mut d1_in_ready = [false; K_KL_PORTS_PER_HIND];
        d0InValid[K_INTERDIE_PORT_BASE] = d1KlTxValid[K_INTERDIE_PORT_BASE];
        d0InData[K_INTERDIE_PORT_BASE] = d1KlTxData[K_INTERDIE_PORT_BASE];
        d0InReady[K_INTERDIE_PORT_BASE] = d1KlRxReady[K_INTERDIE_PORT_BASE];
        d0InValid[K_INTERDIE_PORT_BASE + 1] = d1KlTxValid[K_INTERDIE_PORT_BASE + 1];
        d0InData[K_INTERDIE_PORT_BASE + 1] = d1KlTxData[K_INTERDIE_PORT_BASE + 1];
        d0InReady[K_INTERDIE_PORT_BASE + 1] = d1KlRxReady[K_INTERDIE_PORT_BASE + 1];

        let mut d1InValid = [false; K_KL_PORTS_PER_HIND];
        let mut d1InData = [0u64; K_KL_PORTS_PER_HIND];
        let mut d1InReady = [false; K_KL_PORTS_PER_HIND];
        // Ports 0..3: Hosts 4..7 Link0
        d1_in_valid[..K_HOSTS_PER_HIND]
            .copy_from_slice(&h_l0_tx_valid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d1_in_data[..K_HOSTS_PER_HIND]
            .copy_from_slice(&h_l0_tx_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d1_in_ready[..K_HOSTS_PER_HIND]
            .copy_from_slice(&h_l0_rx_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d1InValid[..K_HOSTS_PER_HIND]
            .copy_from_slice(&hL0TxValid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d1InData[..K_HOSTS_PER_HIND]
            .copy_from_slice(&hL0TxData[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d1InReady[..K_HOSTS_PER_HIND]
            .copy_from_slice(&hL0RxReady[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        // Ports 4..7: Hosts 0..3 Link1
        d1_in_valid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_tx_valid[..K_HOSTS_PER_HIND]);
        d1_in_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_tx_data[..K_HOSTS_PER_HIND]);
        d1_in_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_rx_ready[..K_HOSTS_PER_HIND]);
        d1InValid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&hL1TxValid[..K_HOSTS_PER_HIND]);
        d1InData[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&hL1TxData[..K_HOSTS_PER_HIND]);
        d1InReady[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&hL1RxReady[..K_HOSTS_PER_HIND]);
        // Ports 8..9: Inter-die links from Die 0
        d1_in_valid[K_INTERDIE_PORT_BASE] = d0_kl_tx_valid[K_INTERDIE_PORT_BASE];
        d1_in_data[K_INTERDIE_PORT_BASE] = d0_kl_tx_data[K_INTERDIE_PORT_BASE];
        d1_in_ready[K_INTERDIE_PORT_BASE] = d0_kl_rx_ready[K_INTERDIE_PORT_BASE];
        d1_in_valid[K_INTERDIE_PORT_BASE + 1] = d0_kl_tx_valid[K_INTERDIE_PORT_BASE + 1];
        d1_in_data[K_INTERDIE_PORT_BASE + 1] = d0_kl_tx_data[K_INTERDIE_PORT_BASE + 1];
        d1_in_ready[K_INTERDIE_PORT_BASE + 1] = d0_kl_rx_ready[K_INTERDIE_PORT_BASE + 1];
        d1InValid[K_INTERDIE_PORT_BASE] = d0KlTxValid[K_INTERDIE_PORT_BASE];
        d1InData[K_INTERDIE_PORT_BASE] = d0KlTxData[K_INTERDIE_PORT_BASE];
        d1InReady[K_INTERDIE_PORT_BASE] = d0KlRxReady[K_INTERDIE_PORT_BASE];
        d1InValid[K_INTERDIE_PORT_BASE + 1] = d0KlTxValid[K_INTERDIE_PORT_BASE + 1];
        d1InData[K_INTERDIE_PORT_BASE + 1] = d0KlTxData[K_INTERDIE_PORT_BASE + 1];
        d1InReady[K_INTERDIE_PORT_BASE + 1] = d0KlRxReady[K_INTERDIE_PORT_BASE + 1];

        (
            DieInputSignals {
                valid: d0_in_valid,
                data: d0_in_data,
                ready: d0_in_ready,
                valid: d0InValid,
                data: d0InData,
                ready: d0InReady,
            },
            DieInputSignals {
                valid: d1_in_valid,
                data: d1_in_data,
                ready: d1_in_ready,
                valid: d1InValid,
                data: d1InData,
                ready: d1InReady,
            },
        )
    }
    pub fn step_cycle(&mut self) {
        let (d0, d1) = self.prepare_cycle_inputs();
        self._fabrics[0].step(&d0.valid, &d0.data, &d0.ready);
        self._fabrics[1].step(&d1.valid, &d1.data, &d1.ready);
        self._cycle_count += 1;

    pub fn StepCycle(&mut self)
    {
        let (d0, d1) = self.PrepareCycleInputs();
        self._Fabrics[0].Step(&d0.valid, &d0.data, &d0.ready);
        self._Fabrics[1].Step(&d1.valid, &d1.data, &d1.ready);
        self._CycleCount += 1;
    }
    pub fn advance(&mut self, ticks: u32) -> u64 {
        for _ in 0..ticks {
            self.step_cycle();
        }
        self._cycle_count

    #[inline]
    pub fn step_cycle(&mut self)
    {
        self.StepCycle();
    }

    pub fn Advance(&mut self, ticks: u32) -> u64
    {
        USeg::FromLen(ticks).Traverse(|_| {
            self.StepCycle();
        });
        self._CycleCount
    }

    #[inline]
    pub fn Advance(&mut self, ticks: u32) -> u64 {
        self.advance(ticks)
    pub fn advance(&mut self, ticks: u32) -> u64
    {
        self.Advance(ticks)
    }

    #[inline]
    pub fn RunCycles(&mut self, cycles: u32) -> u64
    {
        self.Advance(cycles)
    }

    #[inline]
    pub fn run_cycles(&mut self, cycles: u32) -> u64
    {
        self.Advance(cycles)
    }
}
