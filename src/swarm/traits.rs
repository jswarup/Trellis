// traits.rs -------------------------------------------------------------------------------------------------------
pub use	crate::flock::CpuKernelFn;
use	crate::silo::{ Arr, Buff, MutArr };
use	crate::stalks::work::SpinMutex;
use	crate::symph::StandardOp;
use	std::fmt;
use	std::ops::{ BitOr, BitOrAssign };

//-------------------------------------------------------------------------------------------------
// Target hardware / runtime backend kind.
// Modeled directly from Trellis swarm/traits.h.
#[derive( Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendKind {
    #[default]
    Cpu,
    RustGpu,
    CudaOxide,
}
impl BackendKind
{
    pub const fn	Str( &self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::RustGpu => "Rust-GPU (WebGPU/SPIR-V)",
            Self::CudaOxide => "Cuda-Oxide (CUDA/PTX)",
        }
    }
}
impl fmt::Display for BackendKind {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        write!( f, "{}", self.Str())
    }
}

//-------------------------------------------------------------------------------------------------
// Buffer usage flags for host and device memory management.
#[derive( Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BufferUsage
{
    pub bits: u32,
}
impl BufferUsage
{
    pub const STORAGE: Self = Self { bits: 1 << 0 };
    pub const UNIFORM: Self = Self { bits: 1 << 1 };
    pub const READ_ONLY: Self = Self { bits: 1 << 2 };
    pub const READ_WRITE: Self = Self { bits: 1 << 3 };
    pub const COPY_SRC: Self = Self { bits: 1 << 4 };
    pub const COPY_DST: Self = Self { bits: 1 << 5 };
    pub const fn	Storage() -> Self
    {
        Self::STORAGE
    }
    pub const fn	Uniform() -> Self
    {
        Self::UNIFORM
    }
    pub const fn	ReadOnly() -> Self
    {
        Self::READ_ONLY
    }
    pub const fn	ReadWrite() -> Self
    {
        Self::READ_WRITE
    }
    pub const fn	CopySrc() -> Self
    {
        Self::COPY_SRC
    }
    pub const fn	CopyDst() -> Self
    {
        Self::COPY_DST
    }
    pub const fn	Contains( &self, other: Self) -> bool
    {
        ( self.bits & other.bits) == other.bits
    }
}
impl BitOr for BufferUsage {
    type Output = Self;
    fn	bitor( self, rhs: Self) -> Self
    {
        Self {
            bits: self.bits | rhs.bits,
        }
    }
}
impl BitOrAssign for BufferUsage {
    fn	bitor_assign( &mut self, rhs: Self)
    {
        self.bits |= rhs.bits;
    }
}

//-------------------------------------------------------------------------------------------------
// 3D workgroup / threadblock dispatch dimensions.
#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkgroupDim
{
    pub _X: u32,
    pub _Y: u32,
    pub _Z: u32,
}
impl Default for WorkgroupDim {
    fn	default() -> Self
    {
        Self {
            _X: 1,
            _Y: 1,
            _Z: 1,
        }
    }
}
impl WorkgroupDim
{
    pub const fn	New( x: u32, y: u32, z: u32) -> Self
    {
        Self {
            _X: x,
            _Y: y,
            _Z: z,
        }
    }
    pub const fn	Linear( x: u32) -> Self
    {
        Self {
            _X: x,
            _Y: 1,
            _Z: 1,
        }
    }
    pub const fn	Total( &self) -> u64
    {
        ( self._X as u64) * ( self._Y as u64) * ( self._Z as u64)
    }
}

