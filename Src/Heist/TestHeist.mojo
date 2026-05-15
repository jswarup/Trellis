#- TestHeist.mojo ------------------------------------------------------------------------------------------------------------------

from Heist import * 

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

def MavenTest(): 
    print( "MavenTest:") 
    var    atelier = Atelier()
    mavens = atelier.Mavens()
    var     ms = mavens.PtrAt( 0)
    
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

def LaunchTest(): 
    print( "LaunchTest:") 
    var     atelier = Atelier( 1)
    var     mavens = atelier.Mavens()
    var     ms = mavens.At( 0)
    
    def TrialJob( mut a : Atelier, var mavenInd : UInt16) {}   -> Bool:
        #var    atelier = m.Atelier()
        print( "TrialJob")
        return True

    ms.EnqueueJob( atelier.Construct( ms.Index(), TrialJob)) 

    var     res = atelier.DoLaunch()
    print( res)
    pass

#----------------------------------------------------------------------------------------------------------------------------------

def TestHeist(): 
    #SpinlockTest()
    #MavenTest()
    LaunchTest()
    pass
    
#----------------------------------------------------------------------------------------------------------------------------------
