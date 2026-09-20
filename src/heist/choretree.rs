// choretree.rs ----------------------------------------------------------------------------------------------------
use	crate::heist::maestro::Maestro;
use	crate::silo::buff::Buff;
use	crate::silo::stash::Stash;
use	crate::stalks::work::{ IWorker, WorkPtr };
use	std::ops::{ BitOr, Shr };
use	std::sync::Arc;

//-------------------------------------------------------------------------------------------------
// Chore — concrete execution unit in a Heist chore tree.
// Modeled directly from Trellis heist/choretree.h.
pub type ChoreFn = Box< dyn Fn( &mut dyn IWorker) + Send + Sync>;
pub type ChoreSharedFn = Arc< dyn Fn( &mut dyn IWorker) + Send + Sync>;
#[derive( Clone)]
pub struct Chore
{
    pub _DocStr: &'static str,
    pub _Closure: Option< fn( &mut dyn IWorker)>,
    pub _Work: Option< ChoreSharedFn>,
}
impl Chore
{
    pub const fn	New( f: fn( &mut dyn IWorker)) -> Self
    {
        Self {
            _DocStr: "",
            _Closure: Some( f),
            _Work: None,
        }
    }
    pub const fn	WithDoc( doc_str: &'static str, f: fn(&mut dyn IWorker)) -> Self {
        Self {
            _DocStr: doc_str,
            _Closure: Some( f),
            _Work: None,
        }
    }
    pub fn	FromClosure< F>( doc_str: &'static str, f: F) -> Self
    where
        F: Fn( &mut dyn IWorker) + Send + Sync + 'static,
    {
        Self {
            _DocStr: doc_str,
            _Closure: None,
            _Work: Some( Arc::new( f)),
        }
    }
    pub fn	FromFn( f: fn( &mut dyn IWorker)) -> Self
    {
        Self::New( f)
    }
    pub fn	DocStr( &self) -> &'static str {
        self._DocStr
    }
    pub fn	Post( &self, maestro: &Maestro, tails: &mut Stash< u16>) -> u16
    {
        let  	work = if let  	Some( closure) = self._Closure {
            WorkPtr::FromFn( closure)
        } else if let  	Some( ref work_arc) = self._Work {
            let  	work_clone = work_arc.clone();
            WorkPtr::FromClosure( move |w| work_clone( w))
        } else {
            WorkPtr::Null()
        };
        let  	job_id = maestro.ConstructJob( 0, work);
        tails.PushBack( job_id);
        job_id
    }
    pub fn	Then( self, other: impl Into< ChoreNode>) -> ChoreNode
    {
        ChoreNode::Leaf( self).Then( other)
    }
    pub fn	Par( self, other: impl Into< ChoreNode>) -> ChoreNode
    {
        ChoreNode::Leaf( self).Par( other)
    }
}

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Copy)]
pub enum ChoreTarget {
    Cpu,
    GpuAuto,
}
pub struct SpawnQuellNode< 'a, T> {
    pub _Data: crate::silo::arr::MutArr< 'a, T>,
    pub _Target: ChoreTarget,
    pub _DocStr: &'static str,
    pub _ItemWeight: u32,
    pub _SpawnFn: fn( crate::silo::arr::MutArr< 'a, T>, &mut dyn IWorker),
    pub _QuellFn: fn( crate::silo::arr::MutArr< 'a, T>, &mut dyn IWorker),
}
impl< 'a, T> Clone for SpawnQuellNode<'a, T>
{
    fn	clone( &self) -> Self
    {
        Self {
            _Data: unsafe { self._Data.Alias() },
            _Target: self._Target,
            _DocStr: self._DocStr,
            _ItemWeight: self._ItemWeight,
            _SpawnFn: self._SpawnFn,
            _QuellFn: self._QuellFn,
        }
    }
}
impl< 'a, T> SpawnQuellNode<'a, T>
{
    pub fn	New( 
        data: crate::silo::arr::MutArr< 'a, T>, target: ChoreTarget, itemWeight: u32,
        docStr: &'static str, spawnFn: fn(crate::silo::arr::MutArr<'a, T>, &mut dyn IWorker),
        quellFn: fn( crate::silo::arr::MutArr< 'a, T>, &mut dyn IWorker),
    ) -> Self
    {
        Self {
            _Data: data,
            _Target: target,
            _DocStr: docStr,
            _ItemWeight: itemWeight,
            _SpawnFn: spawnFn,
            _QuellFn: quellFn,
        }
    }
}
impl< 'a, T: Send + Sync> From<SpawnQuellNode<'a, T>> for ChoreNode {
    fn	from( val: SpawnQuellNode< 'a, T>) -> Self
    {
        fn	spawn_thunk< T>( ptr: usize, len: u32, fn_ptr: usize, worker: &mut dyn IWorker)
        {
            let  	arr = crate::silo::arr::MutArr::New( ptr as *mut T, len);
            let  	f: fn( crate::silo::arr::MutArr< '_, T>, &mut dyn IWorker) =
                unsafe { std::mem::transmute( fn_ptr) };
            f( arr, worker);
        }
        fn	quell_thunk< T>( ptr: usize, len: u32, fn_ptr: usize, worker: &mut dyn IWorker)
        {
            let  	arr = crate::silo::arr::MutArr::New( ptr as *mut T, len);
            let  	f: fn( crate::silo::arr::MutArr< '_, T>, &mut dyn IWorker) =
                unsafe { std::mem::transmute( fn_ptr) };
            f( arr, worker);
        }
        ChoreNode::SpawnQuell( ErasedSpawnQuell {
            _DataPtr: val._Data.Data() as usize,
            _DataLen: val._Data.Size(),
            _ElemSize: std::mem::size_of::< T>() as u32,
            _DocStr: val._DocStr,
            _ItemWeight: val._ItemWeight,
            _IsCpu: matches!( val._Target, ChoreTarget::Cpu),
            _SpawnFnPtr: val._SpawnFn as usize,
            _QuellFnPtr: val._QuellFn as usize,
            _SpawnThunk: spawn_thunk::< T>,
            _QuellThunk: quell_thunk::< T>,
        })
    }
}
#[macro_export]
macro_rules! SpawnQuell {
    ( $data:expr, $target:expr, $spawnFn:expr, $quellFn:expr) => {
        $crate::heist::choretree::SpawnQuellNode::New( 
            $data,
            $target,
            1,
            "SpawnQuell",
            $spawnFn,
            $quellFn,
        )
    };
}
#[macro_export]
macro_rules! WeightedSpawnQuell {
    ( $data:expr, $itemWeight:expr, $target:expr, $spawnFn:expr, $quellFn:expr) => {
        $crate::heist::choretree::SpawnQuellNode::New( 
            $data,
            $target,
            $itemWeight,
            "WeightedSpawnQuell",
            $spawnFn,
            $quellFn,
        )
    };
}
#[macro_export]
macro_rules! CpuSpawnQuell {
    ( $data:expr, $spawnFn:expr, $quellFn:expr) => {
        $crate::heist::choretree::SpawnQuellNode::New( 
            $data,
            $crate::heist::choretree::ChoreTarget::Cpu,
            1,
            "CpuSpawnQuell",
            $spawnFn,
            $quellFn,
        )
    };
}
#[macro_export]
macro_rules! GpuSpawnQuell {
    ( $data:expr, $spawnFn:expr, $quellFn:expr) => {
        $crate::heist::choretree::SpawnQuellNode::New( 
            $data,
            $crate::heist::choretree::ChoreTarget::GpuAuto,
            1,
            "GpuSpawnQuell",
            $spawnFn,
            $quellFn,
        )
    };
}

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Copy)]
pub struct ErasedSpawnQuell
{
    pub _DataPtr: usize,
    pub _DataLen: u32,
    pub _ElemSize: u32,
    pub _DocStr: &'static str,
    pub _ItemWeight: u32,
    pub _IsCpu: bool,
    pub _SpawnFnPtr: usize,
    pub _QuellFnPtr: usize,
    pub _SpawnThunk: fn( usize, u32, usize, &mut dyn IWorker),
    pub _QuellThunk: fn( usize, u32, usize, &mut dyn IWorker),
}
#[derive( Clone, Copy)]
pub struct ErasedCoro
{
    pub _DocStr: &'static str,
    pub _Closure: fn( 
        crate::stalks::coro::CoroYielder< '_, crate::heist::corochore::WorkerFatPtr, ()>,
        crate::heist::corochore::WorkerFatPtr,
    ),
}
// ChoreNode — DAG node representing sequential (< or >>) or parallel (|) composition.
#[derive( Clone)]
pub enum ChoreNode {
    Leaf( Chore),
    Seq( Box< ChoreNode>, Box< ChoreNode>),
    Par( Box< ChoreNode>, Box< ChoreNode>),
    SpawnQuell( ErasedSpawnQuell),
    Coro( ErasedCoro),
}
impl ChoreNode
{
    pub fn	Then( self, other: impl Into< ChoreNode>) -> Self
    {
        ChoreNode::Seq( Box::new( self), Box::new( other.into()))
    }
    pub fn	Par( self, other: impl Into< ChoreNode>) -> Self
    {
        ChoreNode::Par( Box::new( self), Box::new( other.into()))
    }
}
impl From< Chore> for ChoreNode {
    fn	from( chore: Chore) -> Self
    {
        ChoreNode::Leaf( chore)
    }
}
// Operator overloads: `>>` for sequential, `|` for parallel composition
impl BitOr< Chore> for Chore {
    type Output = ChoreNode;
    fn	bitor( self, rhs: Chore) -> ChoreNode
    {
        ChoreNode::Par( 
            Box::new( ChoreNode::Leaf( self)),
            Box::new( ChoreNode::Leaf( rhs)),
        )
    }
}
impl BitOr< ChoreNode> for Chore {
    type Output = ChoreNode;
    fn	bitor( self, rhs: ChoreNode) -> ChoreNode
    {
        ChoreNode::Par( Box::new( ChoreNode::Leaf( self)), Box::new( rhs))
    }
}
impl BitOr< Chore> for ChoreNode {
    type Output = ChoreNode;
    fn	bitor( self, rhs: Chore) -> ChoreNode
    {
        ChoreNode::Par( Box::new( self), Box::new( ChoreNode::Leaf( rhs)))
    }
}
impl BitOr< ChoreNode> for ChoreNode {
    type Output = ChoreNode;
    fn	bitor( self, rhs: ChoreNode) -> ChoreNode
    {
        ChoreNode::Par( Box::new( self), Box::new( rhs))
    }
}
impl Shr< Chore> for Chore {
    type Output = ChoreNode;
    fn	shr( self, rhs: Chore) -> ChoreNode
    {
        ChoreNode::Seq( 
            Box::new( ChoreNode::Leaf( self)),
            Box::new( ChoreNode::Leaf( rhs)),
        )
    }
}
impl Shr< ChoreNode> for Chore {
    type Output = ChoreNode;
    fn	shr( self, rhs: ChoreNode) -> ChoreNode
    {
        ChoreNode::Seq( Box::new( ChoreNode::Leaf( self)), Box::new( rhs))
    }
}
impl Shr< Chore> for ChoreNode {
    type Output = ChoreNode;
    fn	shr( self, rhs: Chore) -> ChoreNode
    {
        ChoreNode::Seq( Box::new( self), Box::new( ChoreNode::Leaf( rhs)))
    }
}
impl Shr< ChoreNode> for ChoreNode {
    type Output = ChoreNode;
    fn	shr( self, rhs: ChoreNode) -> ChoreNode
    {
        ChoreNode::Seq( Box::new( self), Box::new( rhs))
    }
}

//-------------------------------------------------------------------------------------------------
// PostChoreNode — recursively posts a ChoreNode into Maestro/Atelier execution graph.
pub fn	PostChoreNode( node: &ChoreNode, maestro: &Maestro, tails: &mut Stash< u16>) -> u16
{
    match node {
        ChoreNode::Leaf( chore) => chore.Post( maestro, tails),
        ChoreNode::Seq( left, right) => {
            let  	mut left_tails = Stash::WithCapacity( 64);
            let  	head_l = PostChoreNode( left, maestro, &mut left_tails);
            let  	head_r = PostChoreNode( right, maestro, tails);
            if let  	Some( state) = maestro.State() {
                while let  	Some( left_tail) = left_tails.Pop() {
                    state.SetSucc( left_tail, head_r);
                }
            }
            head_l
        }
        ChoreNode::Par( left, right) => {
            let  	mut left_tails = Stash::WithCapacity( 64);
            let  	mut right_tails = Stash::WithCapacity( 64);
            let  	head_l = PostChoreNode( left, maestro, &mut left_tails);
            let  	head_r = PostChoreNode( right, maestro, &mut right_tails);
            while let  	Some( t) = left_tails.Pop() {
                tails.PushBack( t);
            }
            while let  	Some( t) = right_tails.Pop() {
                tails.PushBack( t);
            }
            let  	heads = Buff::FromDispenser( 2, |i| if i == 0 { head_l } else { head_r });
            maestro.ConstructEnqueArr( 0, heads)
        }
        ChoreNode::SpawnQuell( sq) => {
            let  	quell_fn = sq._QuellThunk;
            let  	quell_ptr = sq._QuellFnPtr;
            let  	data_ptr = sq._DataPtr;
            let  	data_len = sq._DataLen;
            let  	quell_job = maestro.ConstructJob( 
                0,
                WorkPtr::FromClosure( move |w| {
                    quell_fn( data_ptr, data_len, quell_ptr, w);
                }),
            );
            tails.PushBack( quell_job);
            let  	total_weight = data_len * sq._ItemWeight;
            if !sq._IsCpu || total_weight <= 20 {
                // 20 is placeholder for FusionThres
                let  	spawn_fn = sq._SpawnThunk;
                let  	spawn_ptr = sq._SpawnFnPtr;
                let  	spawn_job = maestro.ConstructJob( 
                    quell_job,
                    WorkPtr::FromClosure( move |w| {
                        spawn_fn( data_ptr, data_len, spawn_ptr, w);
                    }),
                );
                return spawn_job;
            }
            let  	num_maestros = maestro.State().map( |s| s._SzThreads).unwrap_or( 1);
            let  	mut c = num_maestros * 2;
            let  	max_chunks = total_weight / 20;                    // placeholder for FusionThres
            if c > max_chunks {
                c = max_chunks;
            }
            if c == 0 {
                c = 1;
            }
            let  	mut heads = Stash::New();
            let  	chunk_size = data_len.div_ceil( c);
            let  	mut start = 0;
            while start < data_len {
                let  	rem = data_len - start;
                let  	sz = if rem < chunk_size { rem } else { chunk_size };
                let  	chunk_ptr = data_ptr + ( start * sq._ElemSize) as usize;
                let  	spawn_fn = sq._SpawnThunk;
                let  	spawn_ptr = sq._SpawnFnPtr;
                let  	spawn_job = maestro.ConstructJob( 
                    quell_job,
                    WorkPtr::FromClosure( move |w| {
                        spawn_fn( chunk_ptr, sz, spawn_ptr, w);
                    }),
                );
                heads.Push( spawn_job);
                start += sz;
            }
            maestro.ConstructEnqueArr( 0, heads.ExtractBuff())
        }
        ChoreNode::Coro( coro) => {
            let  	closure = coro._Closure;
            let  	c = crate::stalks::coro::Coro::New( closure);
            let  	job = maestro.ConstructJob( 
                0,
                WorkPtr::FromClosure( move |w| {
                    crate::heist::corochore::coro_job_func( c, w);
                }),
            );
            tails.PushBack( job);
            job
        }
    }
}
