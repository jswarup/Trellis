# Atelier.mojo -----------------------------------------------------------------------------------------------------------------------

from std.algorithm import parallelize
from Silo import *
from Heist import Atm, Spinlock, Lockguard, Maven, AtelierT
 #----------------------------------------------------------------------------------------------------------------------------------

struct Pod[ T: AnyType, origin: Origin = MutAnyOrigin]( Copyable, Movable, ImplicitlyCopyable): 
    comptime _UPtr = UnsafePointer[ Self.T, Self.origin] 
    var     _DPtr: Self._UPtr

    def __init__( out self, sz: UInt32, var value: Self.T):
        self._Size = sz
        self._DPtr = alloc[ Self.T]( Int( sz))
        for i in USeg( sz):
            ( self._DPtr + i).init_pointee_copy( value)
#----------------------------------------------------------------------------------------------------------------------------------
 
struct Atelier ( AtelierT):  
    comptime    JobFn_ = def  ( mut atelier : Atelier, var mavenInd : UInt16)  thin -> Bool 
    comptime    JobPtr_ = UnsafePointer[ Self.JobFn_, MutExternalOrigin] 

    var     _StartCount: UInt32                         # Count of Processing Queue started, used for startup and shutdown 
    var     _SzSchedJob: Atm[ DType.uint32]             # Count of cumulative scheduled jobs in Works and Queues
    var     _SzQueue: Atm[ DType.uint32]     
    var     _Lock: Spinlock
    var     _LockedMark: UInt32
    var     _JobSilo: Stash[ UInt16]                    # A Stack of free jobIds
 
    var     _SzPreds: Buff[ UInt16]                     # Count of predessors for job at the jobId
    var     _SuccIds: Buff[ UInt16]                     # Successor job for the job at the jobId

    var     _JobBuff: Buff[ Self.JobFn_]                # Job  at the jobId
    var     _Mavens: Buff[ Maven[ Atelier]]         # All the Mavens
      
    @always_inline
    def __init__( out self, szMaven: UInt32 = 1) :
        self._StartCount = UInt32( 0)
        self._SzSchedJob = Atm[ DType.uint32] ( 0)
        self._SzQueue = Atm[ DType.uint32] ( 0)
        self._LockedMark = UInt32.MAX 
        self._Lock = Spinlock()
        mx = UInt16.MAX.cast[ DType.uint32]()
        self._JobSilo = Stash[ UInt16]( mx, 0)
        _ = self._JobSilo.DoIndexSetup( True) 
        self._SzPreds = Buff[ UInt16]( mx, UInt16( 0))
        self._SuccIds = Buff[ UInt16]( mx, UInt16( 0)) 

        def DefaultJob ( mut a : Atelier, var mavenInd : UInt16) {}   -> Bool:
            print( "hello")
            return True
        self._JobBuff = Buff[ Self.JobFn_]( mx, DefaultJob ) 
        self._Mavens = Buff[ Maven[ Atelier]]( szMaven, Maven[ Atelier]()) 
        var     ind : UInt16 = 0
        for maven in self._Mavens.Arr():
            maven[].SetAtelier( ind, self)
            ind += 1
        pass  

    @always_inline
    def __del__( deinit self): 
        #print( "Atelier: Del ")
        pass

    def Mavens( self) -> Arr[ Maven[ Atelier]]:
        return self._Mavens.Arr()

    @always_inline
    def  IsLocked( self, id: UInt32 ) -> Bool :
        return id > self._LockedMark
	
    @always_inline
    def  IncrSzSchedJob( mut self, var inc : UInt32) -> UInt32:
        return self._SzSchedJob.Incr( inc) 

    @always_inline
    def  SuccIdAt( mut self, jobId: UInt16) -> UInt16:
        return self._SuccIds.Arr().PtrAt( jobId)[]
	
    @always_inline
    def  SetSuccIdAt( mut self, jobId: UInt16, succId: UInt16):
        self._SuccIds.Arr().PtrAt( jobId)[] = succId
     
    @always_inline
    def  IncrPredAt( mut self, jobId: UInt16, inc : UInt16) -> UInt16:
        self._SzPreds.Arr().PtrAt( jobId)[] += inc
        return self._SzPreds.Arr().PtrAt( jobId)[]  

    @always_inline
    def  JobArr( self) ->  Arr[ Self.JobFn_]:
        return self._JobBuff.Arr() 

    @always_inline
    def  AssignSucc( mut self, jobId : UInt16,   succId : UInt16):
        self.SetSuccIdAt( jobId, succId)
        _ = self.IncrPredAt( succId, 1)

    @always_inline
    def  AllocJob( mut self) -> UInt16 :
        var     stk = self._JobSilo.Stk()
        if stk[].Size():
            return stk[].Pop()   
        return 0
 
    @always_inline
    def  AllocJobs( mut self, mut stk : Stk[ UInt16, _]) -> UInt32 :
        var     freeJobs = self._JobSilo.Stk() 
        xSz = freeJobs[].Export( stk) 
        return xSz  
    
    def  FreeJobs( mut self, mut stk : Stk[ UInt16, _]) -> UInt32 :
        var     freeJobs = self._JobSilo.Stk() 
        var     xSz = freeJobs[].Import( stk) 
        return xSz
    
    def   GrabJob( mut self) -> UInt16 :
        for maven in self._Mavens.Arr():
            var     jobId = maven[].PopJob()
            if jobId:
                return jobId
        return 0
    
    def Construct( mut self,  var mavenInd : UInt16, var job : Self.JobFn_) -> UInt16: 
        var     maven = self._Mavens.Arr().At( mavenInd)
        jobId = maven.AllocJob()
        self._JobBuff.Arr().SetAt( jobId, job) 
        self.AssignSucc( jobId, maven.CurSuccId())  
        return jobId   

    def DoLaunch( self) -> Bool: 
        def worker( ind: Int) { self}: 
            self._Mavens.Arr().PtrAt( UInt32( ind +1))[].ExecuteLoop()
        pass
        
        print( "DoLaunch")
        var     szWorker = self._Mavens.Size() -1
        if ( szWorker):
            parallelize( worker, Int( szWorker))
        self._Mavens.Arr().PtrAt( UInt32( 0))[].ExecuteLoop()
        print( "DoLaunch Over")
        return True
      
    def ExecuteJob( mut self, var mavenInd : UInt16, var jobId : UInt16): 
        var     maven = self._Mavens.Arr()[ mavenInd]
        var     jobArr = self._JobBuff.Arr()
        while ( jobId != 0):
            var     job  = jobArr.At( jobId)  
            maven._CurSuccId = self.SuccIdAt( jobId)    
            _ = job ( self, mavenInd)
            maven._SzProcessed += 1
            var     res = maven.FreeJob( jobId)
            _ = res                             #Handle later
            var     szPred = self.IncrPredAt( maven._CurSuccId, -1) 
            jobId = maven._CurSuccId if ( szPred == 0) else 0
            maven._CurSuccId = 0
            _ = self._SzSchedJob.Incr( -1)
        return