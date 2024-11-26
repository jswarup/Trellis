# caper.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from strand import Atm
from stash  import Arr, Buff, Stk, Silo
import heist

#----------------------------------------------------------------------------------------------------------------------------------
  
trait Runnable( CollectionElement):
    fn    Score( inout self, inout grifter:  Grifter) -> Bool:
        pass

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct  Runner[ T : Runnable] :
    var    _Runner : UnsafePointer[ T]
 
    fn __init__( out self) :
        self._Runner = UnsafePointer[ T]()

    fn __init__( out self, inout runable: T) :
        self._Runner = UnsafePointer[ T].address_of( runable)

    fn  DoRun( self, inout grifter:  Grifter) -> Bool:
        return self._Runner[].Score( grifter)    

#----------------------------------------------------------------------------------------------------------------------------------

struct Caper:
    var     _StartCount: UInt32                         # Count of Processing Queue started, used for startup and shutdown 
    var     _SzSchedJob: UInt32                         # Count of cumulative scheduled jobs in Works and Queues
    var     _SzQueue: Atm[ True, DType.uint32]     
    var     _JobSilo: Silo[ UInt16]
    var     _JobBuff: Buff[ Runner[ Runnable], False]

    fn __init__( out self) :
        self._StartCount = 0
        self._SzSchedJob = 0
        self._SzQueue = UInt32( 0)
        mx = UInt16.MAX.cast[ DType.uint32]()
        self._JobSilo = Silo[ UInt16]( mx)
        self._JobBuff = Buff[ Runner[ Runnable], False]( mx, Runner[ Runnable]())
        
        pass
    
    fn  Dump( self): 
        
        pass

fn CaperExample():
    caper = Caper()
    caper.Dump()