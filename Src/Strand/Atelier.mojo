# Atelier.mojo -----------------------------------------------------------------------------------------------------------------------

from Silo import *
from Strand import Atm, Spinlock, Lockguard

#----------------------------------------------------------------------------------------------------------------------------------

struct Atelier:
    var     _StartCount: UInt32                         # Count of Processing Queue started, used for startup and shutdown 
    var     _SzSchedJob: Atm[ DType.uint32]       # Count of cumulative scheduled jobs in Works and Queues
    var     _SzQueue: Atm[ DType.uint32]     
    var     _Lock: Spinlock
    var     _LockedMark: UInt32
    var     _JobSilo: Stash[ UInt16]               # A Stack of free jobIds
 
    var     _SzPreds: Buff[ UInt16]                     # Count of predessors for job at the jobId
    var     _SuccIds: Buff[ UInt16]                     # Successor job for the job at the jobId

#    var     _Maestros: Buff[ Maestro[ Atelier]]                   # All the Maestros
    
    @always_inline
    def __init__( out self ) : 
        self._StartCount = UInt32( 0)
        self._SzSchedJob = Atm[ DType.uint32] ( 0)
        self._SzQueue = Atm[ DType.uint32] ( 0)
        self._LockedMark = UInt32.MAX 
        self._Lock = Spinlock()
        mx = UInt16.MAX.cast[ DType.uint32]()
        self._JobSilo = Stash[ UInt16]( mx)
        _ = self._JobSilo.DoIndexSetup( True) 
        self._SzPreds = Buff[ UInt16]( mx, UInt16( 0))
        self._SuccIds = Buff[ UInt16]( mx, UInt16( 0)) 
        pass
        
    def __del__( deinit self): 
        #print( "Atelier: Del ")
        pass