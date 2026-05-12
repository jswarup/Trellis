# Maestro.mojo -----------------------------------------------------------------------------------------------------------------------

from Silo import *
from Strand import Atm, Spinlock, Lockguard 

#----------------------------------------------------------------------------------------------------------------------------------
trait AtelierT:
    def  AllocJob( mut self) -> UInt16 :
        ...
    def  AllocJobs( mut self, mut stk : Stk[ UInt16, _]) -> UInt32 :
        ...
    
#----------------------------------------------------------------------------------------------------------------------------------

struct Maestro [ Atelier: AtelierT, origin: Origin = MutAnyOrigin]( Movable, Copyable, ImplicitlyCopyable):  
    
    comptime _UPtr = UnsafePointer[ Self.Atelier, MutAnyOrigin]

    var     _Index: UInt32
    var     _CurSuccId: UInt16 
    var     _Atelier: Self._UPtr

    var     _RunQueue : Stash[ UInt16]                 # All runnables.
    var     _RunQlock : Spinlock                            # Spinlock for runnables

    var     _JobCache : Stash[ UInt16]                # Free Jobs Cache
    var     _SzProcessed : UInt32

    var     _TJobSilo : Stash[ UInt16]  

    @always_inline
    def __init__( out self) : 
        self._Atelier = Self._UPtr.unsafe_dangling()
        self._Index = UInt32.MAX
        self._CurSuccId = 0
        self._RunQueue = Stash[ UInt16]( 1024, 0) 
        self._RunQlock = Spinlock()
        self._JobCache = Stash[ UInt16]( 64, 0) 
        self._TJobSilo = Stash[ UInt16]( 1024, 0) 
        self._SzProcessed = 0
        pass
        
    def __init__( out self, *, copy: Self): 
        self._Atelier = copy._Atelier
        self._Index = UInt32.MAX
        self._CurSuccId = 0
        self._RunQueue = Stash[ UInt16]( 1024, 0) 
        self._RunQlock = Spinlock()
        self._JobCache = Stash[ UInt16]( 64, 0) 
        self._TJobSilo = Stash[ UInt16]( 1024, 0) 
        self._SzProcessed = 0
        pass

    def __del__( deinit self): 
        #print( "Maestro: Del ")
        pass 
         
    def CurSuccId( self) ->UInt16:
        return self._CurSuccId

    def SetAtelier( mut self, ind : UInt32, mut atelier: Self.Atelier):
        self._Index = ind
        self._Atelier = Self._UPtr( to= atelier)
        pass
     
    def  AllocJob( mut self) -> UInt16 :
        while True:
            stk = self._JobCache.Stk()
            if stk[].Size():
                return stk[].Pop()   
            xSz = self._Atelier[].AllocJobs( stk[])
            if xSz == 0:
                break
        return 0

    def  FreeJob( mut self, jobId : UInt16) -> Bool:
        stk = self._JobCache.Stk()
        while True:
            if stk[].SzVoid():
                _ = stk[].Push( jobId)
                return True
            xSz = self._Atelier[].FreeJobs( stk[])
            if xSz == 0:
                break
        return False
    
    def EnqueueJob( mut self, jobId : UInt16):  
        xStk = self._RunQueue.Stk() 
        with Lockguard( self._RunQlock): 
            _ = xStk[].Push( jobId)  

    def PopJob( mut self)  -> UInt16:       
        xStk = self._RunQueue.Stk()  
        if xStk[].Size():
            with Lockguard( self._RunQlock): 
                if xStk[].Size():
                    return xStk[].Pop()
        return 0
        
#----------------------------------------------------------------------------------------------------------------------------------
