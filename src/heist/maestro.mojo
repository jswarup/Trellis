# maestro.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from stash import Buff, Silo, Stk
from strand import SpinLock, LockGuard
import heist

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct Maestro( CollectionElement):
    var     _Index: UInt32
    var     _CurSuccId: UInt16

    var     _Atelier: UnsafePointer[ Atelier]

    var     _RunQueue : Silo[ UInt16, True]                 # All runnables.
    var     _RunQlock : SpinLock                            # Spinlock for runnables

    var     _JobCache : Silo[ UInt16, False]                # Free Jobs Cache

    @always_inline
    fn __init__( out self) : 
        self._Atelier = UnsafePointer[ Atelier]()
        self._Index = UInt32.MAX
        self._CurSuccId = 0
        self._RunQueue = Silo[ UInt16, True]( 1024, 0) 
        self._RunQlock = SpinLock()
        self._JobCache = Silo[ UInt16, False]( 64, 0) 
        pass

    @always_inline
    fn __init__( out self, other: Self): 
        self._Index = other._Index  
        self._CurSuccId = other._CurSuccId  
        self._Atelier = other._Atelier  
        self._RunQueue = other._RunQueue 
        self._JobCache = other._JobCache 
        self._RunQlock = SpinLock()
        pass

    @always_inline
    fn __moveinit__( out self, owned other: Self): 
        self._Index = other._Index  
        self._CurSuccId = other._CurSuccId  
        self._Atelier = other._Atelier  
        self._RunQueue = other._RunQueue 
        self._JobCache = other._JobCache 
        self._RunQlock = SpinLock()
        pass
    
    @always_inline
    fn __copyinit__( out self, other: Self): 
        self._Index = other._Index  
        self._CurSuccId = other._CurSuccId  
        self._Atelier = other._Atelier  
        self._RunQueue = other._RunQueue 
        self._JobCache = other._JobCache 
        self._RunQlock = SpinLock()
        pass

    fn SetAtelier( mut self, ind : UInt32, atelier: Atelier):
        self._Index = ind
        self._Atelier = UnsafePointer[ Atelier].address_of( atelier)
        pass

    fn PopJob( mut self)  -> UInt16:         
        with LockGuard( self._RunQlock): 
            xStk = self._RunQueue.Stack() 
            if not xStk[].Size():
                return 0
            jobId = xStk[].Pop()[]
            #print( self._Index, ": PopJob ", jobId)
            return jobId
        
    fn EnqueueJob( mut self, jobId : UInt16): 
        _ = self._Atelier[].IncrSzSchedJob()
        with LockGuard( self._RunQlock): 
            xStk = self._RunQueue.Stack() 
            ind = xStk[].Push( jobId) 
            #print( self._Index, ": EnqueueJob ", jobId, "@", xStk[].Size())
            #xStk[].Print()
        
    fn ExecuteJob( mut self, owned jobId : UInt16): 
        while ( jobId != 0):
            runner = self._Atelier[].JobAt( jobId) 
            _CurSuccId = self._Atelier[].SuccIdAt( jobId) 
            _ = runner.Score( self)
            _ = self.FreeJob( jobId)
            szPred = self._Atelier[].DecrPredAt( _CurSuccId) 
            jobId = _CurSuccId if ( szPred == 0) else 0
            _CurSuccId = UInt16.MAX
        return
    
    fn CurSuccId( self) ->UInt16:
        return self._CurSuccId

    fn ExecuteLoop( mut self) :
        while True:
            jobId = self.PopJob()
            if jobId == 0:
                break
            self.ExecuteJob( jobId)
        print( self._Index, ": Done")
        pass
    
    fn  AllocJob( mut self) -> UInt16 :
        while True:
            stk = self._JobCache.Stack()
            if stk[].Size():
                return stk[].Pop()[]   
            xSz = self._Atelier[].AllocJobs( stk[])
            if xSz == 0:
                break
        return 0

    fn  FreeJob( mut self, jobId : UInt16) -> Bool:
        stk = self._JobCache.Stack()
        while True:
            if stk[].SzVoid():
                _ = stk[].Push( jobId)
                return True
            xSz = self._Atelier[].FreeJobs( stk[])
            if xSz == 0:
                break
        return False
    
    fn  ExtractJobs( mut self, mut stk : Stk[ UInt16, MutableAnyOrigin, _]) -> Bool :
        with LockGuard( self._RunQlock): 
            xStk = self._RunQueue.Stack()
            szX = stk.Import( xStk[])
            return szX != 0
         
 
    fn Construct( mut self, succId : UInt16,  runner : fn( mut maestro : Maestro) escaping -> Bool) -> UInt16: 
        jobId = self.AllocJob()
        self._Atelier[].ConstructJobAt( jobId, succId, runner) 
        return jobId 
     
    
    