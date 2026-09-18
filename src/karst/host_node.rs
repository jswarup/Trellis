// src/karst/host_node.rs
use crate::karst::config::{K_DIE_ADDR_BIT, K_HOSTS_PER_HIND};
use crate::karst::link::{KarstFlit, KarstLinkChannel};
use crate::silo::fifo::Fifo;

//-------------------------------------------------------------------------------------------------

// Host request transaction.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct HostTransaction {
pub struct HostTransaction
{
    pub _Addr: u32,
    pub _Data: u32,
    pub _IsWrite: bool,
}

//-------------------------------------------------------------------------------------------------

// Host read response.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct HostResponse {
pub struct HostResponse
{
    pub _Addr: u32,
    pub _Data: u32,
}

//-------------------------------------------------------------------------------------------------

// Host performance metrics.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct HostStats {
pub struct HostStats
{
    pub _TxCount: u32,
    pub _RxCount: u32,
    pub _WritesPosted: u32,
    pub _ReadsPosted: u32,
}

//-------------------------------------------------------------------------------------------------

// KarstHostNode — KarstFore front-port IO chiplet model.
// Exposes primary and cross-home KarstLink links (Link0 and Link1).
pub struct KarstHostNode {
    _host_id: u32,
    _active_queue: Fifo<HostTransaction, 16>,
    _tx_queue: Fifo<HostTransaction, 32>,
    _rx_queue: Fifo<HostResponse, 32>,
    _stats: HostStats,
    _last_l0_tx_presented: bool,
    _last_l1_tx_presented: bool,
    _presented_is_l1: bool,
pub struct KarstHostNode
{
    _HostId: u32,
    _ActiveQueue: Fifo<HostTransaction, 16>,
    _TxQueue: Fifo<HostTransaction, 32>,
    _RxQueue: Fifo<HostResponse, 32>,
    _Stats: HostStats,
    _LastL0TxPresented: bool,
    _LastL1TxPresented: bool,
    _PresentedIsL1: bool,

