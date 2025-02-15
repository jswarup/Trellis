# maestro.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from stash import Buff, Silo, Stk, Arr, USeg
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
    var     _SzProcessed : UInt32

    @always_inline
    fn __init__( out self) : 
        self._Atelier = UnsafePointer[ Atelier]()
        self._Index = UInt32.MAX
        self._CurSuccId = 0
        self._RunQueue = Silo[ UInt16, True]( 1024, 0) 
        self._RunQlock = SpinLock()
        self._JobCache = Silo[ UInt16, False]( 64, 0) 
        self._SzProcessed = 0
        pass

    @always_inline
    fn __init__( out self, other: Self): 
        self._Index = other._Index  
        self._CurSuccId = other._CurSuccId  
        self._Atelier = other._Atelier  
        self._RunQueue = other._RunQueue 
        self._JobCache = other._JobCache 
        self._RunQlock = SpinLock()
        self._SzProcessed = 0
        pass

    @always_inline
    fn __moveinit__( out self, owned other: Self): 
        self._Index = other._Index  
        self._CurSuccId = other._CurSuccId  
        self._Atelier = other._Atelier  
        self._RunQueue = other._RunQueue 
        self._JobCache = other._JobCache 
        self._RunQlock = SpinLock()
        self._SzProcessed = other._SzProcessed
        pass
    
    @always_inline
    fn __copyinit__( out self, other: Self): 
        self._Index = other._Index  
        self._CurSuccId = other._CurSuccId  
        self._Atelier = other._Atelier  
        self._RunQueue = other._RunQueue 
        self._JobCache = other._JobCache 
        self._RunQlock = SpinLock()
        self._SzProcessed = other._SzProcessed 
        pass

    fn __del__( owned self): 
        #print( "Maestro: Del ")
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
        _ = self._Atelier[].IncrSzSchedJob( 1)
        with LockGuard( self._RunQlock): 
            xStk = self._RunQueue.Stack() 
            ind = xStk[].Push( jobId) 
            #print( self._Index, ": EnqueueJob ", jobId, "@", xStk[].Size())
            #xStk[].Print()
        
    fn ExecuteJob( mut self, owned jobId : UInt16): 
        while ( jobId != 0):
            runner = self._Atelier[].JobAt( jobId) 
            self._CurSuccId = self._Atelier[].SuccIdAt( jobId)             
            #print( self._Index, ": ExecuteJob ", jobId)
            _ = runner[].Score( self)
            self._SzProcessed += 1
            _ = self.FreeJob( jobId)
            szPred = self._Atelier[].DecrPredAt( self._CurSuccId) 
            jobId = self._CurSuccId if ( szPred == 0) else 0
            self._CurSuccId = 0
        _ = self._Atelier[].IncrSzSchedJob( -1)
        return
    
    fn CurSuccId( self) ->UInt16:
        return self._CurSuccId

    fn ExecuteLoop( mut self) :
        while  self._Atelier[].IncrSzSchedJob( 0) :
            jobId = UInt16( 0)
            if self._CurSuccId: 
                szPred = self._Atelier[].DecrPredAt( self._CurSuccId) 
                jobId = self._CurSuccId if ( szPred == 0) else 0
                print( jobId, " ", szPred)
            if not jobId:
                jobId = self.PopJob() 
            if jobId == 0:
                break
            self.ExecuteJob( jobId)
        print( self._Index, ": ", self._SzProcessed, " Done")
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
 
    fn Construct( mut self, succId : UInt16,  owned runner : Runner) -> UInt16: 
        jobId = self.AllocJob()
        self._Atelier[].SetJobAt( jobId, runner^) 
        self._Atelier[].AssignSucc( jobId, succId)  
        return jobId  

    fn Dispatch( mut self, owned runner : Runner):  
        jId = self.Construct( self._CurSuccId, runner) 
        self.EnqueueJob( jId) 
     
    fn PostBefore( mut self, owned runner : Runner):  
        _ = self._Atelier[].IncrSzSchedJob( 1) 
        _ = self._Atelier[].DecrPredAt( self._CurSuccId) 
        self._CurSuccId = self.Construct( self._CurSuccId, runner._Runner)  
        _ = self._Atelier[].IncrPredAt( self._CurSuccId) 

    fn PostAlong( mut self, owned runner : Runner):  
        jId = self.Construct( self._Atelier[].SuccIdAt( self._CurSuccId), runner._Runner)  
        self.EnqueueJob( jId)  

    fn Post1[ Chore : ChoreIfc]( mut self, mut mule : Chore) :
        ctxt = ChoreContext( 0)
        mule.Sched( self, ctxt)  
        self.Dispatch( ctxt.SuccJobs().Arr())

    fn Dispatch( mut self, jobArr : Arr[ UInt16, _]) :
        j0 = jobArr.At( 0)   
        for i in USeg( 1, jobArr.Size()):
            jobId = jobArr.At( i)
            self._Atelier[].AssignSucc( jobId, self._CurSuccId) 
            self.EnqueueJob( jobId)   
        self._Atelier[].AssignSucc( j0, self._CurSuccId) 
        _ = self._Atelier[].DecrPredAt( self._CurSuccId) 
        self._CurSuccId = j0
        _ = self._Atelier[].IncrPredAt( self._CurSuccId) 

    
    fn Dispatch2( mut self, jobArr : Arr[ UInt16, _]) :
        j0 = jobArr.At( 0)   
        for i in USeg( 1, jobArr.Size()):
            jobId = jobArr.At( i)
            self.EnqueueJob( jobId)    
        _ = self._Atelier[].DecrPredAt( self._CurSuccId) 
        self._CurSuccId = j0
        _ = self._Atelier[].IncrPredAt( self._CurSuccId) 

    fn Post[ Chore : ChoreIfc]( mut self, mut mule : Chore) :
        outJobs = Silo[ UInt16]( 1024, 0)
        mule.SchedBefore( self, outJobs, self._CurSuccId)
        self.Dispatch2( outJobs.Stack()[].Arr())