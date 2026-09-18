// src/karst/fabric_node.rs
use crate::karst::config::{K_KL_PORTS_PER_HIND, K_MC_PORTS_PER_HIND};
use crate::karst::link::KarstFlit;
use crate::karst::memchan::MemChan;
use crate::karst::memchan::KarstDChan;
use crate::karst::noc::KarstNoc;
use crate::karst::pipe::KarstPipe;
use crate::karst::vpu::Vpu;
use crate::karst::vpu::KarstVPU;
use crate::silo::fifo::Fifo;
use crate::silo::USeg;
use crate::swarm::cpu::ComputeDevice;

//-------------------------------------------------------------------------------------------------

// KarstFabricNode — KarstHind Memory Fabric die composite model.
// Integrates the MemFabric crossbar, 4 DDR5 memory controllers with mpipe retiming,
// 4 physical DDR5 memory channels (MemChan), and 4 Near-Memory VPUs (Vpu).
pub struct KarstFabricNode {
    _die_id: u32,
    _noc: KarstNoc,
    _pipes: [KarstPipe; K_MC_PORTS_PER_HIND],
    _mem_chans: [MemChan; K_MC_PORTS_PER_HIND],
    _vpus: [Vpu; K_MC_PORTS_PER_HIND],
    // MC response staging
    _mc_resp_queue: [Fifo<u64, 16>; K_MC_PORTS_PER_HIND],
    _last_resp_presented: [bool; K_MC_PORTS_PER_HIND],
// 4 physical DDR5 memory channels (KarstDChan), and 4 Near-Memory VPUs (KarstVPU).
pub struct KarstFabricNode
{
    _DieId: u32,
    _Noc: KarstNoc,
    _Pipes: [KarstPipe; K_MC_PORTS_PER_HIND],
    _MemChans: [KarstDChan; K_MC_PORTS_PER_HIND],
    _Vpus: [KarstVPU; K_MC_PORTS_PER_HIND],
    _McRespQueue: [Fifo<u64, 16>; K_MC_PORTS_PER_HIND],
    _LastRespPresented: [bool; K_MC_PORTS_PER_HIND],
}
impl KarstFabricNode {
    pub fn new(device: &ComputeDevice, die_id: u32) -> Self {
        let base_chan = die_id * (K_MC_PORTS_PER_HIND as u32);

impl KarstFabricNode
{
    pub fn New(device: &ComputeDevice, dieId: u32) -> Self
    {
        let baseChan = dieId * (K_MC_PORTS_PER_HIND as u32);
        Self {
            _die_id: die_id,
            _noc: KarstNoc::new(die_id),
            _pipes: [
                KarstPipe::new(),
                KarstPipe::new(),
                KarstPipe::new(),
                KarstPipe::new(),
            _DieId: dieId,
            _Noc: KarstNoc::New(dieId),
            _Pipes: [
                KarstPipe::New(),
                KarstPipe::New(),
                KarstPipe::New(),
                KarstPipe::New(),
            ],
            _mem_chans: [
                MemChan::new(device, base_chan, 4096),
                MemChan::new(device, base_chan + 1, 4096),
                MemChan::new(device, base_chan + 2, 4096),
                MemChan::new(device, base_chan + 3, 4096),
            _MemChans: [
                KarstDChan::New(device, baseChan, 4096),
                KarstDChan::New(device, baseChan + 1, 4096),
                KarstDChan::New(device, baseChan + 2, 4096),
                KarstDChan::New(device, baseChan + 3, 4096),
            ],
            _vpus: [
                Vpu::new(base_chan),
                Vpu::new(base_chan + 1),
                Vpu::new(base_chan + 2),
                Vpu::new(base_chan + 3),
            _Vpus: [
                KarstVPU::New(baseChan),
                KarstVPU::New(baseChan + 1),
                KarstVPU::New(baseChan + 2),
                KarstVPU::New(baseChan + 3),
            ],
            _mc_resp_queue: [Fifo::New(), Fifo::New(), Fifo::New(), Fifo::New()],
            _last_resp_presented: [false; K_MC_PORTS_PER_HIND],
            _McRespQueue: [Fifo::New(), Fifo::New(), Fifo::New(), Fifo::New()],
            _LastRespPresented: [false; K_MC_PORTS_PER_HIND],
        }
    }

    #[inline]
    pub fn die_id(&self) -> u32 {
        self._die_id
    pub fn new(device: &ComputeDevice, dieId: u32) -> Self
    {
        Self::New(device, dieId)
    }

    #[inline]
    pub fn noc(&self) -> &KarstNoc {
        &self._noc
    pub fn DieId(&self) -> u32
    {
        self._DieId
    }

    #[inline]
    pub fn noc_mut(&mut self) -> &mut KarstNoc {
        &mut self._noc
    pub fn die_id(&self) -> u32
    {
        self.DieId()
    }

    #[inline]
    pub fn pipe(&self, idx: usize) -> &KarstPipe {
        &self._pipes[idx]
    pub fn Noc(&self) -> &KarstNoc
    {
        &self._Noc
    }

    #[inline]
    pub fn mem_chan(&self, idx: usize) -> &MemChan {
        &self._mem_chans[idx]
    pub fn noc(&self) -> &KarstNoc
    {
        self.Noc()
    }

    #[inline]
    pub fn mem_chan_mut(&mut self, idx: usize) -> &mut MemChan {
        &mut self._mem_chans[idx]
    pub fn NocMut(&mut self) -> &mut KarstNoc
    {
        &mut self._Noc
    }

    #[inline]
    pub fn vpu(&self, idx: usize) -> &Vpu {
        &self._vpus[idx]
    pub fn noc_mut(&mut self) -> &mut KarstNoc
    {
        self.NocMut()
    }

    #[inline]
    pub fn vpu_mut(&mut self, idx: usize) -> &mut Vpu {
        &mut self._vpus[idx]
    pub fn Pipe(&self, idx: usize) -> &KarstPipe
    {
        &self._Pipes[idx]
    }
    #[allow(clippy::needless_range_loop)]
    pub fn step(

    #[inline]
    pub fn pipe(&self, idx: usize) -> &KarstPipe
    {
        self.Pipe(idx)
    }

    #[inline]
    pub fn DChan(&self, idx: usize) -> &KarstDChan
    {
        &self._MemChans[idx]
    }

    #[inline]
    pub fn DChanMut(&mut self, idx: usize) -> &mut KarstDChan
    {
        &mut self._MemChans[idx]
    }

    #[inline]
    pub fn MemChan(&self, idx: usize) -> &KarstDChan
    {
        self.DChan(idx)
    }

    #[inline]
    pub fn mem_chan(&self, idx: usize) -> &KarstDChan
    {
        self.DChan(idx)
    }

    #[inline]
    pub fn MemChanMut(&mut self, idx: usize) -> &mut KarstDChan
    {
        self.DChanMut(idx)
    }

    #[inline]
    pub fn mem_chan_mut(&mut self, idx: usize) -> &mut KarstDChan
    {
        self.DChanMut(idx)
    }

    #[inline]
    pub fn VPU(&self, idx: usize) -> &KarstVPU
    {
        &self._Vpus[idx]
    }

    #[inline]
    pub fn VPUMut(&mut self, idx: usize) -> &mut KarstVPU
    {
        &mut self._Vpus[idx]
    }

    #[inline]
    pub fn Vpu(&self, idx: usize) -> &KarstVPU
    {
        self.VPU(idx)
    }

    #[inline]
    pub fn vpu(&self, idx: usize) -> &KarstVPU
    {
        self.VPU(idx)
    }

    #[inline]
    pub fn VpuMut(&mut self, idx: usize) -> &mut KarstVPU
    {
        self.VPUMut(idx)
    }

    #[inline]
    pub fn vpu_mut(&mut self, idx: usize) -> &mut KarstVPU
    {
        self.VPUMut(idx)
    }

    pub fn Step(
        &mut self,
        kl_rx_valid: &[bool; K_KL_PORTS_PER_HIND],
        kl_rx_data: &[u64; K_KL_PORTS_PER_HIND],
        kl_tx_ready: &[bool; K_KL_PORTS_PER_HIND],
    ) {
        klRxValid: &[bool; K_KL_PORTS_PER_HIND],
        klRxData: &[u64; K_KL_PORTS_PER_HIND],
        klTxReady: &[bool; K_KL_PORTS_PER_HIND],
    )
    {
        // 1. Retire previously accepted responses from MC
        for m in 0..K_MC_PORTS_PER_HIND {
            if self._last_resp_presented[m]
                && self._noc.mc_resp_ready(m)
                && !self._mc_resp_queue[m].IsEmpty()
        USeg::FromLen(K_MC_PORTS_PER_HIND as u32).Traverse(|m_u| {
            let m = m_u as usize;
            if self._LastRespPresented[m]
                && self._Noc.McRespReady(m)
                && !self._McRespQueue[m].IsEmpty()
            {
                self._mc_resp_queue[m].PopFront();
                self._McRespQueue[m].PopFront();
            }
        }
        });

        // 2. Process requests emerging from retiming pipe to Memory Controller
        for m in 0..K_MC_PORTS_PER_HIND {
            let pipe_out_valid = self._pipes[m].out_valid();
            let pipe_out_data = self._pipes[m].out_data();
            if pipe_out_valid && !self._mc_resp_queue[m].IsFull() {
                let flit = KarstFlit::Unpack(pipe_out_data);
                let channel_addr = flit._Addr % (self._mem_chans[m].capacity() as u32);
        USeg::FromLen(K_MC_PORTS_PER_HIND as u32).Traverse(|m_u| {
            let m = m_u as usize;
            let pipeOutValid = self._Pipes[m].OutValid();
            let pipeOutData = self._Pipes[m].OutData();
            if pipeOutValid && !self._McRespQueue[m].IsFull() {
                let flit = KarstFlit::Unpack(pipeOutData);
                let channelAddr = flit._Addr % (self._MemChans[m].Capacity() as u32);
                if flit._IsWrite {
                    let _ = self._mem_chans[m].write_word(channel_addr, flit._Data);
                    let _ = self._MemChans[m].WriteWord(channelAddr, flit._Data);
                } else {
                    let read_val = self._mem_chans[m]
                        .read_word(channel_addr)
                    let readVal = self._MemChans[m]
                        .ReadWord(channelAddr)
                        .unwrap_or(0xDEADBEEF);
                    let resp_raw = KarstFlit::Pack(flit._Addr, read_val, flit._SrcId, false);
                    self._mc_resp_queue[m].PushBack(resp_raw);
                    let respRaw = KarstFlit::Pack(flit._Addr, readVal, flit._SrcId, false);
                    self._McRespQueue[m].PushBack(respRaw);
                }
            }
        }
        });

        // 3. Step each mpipe retimer between NoC and MC
        let mut mc_req_ready = [false; K_MC_PORTS_PER_HIND];
        for m in 0..K_MC_PORTS_PER_HIND {
            let mc_down_ready = !self._mc_resp_queue[m].IsFull();
            let noc_req_valid = self._noc.mc_req_valid(m);
            let noc_req_data = self._noc.mc_req_data(m);
            self._pipes[m].step(noc_req_valid, noc_req_data, mc_down_ready);
            mc_req_ready[m] = self._pipes[m].up_ready();
        }
        let mut mcReqReady = [false; K_MC_PORTS_PER_HIND];
        USeg::FromLen(K_MC_PORTS_PER_HIND as u32).Traverse(|m_u| {
            let m = m_u as usize;
            let mcDownReady = !self._McRespQueue[m].IsFull();
            let nocReqValid = self._Noc.McReqValid(m);
            let nocReqData = self._Noc.McReqData(m);
            self._Pipes[m].Step(nocReqValid, nocReqData, mcDownReady);
            mcReqReady[m] = self._Pipes[m].UpReady();
        });

        // 4. Form MC response presentation signals to NoC
        let mut mc_resp_valid = [false; K_MC_PORTS_PER_HIND];
        let mut mc_resp_data = [0u64; K_MC_PORTS_PER_HIND];
        for m in 0..K_MC_PORTS_PER_HIND {
            if !self._mc_resp_queue[m].IsEmpty() {
                mc_resp_valid[m] = true;
                mc_resp_data[m] = *self._mc_resp_queue[m].Front().unwrap();
        let mut mcRespValid = [false; K_MC_PORTS_PER_HIND];
        let mut mcRespData = [0u64; K_MC_PORTS_PER_HIND];
        USeg::FromLen(K_MC_PORTS_PER_HIND as u32).Traverse(|m_u| {
            let m = m_u as usize;
            if !self._McRespQueue[m].IsEmpty() {
                mcRespValid[m] = true;
                mcRespData[m] = *self._McRespQueue[m].Front().unwrap();
            }
            self._last_resp_presented[m] = mc_resp_valid[m];
        }
            self._LastRespPresented[m] = mcRespValid[m];
        });

        // 5. Step NoC
        self._noc.step(
            kl_rx_valid,
            kl_rx_data,
            kl_tx_ready,
            &mc_req_ready,
            &mc_resp_valid,
            &mc_resp_data,
        self._Noc.Step(
            klRxValid,
            klRxData,
            klTxReady,
            &mcReqReady,
            &mcRespValid,
            &mcRespData,
        );
    }

    #[inline]
    pub fn step(
        &mut self,
        klRxValid: &[bool; K_KL_PORTS_PER_HIND],
        klRxData: &[u64; K_KL_PORTS_PER_HIND],
        klTxReady: &[bool; K_KL_PORTS_PER_HIND],
    )
    {
        self.Step(klRxValid, klRxData, klTxReady);
    }
}
