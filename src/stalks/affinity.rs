//-- affinity.rs -----------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

//--------------------------------------------------------------------------------------------------
/// Bitmask representing CPU core affinity for thread execution and NUMA pinning.
#[derive( Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct CpuCoreMask
{
    _Bits: u64,
}

impl CpuCoreMask
{
    pub const fn	New( bits: u64) -> Self
    {
        Self { _Bits: bits }
    }

    pub const fn	All() -> Self
    {
        Self { _Bits: u64::MAX }
    }

    pub const fn	Single( coreId: u32) -> Self
    {
        if coreId < 64
        {
            Self {
                _Bits: 1u64 << coreId,
            }
        }
        else
        {
            Self { _Bits: 0 }
        }
    }

    pub const fn	Range( startCore: u32, count: u32) -> Self
    {
        if startCore >= 64 || count == 0
        {
            return Self { _Bits: 0 };
        }
        let  	effCount = if startCore + count > 64 { 64 - startCore } else { count };
        let  	mask = if effCount == 64 {
            u64::MAX
        } else {
            ( ( 1u64 << effCount) - 1) << startCore
        };
        Self { _Bits: mask }
    }

    #[inline]
    pub const fn	Bits( &self) -> u64
    {
        self._Bits
    }

    #[inline]
    pub const fn	IsCoreSet( &self, coreId: u32) -> bool
    {
        if coreId < 64
        {
            ( self._Bits & ( 1u64 << coreId)) != 0
        }
        else
        {
            false
        }
    }

    #[inline]
    pub const fn	Count( &self) -> u32
    {
        self._Bits.count_ones()
    }
}

//--------------------------------------------------------------------------------------------------
/// Binds the current calling thread to the designated CPU core affinity mask.
pub fn	ApplyThreadAffinity( mask: &CpuCoreMask) -> bool
{
    #[cfg( target_os = "windows")]
    {
        #[link( name = "kernel32")]
        unsafe extern "system" {
            fn	GetCurrentThread() -> *mut std::ffi::c_void;
            fn	SetThreadAffinityMask(
                hThread: *mut std::ffi::c_void,
                dwThreadAffinityMask: usize,
            ) -> usize;
        }
        unsafe {
            let  	hThread = GetCurrentThread();
            let  	prev = SetThreadAffinityMask( hThread, mask.Bits() as usize);
            prev != 0
        }
    }
    #[cfg( not( target_os = "windows"))]
    {
        let  	_ = mask;
        true
    }
}

/// Binds the current thread to a single CPU core.
#[inline]
pub fn	BindWorkerToCore( coreId: u32) -> bool
{
    let  	mask = CpuCoreMask::Single( coreId);
    ApplyThreadAffinity( &mask)
}
