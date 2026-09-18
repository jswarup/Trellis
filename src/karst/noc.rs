// src/karst/noc.rs
use crate::karst::config::{
    K_DIE_ADDR_BIT, K_HOSTS_PER_FABRIC, K_HOSTS_PER_HIND, K_INTERDIE_PORT_BASE,
    K_KL_PORTS_PER_HIND, K_MC_ADDR_MASK, K_MC_ADDR_SHIFT, K_MC_PORTS_PER_HIND,
};
use crate::karst::link::{KarstFlit, KarstLinkChannel};
use crate::silo::fifo::Fifo;
use crate::silo::USeg;

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
pub struct KarstNoc
{
    _DieId: u32,
    _McReqQueue: [Fifo<u64, 16>; K_MC_PORTS_PER_HIND],
    _KlTxQueue: [Fifo<u64, 16>; K_KL_PORTS_PER_HIND],
    _KlRxQueue: [Fifo<u64, 16>; K_KL_PORTS_PER_HIND],
    _McRespQueue: [Fifo<u64, 16>; K_MC_PORTS_PER_HIND],
    _LastMcReqPresented: [bool; K_MC_PORTS_PER_HIND],
    _LastKlTxPresented: [bool; K_KL_PORTS_PER_HIND],

    pub _KlRxReadyOut: [bool; K_KL_PORTS_PER_HIND],
    pub _KlTxValidOut: [bool; K_KL_PORTS_PER_HIND],
    pub _KlTxDataOut: [u64; K_KL_PORTS_PER_HIND],
    pub _McReqValidOut: [bool; K_MC_PORTS_PER_HIND],
    pub _McReqDataOut: [u64; K_MC_PORTS_PER_HIND],
    pub _McRespReadyOut: [bool; K_MC_PORTS_PER_HIND],
}
impl KarstNoc {
    pub fn new(die_id: u32) -> Self {

impl KarstNoc
{
    pub fn New(dieId: u32) -> Self
    {
        let mut noc = Self {
            _die_id: die_id,
            _mc_req_queue: [Fifo::New(), Fifo::New(), Fifo::New(), Fifo::New()],
            _kl_tx_queue: [
            _DieId: dieId,
            _McReqQueue: [Fifo::New(), Fifo::New(), Fifo::New(), Fifo::New()],
            _KlTxQueue: [
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
            _KlRxQueue: [
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
            _McRespQueue: [Fifo::New(), Fifo::New(), Fifo::New(), Fifo::New()],
            _LastMcReqPresented: [false; K_MC_PORTS_PER_HIND],
            _LastKlTxPresented: [false; K_KL_PORTS_PER_HIND],
            _KlRxReadyOut: [true; K_KL_PORTS_PER_HIND],
            _KlTxValidOut: [false; K_KL_PORTS_PER_HIND],
            _KlTxDataOut: [0; K_KL_PORTS_PER_HIND],
            _McReqValidOut: [false; K_MC_PORTS_PER_HIND],
            _McReqDataOut: [0; K_MC_PORTS_PER_HIND],
            _McRespReadyOut: [true; K_MC_PORTS_PER_HIND],
        };
        noc.update_outputs();
        noc.UpdateOutputs();
        noc
    }

    #[inline]
    pub fn die_id(&self) -> u32 {
        self._die_id
    pub fn new(dieId: u32) -> Self
    {
        Self::New(dieId)
    }

    #[inline]
    pub fn kl_rx_ready(&self, port: usize) -> bool {
        self.kl_rx_ready_out[port]
    pub fn DieId(&self) -> u32
    {
        self._DieId
    }

    #[inline]
    pub fn kl_tx_valid(&self, port: usize) -> bool {
        self.kl_tx_valid_out[port]
    pub fn die_id(&self) -> u32
    {
        self.DieId()
    }

    #[inline]
    pub fn kl_tx_data(&self, port: usize) -> u64 {
        self.kl_tx_data_out[port]
    pub fn KlRxReady(&self, port: usize) -> bool
    {
        self._KlRxReadyOut[port]
    }

    #[inline]
    pub fn kl_tx_channel(&self, port: usize) -> KarstLinkChannel {
    pub fn kl_rx_ready(&self, port: usize) -> bool
    {
        self.KlRxReady(port)
    }

    #[inline]
    pub fn KlTxValid(&self, port: usize) -> bool
    {
        self._KlTxValidOut[port]
    }

    #[inline]
    pub fn kl_tx_valid(&self, port: usize) -> bool
    {
        self.KlTxValid(port)
    }

    #[inline]
    pub fn KlTxData(&self, port: usize) -> u64
    {
        self._KlTxDataOut[port]
    }

    #[inline]
    pub fn kl_tx_data(&self, port: usize) -> u64
    {
        self.KlTxData(port)
    }

    #[inline]
    pub fn KlTxChannel(&self, port: usize) -> KarstLinkChannel
    {
        KarstLinkChannel {
            valid: self.kl_tx_valid_out[port],
            data: self.kl_tx_data_out[port],
            valid: self._KlTxValidOut[port],
            data: self._KlTxDataOut[port],
            ready: true,
        }
    }

    #[inline]
    pub fn mc_req_valid(&self, m: usize) -> bool {
        self.mc_req_valid_out[m]
    pub fn kl_tx_channel(&self, port: usize) -> KarstLinkChannel
    {
        self.KlTxChannel(port)
    }

    #[inline]
    pub fn mc_req_data(&self, m: usize) -> u64 {
        self.mc_req_data_out[m]
    pub fn McReqValid(&self, m: usize) -> bool
    {
        self._McReqValidOut[m]
    }

    #[inline]
    pub fn mc_resp_ready(&self, m: usize) -> bool {
        self.mc_resp_ready_out[m]
    pub fn mc_req_valid(&self, m: usize) -> bool
    {
        self.McReqValid(m)
    }
    fn update_outputs(&mut self) {
        for t in 0..K_KL_PORTS_PER_HIND {
            let mut tx_valid = false;
            let mut tx_data = 0u64;
            if !self._kl_tx_queue[t].IsEmpty() {
                tx_valid = true;
                tx_data = *self._kl_tx_queue[t].Front().unwrap();

    #[inline]
    pub fn McReqData(&self, m: usize) -> u64
    {
        self._McReqDataOut[m]
    }

    #[inline]
    pub fn mc_req_data(&self, m: usize) -> u64
    {
        self.McReqData(m)
    }

    #[inline]
    pub fn McRespReady(&self, m: usize) -> bool
    {
        self._McRespReadyOut[m]
    }

    #[inline]
    pub fn mc_resp_ready(&self, m: usize) -> bool
    {
        self.McRespReady(m)
    }

    pub fn UpdateOutputs(&mut self)
    {
        USeg::FromLen(K_KL_PORTS_PER_HIND as u32).Traverse(|t_u| {
            let t = t_u as usize;
            let mut txValid = false;
            let mut txData = 0u64;
            if !self._KlTxQueue[t].IsEmpty() {
                txValid = true;
                txData = *self._KlTxQueue[t].Front().unwrap();
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
            self._LastKlTxPresented[t] = txValid;
            self._KlRxReadyOut[t] = !self._KlRxQueue[t].IsFull();
            self._KlTxValidOut[t] = txValid;
            self._KlTxDataOut[t] = txData;
        });

        USeg::FromLen(K_MC_PORTS_PER_HIND as u32).Traverse(|m_u| {
            let m = m_u as usize;
            let mut reqValid = false;
            let mut reqData = 0u64;
            if !self._McReqQueue[m].IsEmpty() {
                reqValid = true;
                reqData = *self._McReqQueue[m].Front().unwrap();
            }
            self._last_mc_req_presented[m] = req_valid;
            self.mc_resp_ready_out[m] = !self._mc_resp_queue[m].IsFull();
            self.mc_req_valid_out[m] = req_valid;
            self.mc_req_data_out[m] = req_data;
        }
            self._LastMcReqPresented[m] = reqValid;
            self._McRespReadyOut[m] = !self._McRespQueue[m].IsFull();
            self._McReqValidOut[m] = reqValid;
            self._McReqDataOut[m] = reqData;
        });
    }
    #[allow(clippy::needless_range_loop)]
    pub fn step(

    #[inline]
    pub fn update_outputs(&mut self)
    {
        self.UpdateOutputs();
    }

    pub fn Step(
        &mut self,
        kl_rx_valid: &[bool; K_KL_PORTS_PER_HIND],
        kl_rx_data: &[u64; K_KL_PORTS_PER_HIND],
        kl_tx_ready: &[bool; K_KL_PORTS_PER_HIND],
        mc_req_ready: &[bool; K_MC_PORTS_PER_HIND],
        mc_resp_valid: &[bool; K_MC_PORTS_PER_HIND],
        mc_resp_data: &[u64; K_MC_PORTS_PER_HIND],
    ) {
        klRxValid: &[bool; K_KL_PORTS_PER_HIND],
        klRxData: &[u64; K_KL_PORTS_PER_HIND],
        klTxReady: &[bool; K_KL_PORTS_PER_HIND],
        mcReqReady: &[bool; K_MC_PORTS_PER_HIND],
        mcRespValid: &[bool; K_MC_PORTS_PER_HIND],
        mcRespData: &[u64; K_MC_PORTS_PER_HIND],
    )
    {
        // 1. Retire successfully transferred outputs
        for m in 0..K_MC_PORTS_PER_HIND {
            if self._last_mc_req_presented[m] && mc_req_ready[m] && !self._mc_req_queue[m].IsEmpty()
            {
                self._mc_req_queue[m].PopFront();
        USeg::FromLen(K_MC_PORTS_PER_HIND as u32).Traverse(|m_u| {
            let m = m_u as usize;
            if self._LastMcReqPresented[m] && mcReqReady[m] && !self._McReqQueue[m].IsEmpty() {
                self._McReqQueue[m].PopFront();
            }
        }
        for t in 0..K_KL_PORTS_PER_HIND {
            if self._last_kl_tx_presented[t] && kl_tx_ready[t] && !self._kl_tx_queue[t].IsEmpty() {
                self._kl_tx_queue[t].PopFront();
        });
        USeg::FromLen(K_KL_PORTS_PER_HIND as u32).Traverse(|t_u| {
            let t = t_u as usize;
            if self._LastKlTxPresented[t] && klTxReady[t] && !self._KlTxQueue[t].IsEmpty() {
                self._KlTxQueue[t].PopFront();
            }
        }
        });

        // 2. Capture ingress independently before routing to shared queues
        for t in 0..K_KL_PORTS_PER_HIND {
            if kl_rx_valid[t] && !self._kl_rx_queue[t].IsFull() {
                self._kl_rx_queue[t].PushBack(kl_rx_data[t]);
        USeg::FromLen(K_KL_PORTS_PER_HIND as u32).Traverse(|t_u| {
            let t = t_u as usize;
            if klRxValid[t] && !self._KlRxQueue[t].IsFull() {
                self._KlRxQueue[t].PushBack(klRxData[t]);
            }
        }
        });

        // 3. Capture MC responses independently before routing them to shared links
        for m in 0..K_MC_PORTS_PER_HIND {
            if mc_resp_valid[m] && !self._mc_resp_queue[m].IsFull() {
                self._mc_resp_queue[m].PushBack(mc_resp_data[m]);
        USeg::FromLen(K_MC_PORTS_PER_HIND as u32).Traverse(|m_u| {
            let m = m_u as usize;
            if mcRespValid[m] && !self._McRespQueue[m].IsFull() {
                self._McRespQueue[m].PushBack(mcRespData[m]);
            }
        }
        });

        // 4. Route buffered ingress to its selected output queue
        for t in 0..K_KL_PORTS_PER_HIND {
            if !self._kl_rx_queue[t].IsEmpty() {
                let raw = *self._kl_rx_queue[t].Front().unwrap();
        USeg::FromLen(K_KL_PORTS_PER_HIND as u32).Traverse(|t_u| {
            let t = t_u as usize;
            if !self._KlRxQueue[t].IsEmpty() {
                let raw = *self._KlRxQueue[t].Front().unwrap();
                let flit = KarstFlit::Unpack(raw);
                let target_die = (flit._Addr >> K_DIE_ADDR_BIT) & 1;
                if target_die == self._die_id {
                    let mc_idx = ((flit._Addr >> K_MC_ADDR_SHIFT) & K_MC_ADDR_MASK) as usize;
                    if !self._mc_req_queue[mc_idx].IsFull() {
                        self._mc_req_queue[mc_idx].PushBack(raw);
                        self._kl_rx_queue[t].PopFront();
                let targetDie = (flit._Addr >> K_DIE_ADDR_BIT) & 1;
                if targetDie == self._DieId {
                    let mcIdx = ((flit._Addr >> K_MC_ADDR_SHIFT) & K_MC_ADDR_MASK) as usize;
                    if !self._McReqQueue[mcIdx].IsFull() {
                        self._McReqQueue[mcIdx].PushBack(raw);
                        self._KlRxQueue[t].PopFront();
                    }
                } else if !self._kl_tx_queue[K_INTERDIE_PORT_BASE].IsFull() {
                    self._kl_tx_queue[K_INTERDIE_PORT_BASE].PushBack(raw);
                    self._kl_rx_queue[t].PopFront();
                } else if !self._KlTxQueue[K_INTERDIE_PORT_BASE].IsFull() {
                    self._KlTxQueue[K_INTERDIE_PORT_BASE].PushBack(raw);
                    self._KlRxQueue[t].PopFront();
                }
            }
        }
        for m in 0..K_MC_PORTS_PER_HIND {
            if !self._mc_resp_queue[m].IsEmpty() {
                let raw = *self._mc_resp_queue[m].Front().unwrap();
        });

        USeg::FromLen(K_MC_PORTS_PER_HIND as u32).Traverse(|m_u| {
            let m = m_u as usize;
            if !self._McRespQueue[m].IsEmpty() {
                let raw = *self._McRespQueue[m].Front().unwrap();
                let flit = KarstFlit::Unpack(raw);
                let target_kl = if self._die_id == 0 {
                let targetKl = if self._DieId == 0 {
                    (flit._SrcId % (K_HOSTS_PER_FABRIC as u8)) as usize
                } else if flit._SrcId >= (K_HOSTS_PER_HIND as u8) {
                    ((flit._SrcId - (K_HOSTS_PER_HIND as u8)) % (K_HOSTS_PER_FABRIC as u8)) as usize
                } else {
                    ((flit._SrcId + (K_HOSTS_PER_HIND as u8)) % (K_HOSTS_PER_FABRIC as u8)) as usize
                };
                if target_kl < K_KL_PORTS_PER_HIND && !self._kl_tx_queue[target_kl].IsFull() {
                    self._kl_tx_queue[target_kl].PushBack(raw);
                    self._mc_resp_queue[m].PopFront();
                if targetKl < K_KL_PORTS_PER_HIND && !self._KlTxQueue[targetKl].IsFull() {
                    self._KlTxQueue[targetKl].PushBack(raw);
                    self._McRespQueue[m].PopFront();
                }
            }
        }
        });

        // 5. Drive outputs for the next cycle
        self.update_outputs();
        self.UpdateOutputs();
    }

    #[inline]
    pub fn step(
        &mut self,
        klRxValid: &[bool; K_KL_PORTS_PER_HIND],
        klRxData: &[u64; K_KL_PORTS_PER_HIND],
        klTxReady: &[bool; K_KL_PORTS_PER_HIND],
        mcReqReady: &[bool; K_MC_PORTS_PER_HIND],
        mcRespValid: &[bool; K_MC_PORTS_PER_HIND],
        mcRespData: &[u64; K_MC_PORTS_PER_HIND],
    )
    {
        self.Step(klRxValid, klRxData, klTxReady, mcReqReady, mcRespValid, mcRespData);
    }
}