//-------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------
// Unified compute kernel source representation.
#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelSourceKind {
    Wgsl,
    SpirV,
    Ptx,
    CpuClosure,
}
#[derive( Clone)]
pub struct KernelSource
{
    pub _Kind: KernelSourceKind,
    pub _CodeStr: String,
    pub _ByteCode: Buff< u8>,
    pub _Closure: Option< CpuKernelFn>,
    _StandardOp: Option< StandardOp>,
}
impl KernelSource
{
    pub fn	Wgsl( code: impl Into< String>) -> Self
    {
        Self {
            _Kind: KernelSourceKind::Wgsl,
            _CodeStr: code.into(),
            _ByteCode: Buff::WithCapacity( 0),
            _Closure: None,
            _StandardOp: None,
        }
    }
    pub fn	SpirV( bytes: Arr< '_, u8>) -> Self
    {
        Self {
            _Kind: KernelSourceKind::SpirV,
            _CodeStr: String::new(),
            _ByteCode: Buff::FromDispenser( bytes.Len(), |i| bytes[i]),
            _Closure: None,
            _StandardOp: None,
        }
    }
    pub fn	Ptx( code: impl Into< String>) -> Self
    {
        Self {
            _Kind: KernelSourceKind::Ptx,
            _CodeStr: code.into(),
            _ByteCode: Buff::WithCapacity( 0),
            _Closure: None,
            _StandardOp: None,
        }
    }
    pub fn	Cpu( closure: CpuKernelFn) -> Self
    {
        Self {
            _Kind: KernelSourceKind::CpuClosure,
            _CodeStr: String::new(),
            _ByteCode: Buff::WithCapacity( 0),
            _Closure: Some( closure),
            _StandardOp: None,
        }
    }
    pub fn	CpuStandard( op: StandardOp, closure: CpuKernelFn) -> Self
    {
        Self {
            _Kind: KernelSourceKind::CpuClosure,
            _CodeStr: String::new(),
            _ByteCode: Buff::WithCapacity( 0),
            _Closure: Some( closure),
            _StandardOp: Some( op),
        }
    }
    pub fn	StandardOp( &self) -> Option< StandardOp>
    {
        self._StandardOp
    }
}

//-------------------------------------------------------------------------------------------------
// Error types occurring during compute operations.
#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmErrorKind {
    None,
    DeviceUnavailable,
    CompilationError,
    BufferError,
    ExecutionError,
    UnsupportedBackend,
    InvalidKernelSource,
}
#[derive( Debug, Clone, PartialEq, Eq)]
pub struct SwarmError
{
    pub _Kind: SwarmErrorKind,
    pub _Message: String,
}
impl SwarmError
{
    pub const fn	Ok() -> Self
    {
        Self {
            _Kind: SwarmErrorKind::None,
            _Message: String::new(),
        }
    }
    pub fn	DeviceUnavailable( msg: impl Into< String>) -> Self
    {
        Self {
            _Kind: SwarmErrorKind::DeviceUnavailable,
            _Message: msg.into(),
        }
    }
    pub fn	CompilationError( msg: impl Into< String>) -> Self
    {
        Self {
            _Kind: SwarmErrorKind::CompilationError,
            _Message: msg.into(),
        }
    }
    pub fn	BufferError( msg: impl Into< String>) -> Self
    {
        Self {
            _Kind: SwarmErrorKind::BufferError,
            _Message: msg.into(),
        }
    }
    pub fn	ExecutionError( msg: impl Into< String>) -> Self
    {
        Self {
            _Kind: SwarmErrorKind::ExecutionError,
            _Message: msg.into(),
        }
    }
    pub fn	UnsupportedBackend( backend: BackendKind) -> Self
    {
        Self {
            _Kind: SwarmErrorKind::UnsupportedBackend,
            _Message: format!( "Unsupported compute backend: {}", backend),
        }
    }
    pub fn	InvalidKernelSource( msg: impl Into< String>) -> Self
    {
        Self {
            _Kind: SwarmErrorKind::InvalidKernelSource,
            _Message: msg.into(),
        }
    }
    pub fn	IsOk( &self) -> bool
    {
        self._Kind == SwarmErrorKind::None
    }
    pub fn	IsError( &self) -> bool
    {
        self._Kind != SwarmErrorKind::None
    }
}
impl fmt::Display for SwarmError {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        write!( f, "{:?}: {}", self._Kind, self._Message)
    }
}
impl std::error::Error for SwarmError {}

