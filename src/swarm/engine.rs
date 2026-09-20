// engine.rs -------------------------------------------------------------------------------------------------------
use	crate::swarm::cpu::ComputeDevice;
use	crate::silo::Arr;
use	crate::swarm::ops::{ StandardOp, StandardOpEntryPoint, StandardOpKernelSource, StandardOpLabel };
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
        let  	label = StandardOpLabel( op);
        let  	entry_point = StandardOpEntryPoint( op, self.Backend());
        let  	source = StandardOpKernelSource( op, self.Backend());
        let  	kernel = self._Device.CompileKernel( label, entry_point, &source)?;
        self._Device.Dispatch( &kernel, buffers, dim)?;
        self._Device.Synchronize()
    }
}
