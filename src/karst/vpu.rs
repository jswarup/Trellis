// src/karst/vpu.rs
use crate::karst::memchan::MemChan;
use crate::karst::memchan::KarstDChan;
use crate::swarm::cpu::ComputeDevice;
use crate::swarm::traits::{ComputeKernel, SwarmError, WorkgroupDim};
use std::sync::atomic::{AtomicU32, Ordering};

//-------------------------------------------------------------------------------------------------

// Vpu — Vector / Near-Memory Processing Unit for compute on DDR5 channels via Swarm kernels.
// Renamed from Epu per architecture specification.
pub struct Vpu {
    _vpu_idx: u32,
    _kernel: ComputeKernel,
    _dispatches: AtomicU32,
// KarstVPU — Edge / Near-Memory Processing Unit for compute on DDR5 channels via Swarm kernels.
pub struct KarstVPU
{
    _VPUIdx: u32,
    _Kernel: ComputeKernel,
    _Dispatches: AtomicU32,
}
impl Vpu {
    pub fn new(vpu_idx: u32) -> Self {

pub type Vpu = KarstVPU;
pub type Epu = KarstVPU;

impl KarstVPU
{
    pub fn New(vpuIdx: u32) -> Self
    {
        Self {
            _vpu_idx: vpu_idx,
            _kernel: ComputeDevice::DoubleKernel(),
            _dispatches: AtomicU32::new(0),
            _VPUIdx: vpuIdx,
            _Kernel: ComputeDevice::DoubleKernel(),
            _Dispatches: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn vpu_idx(&self) -> u32 {
        self._vpu_idx
    pub fn new(vpuIdx: u32) -> Self
    {
        Self::New(vpuIdx)
    }

    #[inline]
    pub fn dispatches(&self) -> u32 {
        self._dispatches.load(Ordering::Acquire)
    pub fn VPUIdx(&self) -> u32
    {
        self._VPUIdx
    }
    pub fn dispatch(

    #[inline]
    pub fn vpu_idx(&self) -> u32
    {
        self.VPUIdx()
    }

    #[inline]
    pub fn Dispatches(&self) -> u32
    {
        self._Dispatches.load(Ordering::Acquire)
    }

    #[inline]
    pub fn dispatches(&self) -> u32
    {
        self.Dispatches()
    }

    pub fn Dispatch(
        &self,
        device: &ComputeDevice,
        chan: &MemChan,
        chan: &KarstDChan,
        dim: WorkgroupDim,
    ) -> Result<(), SwarmError> {
        let bufs = [chan.buffer()];
        let res = device.Dispatch(&self._kernel, &bufs, dim);
    ) -> Result<(), SwarmError>
    {
        let bufs = [chan.Buffer()];
        let res = device.Dispatch(&self._Kernel, &bufs, dim);
        if res.is_ok() {
            self._dispatches.fetch_add(1, Ordering::Release);
            self._Dispatches.fetch_add(1, Ordering::Release);
        }
        res
    }

    #[inline]
    pub fn dispatch(
        &self,
        device: &ComputeDevice,
        chan: &KarstDChan,
        dim: WorkgroupDim,
    ) -> Result<(), SwarmError>
    {
        self.Dispatch(device, chan, dim)
    }
}
