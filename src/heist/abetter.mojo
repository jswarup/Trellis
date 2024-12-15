# abetter.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from stash import Buff, Silo
from strand import SpinLock, LockGuard
import heist

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct Abettor( CollectionElement):
    var     _Index: UInt32
    var     _Crew: UnsafePointer[ Crew]
    var     _JobCache : Silo[ UInt16, True]
    var     _JobDepot : Silo[ UInt16, False]
    var     _Spinlock : SpinLock

    @always_inline
    fn __init__( out self) : 
        self._Crew = UnsafePointer[ Crew]()
        self._Index = UInt32.MAX
        self._JobCache = Silo[ UInt16, True]( 1024)
        self._JobDepot = Silo[ UInt16, False]( 64)
        self._Spinlock = SpinLock()
        pass

    @always_inline
    fn __init__( out self, other: Self): 
        self._Index = other._Index  
        self._Crew = other._Crew  
        self._JobCache = other._JobCache
        self._JobDepot = other._JobDepot
        self._Spinlock = SpinLock()
        pass

    @always_inline
    fn __moveinit__( out self, owned other: Self): 
        self._Index = other._Index  
        self._Crew = other._Crew  
        self._JobCache = other._JobCache
        self._JobDepot = other._JobDepot
        self._Spinlock = SpinLock()
        pass

    fn SetCrew( inout self, ind : UInt32, crew: Crew):
        self._Index = ind
        self._Crew = UnsafePointer[ Crew].address_of( crew)
        pass

    fn PopJob( inout self)  -> UInt16:
        with LockGuard( self._Spinlock):
            return self._JobCache.Pop()[] 
       

    fn EnqueueJob( inout self, jobId : UInt16): 
        _ = self._Crew[]._Caper.IncrSzSchedJob()
        
   
    fn ExecuteLoop( self) :
        print( self._Index, ": Done")
        pass

    