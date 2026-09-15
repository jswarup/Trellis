// src/karst/noc.rs

use crate::karst::config::{
    K_DIE_ADDR_BIT, K_HOSTS_PER_FABRIC, K_HOSTS_PER_HIND, K_INTERDIE_PORT_BASE,
    K_KL_PORTS_PER_HIND, K_MC_ADDR_MASK, K_MC_ADDR_SHIFT, K_MC_PORTS_PER_HIND,
};
use crate::karst::link::{KarstFlit, KarstLinkChannel};
use crate::silo::fifo::Fifo;

//-------------------------------------------------------------------------------------------------
// KarstNoc — KarstHind MemFabric crossbar switch.
// Switches 10 KarstLink ports (KL0..KL9) and 4 local DDR5 Memory Controllers (MC0..MC3).
// Implements address decoding, 1 kB striped memory interleaving, and inter-die KarstLink routing.

pub struct KarstNoc {
    _die_id: u32,

    // Internal queues
    _mc_req_queue: [Fifo<u64, 16>; K_MC_PORTS_PER_HIND],
    _kl_tx_queue: [Fifo<u64, 16>; K_KL_PORTS_PER_HIND],
    _kl_rx_queue: [Fifo<u64, 16>; K_KL_PORTS_PER_HIND],
    _mc_resp_queue: [Fifo<u64, 16>; K_MC_PORTS_PER_HIND],

    // Output registers
    _last_mc_req_presented: [bool; K_MC_PORTS_PER_HIND],
    _last_kl_tx_presented: [bool; K_KL_PORTS_PER_HIND],

    // Current output signals
    pub kl_rx_ready_out: [bool; K_KL_PORTS_PER_HIND],
    pub kl_tx_valid_out: [bool; K_KL_PORTS_PER_HIND],
    pub kl_tx_data_out: [u64; K_KL_PORTS_PER_HIND],
    pub mc_req_valid_out: [bool; K_MC_PORTS_PER_HIND],
    pub mc_req_data_out: [u64; K_MC_PORTS_PER_HIND],
    pub mc_resp_ready_out: [bool; K_MC_PORTS_PER_HIND],
}

impl KarstNoc {
    pub fn new(die_id: u32) -> Self {
        let mut noc = Self {
            _die_id: die_id,
            _mc_req_queue: [Fifo::New(), Fifo::New(), Fifo::New(), Fifo::New()],
            _kl_tx_queue: [
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
            ],
            _kl_rx_queue: [
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
                Fifo::New(),
            ],
            _mc_resp_queue: [Fifo::New(), Fifo::New(), Fifo::New(), Fifo::New()],
            _last_mc_req_presented: [false; K_MC_PORTS_PER_HIND],
            _last_kl_tx_presented: [false; K_KL_PORTS_PER_HIND],
            kl_rx_ready_out: [true; K_KL_PORTS_PER_HIND],
            kl_tx_valid_out: [false; K_KL_PORTS_PER_HIND],
            kl_tx_data_out: [0; K_KL_PORTS_PER_HIND],
            mc_req_valid_out: [false; K_MC_PORTS_PER_HIND],
            mc_req_data_out: [0; K_MC_PORTS_PER_HIND],
            mc_resp_ready_out: [true; K_MC_PORTS_PER_HIND],
        };
        noc.update_outputs();
        noc
    }

    #[inline]
    pub fn die_id(&self) -> u32 {
        self._die_id
    }

    #[inline]
    pub fn kl_rx_ready(&self, port: usize) -> bool {
        self.kl_rx_ready_out[port]
    }

    #[inline]
    pub fn kl_tx_valid(&self, port: usize) -> bool {
        self.kl_tx_valid_out[port]
    }

    #[inline]
    pub fn kl_tx_data(&self, port: usize) -> u64 {
        self.kl_tx_data_out[port]
    }

    #[inline]
    pub fn kl_tx_channel(&self, port: usize) -> KarstLinkChannel {
        KarstLinkChannel {
            valid: self.kl_tx_valid_out[port],
            data: self.kl_tx_data_out[port],
            ready: true,
        }
    }

    #[inline]
    pub fn mc_req_valid(&self, m: usize) -> bool {
        self.mc_req_valid_out[m]
    }

    #[inline]
    pub fn mc_req_data(&self, m: usize) -> u64 {
        self.mc_req_data_out[m]
    }

    #[inline]
    pub fn mc_resp_ready(&self, m: usize) -> bool {
        self.mc_resp_ready_out[m]
    }

