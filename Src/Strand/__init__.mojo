from .Atm import *
from .Spinlock import *
 

def SpinlockTest(): 
     
    var     slock = Spinlock()
    slock.Lock()
    slock.Unlock()

    with Lockguard( slock):
        print( "Got Lock")
    pass

def StrandTest(): 
    SpinlockTest()
    pass
