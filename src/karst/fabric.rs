// src/karst/fabric.rs

use crate::karst::config::{
    K_HIND_DIES_PER_FABRIC, K_HOSTS_PER_FABRIC, K_HOSTS_PER_HIND, K_INTERDIE_PORT_BASE,
    K_KL_PORTS_PER_HIND, K_MEM_CHANS_PER_FABRIC,
};
use crate::karst::fabric_node::KarstFabricNode;
use crate::karst::host_node::{HostResponse, KarstHostNode};
use crate::karst::memchan::MemChan;
use crate::karst::vpu::Vpu;
use crate::swarm::cpu::ComputeDevice;

//-------------------------------------------------------------------------------------------------
// Aggregate fabric metrics across all host ports, memory channels, and VPUs.

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct KarstStats {
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
    pub _CycleCount: u64,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct DieInputSignals {
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
}

impl Default for KarstFabric {
    fn default() -> Self {
        Self::new()
    }
}

impl KarstFabric {
    pub fn new() -> Self {
        Self::with_workers(1)
    }

    pub fn with_workers(workers: u32) -> Self {
        let compute_device = ComputeDevice::WithWorkers(workers);

        let hosts = [
            KarstHostNode::new(0),
            KarstHostNode::new(1),
            KarstHostNode::new(2),
            KarstHostNode::new(3),
            KarstHostNode::new(4),
            KarstHostNode::new(5),
            KarstHostNode::new(6),
            KarstHostNode::new(7),
        ];

        let fabrics = [
            KarstFabricNode::new(&compute_device, 0),
            KarstFabricNode::new(&compute_device, 1),
        ];

        Self {
            _compute_device: compute_device,
            _hosts: hosts,
            _fabrics: fabrics,
            _cycle_count: 0,
        }
    }

    #[inline]
    pub fn cycle_count(&self) -> u64 {
        self._cycle_count
    }

    #[inline]
    pub fn workers(&self) -> u32 {
        1
    }

    #[inline]
    pub fn compute_device(&self) -> &ComputeDevice {
        &self._compute_device
    }

    #[inline]
    pub fn compute_device_mut(&mut self) -> &mut ComputeDevice {
        &mut self._compute_device
    }

    #[inline]
    pub fn engine(&self) -> KarstEngineInfo {
        KarstEngineInfo {
            _CycleCount: self._cycle_count,
        }
    }

    #[inline]
    pub fn Engine(&self) -> KarstEngineInfo {
        self.engine()
    }

    #[inline]
    pub fn host(&self, id: u32) -> &KarstHostNode {
        &self._hosts[id as usize]
    }

    #[inline]
    pub fn host_mut(&mut self, id: u32) -> &mut KarstHostNode {
        &mut self._hosts[id as usize]
    }

    #[inline]
    pub fn Host(&self, id: u32) -> &KarstHostNode {
        self.host(id)
    }

    #[inline]
    pub fn HostMut(&mut self, id: u32) -> &mut KarstHostNode {
        self.host_mut(id)
    }

    #[inline]
    pub fn fabric_node(&self, die_id: u32) -> &KarstFabricNode {
        &self._fabrics[die_id as usize]
    }

    #[inline]
    pub fn fabric_node_mut(&mut self, die_id: u32) -> &mut KarstFabricNode {
        &mut self._fabrics[die_id as usize]
    }

    #[inline]
    pub fn FabricNode(&self, die_id: u32) -> &KarstFabricNode {
        self.fabric_node(die_id)
    }

    #[inline]
    pub fn FabricNodeMut(&mut self, die_id: u32) -> &mut KarstFabricNode {
        self.fabric_node_mut(die_id)
    }

    #[inline]
    pub fn mem_chan(&self, global_idx: u32) -> &MemChan {
        let die_id = (global_idx / 4) as usize;
        let mc_idx = (global_idx % 4) as usize;
        self._fabrics[die_id].mem_chan(mc_idx)
    }

    #[inline]
    pub fn mem_chan_mut(&mut self, global_idx: u32) -> &mut MemChan {
        let die_id = (global_idx / 4) as usize;
        let mc_idx = (global_idx % 4) as usize;
        self._fabrics[die_id].mem_chan_mut(mc_idx)
    }

    #[inline]
    pub fn MemChan(&self, global_idx: u32) -> &MemChan {
        self.mem_chan(global_idx)
    }

    #[inline]
    pub fn MemChanMut(&mut self, global_idx: u32) -> &mut MemChan {
        self.mem_chan_mut(global_idx)
    }

    // Trellis alias DChan -> MemChan
    #[inline]
    pub fn DChan(&self, global_idx: u32) -> &MemChan {
        self.mem_chan(global_idx)
    }

    #[inline]
    pub fn DChanMut(&mut self, global_idx: u32) -> &mut MemChan {
        self.mem_chan_mut(global_idx)
    }

    #[inline]
    pub fn vpu(&self, global_idx: u32) -> &Vpu {
        let die_id = (global_idx / 4) as usize;
        let mc_idx = (global_idx % 4) as usize;
        self._fabrics[die_id].vpu(mc_idx)
    }

    #[inline]
    pub fn vpu_mut(&mut self, global_idx: u32) -> &mut Vpu {
        let die_id = (global_idx / 4) as usize;
        let mc_idx = (global_idx % 4) as usize;
        self._fabrics[die_id].vpu_mut(mc_idx)
    }

    #[inline]
    pub fn VPU(&self, global_idx: u32) -> &Vpu {
        self.vpu(global_idx)
    }

    #[inline]
    pub fn VPUMut(&mut self, global_idx: u32) -> &mut Vpu {
        self.vpu_mut(global_idx)
    }

    #[inline]
    pub fn VPU_mut(&mut self, global_idx: u32) -> &mut Vpu {
        self.vpu_mut(global_idx)
    }

    #[inline]
    pub fn MemChan_mut(&mut self, global_idx: u32) -> &mut MemChan {
        self.mem_chan_mut(global_idx)
    }

    pub fn dispatch_vpu(
        &self,
        vpu_idx: u32,
        chan_idx: u32,
        dim: crate::swarm::traits::WorkgroupDim,
    ) -> Result<(), crate::swarm::traits::SwarmError> {
        let vpu = self.vpu(vpu_idx);
        let chan = self.mem_chan(chan_idx);
        vpu.dispatch(&self._compute_device, chan, dim)
    }

    pub fn post_host_write(&mut self, host_id: u32, addr: u32, data: u32) {
        if host_id < K_HOSTS_PER_FABRIC {
            self._hosts[host_id as usize].post_write(addr, data);
        }
    }

    #[inline]
    pub fn PostHostWrite(&mut self, host_id: u32, addr: u32, data: u32) {
        self.post_host_write(host_id, addr, data);
    }

    pub fn post_host_read(&mut self, host_id: u32, addr: u32) {
        if host_id < K_HOSTS_PER_FABRIC {
            self._hosts[host_id as usize].post_read(addr);
        }
    }

    #[inline]
    pub fn PostHostRead(&mut self, host_id: u32, addr: u32) {
        self.post_host_read(host_id, addr);
    }

    pub fn pop_host_response(&mut self, host_id: u32, out: &mut HostResponse) -> bool {
        if host_id < K_HOSTS_PER_FABRIC {
            self._hosts[host_id as usize].pop_response(out)
        } else {
            false
        }
    }

    #[inline]
    pub fn PopHostResponse(&mut self, host_id: u32, out: &mut HostResponse) -> bool {
        self.pop_host_response(host_id, out)
    }

    pub fn stats(&self) -> KarstStats {
        let mut total = KarstStats {
            _TotalTxCount: 0,
            _TotalRxCount: 0,
            _TotalBytesWritten: 0,
            _TotalBytesRead: 0,
            _TotalVPUDispatches: 0,
            _CycleCount: self._cycle_count,
        };

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
    pub fn Stats(&self) -> KarstStats {
        self.stats()
    }

    fn prepare_cycle_inputs(&mut self) -> (DieInputSignals, DieInputSignals) {
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

            self._hosts[h].step(
                l0_tx_ready,
                l0_rx_valid,
                l0_rx_data,
                l1_tx_ready,
                l1_rx_valid,
                l1_rx_data,
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
            );
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
            .copy_from_slice(&h_l1_tx_valid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d0_in_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_tx_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d0_in_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_rx_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);

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
        d1_in_valid[..K_HOSTS_PER_HIND]
            .copy_from_slice(&h_l0_tx_valid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d1_in_data[..K_HOSTS_PER_HIND]
            .copy_from_slice(&h_l0_tx_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);
        d1_in_ready[..K_HOSTS_PER_HIND]
            .copy_from_slice(&h_l0_rx_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]);

        // Ports 4..7: Hosts 0..3 Link1
        d1_in_valid[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_tx_valid[..K_HOSTS_PER_HIND]);
        d1_in_data[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_tx_data[..K_HOSTS_PER_HIND]);
        d1_in_ready[K_HOSTS_PER_HIND..K_INTERDIE_PORT_BASE]
            .copy_from_slice(&h_l1_rx_ready[..K_HOSTS_PER_HIND]);

        // Ports 8..9: Inter-die links from Die 0
        d1_in_valid[K_INTERDIE_PORT_BASE] = d0_kl_tx_valid[K_INTERDIE_PORT_BASE];
        d1_in_data[K_INTERDIE_PORT_BASE] = d0_kl_tx_data[K_INTERDIE_PORT_BASE];
        d1_in_ready[K_INTERDIE_PORT_BASE] = d0_kl_rx_ready[K_INTERDIE_PORT_BASE];

        d1_in_valid[K_INTERDIE_PORT_BASE + 1] = d0_kl_tx_valid[K_INTERDIE_PORT_BASE + 1];
        d1_in_data[K_INTERDIE_PORT_BASE + 1] = d0_kl_tx_data[K_INTERDIE_PORT_BASE + 1];
        d1_in_ready[K_INTERDIE_PORT_BASE + 1] = d0_kl_rx_ready[K_INTERDIE_PORT_BASE + 1];

        (
            DieInputSignals {
                valid: d0_in_valid,
                data: d0_in_data,
                ready: d0_in_ready,
            },
            DieInputSignals {
                valid: d1_in_valid,
                data: d1_in_data,
                ready: d1_in_ready,
            },
        )
    }

    pub fn step_cycle(&mut self) {
        let (d0, d1) = self.prepare_cycle_inputs();

        self._fabrics[0].step(&d0.valid, &d0.data, &d0.ready);
        self._fabrics[1].step(&d1.valid, &d1.data, &d1.ready);

        self._cycle_count += 1;
    }

    pub fn advance(&mut self, ticks: u32) -> u64 {
        for _ in 0..ticks {
            self.step_cycle();
        }
        self._cycle_count
    }

    #[inline]
    pub fn Advance(&mut self, ticks: u32) -> u64 {
        self.advance(ticks)
    }
}
