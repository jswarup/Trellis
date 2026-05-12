from .Atm import *
from .Spinlock import *
from .Maestro import *
from .Atelier import *
 
#----------------------------------------------------------------------------------------------------------------------------------

def MaestroTest(): 
    print( "MaestroTest:") 
    var    atelier = Atelier()
    var m = Maestro[ Atelier]()
    a = atelier.SuccIdAt( 0)
    _ = atelier.IncrPredAt( 4, 1)
    _ = atelier.AllocJob()
    
    maestros = atelier.Maestros()
    ms = maestros[ 0]
    jobId = ms.AllocJob()
    print( "JobId: ", jobId)

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
