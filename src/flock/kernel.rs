// kernel.rs ------------------------------------------------------------------------------------------------------
use	crate::heist::Atelier;
use	crate::silo::{ Arr, MutArr, Stash };
use	crate::stalks::work::SpinMutex;
use	crate::symph::{ Collatz, HashToFloat, StandardOp, WangHash };
use	std::sync::Arc;

//-------------------------------------------------------------------------------------------------
// Owned, disjoint output region for a standard CPU operation. It is deliberately
// non-cloneable: ownership can only move or be split into non-overlapping regions.
pub struct CpuOutputPartition< 'a, T>
{
    _GlobalBase: u32,
    _Output: MutArr< 'a, T>,
}
impl< 'a, T> CpuOutputPartition< 'a, T>
{
    pub fn	New( global_base: u32, output: MutArr< 'a, T>) -> Self
    {
        Self {
            _GlobalBase: global_base,
            _Output: output,
        }
    }
    #[inline]
    pub fn	GlobalBase( &self) -> u32
    {
        self._GlobalBase
    }
    #[inline]
    pub fn	Count( &self) -> u32
    {
        self._Output.Len()
    }
    pub fn	SplitAt( self, count: u32) -> ( Self, Self)
    {
        let  	( left, right) = self._Output.SplitAt( count);
        let  	right_base = self
            ._GlobalBase
            .checked_add( left.Len())
            .expect( "CPU output partition index overflow");
        (
            Self::New( self._GlobalBase, left),
            Self::New( right_base, right),
        )
    }
    fn	SplitForWorkers( self, workers: u32) -> Stash< Self>
    {
        let  	target = workers.max( 1).min( self.Count().max( 1));
        let  	mut partitions = Stash::WithCapacity( target);
        partitions.Push( self);
        while partitions.Size() < target {
            let  	next = partitions.Pop().expect( "CPU output partition is missing");
            let  	count = next.Count();
            let  	( left, right) = next.SplitAt( count.div_ceil( 2));
            partitions.Push( left);
            partitions.Push( right);
        }
        partitions
    }
    /// Execute disjoint output partitions in a caller-owned Heist scope.
    /// Every worker joins before this returns, so `action` may borrow caller data.
    pub fn	ExecuteScoped< F>( self, atelier: &Atelier, action: F)
    where
        T: Send,
        F: Fn( u32, CpuOutputPartition< 'a, T>) + Sync,
    {
        let  	worker_count = atelier.SzThreads().max( 1).min( self.Count().max( 1));
        let  	partitions = SpinMutex::New( self.SplitForWorkers( worker_count));
        atelier.ForEachScopedRange( worker_count, |worker, _start, _end| {
            let  	partition = partitions.Lock().Pop()
                .expect( "CPU output partition is missing for scoped worker");
            action( worker, partition);
        });
    }
    pub fn	ForEach( &mut self, mut action: impl FnMut( u32, &mut T))
    {
        let  	base = self._GlobalBase;
        self._Output.USeg().Traverse( |local| {
            let  	global = base.checked_add( local).expect( "CPU output partition index overflow");
            action( global, self._Output.GetMut( local).unwrap());
        });
    }
    pub fn	IntoOutput( self) -> MutArr< 'a, T>
    {
        self._Output
    }
}

//-------------------------------------------------------------------------------------------------
// CPU SIMT kernel signature. The caller supplies immutable inputs and one or more output views.
pub type CpuKernelFn = Arc< dyn for< 'i, 'o> Fn(
    Arr< 'i, Arr<'i, u8>>, MutArr< 'o, MutArr<'o, u8>>, u32, u32, u32,
) + Send + Sync>;

//-------------------------------------------------------------------------------------------------
pub fn	StandardOpCpuKernelFn( op: StandardOp) -> CpuKernelFn
{
    match op {
        StandardOp::Double => Arc::new( |_inputs, outputs, gid_x, _gid_y, _gid_z| {
            if outputs.IsEmpty() {
                return;
            }
            let   output = outputs.Get( 0).unwrap();
            let   count = output.Len() / std::mem::size_of::< f32>() as u32;
            if gid_x < count {
                unsafe {
                    output.WriteValue( gid_x, output.Arr().ReadValue::< f32>( gid_x) * 2.0);
                }
            }
        }),
        StandardOp::VectorAdd => Arc::new( |inputs, outputs, gid_x, _gid_y, _gid_z| {
            if inputs.Len() < 2 || outputs.IsEmpty() {
                return;
            }
            let   a = inputs.Get( 0).unwrap();
            let   b = inputs.Get( 1).unwrap();
            let   output = outputs.Get( 0).unwrap();
            let   count = ( output.Len() / 4).min( a.Len() / 4).min( b.Len() / 4);
            if gid_x < count {
                unsafe {
                    output.WriteValue( gid_x, a.ReadValue::< f32>( gid_x) + b.ReadValue::< f32>( gid_x));
                }
            }
        }),
        StandardOp::Collatz => Arc::new( |inputs, outputs, gid_x, _gid_y, _gid_z| {
            if inputs.IsEmpty() || outputs.IsEmpty() {
                return;
            }
            let   input = inputs.Get( 0).unwrap();
            let   output = outputs.Get( 0).unwrap();
            let   count = ( output.Len() / 4).min( input.Len() / 4);
            if gid_x < count {
                unsafe {
                    output.WriteValue( gid_x, Collatz( input.ReadValue::< u32>( gid_x)));
                }
            }
        }),
        StandardOp::PointCloud => Arc::new( |_inputs, outputs, gid_x, _gid_y, _gid_z| {
            if outputs.IsEmpty() {
                return;
            }
            let   output = outputs.Get( 0).unwrap();
            let   base = gid_x * 4;
            if base + 3 < output.Len() / 4 {
                let   x = HashToFloat( WangHash( gid_x * 3)) * 40.0 - 20.0;
                let   y = HashToFloat( WangHash( gid_x * 3 + 1)) * 40.0 - 20.0;
                let   z = HashToFloat( WangHash( gid_x * 3 + 2)) * 40.0 - 20.0;
                unsafe {
                    output.WriteValue( base, x);
                    output.WriteValue( base + 1, y);
                    output.WriteValue( base + 2, z);
                    output.WriteValue( base + 3, 1.0f32);
                }
            }
        }),
        StandardOp::CameraTransform => Arc::new( |inputs, outputs, gid_x, _gid_y, _gid_z| {
            if inputs.Len() < 2 || outputs.IsEmpty() {
                return;
            }
            let   points = inputs.Get( 0).unwrap();
            let   camera = inputs.Get( 1).unwrap();
            let   output = outputs.Get( 0).unwrap();
            let   inputBase = gid_x * 3;
            let   outputBase = gid_x * 6;
            if inputBase + 2 >= points.Len() / 4 || outputBase + 5 >= output.Len() / 4 || camera.Len() / 4 < 13 {
                return;
            }
            unsafe {
                let   x = points.ReadValue::< f32>( inputBase);
                let   y = points.ReadValue::< f32>( inputBase + 1);
                let   z = points.ReadValue::< f32>( inputBase + 2);
                let values      = std::array::from_fn::<_, 13, _>( |index| camera.ReadValue::<f32>( index as u32));
                let uniforms    = crate::symph::CameraUniforms::FromValues( ( &values).into()).unwrap();
                let projection  = crate::symph::CameraProjection::New( &uniforms);
                let projected   = projection.Project( &crate::symph::Vec3 { x, y, z });
                crate::silo::USeg::FromLen( 6).Traverse( |component| {
                    output.WriteValue( outputBase + component, projected[component as usize]);
                });
            }
        }),
    }
}

//-------------------------------------------------------------------------------------------------
