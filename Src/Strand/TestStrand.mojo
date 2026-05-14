#- StrandTests.mojo ------------------------------------------------------------------------------------------------------------------

from Strand import *

#----------------------------------------------------------------------------------------------------------------------------------

def MaestroTest(): 
    print( "MaestroTest:") 
    var    atelier = Atelier()
    maestros = atelier.Maestros()
    var     ms = maestros.PtrAt( 0)
    
    a = atelier.SuccIdAt( 0)
    _ = atelier.IncrPredAt( 4, 1)
    jId = atelier.AllocJob()
    print( "JobId: ", jId)
    ms[].EnqueueJob( jId)
    
    jobId = ms[].AllocJob()
    print( "JobId: ", jobId)
    ms[].EnqueueJob( jobId)
    ms[].EnqueueJob( ms[].AllocJob())
    ms[].EnqueueJob( ms[].AllocJob())
    var     jobArr = atelier.JobArr()
    while True:
        var    jobId = atelier.GrabJob()
        if ( jobId == 0):
            break;
        print( "Grab:", jobId)
        _ = jobArr[ jobId]( ms[])

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

def TestStrand(): 
    SpinlockTest()
    MaestroTest()
    pass
    
#----------------------------------------------------------------------------------------------------------------------------------
