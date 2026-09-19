// src/karst/vpu.rs
use	crate::karst::memchan::MemChan;
use	crate::swarm::cpu::ComputeDevice;
use	crate::swarm::traits::{ ComputeKernel, SwarmError, WorkgroupDim };
use	std::sync::atomic::{ AtomicU32, Ordering };

//-------------------------------------------------------------------------------------------------
// Vpu — Vector / Near-Memory Processing Unit for compute on DDR5 channels via Swarm kernels.
// Renamed from Epu per architecture specification.
pub struct Vpu
{
    _vpu_idx: u32,
    _kernel: ComputeKernel,
    _dispatches: AtomicU32,
}
impl Vpu
{
    pub fn	new( vpu_idx: u32) -> Self
    {
        Self {
            _vpu_idx: vpu_idx,
            _kernel: ComputeDevice::DoubleKernel(),
            _dispatches: AtomicU32::new( 0),
        }
    }
    #[inline]
    pub fn	vpu_idx( &self) -> u32
    {
        self._vpu_idx
    }
    #[inline]
    pub fn	dispatches( &self) -> u32
    {
        self._dispatches.load( Ordering::Acquire)
    }
    pub fn	dispatch( 
        &self,
        device: &ComputeDevice,
        chan: &MemChan,
        dim: WorkgroupDim,
    ) -> Result< (), SwarmError>
    {
        let  	bufs = [chan.buffer()];
        let  	res = device.Dispatch( &self._kernel, &bufs, dim);
        if res.is_ok() {
            self._dispatches.fetch_add( 1, Ordering::Release);
        }
        res
    }
}
