// src/karst/fabric_node.rs
use	crate::karst::config::{ K_KL_PORTS_PER_HIND, K_MC_PORTS_PER_HIND };
use	crate::karst::link::KarstFlit;
use	crate::karst::memchan::MemChan;
use	crate::karst::noc::KarstNoc;
use	crate::karst::pipe::KarstPipe;
use	crate::karst::vpu::Vpu;
use	crate::silo::fifo::Fifo;
use	crate::swarm::cpu::ComputeDevice;

//-------------------------------------------------------------------------------------------------
// KarstFabricNode — KarstHind Memory Fabric die composite model.
// Integrates the MemFabric crossbar, 4 DDR5 memory controllers with mpipe retiming,
// 4 physical DDR5 memory channels (MemChan), and 4 Near-Memory VPUs (Vpu).
pub struct KarstFabricNode
{
    _die_id: u32,
    _noc: KarstNoc,
    _pipes: [KarstPipe; K_MC_PORTS_PER_HIND],
    _mem_chans: [MemChan; K_MC_PORTS_PER_HIND],
    _vpus: [Vpu; K_MC_PORTS_PER_HIND],
    // MC response staging
    _mc_resp_queue: [Fifo< u64, 16>; K_MC_PORTS_PER_HIND],
    _last_resp_accepted: [bool; K_MC_PORTS_PER_HIND],
}
impl KarstFabricNode
{
    pub fn	new( device: &ComputeDevice, die_id: u32) -> Self
    {
        let  	base_chan = die_id * ( K_MC_PORTS_PER_HIND as u32);
        Self {
            _die_id: die_id,
            _noc: KarstNoc::new( die_id),
            _pipes: [
                KarstPipe::new(),
                KarstPipe::new(),
                KarstPipe::new(),
                KarstPipe::new(),
            ],
            _mem_chans: [
                MemChan::new( device, base_chan, 4096),
                MemChan::new( device, base_chan + 1, 4096),
                MemChan::new( device, base_chan + 2, 4096),
                MemChan::new( device, base_chan + 3, 4096),
            ],
            _vpus: [
                Vpu::new( base_chan),
                Vpu::new( base_chan + 1),
                Vpu::new( base_chan + 2),
                Vpu::new( base_chan + 3),
            ],
            _mc_resp_queue: [Fifo::New(), Fifo::New(), Fifo::New(), Fifo::New()],
            _last_resp_accepted: [false; K_MC_PORTS_PER_HIND],
        }
    }
    #[inline]
    pub fn	die_id( &self) -> u32
    {
        self._die_id
    }
    #[inline]
    pub fn	noc( &self) -> &KarstNoc
    {
        &self._noc
    }
    #[inline]
    pub fn	noc_mut( &mut self) -> &mut KarstNoc
    {
        &mut self._noc
    }
    #[inline]
    pub fn	pipe( &self, idx: usize) -> &KarstPipe
    {
        &self._pipes[idx]
    }
    #[inline]
    pub fn	mem_chan( &self, idx: usize) -> &MemChan
    {
        &self._mem_chans[idx]
    }
    #[inline]
    pub fn	mem_chan_mut( &mut self, idx: usize) -> &mut MemChan
    {
        &mut self._mem_chans[idx]
    }
    #[inline]
    pub fn	vpu( &self, idx: usize) -> &Vpu
    {
        &self._vpus[idx]
    }
    #[inline]
    pub fn	vpu_mut( &mut self, idx: usize) -> &mut Vpu
    {
        &mut self._vpus[idx]
    }
    #[allow( clippy::needless_range_loop)]
    pub fn	step( 
        &mut self, kl_rx_valid: &[bool; K_KL_PORTS_PER_HIND],
        kl_rx_data: &[u64; K_KL_PORTS_PER_HIND], kl_tx_ready: &[bool; K_KL_PORTS_PER_HIND],
    )
    {
        // 1. Retire responses accepted by the NoC in the preceding cycle.
        for m in 0..K_MC_PORTS_PER_HIND {
            if self._last_resp_accepted[m] && !self._mc_resp_queue[m].IsEmpty() {
                self._mc_resp_queue[m].PopFront();
            }
        }
        // 2. Process requests emerging from retiming pipe to Memory Controller
        let  	mut mc_down_ready = [false; K_MC_PORTS_PER_HIND];
        for m in 0..K_MC_PORTS_PER_HIND {
            let  	pipe_out_valid = self._pipes[m].out_valid();
            let  	pipe_out_data = self._pipes[m].out_data();
            mc_down_ready[m] = !pipe_out_valid;
            if pipe_out_valid {
                let  	flit = KarstFlit::Unpack( pipe_out_data);
                let  	channel_addr = flit._Addr % ( self._mem_chans[m].capacity() as u32);
                if flit._IsWrite {
                    let  	_ = self._mem_chans[m].write_word( channel_addr, flit._Data);
                    mc_down_ready[m] = true;
                } else if !self._mc_resp_queue[m].IsFull() {
                    let  	read_val = self._mem_chans[m]
                        .read_word( channel_addr)
                        .unwrap_or( 0xDEADBEEF);
                    let  	resp_raw = KarstFlit::Pack( flit._Addr, read_val, flit._SrcId, false);
                    self._mc_resp_queue[m].PushBack( resp_raw);
                    mc_down_ready[m] = true;
                }
            }
        }
        // 3. Step each mpipe retimer between NoC and MC
        let  	mut mc_req_ready = [false; K_MC_PORTS_PER_HIND];
        for m in 0..K_MC_PORTS_PER_HIND {
            mc_req_ready[m] = self._pipes[m].up_ready();
            let  	noc_req_valid = self._noc.mc_req_valid( m);
            let  	noc_req_data = self._noc.mc_req_data( m);
            self._pipes[m].step( noc_req_valid, noc_req_data, mc_down_ready[m]);
        }
        // 4. Form MC response presentation signals to NoC
        let  	mut mc_resp_valid = [false; K_MC_PORTS_PER_HIND];
        let  	mut mc_resp_data = [0u64; K_MC_PORTS_PER_HIND];
        for m in 0..K_MC_PORTS_PER_HIND {
            if !self._mc_resp_queue[m].IsEmpty() {
                mc_resp_valid[m] = true;
                mc_resp_data[m] = *self._mc_resp_queue[m].Front().unwrap();
            }
            self._last_resp_accepted[m] = mc_resp_valid[m] && self._noc.mc_resp_ready( m);
        }
        // 5. Step NoC
        self._noc.step( 
            kl_rx_valid,
            kl_rx_data,
            kl_tx_ready,
            &mc_req_ready,
            &mc_resp_valid,
            &mc_resp_data,
        );
    }
}
