# Atelier.mojo -----------------------------------------------------------------------------------------------------------------------

from std.algorithm import parallelize
from Silo import *
from Strand import Atm, Spinlock, Lockguard, Maestro, AtelierT
 
#----------------------------------------------------------------------------------------------------------------------------------
 
struct Atelier ( AtelierT):  
    comptime    JobFn_ = def  ( mut atelier : Atelier, var maestroInd : UInt16)  thin -> Bool 

    var     _StartCount: UInt32                         # Count of Processing Queue started, used for startup and shutdown 
    var     _SzSchedJob: Atm[ DType.uint32]             # Count of cumulative scheduled jobs in Works and Queues
    var     _SzQueue: Atm[ DType.uint32]     
    var     _Lock: Spinlock
    var     _LockedMark: UInt32
    var     _JobSilo: Stash[ UInt16]                    # A Stack of free jobIds
 
    var     _SzPreds: Buff[ UInt16]                     # Count of predessors for job at the jobId
    var     _SuccIds: Buff[ UInt16]                     # Successor job for the job at the jobId

    var     _JobBuff: Buff[ Self.JobFn_]                # Job  at the jobId
    var     _Maestros: Buff[ Maestro[ Atelier]]         # All the Maestros
      
    @always_inline
    def __init__( out self, szMaestro: UInt32 = 1) :
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

        def DefaultJob ( mut a : Atelier, var maestroInd : UInt16) {}   -> Bool:
            print( "hello")
            return True
        self._JobBuff = Buff[ Self.JobFn_]( mx, DefaultJob ) 
        self._Maestros = Buff[ Maestro[ Atelier]]( szMaestro, Maestro[ Atelier]()) 
        var     ind : UInt16 = 0
        for maestro in self._Maestros.Arr():
            maestro[].SetAtelier( ind, self)
            ind += 1
        pass  

    @always_inline
    def __del__( deinit self): 
        #print( "Atelier: Del ")
        pass

    def Maestros( self) -> Arr[ Maestro[ Atelier]]:
        return self._Maestros.Arr()

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
        for maestro in self._Maestros.Arr():
            var     jobId = maestro[].PopJob()
            if jobId:
                return jobId
        return 0
    
    def Construct( mut self,  var maestroInd : UInt16, var job : Self.JobFn_) -> UInt16: 
        var     maestro = self._Maestros.Arr().At( maestroInd)
        jobId = maestro.AllocJob()
        self._JobBuff.Arr().SetAt( jobId, job) 
        self.AssignSucc( jobId, maestro.CurSuccId())  
        return jobId   

    def DoLaunch( self) -> Bool: 
        def worker( ind: Int) { self}: 
            self._Maestros.Arr().PtrAt( UInt32( ind +1))[].ExecuteLoop()
        pass
        
        print( "DoLaunch")
        var     szWorker = self._Maestros.Size() -1
        if ( szWorker):
            parallelize( worker, Int( szWorker))
        self._Maestros.Arr().PtrAt( UInt32( 0))[].ExecuteLoop()
        print( "DoLaunch Over")
        return True
      
    def ExecuteJob( mut self, var maestroInd : UInt16, var jobId : UInt16): 
        var     maestro = self._Maestros.Arr()[ maestroInd]
        var     jobArr = self._JobBuff.Arr()
        while ( jobId != 0):
            var     job  = jobArr.At( jobId)  
            maestro._CurSuccId = self.SuccIdAt( jobId)    
            _ = job ( self, maestroInd)
            maestro._SzProcessed += 1
            var     res = maestro.FreeJob( jobId)
            var     szPred = self.IncrPredAt( maestro._CurSuccId, -1) 
            jobId = maestro._CurSuccId if ( szPred == 0) else 0
            maestro._CurSuccId = 0
            _ = self._SzSchedJob.Incr( -1)
        return