# atelier.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from algorithm import parallelize, vectorize
from strand import Atm, SpinLock
from stash import Buff, Arr, Silo, Stk, USeg
import heist

#----------------------------------------------------------------------------------------------------------------------------------

 
@value
struct Runner( CollectionElement):
    var     _Runner : fn( mut maestro : Maestro)  escaping-> Bool 
    
    fn __init__( out self) : 
        x = 0
        fn  default( mut maestro : Maestro) -> Bool:
            return x == 0

        self._Runner = default 

    @implicit
    fn __init__( out self,   runner : fn( mut maestro : Maestro) escaping -> Bool) : 
        self._Runner = runner 

    fn    Score(  self, mut maestro : Maestro) -> Bool:
        return self._Runner( maestro)

 #----------------------------------------------------------------------------------------------------------------------------------

struct Atelier:
    var     _StartCount: UInt32                         # Count of Processing Queue started, used for startup and shutdown 
    var     _SzSchedJob: Atm[ True, DType.uint32]       # Count of cumulative scheduled jobs in Works and Queues
    var     _SzQueue: Atm[ True, DType.uint32]     
    var     _Lock: SpinLock
    var     _LockedMark: UInt32
    var     _JobSilo: Silo[ UInt16, True]               # A Stack of free jobIds

    var     _JobBuff: Buff[ Runner]                     # Runner at the jobId
    var     _SzPreds: Buff[ UInt16]                     # Count of predessors for job at the jobId
    var     _SuccIds: Buff[ UInt16]                     # Successor job for the job at the jobId

    var     _Maestros: Buff[ Maestro]                   # All the Maestros

    @always_inline
    fn __init__( out self, mxQueue : UInt32 ) : 
        self._StartCount = UInt32( 0)
        self._SzSchedJob = UInt32( 0)
        self._SzQueue = UInt32( 0)
        self._LockedMark = UInt32.MAX 
        self._Lock = SpinLock()
        mx = UInt16.MAX.cast[ DType.uint32]()
        self._JobSilo = Silo[ UInt16, True]( mx, UInt16( 0))
        self._JobSilo.DoIndexSetup( True)
        self._JobBuff = Buff[ Runner]( mx, Runner()) 
        self._SzPreds = Buff[ UInt16]( mx, UInt16( 0))
        self._SuccIds = Buff[ UInt16]( mx, UInt16( 0))
        self._Maestros = Buff[ Maestro]( mxQueue, Maestro()) 
        var ind : UInt32 = 0
        for g in self._Maestros.Arr():
            g[].SetAtelier( ind, self)
            ind += 1
        pass
        
    fn __del__( owned self): 
        print( "Atelier: Del ")
        
    fn Maestros( self) -> Arr[ Maestro, __origin_of( self._Maestros._DPtr)]:
        return self._Maestros.Arr()
    
    fn Honcho( self) -> UnsafePointer[ Maestro]:
        return self._Maestros.PtrAt( UInt32( 0))

    fn DoLaunch( self) -> Bool:
        
        @parameter
        fn worker( ind: Int): 
            self._Maestros.PtrAt( UInt32( ind +1))[].ExecuteLoop()
        pass
        
        print( "DoLaunch")
        szWorker = self._Maestros.Size() -1
        if ( szWorker):
            parallelize[ worker]( int( szWorker))
        self._Maestros.PtrAt( UInt32( 0))[].ExecuteLoop()
        print( "DoLaunch Over")
        return True
     
    fn  Size( self) -> UInt32:
        return self._Maestros.Size()  
        
    fn  IsLocked( self, id: UInt32 ) -> Bool :
        return id > self._LockedMark
    
    fn  IncrSzSchedJob( mut self, inc : UInt32) -> UInt32:
        return self._SzSchedJob.Incr( inc)
  

    fn  SuccIdAt( mut self, jobId: UInt16) -> UInt16:
        return self._SuccIds.PtrAt( jobId)[]

    fn  SetSuccIdAt( mut self, jobId: UInt16, succId: UInt16):
        self._SuccIds.PtrAt( jobId)[] = succId
     
    fn  IncrPredAt( mut self, jobId: UInt16) -> UInt16:
        self._SzPreds.PtrAt( jobId)[] += 1
        return self._SzPreds.PtrAt( jobId)[] 
 
    fn  DecrPredAt( mut self, jobId: UInt16) -> UInt16:
        self._SzPreds.PtrAt( jobId)[] -= 1
        return self._SzPreds.PtrAt( jobId)[] 
        

    fn  SetJobAt( mut self, jobId: UInt16, owned runner : Runner): 
        ly = self._JobBuff.PtrAt( jobId)
        ly[] = runner^
        pass 

    fn  JobAt( mut self, jobId: UInt16) -> UnsafePointer[ Runner]: 
        return self._JobBuff.PtrAt( jobId)
        
    fn  AllocJob( mut self) -> UInt16 :
        stk = self._JobSilo.Stack()
        if stk[].Size():
            return stk[].Pop()[]   
        return 0

    fn ConstructJobAt( mut self, jobId : UInt16,   succId : UInt16,  owned runner : Runner):  
        self.SetJobAt( jobId, runner^) 
        self.SetSuccIdAt( jobId, succId)
        _ = self.IncrPredAt( succId)
        

    fn  AllocJobs( mut self, mut stk : Stk[ UInt16, MutableAnyOrigin, _]) -> Bool :
        freeJobs = self._JobSilo.Stack() 
        xSz = stk.Import( freeJobs[]) 
        return xSz != 0
        
    
    fn  FreeJobs( mut self, mut stk : Stk[ UInt16, MutableAnyOrigin, _]) -> Bool :
        freeJobs = self._JobSilo.Stack() 
        xSz = freeJobs[].Import( stk) 
        return xSz != 0
        

    fn  Dump( self): 
        pass
        
