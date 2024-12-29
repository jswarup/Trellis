# crew.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from algorithm import parallelize, vectorize
from strand import Atm, SpinLock
from stash import Buff, Arr, Silo, Stk
import heist

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct Runner( CollectionElement):
    var     _Runner : fn( )  escaping-> Bool 
    
    fn __init__( out self) : 
        x = 0
        fn  defaut() -> Bool:
            return x == 0

        self._Runner = defaut 

    fn __init__( out self,   runner : fn() escaping -> Bool) : 
        self._Runner = runner 

    fn    Score(  self) -> Bool:
        return self._Runner()
        
 #----------------------------------------------------------------------------------------------------------------------------------

struct Crew:
    var     _StartCount: UInt32                         # Count of Processing Queue started, used for startup and shutdown 
    var     _SzSchedJob: Atm[ True, DType.uint32]       # Count of cumulative scheduled jobs in Works and Queues
    var     _SzQueue: Atm[ True, DType.uint32]     
    var     _Lock: SpinLock
    var     _LockedMark: UInt32
    var     _JobSilo: Silo[ UInt16, True]
    var     _JobBuff: Buff[ Runner]
    var     _SzPreds: Buff[ UInt16]
    var     _SuccIds: Buff[ UInt16]
    var     _Abettors: Buff[ Abettor] 

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
        self._Abettors = Buff[ Abettor]( mxQueue, Abettor()) 
        var ind : UInt32 = 0
        for g in self._Abettors.Arr():
            g[].SetCrew( ind, self)
            ind += 1
        pass
        
    fn Abettors( self) -> Arr[ Abettor, __origin_of( self._Abettors._DPtr)]:
        return self._Abettors.Arr()
    
    fn DoLaunch( self) -> Bool:
        abettors = self.Abettors()
        @parameter
        fn worker( ind: Int):
            abettor = abettors.PtrAt( ind)
            abettor[].ExecuteLoop()
        pass

        parallelize[ worker]( abettors.__len__())
        return True
     
    fn  Size( self) -> UInt32:
        return self._Abettors.Size() 
  

    fn  IsLocked( self, id: UInt32 ) -> Bool :
        return id > self._LockedMark
    
    fn  IncrSzSchedJob( mut self) -> UInt32:
        return self._SzSchedJob.Incr( 1)
 
    fn  DecrSzSchedJob( mut self)  -> UInt32:
        return self._SzSchedJob.Decr( 1)

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
        

    fn  SetJobAt( mut self, jobId: UInt16, runner : fn() escaping -> Bool): 
        ly = self._JobBuff.PtrAt( jobId)
        ly[] = Runner( runner) 
        pass 

    fn  JobAt( mut self, jobId: UInt16) -> Runner: 
        ly = self._JobBuff.PtrAt( jobId)
        return ly[]

    
    fn  AllocJob( mut self) -> UInt16 :
        stk = self._JobSilo.Stack()
        if stk[].Size():
            return stk[].Pop()[]   
        return 0

    fn ConstructJobAt( mut self, jobId : UInt16,   succId : UInt16,  runner : fn() escaping -> Bool):  
        self.SetJobAt( jobId, runner) 
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

fn CrewExample1():
    crew = Crew( 2)  
    g = Abettor()
    x = 10
    fn closure() -> Bool:
        print( x)
        return True
    _ = g.PopJob() 
    crew.SetJobAt( 1, closure) 
    var id : UInt16  = 1
    job = crew._JobBuff.PtrAt( id)
    _ = job[].Score()
    crew.Dump()

fn CrewExample() : 
    print( "CrewExample")  
    crew = Crew( 4)  
    x = 10
    fn c1() -> Bool:
        x += 1
        print( x)
        return True  
    abettors = crew.Abettors()
    abettor = abettors.PtrAt( 0) 
    jId = UInt16( 0)
    jId = abettor[].Construct( jId, c1)
    jId = abettor[].Construct( jId, c1) 
    abettor[].EnqueueJob( jId)
    _ = crew.DoLaunch()
    print( x)
    return 

#----------------------------------------------------------------------------------------------------------------------------------

fn CrewSortExample() : 
    print( "CrewSortExample")  
    crew = Crew( 4)  
 
#----------------------------------------------------------------------------------------------------------------------------------
