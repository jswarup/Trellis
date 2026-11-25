// fresco_perf.rs ----------------------------------------------------------------------------------
#![allow(non_snake_case)]

use std::alloc::{GlobalAlloc, Layout as AllocLayout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use trellis::flux::{FieldImp, FixedStream, IFluxExportSource};
use trellis::fresco::{
    CanonicalPolynomial, Differentiate, EvalLimits, ExprGrammar, ExprParseCtx, ExprRepos,
    LeastSquaresLimits, LeastSquaresOptions, LeastSquaresProblem, LeastSquaresSpec, TensorExpr,
    TensorShape,
};
use trellis::shard::{JsonGrammar, JsonParseCtx, Parser};
use trellis::silo::{Arr, Buff};

//-------------------------------------------------------------------------------------------------

struct CountingAllocator;
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static PEAK_LIVE_BYTES: AtomicU64 = AtomicU64::new(0);

fn UpdatePeak(live: u64)
{
    let mut prev = PEAK_LIVE_BYTES.load(Ordering::Relaxed);
    while live > prev
    {
        match PEAK_LIVE_BYTES.compare_exchange_weak(
            prev,
            live,
            Ordering::Relaxed,
            Ordering::Relaxed,
        )
        {
            Ok(_) => break,
            Err(actual) => prev = actual,
        }
    }
}

unsafe impl GlobalAlloc for CountingAllocator
{
    unsafe fn alloc(&self, layout: AllocLayout) -> *mut u8
    {
        let sz = layout.size() as u64;
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(sz, Ordering::Relaxed);
        let live = LIVE_BYTES.fetch_add(sz, Ordering::Relaxed) + sz;
        UpdatePeak(live);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: AllocLayout) -> *mut u8
    {
        let sz = layout.size() as u64;
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(sz, Ordering::Relaxed);
        let live = LIVE_BYTES.fetch_add(sz, Ordering::Relaxed) + sz;
        UpdatePeak(live);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: AllocLayout, size: usize) -> *mut u8
    {
        let oldSz = layout.size() as u64;
        let newSz = size as u64;
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(newSz, Ordering::Relaxed);
        if newSz >= oldSz
        {
            let diff = newSz - oldSz;
            let live = LIVE_BYTES.fetch_add(diff, Ordering::Relaxed) + diff;
            UpdatePeak(live);
        }
        else
        {
            LIVE_BYTES.fetch_sub(oldSz - newSz, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: AllocLayout)
    {
        LIVE_BYTES.fetch_sub(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

//-------------------------------------------------------------------------------------------------

fn ResetCounters()
{
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    PEAK_LIVE_BYTES.store(LIVE_BYTES.load(Ordering::Relaxed), Ordering::Relaxed);
}

fn SnapshotAllocations() -> (u64, u64)
{
    (
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
    )
}

fn SnapshotPeakLiveBytes() -> u64
{
    PEAK_LIVE_BYTES.load(Ordering::Relaxed)
}

//-------------------------------------------------------------------------------------------------

fn main()
{
    println!("=== Fresco Workload Baselines & Resource Profiling ===\n");

    // 1. Small Scalar Workload: Parsing, Differentiation, Polynomial Expansion, Evaluation
    {
        let mut repo = ExprRepos::New();
        let varX = repo.VarCreate("x".to_string(), true);
        let exprText = "x^2 + 2*x + 1";

        ResetCounters();
        let t0 = Instant::now();
        let parsed = repo.ParseExpr(exprText).expect("parse quadratic formula");
        let parseElapsed = t0.elapsed();
        let (parseAllocs, parseBytes) = SnapshotAllocations();

        ResetCounters();
        let t1 = Instant::now();
        let _deriv = Differentiate(&mut repo, parsed, varX).expect("diff");
        let diffElapsed = t1.elapsed();
        let (diffAllocs, diffBytes) = SnapshotAllocations();

        ResetCounters();
        let t2 = Instant::now();
        let poly = CanonicalPolynomial::FromExpr(&repo, parsed).expect("poly expand");
        let polyElapsed = t2.elapsed();
        let (polyAllocs, polyBytes) = SnapshotAllocations();

        let bindings = [3.0f64];
        let bindArr = Arr::from(&bindings);

        ResetCounters();
        let t3 = Instant::now();
        let valIndexed = repo.Evaluate(parsed, bindArr).expect("indexed eval");
        let evalElapsed = t3.elapsed();
        let (evalAllocs, evalBytes) = SnapshotAllocations();

        assert_eq!(valIndexed, 16.0);
        let namedBindings = [("x", 3.0f64)];
        let valNamed = repo
            .EvaluateNamed(parsed, Arr::from(&namedBindings))
            .expect("named eval");
        assert_eq!(valNamed, 16.0);

        println!("-- 1. Small Scalar Workload (x^2 + 2x + 1) --");
        println!(
            "  Nodes: {}, Live Vars: {}, Poly Degree: {}",
            repo.Size(),
            repo.SzVar(),
            poly.Degree()
        );
        println!(
            "  Parse:       {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            parseElapsed.as_secs_f64() * 1e6,
            parseAllocs,
            parseBytes
        );
        println!(
            "  Derivative:  {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            diffElapsed.as_secs_f64() * 1e6,
            diffAllocs,
            diffBytes
        );
        println!(
            "  Polynomial:  {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            polyElapsed.as_secs_f64() * 1e6,
            polyAllocs,
            polyBytes
        );
        println!(
            "  Evaluation:  {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            evalElapsed.as_secs_f64() * 1e6,
            evalAllocs,
            evalBytes
        );
        println!();
    }

    // 2. Shared Subgraph DAG Workload: Exponential Tree Sharing
    {
        let mut repo = ExprRepos::New();
        let x = repo.VarCreate("x".to_string(), true);
        let y = repo.VarCreate("y".to_string(), true);

        // x0 = x + y, x1 = x0 * x0, x2 = x1 + x1, x3 = x2 * x2, x4 = x3 + x3
        let x0 = repo.AddCreate(x, y);
        let x1 = repo.MultCreate(x0, x0);
        let x2 = repo.AddCreate(x1, x1);
        let x3 = repo.MultCreate(x2, x2);
        let x4 = repo.AddCreate(x3, x3);

        let limits = EvalLimits::default();
        let bindings = [2.0f64, 3.0f64];

        ResetCounters();
        let t0 = Instant::now();
        let outcome = repo
            .EvaluateWithLimits(x4, Arr::from(&bindings), &limits)
            .expect("shared DAG eval");
        let elapsed = t0.elapsed();
        let (allocs, bytes) = SnapshotAllocations();

        println!("-- 2. Highly Shared Subgraph Workload (Depth 5 DAG) --");
        println!(
            "  Nodes In Repo: {}, Value: {}",
            repo.Size(),
            outcome._Value
        );
        println!(
            "  Telemetry -> Visited Nodes: {}, Edges: {}, Shared Visits: {}, Peak Depth: {}",
            outcome._Counters._NodesVisited,
            outcome._Counters._EdgesTraversed,
            outcome._Counters._SharedVisits,
            outcome._Counters._PeakDepth
        );
        println!(
            "  Evaluation:  {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            elapsed.as_secs_f64() * 1e6,
            allocs,
            bytes
        );
        println!();
    }

    // 3. Deep Linear Expression Workload
    {
        let mut repo = ExprRepos::New();
        let x = repo.VarCreate("x".to_string(), true);
        let one = repo.RealCreate(1.0);
        let mut cur = x;
        let chainLen = 50u32;
        let mut i = 0u32;
        while i < chainLen {
            cur = repo.AddCreate(cur, one);
            i += 1;
        }

        let bindings = [10.0f64];
        let limits = EvalLimits::default();

        ResetCounters();
        let t0 = Instant::now();
        let outcome = repo
            .EvaluateWithLimits(cur, Arr::from(&bindings), &limits)
            .expect("deep linear eval");
        let elapsed = t0.elapsed();
        let (allocs, bytes) = SnapshotAllocations();

        assert_eq!(outcome._Value, 60.0);
        println!("-- 3. Deep Linear Expression Workload (Depth 50 Chain) --");
        println!("  Repo Nodes: {}, Result: {}", repo.Size(), outcome._Value);
        println!(
            "  Telemetry -> Visited Nodes: {}, Peak Depth: {}",
            outcome._Counters._NodesVisited, outcome._Counters._PeakDepth
        );
        println!(
            "  Evaluation:  {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            elapsed.as_secs_f64() * 1e6,
            allocs,
            bytes
        );
        println!();
    }

    // 4. Tensor Matrix-Vector Workload (4x4 Matrix * 4x1 Vector)
    {
        let mut repo = ExprRepos::New();
        let matShape = TensorShape::Matrix(4, 4).expect("matrix 4x4");
        let vecShape = TensorShape::Vector(4).expect("vector 4");

        let matA = TensorExpr::FromRealsSlice(
            &mut repo,
            matShape,
            &[
                1.0, 2.0, 3.0, 4.0,
                5.0, 6.0, 7.0, 8.0,
                9.0, 1.0, 2.0, 3.0,
                4.0, 5.0, 6.0, 7.0,
            ],
        )
        .expect("matA");

        let vecV = TensorExpr::FromRealsSlice(&mut repo, vecShape, &[1.0, 1.0, 1.0, 1.0])
            .expect("vecV");

        ResetCounters();
        let t0 = Instant::now();
        let mv = TensorExpr::MatVecMul(&mut repo, &matA, &vecV).expect("matvecmul");
        let prepElapsed = t0.elapsed();
        let (prepAllocs, prepBytes) = SnapshotAllocations();

        let limits = EvalLimits::default();
        ResetCounters();
        let t1 = Instant::now();
        let outcome = repo
            .EvaluateTensorWithLimits(&mv, Arr::Empty(), &limits)
            .expect("tensor eval");
        let evalElapsed = t1.elapsed();
        let (evalAllocs, evalBytes) = SnapshotAllocations();

        println!("-- 4. Tensor Workload (4x4 Matrix * 4x1 Vector) --");
        println!(
            "  Repo Nodes: {}, Output Vector Elements: {}",
            repo.Size(),
            outcome._Value.Size()
        );
        println!(
            "  Prep:        {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            prepElapsed.as_secs_f64() * 1e6,
            prepAllocs,
            prepBytes
        );
        println!(
            "  Evaluation:  {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            evalElapsed.as_secs_f64() * 1e6,
            evalAllocs,
            evalBytes
        );
        println!(
            "  Telemetry -> Results Computed: {}, Visited Nodes: {}",
            outcome._Counters._ResultsComputed, outcome._Counters._NodesVisited
        );
        println!();
    }

    // 5. Solver Multi-Root Residual & Jacobian Workload (Rosenbrock Problem)
    {
        let mut repo = ExprRepos::New();
        let x = repo.VarCreate("x".to_string(), true);
        let y = repo.VarCreate("y".to_string(), true);

        // Rosenbrock residuals:
        // r0(x, y) = 10 * (y - x^2)
        // r1(x, y) = 1 - x
        let ten = repo.RealCreate(10.0);
        let one = repo.RealCreate(1.0);
        let two = repo.RealCreate(2.0);

        let xSq = repo.PowerCreate(x, two);
        let yMinusXSq = repo.DiffCreate(y, xSq);
        let r0 = repo.MultCreate(ten, yMinusXSq);
        let r1 = repo.DiffCreate(one, x);

        ResetCounters();
        let t0 = Instant::now();
        let dr0_dx = Differentiate(&mut repo, r0, x).expect("dr0_dx");
        let dr0_dy = Differentiate(&mut repo, r0, y).expect("dr0_dy");
        let dr1_dx = Differentiate(&mut repo, r1, x).expect("dr1_dx");
        let dr1_dy = Differentiate(&mut repo, r1, y).expect("dr1_dy");
        let jacElapsed = t0.elapsed();
        let (jacAllocs, jacBytes) = SnapshotAllocations();

        let bindings = [1.2f64, 1.44f64]; // x = 1.2, y = 1.44 (near root x=1, y=1)
        let bindArr = Arr::from(&bindings);

        ResetCounters();
        let t1 = Instant::now();
        let r0Val = repo.Evaluate(r0, bindArr).expect("r0 eval");
        let r1Val = repo.Evaluate(r1, bindArr).expect("r1 eval");
        let j00 = repo.Evaluate(dr0_dx, bindArr).expect("j00");
        let j01 = repo.Evaluate(dr0_dy, bindArr).expect("j01");
        let j10 = repo.Evaluate(dr1_dx, bindArr).expect("j10");
        let j11 = repo.Evaluate(dr1_dy, bindArr).expect("j11");
        let evalElapsed = t1.elapsed();
        let (evalAllocs, evalBytes) = SnapshotAllocations();

        println!("-- 5. Solver Multi-Root Workload (Rosenbrock Residuals & Jacobian) --");
        println!(
            "  Repo Nodes: {}, Live Vars: {}, r = [{:.2}, {:.2}]",
            repo.Size(),
            repo.SzVar(),
            r0Val,
            r1Val
        );
        println!(
            "  Jacobian: [[{:.2}, {:.2}], [{:.2}, {:.2}]]",
            j00, j01, j10, j11
        );
        println!(
            "  Jacobian Prep: {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            jacElapsed.as_secs_f64() * 1e6,
            jacAllocs,
            jacBytes
        );
        println!(
            "  Full Eval:     {:>8.2} us | Allocs: {:>4} | Bytes: {:>6} | PeakLive: {:>6}",
            evalElapsed.as_secs_f64() * 1e6,
            evalAllocs,
            evalBytes,
            SnapshotPeakLiveBytes()
        );
        println!();
    }

    // 6. Isolated Staged Publication, Flux Round-Trip, and Repeated Indexed vs Named Eval
    {
        let mut repo = ExprRepos::New();
        let formula = "(a + b * c) - sin(x)^2 + if (a > b) (x + 1) else (y - 2)";

        ResetCounters();
        let tStage = Instant::now();
        let mut stream = FixedStream::from(formula);
        let mut parser = Parser::New(&mut stream);
        let mut ctx = ExprParseCtx::New(&mut repo);
        assert!(parser.ParseGrammar(&ExprGrammar, &mut ctx, 0).is_some());
        let stageElapsed = tStage.elapsed();
        let (stageAllocs, stageBytes) = SnapshotAllocations();

        ResetCounters();
        let tPub = Instant::now();
        let rootId = ctx.Publish().expect("publish staged");
        let pubElapsed = tPub.elapsed();
        let (pubAllocs, pubBytes) = SnapshotAllocations();

        ResetCounters();
        let tExp = Instant::now();
        let fluxJson = format!("{}", &repo as &dyn IFluxExportSource);
        let expElapsed = tExp.elapsed();
        let (expAllocs, expBytes) = SnapshotAllocations();

        let mut restored = ExprRepos::New();
        ResetCounters();
        let tImp = Instant::now();
        {
            let mut jStream = FixedStream::from(fluxJson.as_str());
            let mut jParser = Parser::New(&mut jStream);
            let mut jCtx = JsonParseCtx::New(FieldImp::FluxSource(&mut restored));
            assert!(jParser.ParseGrammar(&JsonGrammar, &mut jCtx, 0).is_some());
        }
        let impElapsed = tImp.elapsed();
        let (impAllocs, impBytes) = SnapshotAllocations();

        let iters = 100u32;
        let idxBindings = [1.0f64, 2.0, 3.0, 0.5, 4.0];
        let namedBindings = [("a", 1.0), ("b", 2.0), ("c", 3.0), ("x", 0.5), ("y", 4.0)];
        let limits = EvalLimits::default();

        ResetCounters();
        let tIdx = Instant::now();
        let mut k = 0u32;
        while k < iters
        {
            let _ = std::hint::black_box(&restored)
                .EvaluateOutcomeWithLimits(rootId, Arr::from(&idxBindings), &limits);
            k += 1;
        }
        let idxElapsed = tIdx.elapsed();
        let (idxAllocs, idxBytes) = SnapshotAllocations();

        ResetCounters();
        let tNam = Instant::now();
        k = 0;
        while k < iters
        {
            let _ = std::hint::black_box(&restored)
                .EvaluateNamedOutcomeWithLimits(rootId, Arr::from(&namedBindings), &limits);
            k += 1;
        }
        let namElapsed = tNam.elapsed();
        let (namAllocs, namBytes) = SnapshotAllocations();

        println!("-- 6. Staged Publication, Flux Round-Trip & Repeated Eval (100x) --");
        println!(
            "  Repo Nodes: {}, Flux JSON Bytes: {}",
            restored.Size(),
            fluxJson.len()
        );
        println!(
            "  Stage Parse:   {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            stageElapsed.as_secs_f64() * 1e6,
            stageAllocs,
            stageBytes
        );
        println!(
            "  Publish DAG:   {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            pubElapsed.as_secs_f64() * 1e6,
            pubAllocs,
            pubBytes
        );
        println!(
            "  Flux Export:   {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            expElapsed.as_secs_f64() * 1e6,
            expAllocs,
            expBytes
        );
        println!(
            "  Flux Import:   {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            impElapsed.as_secs_f64() * 1e6,
            impAllocs,
            impBytes
        );
        println!(
            "  Indexed 100x:  {:>8.2} us/iter | Allocs/iter: {:>2} | Bytes/iter: {:>4}",
            idxElapsed.as_secs_f64() * 1e6 / f64::from(iters),
            idxAllocs / u64::from(iters),
            idxBytes / u64::from(iters)
        );
        println!(
            "  Named 100x:    {:>8.2} us/iter | Allocs/iter: {:>2} | Bytes/iter: {:>4}",
            namElapsed.as_secs_f64() * 1e6 / f64::from(iters),
            namAllocs / u64::from(iters),
            namBytes / u64::from(iters)
        );
        println!();
    }

    // 7. Complete Levenberg-Marquardt Least-Squares Solver Profile (Rosenbrock)
    {
        let mut repo = ExprRepos::New();
        let r0 = repo.ParseExpr("10 * (y - x^2)").expect("r0");
        let r1 = repo.ParseExpr("1 - x").expect("r1");

        ResetCounters();
        let tPrep = Instant::now();
        let mut prob = LeastSquaresProblem::Prepare(
            &mut repo,
            LeastSquaresSpec {
                _ResidualRoots:  Buff::FromArr(Arr::from(&[r0, r1])),
                _ActiveVars:     Buff::FromArr(Arr::from(&["x".to_string(), "y".to_string()])),
                _FixedBindings:  Buff::New(),
                _VarScales:      None,
                _ResidualScales: None,
                _Options:        LeastSquaresOptions::default(),
                _Limits:         LeastSquaresLimits::default(),
            },
        )
        .expect("prepare LS");
        let prepElapsed = tPrep.elapsed();
        let (prepAllocs, prepBytes) = SnapshotAllocations();

        ResetCounters();
        let tSolve = Instant::now();
        let result = prob.Solve(&repo, Arr::from(&[-1.2, 1.0]));
        let solveElapsed = tSolve.elapsed();
        let (solveAllocs, solveBytes) = SnapshotAllocations();

        println!("-- 7. Complete LM Least-Squares Solve (Rosenbrock) --");
        println!(
            "  Status: {:?}, x = [{:.6}, {:.6}], iters = {}, trials = {}",
            result._Status,
            result._BestX[0],
            result._BestX[1],
            result._Counters._AcceptedIterations,
            result._Counters._TrialAttempts
        );
        println!(
            "  LS Prep:       {:>8.2} us | Allocs: {:>4} | Bytes: {:>6}",
            prepElapsed.as_secs_f64() * 1e6,
            prepAllocs,
            prepBytes
        );
        println!(
            "  LM Solve:      {:>8.2} us | Allocs: {:>4} | Bytes: {:>6} | PeakLive: {:>6}",
            solveElapsed.as_secs_f64() * 1e6,
            solveAllocs,
            solveBytes,
            SnapshotPeakLiveBytes()
        );
        println!();
    }

    println!("=== All baseline performance measurements completed successfully ===");
}
