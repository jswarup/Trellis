# spinlock.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer
from os import Atomic

import strand

#----------------------------------------------------------------------------------------------------------------------------------
 
struct SpinLock :  
    alias   AtmFlag = Atm[ True, DType.int64]
    
    var     _Flag: Self.AtmFlag  

    fn __init__( out self):
        self._Flag = Self.AtmFlag( 0)

    fn Lock( inout self):   
        var     expected  = Int64( 0)
        while not self._Flag.CompareExchange( expected, 1): 
            pass
        return

    fn Unlock(inout self): 
        self._Flag.Set( 0)
        return

#----------------------------------------------------------------------------------------------------------------------------------

struct LockGuard:
    
    var lock: UnsafePointer[ SpinLock]

    fn __init__( inout self, inout lock: SpinLock) :
        self.lock = UnsafePointer.address_of(lock)

    @no_inline
    fn __enter__(inout self):
        self.lock[].Lock()

    @no_inline
    fn __exit__(inout self):
        self.lock[].Unlock()

#----------------------------------------------------------------------------------------------------------------------------------

fn SpinLockExample():   
    var slock = SpinLock()
    slock.Lock()
    slock.Unlock()

    with LockGuard( slock):
        print( "Got Lock")
        
    return

