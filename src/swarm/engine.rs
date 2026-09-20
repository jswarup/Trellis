// engine.rs -------------------------------------------------------------------------------------------------------
use	crate::swarm::cpu::ComputeDevice;
use	crate::swarm::backend::IComputeBackend;
use	crate::silo::Arr;
use	crate::swarm::ops::{ StandardOp, StandardOpLabel };
use	crate::swarm::traits::{ BackendKind, ComputeBuffer, SwarmError, WorkgroupDim };

//-------------------------------------------------------------------------------------------------
// SwarmEngine: unified high-level compute engine.
// Modeled directly from Trellis swarm/engine.h.
pub struct SwarmEngine
{
    _Device: ComputeDevice,
}
impl Default for SwarmEngine {
    fn	default() -> Self
    {
        Self::Auto()
    }
}
impl SwarmEngine
{
    pub fn	New( backend: BackendKind) -> Self
    {
        Self {
            _Device: ComputeDevice::WithBackend( backend, 0),
        }
    }
    pub fn	Auto() -> Self
    {
        // Prioritize CPU as the fully implemented reference backend in Segue
        Self::New( BackendKind::Cpu)
    }
    pub fn	Device( &self) -> &ComputeDevice
    {
        &self._Device
    }
    pub fn	DeviceMut( &mut self) -> &mut ComputeDevice
    {
        &mut self._Device
    }
    pub fn	Backend( &self) -> BackendKind
    {
        self._Device.Backend()
    }
    pub fn	ExecuteOp( 
        &self, op: StandardOp, buffers: Arr< '_, &ComputeBuffer>, dim: WorkgroupDim,
    ) -> Result< (), SwarmError>
    {
        self.ExecuteWith( &self._Device, op, buffers, dim)
    }

    /// Compile and dispatch through an explicitly supplied backend.
    ///
    /// Backends may cache kernels in `CompileKernel`. Call `Synchronize` after
    /// one or more dispatches when the caller needs completed GPU work.
    pub fn	DispatchWith< B>(
        &self, backend: &B, op: StandardOp, buffers: Arr< '_, &B::Buffer>, dim: WorkgroupDim,
    ) -> Result< (), SwarmError>
    where
        B: IComputeBackend,
    {
        let   label = StandardOpLabel( op);
        let   entry_point = backend.EntryPoint( op);
        let   source = backend.Source( op)?;
        let   kernel = backend.CompileKernel( label, entry_point, &source)?;
        backend.Dispatch( &kernel, buffers, dim)
    }

    /// Execute and wait for completion through an explicitly supplied backend.
    pub fn	ExecuteWith< B>(
        &self, backend: &B, op: StandardOp, buffers: Arr< '_, &B::Buffer>, dim: WorkgroupDim,
    ) -> Result< (), SwarmError>
    where
        B: IComputeBackend,
    {
        self.DispatchWith( backend, op, buffers, dim)?;
        backend.Synchronize()
    }
}
