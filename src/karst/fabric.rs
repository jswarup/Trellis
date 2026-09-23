// src/karst/fabric.rs
use crate::heist::Atelier;
use crate::karst::config::{
    K_HIND_DIES_PER_FABRIC, K_HOSTS_PER_FABRIC, K_HOSTS_PER_HIND, K_INTERDIE_PORT_BASE,
    K_KL_PORTS_PER_HIND, K_MC_PORTS_PER_HIND, K_MEM_CHANS_PER_FABRIC,
};
use crate::karst::fabric_node::KarstFabricNode;
use crate::karst::host_node::{HostResponse, KarstHostNode};
use crate::karst::memchan::MemChan;
use crate::karst::noc::KarstNocQueueDepths;
use crate::karst::vpu::Vpu;
use crate::silo::{Arr, Buff, MutArr, USeg};
use crate::swarm::cpu::ComputeDevice;

pub const K_CYCLE_TRACE_CAPACITY: u32 = 1024;

//-------------------------------------------------------------------------------------------------
// Aggregate fabric metrics across all host ports, memory channels, and VPUs.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct KarstStats
{
    pub _TotalTxCount:       u32,
    pub _TotalRxCount:       u32,
    pub _TotalBytesWritten:  u64,
    pub _TotalBytesRead:     u64,
    pub _TotalVPUDispatches: u32,
    pub _CycleCount:         u64,
}

//-------------------------------------------------------------------------------------------------
// Simulation engine representation for inspection and diagnostics.
#[derive(Copy, Clone, Debug, Default)]
pub struct KarstEngineInfo
{
    pub _CycleCount: u64,
}
#[derive(Copy, Clone, Debug, Default)]
pub struct DieInputSignals
{
    pub valid:         [bool; K_KL_PORTS_PER_HIND],
    pub data:          [u64; K_KL_PORTS_PER_HIND],
    pub ready:         [bool; K_KL_PORTS_PER_HIND],
    pub ingress_ready: [bool; K_KL_PORTS_PER_HIND],
    pub egress_valid:  [bool; K_KL_PORTS_PER_HIND],
}
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VpuDispatchDesc
{
    pub _vpu_idx:  u32,
    pub _chan_idx: u32,
    pub _dim:      crate::swarm::traits::WorkgroupDim,
}

//-------------------------------------------------------------------------------------------------
// Bounded serial reference record for validating future parallel cycle policies.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct KarstCycleTrace
{
    pub _Cycle:        u64,
    pub _IngressValid: [u16; K_HIND_DIES_PER_FABRIC as usize],
    pub _IngressReady: [u16; K_HIND_DIES_PER_FABRIC as usize],
    pub _IngressData:  [[u64; K_KL_PORTS_PER_HIND]; K_HIND_DIES_PER_FABRIC as usize],
    pub _EgressValid:  [u16; K_HIND_DIES_PER_FABRIC as usize],
    pub _EgressReady:  [u16; K_HIND_DIES_PER_FABRIC as usize],
    pub _EgressData:   [[u64; K_KL_PORTS_PER_HIND]; K_HIND_DIES_PER_FABRIC as usize],
    pub _HostTxCount:  u32,
    pub _HostRxCount:  u32,
}

//-------------------------------------------------------------------------------------------------
// Per-link handshake counters retained across the fabric lifetime.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct KarstLinkStats
{
    pub _IngressAccepted: [[u64; K_KL_PORTS_PER_HIND]; K_HIND_DIES_PER_FABRIC as usize],
    pub _IngressStalled:  [[u64; K_KL_PORTS_PER_HIND]; K_HIND_DIES_PER_FABRIC as usize],
    pub _EgressAccepted:  [[u64; K_KL_PORTS_PER_HIND]; K_HIND_DIES_PER_FABRIC as usize],
    pub _EgressStalled:   [[u64; K_KL_PORTS_PER_HIND]; K_HIND_DIES_PER_FABRIC as usize],
}

