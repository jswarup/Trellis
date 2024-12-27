# abetter.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from stash import Buff, Silo, Stk
from strand import SpinLock, LockGuard
import heist

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct Abettor( CollectionElement):
    var     _Index: UInt32
    var     _Crew: UnsafePointer[ Crew]
    var     _RunQueue : Silo[ UInt16, True]
    var     _JobShed : Silo[ UInt16, False]
    var     _Spinlock : SpinLock

    @always_inline
    fn __init__( out self) : 
        self._Crew = UnsafePointer[ Crew]()
        self._Index = UInt32.MAX
        self._RunQueue = Silo[ UInt16, True]( 1024)
        self._JobShed = Silo[ UInt16, False]( 64)
        self._Spinlock = SpinLock()
        pass

    @always_inline
    fn __init__( out self, other: Self): 
        self._Index = other._Index  
        self._Crew = other._Crew  
        self._RunQueue = other._RunQueue
        self._JobShed = other._JobShed
        self._Spinlock = SpinLock()
        pass

    @always_inline
    fn __moveinit__( out self, owned other: Self): 
        self._Index = other._Index  
        self._Crew = other._Crew  
        self._RunQueue = other._RunQueue
        self._JobShed = other._JobShed
        self._Spinlock = SpinLock()
        pass

    fn SetCrew( mut self, ind : UInt32, crew: Crew):
        self._Index = ind
        self._Crew = UnsafePointer[ Crew].address_of( crew)
        pass

    fn PopJob( mut self)  -> UInt16: 
        return self._RunQueue.Pop()[] 
       

    fn EnqueueJob( mut self, jobId : UInt16): 
        _ = self._Crew[]._Atelier.IncrSzSchedJob()
        with LockGuard( self._Spinlock): 
            xStk = self._RunQueue.Stack()
            _ = xStk.Push( jobId) 
   
    fn ExecuteLoop( mut self) :
        while True:
            jobId = self.PopJob()
            if not jobId:
                break
            runner = self._Crew[]._Atelier.FetchJobAt( jobId) 
            _ = runner.Score()
        print( self._Index, ": Done")
        pass
 
    fn  ExtractJobs( mut self, mut stk : Stk[ UInt16, MutableAnyOrigin, _]) -> Bool :
        with LockGuard( self._Spinlock): 
            xStk = self._RunQueue.Stack()
            szX = stk.Import( xStk)
            return szX != 0
    
    fn  FillShed( mut self) -> Bool:
        shedStk = self._JobShed.Stack()
        res = self.ExtractJobs( shedStk)
        if res:
            return True
        while self._Crew[].HuntJob( shedStk):
            pass
        return shedStk.Size()

    
        
    fn  AllocJob( mut self) -> UInt16 :
        stk = self._JobShed.Stack()
        if stk.Size():
            return stk.Pop()[]   
        return 0
 
    fn Construct( mut self, succId : UInt16,  runner : fn() escaping -> Bool) -> UInt16: 
        jobId = self.AllocJob()
        self._Crew[]._Atelier.FillJobAt( jobId, runner) 
        self._Crew[]._Atelier.SetSuccIdAt( jobId, succId);
        return jobId 
     
    