#----------------------------------------------------------------------------------------------------------------------------------

fn AtelierExample1():
    atelier = Atelier( 2)  
    g = Maestro()
    x = 10
    fn closure( mut maestro : Maestro) -> Bool:
        print( x)
        return True
    _ = g.PopJob() 
    atelier.SetJobAt( 1, closure)
    var id : UInt16  = 1
    job = atelier._JobBuff.PtrAt( id)
    _ = job[].Score( g)
    atelier.Dump()


fn AtelierExample() : 
    print( "AtelierExample")  
    atelier = Atelier( 4)  
    x = 10
    fn c1( mut maestro : Maestro) -> Bool:
        x += 1
        print( x)
        return True  
    maestro = atelier.Honcho()
    jId = UInt16( 0)
    jId = maestro[].Construct( jId, c1)
    jId = maestro[].Construct( jId, c1) 
    maestro[].EnqueueJob( jId)
    _ = atelier.DoLaunch()
    print( x)
    return 

#--------------------------------------------------------------------------------------------------------------------------------
 
@value
struct SegSort[ Less: fn( p: UInt32, q: UInt32) capturing -> Bool, Swap: fn( p: UInt32, q: UInt32) capturing -> None] :
    var     uSeg : USeg
    
    fn __init__( out self, uSeg : USeg) :
        self.uSeg = uSeg

    fn  BiSort( self, mut maestro : Maestro) -> Bool:
        print( "BiSort : ", str( self.uSeg))
        piv  = self.uSeg.QSortPartition[ Less, Swap]()
        fSz = piv -self.uSeg._First +1 
        if ( fSz > 1):
            segSort = SegSort[ Less, Swap]( USeg( self.uSeg._First, fSz))
            segEncap = segSort.Encap()
            jId = maestro.CurSuccId();
            jId = maestro.Construct( jId, segEncap) 
            maestro.EnqueueJob( jId)
        piv += 1
        sSz = self.uSeg._Last -piv +1
        if ( sSz > 1 ):
            segSort = SegSort[ Less, Swap]( USeg( piv, sSz))
            segEncap = segSort.Encap()
            jId = maestro.CurSuccId();
            jId = maestro.Construct( jId, segEncap) 
            maestro.EnqueueJob( jId)
        return True

    fn Encap( mut self) -> fn( mut maestro : Maestro) escaping -> Bool: 
        fn c1( mut maestro : Maestro) -> Bool:
            return self.BiSort( maestro)
        return c1

import random

fn AtelierSortExample() : 
    print( "AtelierSortExample")  
    vec  = Buff[ Float32]( 80, 0) 
    arr = vec.Arr()  
    for iter in arr: 
        iter[] = int( random.random_ui64( 13, 113))
    arr.SwapAt( 3, 5)   

    @parameter
    fn Less( p: UInt32, q: UInt32) -> Bool: 
        #return  arr.At( p) < arr.At( q)
        return  p < q
        
    @parameter
    fn Swap( p: UInt32, q: UInt32) -> None: 
        arr.SwapAt( p, q) 

    uSeg = USeg( 0, arr.Size())
    segSort = SegSort[ Less, Swap]( uSeg)
    segEncap = segSort.Encap()
    jId = UInt16( 0)
    atelier = Atelier( 1)  

    maestro = atelier.Honcho()
    jId = maestro[].Construct( jId, segEncap)
    maestro[].EnqueueJob( jId)
    _ = atelier.DoLaunch()
    arr.Print()
 
#----------------------------------------------------------------------------------------------------------------------------------
