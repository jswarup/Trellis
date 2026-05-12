# Atelier.mojo -----------------------------------------------------------------------------------------------------------------------

from Silo import *
from Strand import Atm, Spinlock, Lockguard, Maestro, AtelierT

#----------------------------------------------------------------------------------------------------------------------------------

struct Runner( ImplicitlyCopyable):
    var     _Doc : String
    var     _JobId : UInt16  
 
    @always_inline
    def __init__( out self) : 
        self._JobId = UInt16.MAX
        self._Doc = String()   

    @always_inline
    def      SetJobId( mut self, jobId : UInt16) :
        self._JobId = jobId
    
    @always_inline
    def  write_to[W: Writer](self, mut writer: W):
        writer.write( "[ " + String( self._JobId) + "]")

#----------------------------------------------------------------------------------------------------------------------------------

struct Atelier ( AtelierT):
    var     _StartCount: UInt32                         # Count of Processing Queue started, used for startup and shutdown 
    var     _SzSchedJob: Atm[ DType.uint32]       # Count of cumulative scheduled jobs in Works and Queues
    var     _SzQueue: Atm[ DType.uint32]     
    var     _Lock: Spinlock
    var     _LockedMark: UInt32
    var     _JobSilo: Stash[ UInt16]               # A Stack of free jobIds
 
    var     _SzPreds: Buff[ UInt16]                     # Count of predessors for job at the jobId
    var     _SuccIds: Buff[ UInt16]                     # Successor job for the job at the jobId

    var     _JobBuff: Buff[ Runner]                     # Runner at the jobId
    var     _Maestros: Buff[ Maestro[ Atelier]]                   # All the Maestros
    
    @always_inline
    def __init__( out self, szMaestro: UInt32 = 4) :
        self._StartCount = UInt32( 0)
        self._SzSchedJob = Atm[ DType.uint32] ( 0)
        self._SzQueue = Atm[ DType.uint32] ( 0)
        self._LockedMark = UInt32.MAX 
        self._Lock = Spinlock()
        mx = UInt16.MAX.cast[ DType.uint32]()
        self._JobSilo = Stash[ UInt16]( mx)
        _ = self._JobSilo.DoIndexSetup( True) 
        self._SzPreds = Buff[ UInt16]( mx, UInt16( 0))
        self._SuccIds = Buff[ UInt16]( mx, UInt16( 0)) 
        self._JobBuff = Buff[ Runner]( mx, Runner()) 
        self._Maestros = Buff[ Maestro[ Atelier]]( szMaestro) 
        var ind : UInt32 = 0
        for g in self._Maestros.Arr():
            g[].SetAtelier( ind, self)
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
        
    def  JobAt( mut self, jobId: UInt16) -> UnsafePointer[ Runner]: 
        return self._JobBuff.PtrAt( jobId)
        
    def  SetJobAt( mut self, jobId: UInt16, deinit runner : Runner): 
        ly = self._JobBuff.Arr().PtrAt( jobId)
        ly[] = runner^
        ly[].SetJobId( jobId)
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
        xSz = stk.Import( freeJobs[]) 
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
 