    // Outbound link signals
    pub l0_tx_valid: bool,
    pub l0_tx_data: u64,
    pub l0_rx_ready: bool,
    pub l1_tx_valid: bool,
    pub l1_tx_data: u64,
    pub l1_rx_ready: bool,
}
impl KarstHostNode {
    pub fn new(host_id: u32) -> Self {

impl KarstHostNode
{
    pub fn New(hostId: u32) -> Self
    {
        Self {
            _host_id: host_id,
            _active_queue: Fifo::New(),
            _tx_queue: Fifo::New(),
            _rx_queue: Fifo::New(),
            _stats: HostStats::default(),
            _last_l0_tx_presented: false,
            _last_l1_tx_presented: false,
            _presented_is_l1: false,
            _HostId: hostId,
            _ActiveQueue: Fifo::New(),
            _TxQueue: Fifo::New(),
            _RxQueue: Fifo::New(),
            _Stats: HostStats::default(),
            _LastL0TxPresented: false,
            _LastL1TxPresented: false,
            _PresentedIsL1: false,
            l0_tx_valid: false,
            l0_tx_data: 0,
            l0_rx_ready: true,
            l1_tx_valid: false,
            l1_tx_data: 0,
            l1_rx_ready: true,
        }
    }

    #[inline]
    pub fn link0(&self) -> KarstLinkChannel {
    pub fn new(hostId: u32) -> Self
    {
        Self::New(hostId)
    }

    #[inline]
    pub fn Link0(&self) -> KarstLinkChannel
    {
        KarstLinkChannel {
            valid: self.l0_tx_valid,
            data: self.l0_tx_data,
            ready: self.l0_rx_ready,
        }
    }

    #[inline]
    pub fn link1(&self) -> KarstLinkChannel {
    pub fn link0(&self) -> KarstLinkChannel
    {
        self.Link0()
    }

    #[inline]
    pub fn Link1(&self) -> KarstLinkChannel
    {
        KarstLinkChannel {
            valid: self.l1_tx_valid,
            data: self.l1_tx_data,
            ready: self.l1_rx_ready,
        }
    }

    #[inline]
    pub fn host_id(&self) -> u32 {
        self._host_id
    pub fn link1(&self) -> KarstLinkChannel
    {
        self.Link1()
    }

    #[inline]
    pub fn stats(&self) -> HostStats {
        self._stats
    pub fn HostId(&self) -> u32
    {
        self._HostId
    }
    pub fn post_write(&mut self, addr: u32, data: u32) -> Result<(), &'static str> {
        if !self._tx_queue.IsFull() {
            self._tx_queue.PushBack(HostTransaction {

    #[inline]
    pub fn host_id(&self) -> u32
    {
        self.HostId()
    }

    #[inline]
    pub fn Stats(&self) -> HostStats
    {
        self._Stats
    }

    #[inline]
    pub fn stats(&self) -> HostStats
    {
        self.Stats()
    }

    pub fn PostWrite(&mut self, addr: u32, data: u32) -> Result<(), &'static str>
    {
        if !self._TxQueue.IsFull() {
            self._TxQueue.PushBack(HostTransaction {
                _Addr: addr,
                _Data: data,
                _IsWrite: true,
            });
            self._stats._WritesPosted += 1;
            self._Stats._WritesPosted += 1;
            Ok(())
        } else {
            Err("Host TX queue is full")
        }
    }
    pub fn post_read(&mut self, addr: u32) -> Result<(), &'static str> {
        if !self._tx_queue.IsFull() {
            self._tx_queue.PushBack(HostTransaction {

    #[inline]
    pub fn post_write(&mut self, addr: u32, data: u32) -> Result<(), &'static str>
    {
        self.PostWrite(addr, data)
    }

    pub fn PostRead(&mut self, addr: u32) -> Result<(), &'static str>
    {
        if !self._TxQueue.IsFull() {
            self._TxQueue.PushBack(HostTransaction {
                _Addr: addr,
                _Data: 0,
                _IsWrite: false,
            });
            self._stats._ReadsPosted += 1;
            self._Stats._ReadsPosted += 1;
            Ok(())
        } else {
            Err("Host TX queue is full")
        }
    }
    pub fn has_responses(&self) -> bool {
        !self._rx_queue.IsEmpty()

    #[inline]
    pub fn post_read(&mut self, addr: u32) -> Result<(), &'static str>
    {
        self.PostRead(addr)
    }
    pub fn pop_response(&mut self, out: &mut HostResponse) -> bool {
        if !self._rx_queue.IsEmpty()
            && let Some(resp) = self._rx_queue.PopFront()

    #[inline]
    pub fn HasResponses(&self) -> bool
    {
        !self._RxQueue.IsEmpty()
    }

    #[inline]
    pub fn has_responses(&self) -> bool
    {
        self.HasResponses()
    }

    pub fn PopResponse(&mut self, out: &mut HostResponse) -> bool
    {
        if !self._RxQueue.IsEmpty()
            && let Some(resp) = self._RxQueue.PopFront()
        {
            *out = resp;
            return true;
        }
        false
    }
    pub fn step(

    #[inline]
    pub fn pop_response(&mut self, out: &mut HostResponse) -> bool
    {
        self.PopResponse(out)
    }

    pub fn Step(
        &mut self,
        l0_tx_ready: bool,
        l0_rx_valid: bool,
        l0_rx_data: u64,
        l1_tx_ready: bool,
        l1_rx_valid: bool,
        l1_rx_data: u64,
    ) {
        l0TxReady: bool,
        l0RxValid: bool,
        l0RxData: u64,
        l1TxReady: bool,
        l1RxValid: bool,
        l1RxData: u64,
    )
    {
        // 1. Check completion of previous TX
        if self._last_l0_tx_presented
            && l0_tx_ready
            && !self._active_queue.IsEmpty()
            && !self._presented_is_l1
        if self._LastL0TxPresented
            && l0TxReady
            && !self._ActiveQueue.IsEmpty()
            && !self._PresentedIsL1
        {
            self._active_queue.PopFront();
            self._stats._TxCount += 1;
            self._ActiveQueue.PopFront();
            self._Stats._TxCount += 1;
        }
        if self._last_l1_tx_presented
            && l1_tx_ready
            && !self._active_queue.IsEmpty()
            && self._presented_is_l1
        if self._LastL1TxPresented
            && l1TxReady
            && !self._ActiveQueue.IsEmpty()
            && self._PresentedIsL1
        {
            self._active_queue.PopFront();
            self._stats._TxCount += 1;
            self._ActiveQueue.PopFront();
            self._Stats._TxCount += 1;
        }

        // 2. Sample incoming responses from Link0
        if l0_rx_valid && !self._rx_queue.IsFull() {
            let flit = KarstFlit::Unpack(l0_rx_data);
            self._rx_queue.PushBack(HostResponse {
        if l0RxValid && !self._RxQueue.IsFull() {
            let flit = KarstFlit::Unpack(l0RxData);
            self._RxQueue.PushBack(HostResponse {
                _Addr: flit._Addr,
                _Data: flit._Data,
            });
            self._stats._RxCount += 1;
            self._Stats._RxCount += 1;
        }

        // 3. Sample incoming responses from Link1
        if l1_rx_valid && !self._rx_queue.IsFull() {
            let flit = KarstFlit::Unpack(l1_rx_data);
            self._rx_queue.PushBack(HostResponse {
        if l1RxValid && !self._RxQueue.IsFull() {
            let flit = KarstFlit::Unpack(l1RxData);
            self._RxQueue.PushBack(HostResponse {
                _Addr: flit._Addr,
                _Data: flit._Data,
            });
            self._stats._RxCount += 1;
            self._Stats._RxCount += 1;
        }

        // 4. Fetch new transactions from staging queue
        while !self._tx_queue.IsEmpty() && !self._active_queue.IsFull() {
            let tx = self._tx_queue.PopFront().unwrap();
            self._active_queue.PushBack(tx);
        while !self._TxQueue.IsEmpty() && !self._ActiveQueue.IsFull() {
            let tx = self._TxQueue.PopFront().unwrap();
            self._ActiveQueue.PushBack(tx);
        }

        // 5. Present outgoing request
        let mut l0_tx_valid = false;
        let mut l0_tx_data = 0u64;
        let mut l1_tx_valid = false;
        let mut l1_tx_data = 0u64;
        if !self._active_queue.IsEmpty() {
            let tx = *self._active_queue.Front().unwrap();
            let raw = KarstFlit::Pack(tx._Addr, tx._Data, self._host_id as u8, tx._IsWrite);
            let target_die = (tx._Addr >> K_DIE_ADDR_BIT) & 1;
            let primary_die = if self._host_id < (K_HOSTS_PER_HIND as u32) {
        let mut l0TxValid = false;
        let mut l0TxData = 0u64;
        let mut l1TxValid = false;
        let mut l1TxData = 0u64;
        if !self._ActiveQueue.IsEmpty() {
            let tx = *self._ActiveQueue.Front().unwrap();
            let raw = KarstFlit::Pack(tx._Addr, tx._Data, self._HostId as u8, tx._IsWrite);
            let targetDie = (tx._Addr >> K_DIE_ADDR_BIT) & 1;
            let primaryDie = if self._HostId < (K_HOSTS_PER_HIND as u32) {
                0
            } else {
                1
            };
            if target_die == primary_die {
                l0_tx_valid = true;
                l0_tx_data = raw;
                self._presented_is_l1 = false;
            if targetDie == primaryDie {
                l0TxValid = true;
                l0TxData = raw;
                self._PresentedIsL1 = false;
            } else {
                l1_tx_valid = true;
                l1_tx_data = raw;
                self._presented_is_l1 = true;
                l1TxValid = true;
                l1TxData = raw;
                self._PresentedIsL1 = true;
            }
        }
        self._last_l0_tx_presented = l0_tx_valid;
        self._last_l1_tx_presented = l1_tx_valid;
        self.l0_tx_valid = l0_tx_valid;
        self.l0_tx_data = l0_tx_data;
        self._LastL0TxPresented = l0TxValid;
        self._LastL1TxPresented = l1TxValid;
        self.l0_tx_valid = l0TxValid;
        self.l0_tx_data = l0TxData;
        self.l0_rx_ready = true;
        self.l1_tx_valid = l1_tx_valid;
        self.l1_tx_data = l1_tx_data;
        self.l1_tx_valid = l1TxValid;
        self.l1_tx_data = l1TxData;
        self.l1_rx_ready = true;
    }

    #[inline]
    pub fn step(
        &mut self,
        l0TxReady: bool,
        l0RxValid: bool,
        l0RxData: u64,
        l1TxReady: bool,
        l1RxValid: bool,
        l1RxData: u64,
    )
    {
        self.Step(l0TxReady, l0RxValid, l0RxData, l1TxReady, l1RxValid, l1RxData);
    }
}