//-------------------------------------------------------------------------------------------------
// Maximum observed queue occupancy per die and NoC queue class.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct KarstQueueStats
{
    pub _KlIngressHighWater:  [[u32; K_KL_PORTS_PER_HIND]; K_HIND_DIES_PER_FABRIC as usize],
    pub _KlEgressHighWater:   [[u32; K_KL_PORTS_PER_HIND]; K_HIND_DIES_PER_FABRIC as usize],
    pub _McRequestHighWater:  [[u32; K_MC_PORTS_PER_HIND]; K_HIND_DIES_PER_FABRIC as usize],
    pub _McResponseHighWater: [[u32; K_MC_PORTS_PER_HIND]; K_HIND_DIES_PER_FABRIC as usize],
}

fn SignalMask(signals: &[bool; K_KL_PORTS_PER_HIND]) -> u16
{
    let mut mask = 0u16;
    USeg::FromLen(K_KL_PORTS_PER_HIND as u32).Traverse(|port| {
                                                 if signals[port as usize] {
                                                     mask |= 1 << port;
                                                 }
                                             });
    mask
}

//-------------------------------------------------------------------------------------------------
// KarstFabric — top-level system framework orchestrating the balanced Karst(8, 8) topology.
// Wires 8 KarstFore host nodes with 2 KarstHind Memory Fabric dies over KarstLinks.
pub struct KarstFabric
{
    _compute_device:         ComputeDevice,
    _hosts:                  [KarstHostNode; K_HOSTS_PER_FABRIC as usize],
    _fabrics:                [KarstFabricNode; K_HIND_DIES_PER_FABRIC as usize],
    _cycle_count:            u64,
    _workers:                u32,
    _cycle_trace:            Buff<KarstCycleTrace>,
    _cycle_trace_len:        u32,
    _cycle_trace_enabled:    bool,
    _cycle_trace_overflowed: bool,
    _link_stats:             KarstLinkStats,
    _queue_stats:            KarstQueueStats,
    _parallel_dies:          bool,
}
impl Default for KarstFabric
{
    fn default() -> Self { Self::new() }
}
impl KarstFabric
{
    pub fn new() -> Self { Self::with_workers(1) }
    pub fn with_workers(workers: u32) -> Self
    {
        let compute_device = ComputeDevice::WithWorkers(workers);
        let hosts = [KarstHostNode::new(0),
                     KarstHostNode::new(1),
                     KarstHostNode::new(2),
                     KarstHostNode::new(3),
                     KarstHostNode::new(4),
                     KarstHostNode::new(5),
                     KarstHostNode::new(6),
                     KarstHostNode::new(7)];
        let fabrics = [KarstFabricNode::new(&compute_device, 0),
                       KarstFabricNode::new(&compute_device, 1)];
        Self { _compute_device:         compute_device,
               _hosts:                  hosts,
               _fabrics:                fabrics,
               _cycle_count:            0,
               _workers:                workers.max(1),
               _cycle_trace:            Buff::FromDispenser(K_CYCLE_TRACE_CAPACITY, |_| {
                   KarstCycleTrace::default()
               }),
               _cycle_trace_len:        0,
               _cycle_trace_enabled:    false,
               _cycle_trace_overflowed: false,
               _link_stats:             KarstLinkStats::default(),
               _queue_stats:            KarstQueueStats::default(),
               _parallel_dies:          workers >= 2, }
    }
    #[inline]
    pub fn cycle_count(&self) -> u64 { self._cycle_count }
    #[inline]
    pub fn workers(&self) -> u32 { self._workers }
    pub fn EnableParallelDies(&mut self, enabled: bool) { self._parallel_dies = enabled; }
    #[inline]
    pub fn ParallelDiesEnabled(&self) -> bool { self._parallel_dies }
    pub fn EnableCycleTrace(&mut self, enabled: bool) { self._cycle_trace_enabled = enabled; }
    pub fn ClearCycleTrace(&mut self)
    {
        self._cycle_trace_len = 0;
        self._cycle_trace_overflowed = false;
    }
    #[inline]
    pub fn CycleTrace(&self) -> Arr<'_, KarstCycleTrace>
    {
        self._cycle_trace.Arr().Slice(0, self._cycle_trace_len)
    }
    #[inline]
    pub fn CycleTraceOverflowed(&self) -> bool { self._cycle_trace_overflowed }
    #[inline]
    pub fn LinkStats(&self) -> KarstLinkStats { self._link_stats }
    #[inline]
    pub fn QueueStats(&self) -> KarstQueueStats { self._queue_stats }
    #[inline]
    pub fn compute_device(&self) -> &ComputeDevice { &self._compute_device }
    #[inline]
    pub fn compute_device_mut(&mut self) -> &mut ComputeDevice { &mut self._compute_device }
    #[inline]
    pub fn engine(&self) -> KarstEngineInfo { KarstEngineInfo { _CycleCount: self._cycle_count, } }
    #[inline]
    pub fn Engine(&self) -> KarstEngineInfo { self.engine() }
    #[inline]
    pub fn host(&self, id: u32) -> &KarstHostNode { &self._hosts[id as usize] }
    #[inline]
    pub fn host_mut(&mut self, id: u32) -> &mut KarstHostNode { &mut self._hosts[id as usize] }
    #[inline]
    pub fn Host(&self, id: u32) -> &KarstHostNode { self.host(id) }
    #[inline]
    pub fn HostMut(&mut self, id: u32) -> &mut KarstHostNode { self.host_mut(id) }
    #[inline]
    pub fn fabric_node(&self, die_id: u32) -> &KarstFabricNode { &self._fabrics[die_id as usize] }
    #[inline]
    pub fn fabric_node_mut(&mut self, die_id: u32) -> &mut KarstFabricNode
    {
        &mut self._fabrics[die_id as usize]
    }
    #[inline]
    pub fn FabricNode(&self, die_id: u32) -> &KarstFabricNode { self.fabric_node(die_id) }
    #[inline]
    pub fn FabricNodeMut(&mut self, die_id: u32) -> &mut KarstFabricNode
    {
        self.fabric_node_mut(die_id)
    }
    #[inline]
    pub fn mem_chan(&self, global_idx: u32) -> &MemChan
    {
        let die_id = global_idx / 4;
        let mc_idx = global_idx % 4;
        self._fabrics[die_id as usize].mem_chan(mc_idx as usize)
    }
    #[inline]
    pub fn mem_chan_mut(&mut self, global_idx: u32) -> &mut MemChan
    {
        let die_id = global_idx / 4;
        let mc_idx = global_idx % 4;
        self._fabrics[die_id as usize].mem_chan_mut(mc_idx as usize)
    }
    #[inline]
    pub fn MemChan(&self, global_idx: u32) -> &MemChan { self.mem_chan(global_idx) }
    #[inline]
    pub fn MemChanMut(&mut self, global_idx: u32) -> &mut MemChan { self.mem_chan_mut(global_idx) }
    // Trellis alias DChan -> MemChan
    #[inline]
    pub fn DChan(&self, global_idx: u32) -> &MemChan { self.mem_chan(global_idx) }
    #[inline]
    pub fn DChanMut(&mut self, global_idx: u32) -> &mut MemChan { self.mem_chan_mut(global_idx) }
    #[inline]
    pub fn vpu(&self, global_idx: u32) -> &Vpu
    {
        let die_id = global_idx / 4;
        let mc_idx = global_idx % 4;
        self._fabrics[die_id as usize].vpu(mc_idx as usize)
    }
    #[inline]
    pub fn vpu_mut(&mut self, global_idx: u32) -> &mut Vpu
    {
        let die_id = global_idx / 4;
        let mc_idx = global_idx % 4;
        self._fabrics[die_id as usize].vpu_mut(mc_idx as usize)
    }
    #[inline]
    pub fn VPU(&self, global_idx: u32) -> &Vpu { self.vpu(global_idx) }
    #[inline]
    pub fn VPUMut(&mut self, global_idx: u32) -> &mut Vpu { self.vpu_mut(global_idx) }
    #[inline]
    pub fn VPU_mut(&mut self, global_idx: u32) -> &mut Vpu { self.vpu_mut(global_idx) }
    #[inline]
    pub fn MemChan_mut(&mut self, global_idx: u32) -> &mut MemChan { self.mem_chan_mut(global_idx) }
    pub fn dispatch_vpu(&self, vpu_idx: u32, chan_idx: u32,
                        dim: crate::swarm::traits::WorkgroupDim)
                        -> Result<(), crate::swarm::traits::SwarmError>
    {
        let vpu = self.vpu(vpu_idx);
        let chan = self.mem_chan(chan_idx);
        vpu.dispatch(&self._compute_device, chan, dim)
    }
    pub fn dispatch_vpu_batch(&mut self, dispatches: &[VpuDispatchDesc])
                              -> Result<(), crate::swarm::traits::SwarmError>
    {
        if dispatches.is_empty() {
            return Ok(());
        }
        for desc in dispatches {
            if desc._vpu_idx >= K_MEM_CHANS_PER_FABRIC || desc._chan_idx >= K_MEM_CHANS_PER_FABRIC {
                return Err(crate::swarm::traits::SwarmError::ExecutionError("Invalid VPU or channel index"));
            }
        }

        let mut chan_ops: [Vec<VpuDispatchDesc>; K_MEM_CHANS_PER_FABRIC as usize] =
            Default::default();
        for &desc in dispatches {
            chan_ops[desc._chan_idx as usize].push(desc);
        }

        let workers = self._workers.max(1);
        let atelier = Atelier::Reset(workers);
        let fabric_ptr = self as *mut KarstFabric as usize;

        let mut root_node: Option<crate::heist::choretree::ChoreNode> = None;

        for (chan_idx, ops) in chan_ops.iter().enumerate() {
            if ops.is_empty() {
                continue;
            }
            let target_worker = ((chan_idx as u32) / 4) % workers;
            let mut chan_chain: Option<crate::heist::choretree::ChoreNode> = None;

            for &desc in ops {
                let device = ComputeDevice::WithWorkers(1);
                let chore = crate::heist::choretree::Chore::FromClosure("VpuDispatch", move |_w| {
                                let f = unsafe { &*(fabric_ptr as *const KarstFabric) };
                                let vpu = f.vpu(desc._vpu_idx);
                                let chan = f.mem_chan(desc._chan_idx);
                                let _ = vpu.dispatch(&device, chan, desc._dim);
                            }).Require(target_worker);

                if let Some(chain) = chan_chain {
                    chan_chain = Some(chain >> chore);
                } else {
                    chan_chain = Some(chore.into());
                }
            }

            if let Some(chain) = chan_chain {
                if let Some(root) = root_node {
                    root_node = Some(root | chain);
                } else {
                    root_node = Some(chain);
                }
            }
        }

        if let Some(node) = root_node {
            atelier.MainMaestro().PostChoreTree(node);
            atelier.DoLaunch();
        }

        Ok(())
    }
    pub fn try_post_host_write(&mut self, host_id: u32, addr: u32, data: u32)
                               -> Result<(), &'static str>
    {
        if host_id < K_HOSTS_PER_FABRIC {
            self._hosts[host_id as usize].post_write(addr, data)
        } else {
            Err("Invalid host id")
        }
    }
    pub fn post_host_write(&mut self, host_id: u32, addr: u32, data: u32)
    {
        self.try_post_host_write(host_id, addr, data)
            .expect("Host write rejected");
    }
    #[inline]
    pub fn PostHostWrite(&mut self, host_id: u32, addr: u32, data: u32)
    {
        self.post_host_write(host_id, addr, data);
    }
    pub fn try_post_host_read(&mut self, host_id: u32, addr: u32) -> Result<(), &'static str>
    {
        if host_id < K_HOSTS_PER_FABRIC {
            self._hosts[host_id as usize].post_read(addr)
        } else {
            Err("Invalid host id")
        }
    }
    pub fn post_host_read(&mut self, host_id: u32, addr: u32)
    {
        self.try_post_host_read(host_id, addr)
            .expect("Host read rejected");
    }
    #[inline]
    pub fn PostHostRead(&mut self, host_id: u32, addr: u32) { self.post_host_read(host_id, addr); }
    pub fn pop_host_response(&mut self, host_id: u32, out: &mut HostResponse) -> bool
    {
        if host_id < K_HOSTS_PER_FABRIC {
            self._hosts[host_id as usize].pop_response(out)
        } else {
            false
        }
    }
    #[inline]
    pub fn PopHostResponse(&mut self, host_id: u32, out: &mut HostResponse) -> bool
    {
        self.pop_host_response(host_id, out)
    }
    pub fn stats(&self) -> KarstStats
    {
        let mut total = KarstStats { _TotalTxCount:       0,
                                     _TotalRxCount:       0,
                                     _TotalBytesWritten:  0,
                                     _TotalBytesRead:     0,
                                     _TotalVPUDispatches: 0,
                                     _CycleCount:         self._cycle_count, };
        for h in 0..K_HOSTS_PER_FABRIC {
            let hs = self._hosts[h as usize].stats();
            total._TotalTxCount += hs._TxCount;
            total._TotalRxCount += hs._RxCount;
        }
        for c in 0..K_MEM_CHANS_PER_FABRIC {
            let ds = self.mem_chan(c).stats();
            total._TotalBytesWritten += ds._BytesWritten;
            total._TotalBytesRead += ds._BytesRead;
        }
        for v in 0..K_MEM_CHANS_PER_FABRIC {
            total._TotalVPUDispatches += self.vpu(v).dispatches();
        }
        total
    }
    #[inline]
    pub fn Stats(&self) -> KarstStats { self.stats() }
    fn prepare_cycle_inputs(&mut self) -> (DieInputSignals, DieInputSignals)
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
        // 3. Step Hosts
        for h in 0..K_HOSTS_PER_HIND {
            let l0_tx_ready = d0_kl_rx_ready[h];
            let l0_rx_valid = d0_kl_tx_valid[h];
            let l0_rx_data = d0_kl_tx_data[h];
            let l1_tx_ready = d1_kl_rx_ready[K_HOSTS_PER_HIND + h];
            let l1_rx_valid = d1_kl_tx_valid[K_HOSTS_PER_HIND + h];
            let l1_rx_data = d1_kl_tx_data[K_HOSTS_PER_HIND + h];
            self._hosts[h].step(l0_tx_ready,
                                l0_rx_valid,
                                l0_rx_data,
                                l1_tx_ready,
                                l1_rx_valid,
                                l1_rx_data);
        }
        for h in K_HOSTS_PER_HIND..K_HOSTS_PER_FABRIC as usize {
            let l0_tx_ready = d1_kl_rx_ready[h - K_HOSTS_PER_HIND];
            let l0_rx_valid = d1_kl_tx_valid[h - K_HOSTS_PER_HIND];
            let l0_rx_data = d1_kl_tx_data[h - K_HOSTS_PER_HIND];
            let l1_tx_ready = d0_kl_rx_ready[h];
            let l1_rx_valid = d0_kl_tx_valid[h];
            let l1_rx_data = d0_kl_tx_data[h];
            self._hosts[h].step(l0_tx_ready,
                                l0_rx_valid,
                                l0_rx_data,
                                l1_tx_ready,
                                l1_rx_valid,
                                l1_rx_data);
        }
        // 4. Form inputs for Die 0 and Die 1
        let mut d0_in_valid = [false; K_KL_PORTS_PER_HIND];
        let mut d0_in_data = [0u64; K_KL_PORTS_PER_HIND];
        let mut d0_in_ready = [false; K_KL_PORTS_PER_HIND];
        // Ports 0..3: Hosts 0..3 Link0
        d0_in_valid[..K_HOSTS_PER_HIND].copy_from_slice(&h_l0_tx_valid[..K_HOSTS_PER_HIND]);
        d0_in_data[..K_HOSTS_PER_HIND].copy_from_slice(&h_l0_tx_data[..K_HOSTS_PER_HIND]);
        d0_in_ready[..K_HOSTS_PER_HIND].copy_from_slice(&h_l0_rx_ready[..K_HOSTS_PER_HIND]);
        // Ports 4..7: Hosts 4..7 Link1
        d0_in_valid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice( &h_l1_tx_valid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d0_in_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice( &h_l1_tx_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d0_in_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice( &h_l1_rx_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
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
        // Ports 0..3: Hosts 4..7 Link0
        d1_in_valid[..K_HOSTS_PER_HIND].copy_from_slice(&h_l0_tx_valid[K_HOSTS_PER_HIND
                                                                       ..K_INTERDIE_PORT_BASE]);
        d1_in_data[..K_HOSTS_PER_HIND].copy_from_slice(&h_l0_tx_data[K_HOSTS_PER_HIND
                                                                     ..K_INTERDIE_PORT_BASE]);
        d1_in_ready[..K_HOSTS_PER_HIND].copy_from_slice(&h_l0_rx_ready[K_HOSTS_PER_HIND
                                                                       ..K_INTERDIE_PORT_BASE]);
        // Ports 4..7: Hosts 0..3 Link1
        d1_in_valid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice( &h_l1_tx_valid[..K_HOSTS_PER_HIND]);
        d1_in_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice( &h_l1_tx_data[..K_HOSTS_PER_HIND]);
        d1_in_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice( &h_l1_rx_ready[..K_HOSTS_PER_HIND]);
        // Ports 8..9: Inter-die links from Die 0
        d1_in_valid[K_INTERDIE_PORT_BASE] = d0_kl_tx_valid[K_INTERDIE_PORT_BASE];
        d1_in_data[K_INTERDIE_PORT_BASE] = d0_kl_tx_data[K_INTERDIE_PORT_BASE];
        d1_in_ready[K_INTERDIE_PORT_BASE] = d0_kl_rx_ready[K_INTERDIE_PORT_BASE];
        d1_in_valid[K_INTERDIE_PORT_BASE + 1] = d0_kl_tx_valid[K_INTERDIE_PORT_BASE + 1];
        d1_in_data[K_INTERDIE_PORT_BASE + 1] = d0_kl_tx_data[K_INTERDIE_PORT_BASE + 1];
        d1_in_ready[K_INTERDIE_PORT_BASE + 1] = d0_kl_rx_ready[K_INTERDIE_PORT_BASE + 1];
        (DieInputSignals { valid:         d0_in_valid,
                           data:          d0_in_data,
                           ready:         d0_in_ready,
                           ingress_ready: d0_kl_rx_ready,
                           egress_valid:  d0_kl_tx_valid, },
         DieInputSignals { valid:         d1_in_valid,
                           data:          d1_in_data,
                           ready:         d1_in_ready,
                           ingress_ready: d1_kl_rx_ready,
                           egress_valid:  d1_kl_tx_valid, })
    }
    fn record_link_stats(&mut self, d0: &DieInputSignals, d1: &DieInputSignals)
    {
        let signals = [d0, d1];
        USeg::FromLen(K_HIND_DIES_PER_FABRIC).Traverse(|die| {
            let signal = signals[die as usize];
            USeg::FromLen(K_KL_PORTS_PER_HIND as u32).Traverse(|port| {
                                                         let port = port as usize;
                                                         if signal.valid[port] {
                                                             if signal.ingress_ready[port] {
                                                                 self._link_stats
                                                                     ._IngressAccepted
                                                                     [die as usize][port] += 1;
                                                             } else {
                                                                 self._link_stats
                                                                     ._IngressStalled
                                                                     [die as usize][port] += 1;
                                                             }
                                                         }
                                                         if signal.egress_valid[port] {
                                                             if signal.ready[port] {
                                                                 self._link_stats
                                                                     ._EgressAccepted
                                                                     [die as usize][port] += 1;
                                                             } else {
                                                                 self._link_stats._EgressStalled
                                                                     [die as usize][port] += 1;
                                                             }
                                                         }
                                                     });
        });
    }
    fn record_queue_high_water(&mut self)
    {
        let depths: [KarstNocQueueDepths; K_HIND_DIES_PER_FABRIC as usize] =
            [self._fabrics[0].noc().QueueDepths(),
             self._fabrics[1].noc().QueueDepths()];
        USeg::FromLen(K_HIND_DIES_PER_FABRIC).Traverse(|die| {
            let die = die as usize;
            USeg::FromLen(K_KL_PORTS_PER_HIND as u32).Traverse(|port| {
                let port = port as usize;
                self._queue_stats._KlIngressHighWater[die][port] =
                    self._queue_stats._KlIngressHighWater[die][port].max(depths[die]._KlIngress
                                                                             [port]);
                self._queue_stats._KlEgressHighWater[die][port] =
                    self._queue_stats._KlEgressHighWater[die][port].max(depths[die]._KlEgress
                                                                            [port]);
            });
            USeg::FromLen(K_MC_PORTS_PER_HIND as u32).Traverse(|mc| {
                let mc = mc as usize;
                self._queue_stats._McRequestHighWater[die][mc] =
                    self._queue_stats._McRequestHighWater[die][mc].max(depths[die]._McRequest[mc]);
                self._queue_stats._McResponseHighWater[die][mc] =
                    self._queue_stats._McResponseHighWater[die][mc].max(depths[die]._McResponse
                                                                            [mc]);
            });
        });
    }
    fn capture_cycle_trace(&mut self, d0: &DieInputSignals, d1: &DieInputSignals)
    {
        if !self._cycle_trace_enabled {
            return;
        }
        if self._cycle_trace_len >= K_CYCLE_TRACE_CAPACITY {
            self._cycle_trace_overflowed = true;
            return;
        }
        let d0_noc = self._fabrics[0].noc();
        let d1_noc = self._fabrics[1].noc();
        let mut d0_egress_valid = [false; K_KL_PORTS_PER_HIND];
        let mut d0_egress_data = [0u64; K_KL_PORTS_PER_HIND];
        let mut d1_egress_valid = [false; K_KL_PORTS_PER_HIND];
        let mut d1_egress_data = [0u64; K_KL_PORTS_PER_HIND];
        for port in 0..K_KL_PORTS_PER_HIND {
            d0_egress_valid[port] = d0_noc.kl_tx_valid(port);
            d0_egress_data[port] = d0_noc.kl_tx_data(port);
            d1_egress_valid[port] = d1_noc.kl_tx_valid(port);
            d1_egress_data[port] = d1_noc.kl_tx_data(port);
        }
        let stats = self.stats();
        self._cycle_trace[self._cycle_trace_len] =
            KarstCycleTrace { _Cycle:        self._cycle_count,
                              _IngressValid: [SignalMask(&d0.valid), SignalMask(&d1.valid)],
                              _IngressReady: [SignalMask(&d0.ingress_ready),
                                              SignalMask(&d1.ingress_ready)],
                              _IngressData:  [d0.data, d1.data],
                              _EgressValid:  [SignalMask(&d0_egress_valid),
                                              SignalMask(&d1_egress_valid)],
                              _EgressReady:  [SignalMask(&d0.ready), SignalMask(&d1.ready)],
                              _EgressData:   [d0_egress_data, d1_egress_data],
                              _HostTxCount:  stats._TotalTxCount,
                              _HostRxCount:  stats._TotalRxCount, };
        self._cycle_trace_len += 1;
    }
    pub fn step_cycle(&mut self)
    {
        let (d0, d1) = self.prepare_cycle_inputs();
        self.record_link_stats(&d0, &d1);

        if !self._parallel_dies || self._workers < 2 {
            self._fabrics[0].step(&d0.valid, &d0.data, &d0.ready);
            self._fabrics[1].step(&d1.valid, &d1.data, &d1.ready);
        } else {
            self.step_dies_parallel(&d0, &d1);
        }

        self.record_queue_high_water();
        self.capture_cycle_trace(&d0, &d1);
        self._cycle_count += 1;
    }
    fn step_dies_parallel(&mut self, d0: &DieInputSignals, d1: &DieInputSignals)
    {
        let atelier = Atelier::Reset(self._workers);
        let ptr0 = &mut self._fabrics[0] as *mut KarstFabricNode as usize;
        let ptr1 = &mut self._fabrics[1] as *mut KarstFabricNode as usize;
        let d0_ptr = d0 as *const DieInputSignals as usize;
        let d1_ptr = d1 as *const DieInputSignals as usize;

        let c0 = crate::heist::choretree::Chore::FromClosure("Die0", move |_w| {
                     let f = unsafe { &mut *(ptr0 as *mut KarstFabricNode) };
                     let d = unsafe { &*(d0_ptr as *const DieInputSignals) };
                     f.step(&d.valid, &d.data, &d.ready);
                 }).Require(0);

        let c1 = crate::heist::choretree::Chore::FromClosure("Die1", move |_w| {
                     let f = unsafe { &mut *(ptr1 as *mut KarstFabricNode) };
                     let d = unsafe { &*(d1_ptr as *const DieInputSignals) };
                     f.step(&d.valid, &d.data, &d.ready);
                 }).Require(1);

        let tree = c0 | c1;
        atelier.MainMaestro().PostChoreTree(tree);
        atelier.DoLaunch();
    }
    pub fn advance(&mut self, ticks: u32) -> u64
    {
        for _ in 0..ticks {
            self.step_cycle();
        }
        self._cycle_count
    }
    /// Advance independent fabrics concurrently without changing per-fabric cycle order.
    /// This is intentionally separate from single-fabric die scheduling.
    pub fn AdvanceIndependent<'a>(fabrics: MutArr<'a, KarstFabric>, ticks: u32, workers: u32)
    {
        if fabrics.IsEmpty() || ticks == 0 {
            return;
        }
        let workers = workers.max(1);
        let atelier = Atelier::Reset(workers);
        let mut chore_tree: Option<crate::heist::choretree::ChoreNode> = None;
        let ptr = fabrics.Data() as usize;

        for i in 0..fabrics.Len() {
            let worker = i % workers;
            let fabric_ptr = ptr + (i as usize * std::mem::size_of::<KarstFabric>());

            let chore = crate::heist::choretree::Chore::FromClosure("AdvanceFabric", move |_w| {
                            let f = unsafe { &mut *(fabric_ptr as *mut KarstFabric) };
                            f.Advance(ticks);
                        }).Require(worker);

            if let Some(tree) = chore_tree {
                chore_tree = Some(tree | chore);
            } else {
                chore_tree = Some(chore.into());
            }
        }

        if let Some(tree) = chore_tree {
            atelier.MainMaestro().PostChoreTree(tree);
            atelier.DoLaunch();
        }
    }

    pub fn AdvanceIndependentCoro<'a>(fabrics: MutArr<'a, KarstFabric>, ticks: u32, workers: u32,
                                      chunk_ticks: u32)
    {
        if fabrics.IsEmpty() || ticks == 0 {
            return;
        }
        let workers = workers.max(1);
        let atelier = Atelier::Reset(workers);
        let mut chore_tree: Option<crate::heist::choretree::ChoreNode> = None;
        let ptr = fabrics.Data() as usize;

        for i in 0..fabrics.Len() {
            let worker = i % workers;
            let fabric_ptr = ptr + (i as usize * std::mem::size_of::<KarstFabric>());

            let chore =
                crate::heist::corochore::CoroChore::FromClosure("AdvanceFabricCoro",
                                                                move |yielder, _worker| {
                                                                    let f = unsafe {
                                                                        &mut *(fabric_ptr
                                                                               as *mut KarstFabric)
                                                                    };
                                                                    let mut rem = ticks;
                                                                    while rem > 0 {
                                                                        let step =
                                                                            rem.min(chunk_ticks);
                                                                        f.Advance(step);
                                                                        rem -= step;
                                                                        if rem > 0 {
                                                                            yielder.Suspend(());
                                                                        }
                                                                    }
                                                                }).Require(worker);

            if let Some(tree) = chore_tree {
                chore_tree = Some(tree | crate::heist::choretree::ChoreNode::from(chore));
            } else {
                chore_tree = Some(crate::heist::choretree::ChoreNode::from(chore));
            }
        }

        if let Some(tree) = chore_tree {
            atelier.MainMaestro().PostChoreTree(tree);
            atelier.DoLaunch();
        }
    }
    #[inline]
    pub fn Advance(&mut self, ticks: u32) -> u64 { self.advance(ticks) }
}
