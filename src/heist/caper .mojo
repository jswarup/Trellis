# caper.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from strand import Atm
from stash  import Arr, Buff, Stk, Silo
import heist

#----------------------------------------------------------------------------------------------------------------------------------
  
trait Runnable:
    fn    Score( inout self, inout grifter:  Grifter) -> Bool:
        pass

#----------------------------------------------------------------------------------------------------------------------------------

struct Caper:
    var _StartCount: UInt32             # Count of Processing Queue started, used for startup and shutdown 
    var _SzSchedJob: UInt32             # Count of cumulative scheduled jobs in Works and Queues
    var _SzQueue: Atm[ True, DType.uint32]     