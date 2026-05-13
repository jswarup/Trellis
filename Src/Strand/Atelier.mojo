# Atelier.mojo -----------------------------------------------------------------------------------------------------------------------

from Silo import *
from Strand import Atm, Spinlock, Lockguard, MaestroT, Maestro, AtelierT

#----------------------------------------------------------------------------------------------------------------------------------
 
#----------------------------------------------------------------------------------------------------------------------------------

def DefaultRunner( mut maestro :  Maestro[ Atelier])  -> Bool:
    return True

struct Atelier ( AtelierT): 
    comptime  _RunnerPtr = def  ( mut maestro : Maestro[ Atelier])  thin -> Bool  

    var     _StartCount: UInt32                         # Count of Processing Queue started, used for startup and shutdown 
    var     _SzSchedJob: Atm[ DType.uint32]       # Count of cumulative scheduled jobs in Works and Queues
    var     _SzQueue: Atm[ DType.uint32]     
    var     _Lock: Spinlock
    var     _LockedMark: UInt32
    var     _JobSilo: Stash[ UInt16]               # A Stack of free jobIds
 
    var     _SzPreds: Buff[ UInt16]                     # Count of predessors for job at the jobId
    var     _SuccIds: Buff[ UInt16]                     # Successor job for the job at the jobId

    var     _JobBuff: Buff[ Self._RunnerPtr]                     # Runner at the jobId
    var     _Maestros: Buff[ Maestro[ Atelier]]                   # All the Maestros
      
    @always_inline
    def __init__( out self, szMaestro: UInt32 = 4) :
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

        def DefaultRunner( mut m : Maestro[ Atelier]) {}   -> Bool:
            print( "hello")
            return True
        self._JobBuff = Buff[ Self._RunnerPtr]( mx, DefaultRunner) 
        self._Maestros = Buff[ Maestro[ Atelier]]( szMaestro, Maestro[ Atelier]()) 
        var ind : UInt32 = 0
        for maestro in self._Maestros.Arr():
            maestro[].SetAtelier( ind, self)
            ind += 1
        pass  

    def __del__( deinit self): 
        #print( "Atelier: Del ")
        pass

    def Maestros( self) -> Arr[ Maestro[ Atelier]]:
        return self._Maestros.Arr()

    def  IsLocked( self, id: UInt32 ) -> Bool :
        return id > self._LockedMark
    
    def  IncrSzSchedJob( mut self, inc : UInt32) -> UInt32:
        return self._SzSchedJob.Incr( inc) 

    def  SuccIdAt( mut self, jobId: UInt16) -> UInt16:
        return self._SuccIds.Arr().PtrAt( jobId)[]

    def  SetSuccIdAt( mut self, jobId: UInt16, succId: UInt16):
        self._SuccIds.Arr().PtrAt( jobId)[] = succId
     
    def  IncrPredAt( mut self, jobId: UInt16, inc : UInt16) -> UInt16:
        self._SzPreds.Arr().PtrAt( jobId)[] += inc
        return self._SzPreds.Arr().PtrAt( jobId)[]  

    def  JobArr( self) ->  Arr[ Self._RunnerPtr]:
        return self._JobBuff.Arr()

    def  JobAt( mut self, jobId: UInt16) -> ref[ _ ] Self._RunnerPtr: 
        return self._JobBuff.At( jobId)[] 
        
    def  SetJobAt( mut self, jobId: UInt16, var runner : Self._RunnerPtr): 
        ly = self._JobBuff.Arr().PtrAt( jobId)
        ly[] = runner^ 
        pass 

    def  AssignSucc( mut self, jobId : UInt16,   succId : UInt16):
        self.SetSuccIdAt( jobId, succId)
        _ = self.IncrPredAt( succId, 1)

    def  AllocJob( mut self) -> UInt16 :
        stk = self._JobSilo.Stk()
        if stk[].Size():
            return stk[].Pop()   
        return 0
 
    def  AllocJobs( mut self, mut stk : Stk[ UInt16, _]) -> UInt32 :
        freeJobs = self._JobSilo.Stk() 
        xSz = freeJobs[].Export( stk) 
        return xSz  
    
    def  FreeJobs( mut self, mut stk : Stk[ UInt16, _]) -> Bool :
        freeJobs = self._JobSilo.Stk() 
        xSz = freeJobs[].Import( stk) 
        return xSz != 0 
    
    def   GrabJob( mut self) -> UInt16 :
        for maestro in self._Maestros.Arr():
            jobId = maestro[].PopJob()
            if jobId:
                return jobId
        return 0
 