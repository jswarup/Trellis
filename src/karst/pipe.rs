// src/karst/pipe.rs
use	crate::karst::config::K_LINK_DEPTH;
use	crate::silo::fifo::Fifo;

//-------------------------------------------------------------------------------------------------
// KarstPipe — retiming pipeline module implementing mpipe pipeline_axi_channel FIFO semantics.
// Statically staged to model deterministic cycle-accurate delay and backpressure over long routes.
pub struct KarstPipe
{
    _fifo: Fifo< u64, K_LINK_DEPTH>,
    _was_presented: bool,
    _up_ready: bool,
    _out_valid: bool,
    _out_data: u64,
}
impl Default for KarstPipe {
    fn	default() -> Self
    {
        Self::new()
    }
}
impl KarstPipe
{
    pub fn	new() -> Self
    {
        Self {
            _fifo: Fifo::New(),
            _was_presented: false,
            _up_ready: true,
            _out_valid: false,
            _out_data: 0,
        }
    }
    #[inline]
    pub fn	is_empty( &self) -> bool
    {
        self._fifo.IsEmpty()
    }
    #[inline]
    pub fn	is_full( &self) -> bool
    {
        self._fifo.IsFull()
    }
    #[inline]
    pub fn	size( &self) -> u32
    {
        self._fifo.Size()
    }
    #[inline]
    pub fn	up_ready( &self) -> bool
    {
        self._up_ready
    }
    #[inline]
    pub fn	out_valid( &self) -> bool
    {
        self._out_valid
    }
    #[inline]
    pub fn	out_data( &self) -> u64
    {
        self._out_data
    }
    pub fn	step( &mut self, in_valid: bool, in_data: u64, down_ready: bool)
    {
        if self._was_presented && down_ready && !self._fifo.IsEmpty() {
            self._fifo.PopFront();
        }
        if in_valid && !self._fifo.IsFull() {
            self._fifo.PushBack( in_data);
        }
        if !self._fifo.IsEmpty() {
            self._out_valid = true;
            self._out_data = *self._fifo.Front().unwrap();
        } else {
            self._out_valid = false;
            self._out_data = 0;
        }
        self._was_presented = self._out_valid;
        self._up_ready = !self._fifo.IsFull();
    }
}
