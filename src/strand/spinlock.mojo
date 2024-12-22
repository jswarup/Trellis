# spinlock.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer
from os import Atomic

import strand

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct SpinLock :  
    alias   AtmFlag = Atm[ True, DType.int64]
    
    var     _Flag: Self.AtmFlag  

    fn __init__( out self):
        self._Flag = Self.AtmFlag( 0)

    fn Lock( mut self):   
        var     expected  = Int64( 0)
        while not self._Flag.CompareExchange( expected, 1): 
            pass
        return

    fn Unlock(mut self): 
        self._Flag.Set( 0)
        return

#----------------------------------------------------------------------------------------------------------------------------------

struct LockGuard:
    
    var lock: UnsafePointer[ SpinLock]

    fn __init__( mut self, mut  lock: SpinLock) :
        self.lock = UnsafePointer.address_of(lock)

    @no_inline
    fn __enter__(mut self):
        self.lock[].Lock()

    @no_inline
    fn __exit__(mut self):
        self.lock[].Unlock()

#----------------------------------------------------------------------------------------------------------------------------------

fn SpinLockExample():   
    var slock = SpinLock()
    slock.Lock()
    slock.Unlock()

    with LockGuard( slock):
        print( "Got Lock")
        
    return

