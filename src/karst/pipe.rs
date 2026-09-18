// src/karst/pipe.rs
use crate::karst::config::K_LINK_DEPTH;
use crate::silo::fifo::Fifo;

//-------------------------------------------------------------------------------------------------

// KarstPipe — retiming pipeline module implementing mpipe pipeline_axi_channel FIFO semantics.
// Statically staged to model deterministic cycle-accurate delay and backpressure over long routes.
pub struct KarstPipe {
    _fifo: Fifo<u64, K_LINK_DEPTH>,
    _was_presented: bool,
    _up_ready: bool,
    _out_valid: bool,
    _out_data: u64,
pub struct KarstPipe
{
    _Fifo: Fifo<u64, K_LINK_DEPTH>,
    _WasPresented: bool,
    _UpReady: bool,
    _OutValid: bool,
    _OutData: u64,
}
impl Default for KarstPipe {
    fn default() -> Self {
        Self::new()

impl Default for KarstPipe
{
    fn default() -> Self
    {
        Self::New()
    }
}
impl KarstPipe {
    pub fn new() -> Self {

impl KarstPipe
{
    pub fn New() -> Self
    {
        Self {
            _fifo: Fifo::New(),
            _was_presented: false,
            _up_ready: true,
            _out_valid: false,
            _out_data: 0,
            _Fifo: Fifo::New(),
            _WasPresented: false,
            _UpReady: true,
            _OutValid: false,
            _OutData: 0,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self._fifo.IsEmpty()
    pub fn new() -> Self
    {
        Self::New()
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self._fifo.IsFull()
    pub fn IsEmpty(&self) -> bool
    {
        self._Fifo.IsEmpty()
    }

    #[inline]
    pub fn size(&self) -> u32 {
        self._fifo.Size()
    pub fn is_empty(&self) -> bool
    {
        self.IsEmpty()
    }

    #[inline]
    pub fn up_ready(&self) -> bool {
        self._up_ready
    pub fn IsFull(&self) -> bool
    {
        self._Fifo.IsFull()
    }

    #[inline]
    pub fn out_valid(&self) -> bool {
        self._out_valid
    pub fn is_full(&self) -> bool
    {
        self.IsFull()
    }

    #[inline]
    pub fn out_data(&self) -> u64 {
        self._out_data
    pub fn Size(&self) -> u32
    {
        self._Fifo.Size()
    }
    pub fn step(&mut self, in_valid: bool, in_data: u64, down_ready: bool) {
        if self._was_presented && down_ready && !self._fifo.IsEmpty() {
            self._fifo.PopFront();

    #[inline]
    pub fn size(&self) -> u32
    {
        self.Size()
    }

    #[inline]
    pub fn UpReady(&self) -> bool
    {
        self._UpReady
    }

    #[inline]
    pub fn up_ready(&self) -> bool
    {
        self.UpReady()
    }

    #[inline]
    pub fn OutValid(&self) -> bool
    {
        self._OutValid
    }

    #[inline]
    pub fn out_valid(&self) -> bool
    {
        self.OutValid()
    }

    #[inline]
    pub fn OutData(&self) -> u64
    {
        self._OutData
    }

    #[inline]
    pub fn out_data(&self) -> u64
    {
        self.OutData()
    }

    pub fn Step(&mut self, inValid: bool, inData: u64, downReady: bool)
    {
        if self._WasPresented && downReady && !self._Fifo.IsEmpty() {
            self._Fifo.PopFront();
        }
        if in_valid && !self._fifo.IsFull() {
            self._fifo.PushBack(in_data);
        if inValid && !self._Fifo.IsFull() {
            self._Fifo.PushBack(inData);
        }
        if !self._fifo.IsEmpty() {
            self._out_valid = true;
            self._out_data = *self._fifo.Front().unwrap();
        if !self._Fifo.IsEmpty() {
            self._OutValid = true;
            self._OutData = *self._Fifo.Front().unwrap();
        } else {
            self._out_valid = false;
            self._out_data = 0;
            self._OutValid = false;
            self._OutData = 0;
        }
        self._was_presented = self._out_valid;
        self._up_ready = !self._fifo.IsFull();
        self._WasPresented = self._OutValid;
        self._UpReady = !self._Fifo.IsFull();
    }

    #[inline]
    pub fn step(&mut self, inValid: bool, inData: u64, downReady: bool)
    {
        self.Step(inValid, inData, downReady);
    }
}