//-------------------------------------------------------------------------------------------------
// In-memory host/device compute buffer for SIMT execution.
pub struct ComputeBuffer
{
    _Label: String,
    _Data: SpinMutex< Buff< u8>>,
    _Usage: BufferUsage,
    _Backend: BackendKind,
}
impl ComputeBuffer
{
    pub fn	New( label: &str, size: usize, usage: BufferUsage, backend: BackendKind) -> Self
    {
        let  	buff = Buff::FromDispenser( size as u32, |_| 0u8);
        Self {
            _Label: label.to_string(),
            _Data: SpinMutex::New( buff),
            _Usage: usage,
            _Backend: backend,
        }
    }
    pub fn	WithData( label: &str, data: Arr< '_, u8>, usage: BufferUsage, backend: BackendKind) -> Self
    {
        let  	buff = Buff::FromDispenser( data.Len(), |i| data[i]);
        Self {
            _Label: label.to_string(),
            _Data: SpinMutex::New( buff),
            _Usage: usage,
            _Backend: backend,
        }
    }
    pub fn	Size( &self) -> usize
    {
        self._Data.Lock().Cap() as usize
    }
    pub fn	Label( &self) -> &str
    {
        &self._Label
    }
    pub fn	Usage( &self) -> BufferUsage
    {
        self._Usage
    }
    pub fn	Backend( &self) -> BackendKind
    {
        self._Backend
    }
    pub fn	Write( &self, data: Arr< '_, u8>) -> Result< (), SwarmError>
    {
        if self._Backend != BackendKind::Cpu {
            return Err( SwarmError::UnsupportedBackend( self._Backend));
        }
        let  	mut buff = self._Data.Lock();
        if data.Len() > buff.Cap() {
            *buff = Buff::FromDispenser( data.Len(), |i| data[i]);
        } else {
            data.USeg().Traverse( |i| buff[i] = data[i]);
        }
        Ok( ())
    }
    pub fn	Read( &self) -> Buff< u8>
    {
        if self._Backend != BackendKind::Cpu {
            return Buff::New();
        }
        let  	buff = self._Data.Lock();
        Buff::FromDispenser( buff.Cap(), |i| buff[i])
    }
    pub fn	WriteAt( &self, offset: u32, data: Arr< '_, u8>) -> Result< (), SwarmError>
    {
        if self._Backend != BackendKind::Cpu {
            return Err( SwarmError::UnsupportedBackend( self._Backend));
        }
        let  	mut buff = self._Data.Lock();
        let  	cap = buff.Cap();
        if offset + data.Len() <= cap {
            data.USeg().Traverse( |i| buff[offset + i] = data[i]);
            Ok( ())
        } else {
            Err( SwarmError::BufferError( "WriteAt: offset out of bounds"))
        }
    }
    pub fn	ReadAt( &self, offset: u32, mut dest: MutArr< '_, u8>) -> Result< (), SwarmError>
    {
        if self._Backend != BackendKind::Cpu {
            return Err( SwarmError::UnsupportedBackend( self._Backend));
        }
        let  	buff = self._Data.Lock();
        let  	cap = buff.Cap();
        if offset + dest.Len() <= cap {
            dest.USeg().Traverse( |i| dest[i] = buff[offset + i]);
            Ok( ())
        } else {
            Err( SwarmError::BufferError( "ReadAt: offset out of bounds"))
        }
    }
    pub fn	Fill( &self, pattern: u8) -> Result< (), SwarmError>
    {
        if self._Backend != BackendKind::Cpu {
            return Err( SwarmError::UnsupportedBackend( self._Backend));
        }
        let  	mut buff = self._Data.Lock();
        let  	slice: &mut [u8] = buff.MutArr().into();
        slice.fill( pattern);
        Ok( ())
    }
    pub fn	Verify( &self, pattern: u8) -> bool
    {
        if self._Backend != BackendKind::Cpu {
            return false;
        }
        let  	buff = self._Data.Lock();
        let  	slice: &[u8] = buff.Arr().into();
        !slice.is_empty() && slice.iter().all( |&b| b == pattern)
    }
    pub(crate) fn	WithMut< R>( &self, action: impl FnOnce( MutArr< '_, u8>) -> R) -> Result< R, SwarmError>
    {
        if self._Backend != BackendKind::Cpu {
            return Err( SwarmError::UnsupportedBackend( self._Backend));
        }
        let  	mut buff = self._Data.Lock();
        Ok( action( buff.MutArr()))
    }
}
pub type CpuBuffer = ComputeBuffer;
pub type IComputeBuffer = ComputeBuffer;

//-------------------------------------------------------------------------------------------------
// Compiled compute kernel closure representation.
#[derive( Clone)]
pub struct ComputeKernel
{
    _Name: String,
    _EntryPoint: String,
    _Backend: BackendKind,
    _KernelFn: Option< CpuKernelFn>,
    _StandardOp: Option< StandardOp>,
}
impl ComputeKernel
{
    pub fn	New( 
        name: impl Into< String>, entry_point: impl Into< String>, backend: BackendKind,
        kernel_fn: Option< CpuKernelFn>,
    ) -> Self
    {
        Self {
            _Name: name.into(),
            _EntryPoint: entry_point.into(),
            _Backend: backend,
            _KernelFn: kernel_fn,
            _StandardOp: None,
        }
    }
    pub fn	Standard(
        name: impl Into< String>, op: StandardOp, entry_point: impl Into< String>, backend: BackendKind,
        kernel_fn: Option< CpuKernelFn>,
    ) -> Self
    {
        Self {
            _Name: name.into(),
            _EntryPoint: entry_point.into(),
            _Backend: backend,
            _KernelFn: kernel_fn,
            _StandardOp: Some( op),
        }
    }
    pub fn	Name( &self) -> &str
    {
        &self._Name
    }
    pub fn	EntryPoint( &self) -> &str
    {
        &self._EntryPoint
    }
    pub fn	Backend( &self) -> BackendKind
    {
        self._Backend
    }
    pub fn	KernelFn( &self) -> Option< &CpuKernelFn>
    {
        self._KernelFn.as_ref()
    }
    pub fn	StandardOp( &self) -> Option< StandardOp>
    {
        self._StandardOp
    }
    pub fn	Execute< 'i, 'o>( 
        &self, inputs: Arr< 'i, Arr<'i, u8>>, outputs: MutArr< 'o, MutArr<'o, u8>>,
        gid_x: u32, gid_y: u32, gid_z: u32,
    )
    {
        if let  	Some( ref f) = self._KernelFn {
            f( inputs, outputs, gid_x, gid_y, gid_z);
        }
    }
}
pub type CpuKernel = ComputeKernel;
pub type IComputeKernel = ComputeKernel;