    fn update_outputs(&mut self) {
        for t in 0..K_KL_PORTS_PER_HIND {
            let mut tx_valid = false;
            let mut tx_data = 0u64;
            if !self._kl_tx_queue[t].IsEmpty() {
                tx_valid = true;
                tx_data = *self._kl_tx_queue[t].Front().unwrap();
            }
            self._last_kl_tx_presented[t] = tx_valid;
            self.kl_rx_ready_out[t] = !self._kl_rx_queue[t].IsFull();
            self.kl_tx_valid_out[t] = tx_valid;
            self.kl_tx_data_out[t] = tx_data;
        }

        for m in 0..K_MC_PORTS_PER_HIND {
            let mut req_valid = false;
            let mut req_data = 0u64;
            if !self._mc_req_queue[m].IsEmpty() {
                req_valid = true;
                req_data = *self._mc_req_queue[m].Front().unwrap();
            }
            self._last_mc_req_presented[m] = req_valid;
            self.mc_resp_ready_out[m] = !self._mc_resp_queue[m].IsFull();
            self.mc_req_valid_out[m] = req_valid;
            self.mc_req_data_out[m] = req_data;
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub fn step(
        &mut self,
        kl_rx_valid: &[bool; K_KL_PORTS_PER_HIND],
        kl_rx_data: &[u64; K_KL_PORTS_PER_HIND],
        kl_tx_ready: &[bool; K_KL_PORTS_PER_HIND],
        mc_req_ready: &[bool; K_MC_PORTS_PER_HIND],
        mc_resp_valid: &[bool; K_MC_PORTS_PER_HIND],
        mc_resp_data: &[u64; K_MC_PORTS_PER_HIND],
    ) {
        // 1. Retire successfully transferred outputs
        for m in 0..K_MC_PORTS_PER_HIND {
            if self._last_mc_req_presented[m] && mc_req_ready[m] && !self._mc_req_queue[m].IsEmpty()
            {
                self._mc_req_queue[m].PopFront();
            }
        }
        for t in 0..K_KL_PORTS_PER_HIND {
            if self._last_kl_tx_presented[t] && kl_tx_ready[t] && !self._kl_tx_queue[t].IsEmpty() {
                self._kl_tx_queue[t].PopFront();
            }
        }

        // 2. Capture ingress independently before routing to shared queues
        for t in 0..K_KL_PORTS_PER_HIND {
            if kl_rx_valid[t] && !self._kl_rx_queue[t].IsFull() {
                self._kl_rx_queue[t].PushBack(kl_rx_data[t]);
            }
        }

        // 3. Capture MC responses independently before routing them to shared links
        for m in 0..K_MC_PORTS_PER_HIND {
            if mc_resp_valid[m] && !self._mc_resp_queue[m].IsFull() {
                self._mc_resp_queue[m].PushBack(mc_resp_data[m]);
            }
        }

        // 4. Route buffered ingress to its selected output queue
        for t in 0..K_KL_PORTS_PER_HIND {
            if !self._kl_rx_queue[t].IsEmpty() {
                let raw = *self._kl_rx_queue[t].Front().unwrap();
                let flit = KarstFlit::Unpack(raw);

                let target_die = (flit._Addr >> K_DIE_ADDR_BIT) & 1;
                if target_die == self._die_id {
                    let mc_idx = ((flit._Addr >> K_MC_ADDR_SHIFT) & K_MC_ADDR_MASK) as usize;
                    if !self._mc_req_queue[mc_idx].IsFull() {
                        self._mc_req_queue[mc_idx].PushBack(raw);
                        self._kl_rx_queue[t].PopFront();
                    }
                } else if !self._kl_tx_queue[K_INTERDIE_PORT_BASE].IsFull() {
                    self._kl_tx_queue[K_INTERDIE_PORT_BASE].PushBack(raw);
                    self._kl_rx_queue[t].PopFront();
                }
            }
        }

        for m in 0..K_MC_PORTS_PER_HIND {
            if !self._mc_resp_queue[m].IsEmpty() {
                let raw = *self._mc_resp_queue[m].Front().unwrap();
                let flit = KarstFlit::Unpack(raw);
                let target_kl = if self._die_id == 0 {
                    (flit._SrcId % (K_HOSTS_PER_FABRIC as u8)) as usize
                } else if flit._SrcId >= (K_HOSTS_PER_HIND as u8) {
                    ((flit._SrcId - (K_HOSTS_PER_HIND as u8)) % (K_HOSTS_PER_FABRIC as u8)) as usize
                } else {
                    ((flit._SrcId + (K_HOSTS_PER_HIND as u8)) % (K_HOSTS_PER_FABRIC as u8)) as usize
                };

                if target_kl < K_KL_PORTS_PER_HIND && !self._kl_tx_queue[target_kl].IsFull() {
                    self._kl_tx_queue[target_kl].PushBack(raw);
                    self._mc_resp_queue[m].PopFront();
                }
            }
        }

        // 5. Drive outputs for the next cycle
        self.update_outputs();
    }
}
