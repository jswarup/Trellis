//-- worker.rs -------------------------------------------------------------------------------------
//--------------------------------------------------------------------------------------------------

use	crate::karst::fabric::KarstFabric;
use	crate::karst::link::KarstFlit;
use	crate::silo::fifo::Fifo;
use	crate::stalks::affinity::{ ApplyThreadAffinity, CpuCoreMask };
use	crate::stalks::work::SpinMutex;
use	std::sync::Arc;

pub const K_DMA_RING_CAPACITY: usize = 1024;

//--------------------------------------------------------------------------------------------------
/// Configuration parameters for a Karst fabric worker thread.
#[derive( Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct KarstWorkerConfig
{
    _CoreAffinity: CpuCoreMask,
    _WorkerIndex:  u32,
    _FabricIndex:  u32,
}

impl KarstWorkerConfig
{
    pub fn	New( workerIndex: u32, fabricIndex: u32, affinity: CpuCoreMask) -> Self
    {
        Self {
            _CoreAffinity: affinity,
            _WorkerIndex:  workerIndex,
            _FabricIndex:  fabricIndex,
        }
    }

    #[inline]
    pub const fn	CoreAffinity( &self) -> &CpuCoreMask
    {
        &self._CoreAffinity
    }

    #[inline]
    pub const fn	WorkerIndex( &self) -> u32
    {
        self._WorkerIndex
    }

    #[inline]
    pub const fn	FabricIndex( &self) -> u32
    {
        self._FabricIndex
    }
}

//--------------------------------------------------------------------------------------------------
/// Zero-copy inter-fabric DMA ring buffer bypassing host memory bus contention.
pub struct KarstDmaRingBuffer
{
    _Fifo: SpinMutex< Fifo< KarstFlit, K_DMA_RING_CAPACITY>>,
}

impl Default for KarstDmaRingBuffer {
    fn	default() -> Self
    {
        Self::New()
    }
}

impl KarstDmaRingBuffer
{
    pub fn	New() -> Self
    {
        Self {
            _Fifo: SpinMutex::New( Fifo::New()),
        }
    }

    #[inline]
    pub fn	Push( &self, flit: KarstFlit) -> bool
    {
        self._Fifo.Lock().PushBack( flit)
    }

    #[inline]
    pub fn	Pop( &self) -> Option< KarstFlit>
    {
        self._Fifo.Lock().PopFront()
    }

    #[inline]
    pub fn	Size( &self) -> u32
    {
        self._Fifo.Lock().Size()
    }

    #[inline]
    pub fn	IsEmpty( &self) -> bool
    {
        self._Fifo.Lock().IsEmpty()
    }

    #[inline]
    pub fn	IsFull( &self) -> bool
    {
        self._Fifo.Lock().IsFull()
    }

    #[inline]
    pub const fn	Capacity( &self) -> u32
    {
        K_DMA_RING_CAPACITY as u32
    }
}

//--------------------------------------------------------------------------------------------------
/// Execution worker managing affinity binding and DMA transfers for an assigned Karst fabric.
pub struct KarstWorker
{
    _Config:  KarstWorkerConfig,
    _DmaRing: Option< Arc< KarstDmaRingBuffer>>,
}

impl KarstWorker
{
    pub fn	New( config: KarstWorkerConfig) -> Self
    {
        Self {
            _Config:  config,
            _DmaRing: None,
        }
    }

    pub fn	WithDma( config: KarstWorkerConfig, dma: Arc< KarstDmaRingBuffer>) -> Self
    {
        Self {
            _Config:  config,
            _DmaRing: Some( dma),
        }
    }

    #[inline]
    pub const fn	Config( &self) -> &KarstWorkerConfig
    {
        &self._Config
    }

    #[inline]
    pub fn	DmaRing( &self) -> Option< &Arc< KarstDmaRingBuffer>>
    {
        self._DmaRing.as_ref()
    }

    #[inline]
    pub fn	BindAffinity( &self) -> bool
    {
        ApplyThreadAffinity( &self._Config._CoreAffinity)
    }

    /// Drains incoming DMA flits and injects them onto the destination fabric.
    pub fn	TransferDma( &self, targetFabric: &mut KarstFabric) -> u32
    {
        let  	Some( dma) = &self._DmaRing else {
            return 0;
        };
        let  	mut count = 0u32;
        while let Some( flit) = dma.Pop()
        {
            if flit._IsWrite
            {
                targetFabric.PostHostWrite( flit._SrcId as u32, flit._Addr, flit._Data);
            }
            else
            {
                targetFabric.PostHostRead( flit._SrcId as u32, flit._Addr);
            }
            count += 1;
        }
        count
    }
}
