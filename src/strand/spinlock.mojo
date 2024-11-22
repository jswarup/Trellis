# spinlock.mojo ------------------------------------------------------------------------------------------------------------------------

from os import Atomic

import strand

# ----------------------------------------------------------------------------------------------------------------------------------
 
struct SpinLock :  
    var     _Flag:  Atomic[ DType.int64]

    fn __init__(out self):
        self._Flag = Atomic[ DType.int64]( 0)

    fn lock( inout self):   
        var expected  = Int64( 0)
        while not self._Flag.compare_exchange_weak( expected, 1): 
            pass

    fn unlock(inout self, owner: Int) -> Bool: 
        var expected  = self._Flag.load()
        while not self._Flag.compare_exchange_weak( expected, 0):
            pass
        return True
