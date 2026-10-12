// rube_perf.rs ------------------------------------------------------------------------------------
use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use trellis::rube::{KernelKind, KernelOp, Layout, ModuleId, PortDesc, SimEngine, SimEngineMode};
use trellis::silo::USeg;

struct CountingAllocator;
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAllocator
{
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8
    {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: AllocLayout) -> *mut u8
    {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: AllocLayout, size: usize) -> *mut u8
    {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(size as u64, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout)
    {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn main()
{
    let mut args = std::env::args().skip(1);
    let lanes = args.next().map(|s| s.parse::<u32>().unwrap()).unwrap_or(4096);
    let cycles = args.next().map(|s| s.parse::<u32>().unwrap()).unwrap_or(20);
    assert!(lanes > 0 && cycles > 0);
    let mut layout = Layout::New();
    let inputs = [PortDesc::U32("a"), PortDesc::U32("b")];
    let outputs = [PortDesc::U32("out")];
    USeg::FromLen(lanes).Traverse(|_| {
        layout.AddModule("xor", ModuleId::None(), &inputs[..], &outputs[..],
                         KernelKind::Fast(KernelOp::Xor));
    });
    layout.Freeze();
    USeg::FromLen(2).Traverse(|run| {
        let workers = if run == 0 { 1 } else { 4 };
        let mut engine = SimEngine::Create(&layout);
        engine.WithMode(if workers == 1 { SimEngineMode::Serial }
                        else { SimEngineMode::Parallel(workers) });
        engine.Drive();
        ALLOCATIONS.store(0, Ordering::Relaxed);
        ALLOCATED_BYTES.store(0, Ordering::Relaxed);
        let started = Instant::now();
        USeg::FromLen(cycles).Traverse(|_| {
            std::hint::black_box(&mut engine).Drive();
        });
        let micros = started.elapsed().as_secs_f64() * 1_000_000.0 / f64::from(cycles);
        let allocations = ALLOCATIONS.load(Ordering::Relaxed) / u64::from(cycles);
        let bytes = ALLOCATED_BYTES.load(Ordering::Relaxed) / u64::from(cycles);
        std::hint::black_box(engine.Triggers());
        println!("lanes={lanes} workers={workers} us/drive={micros:.2} allocations/drive={allocations} allocated_bytes/drive={bytes}");
    });
}

//-------------------------------------------------------------------------------------------------
