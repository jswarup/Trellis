from .Atm import *
from .Spinlock import *
#from .Maestro import *
from .Atelier import *
 
#----------------------------------------------------------------------------------------------------------------------------------

def MaestroTest(): 
    print( "MaestroTest:") 
    var    atelier = Atelier()
    #var m = Maestro[ UInt32]()

#----------------------------------------------------------------------------------------------------------------------------------

def SpinlockTest(): 
    print( "SpinlockTest:") 
    var     slock = Spinlock()
    slock.Lock()
    slock.Unlock()

    with Lockguard( slock):
        print( "Got Lock")
    pass

#----------------------------------------------------------------------------------------------------------------------------------

def StrandTest(): 
    SpinlockTest()
    MaestroTest()
    pass

#----------------------------------------------------------------------------------------------------------------------------------
