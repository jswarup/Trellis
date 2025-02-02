# atelier.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from algorithm import parallelize, vectorize
from strand import Atm, SpinLock
from stash import Buff, Arr, Silo, Stk, USeg
import heist

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct RunAfter[ TLeft: StringableCollectionElement, TRight: StringableCollectionElement] ( StringableCollectionElement):   
    var     _Left : TLeft
    var     _Right : TRight

    fn __init__( out self, owned left : TLeft, owned right : TRight):  
        self._Left = left^ 
        self._Right = right^

    fn __str__( self) -> String:
        str = "[ " + self._Left.__str__() + " >> " + self._Right.__str__() + "]"
        return str

    fn __rshift__[ TNext: StringableCollectionElement]( owned self, owned succ : TNext) -> RunAfter[ Self, TNext] : 
        return RunAfter( self, succ) 

    fn __or__[ TAlong: StringableCollectionElement]( owned self, owned succ : TAlong) -> RunAlong[ Self, TAlong] : 
        return RunAlong( self, succ) 

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct RunAlong[ TLeft: StringableCollectionElement, TRight: StringableCollectionElement] ( StringableCollectionElement):   
    var     _Left : TLeft
    var     _Right : TRight
 
    @always_inline
    fn __init__( out self, owned left : TLeft, owned right : TRight):  
        self._Left = left^
        self._Right = right^

    @always_inline
    fn __str__( self) -> String:
        str = "[ " + self._Left.__str__() + " | " + self._Right.__str__() + "]"
        return str

    fn __rshift__[ TNext: StringableCollectionElement]( owned self, owned succ : TNext) -> RunAfter[ Self, TNext] : 
        return RunAfter( self, succ) 

    fn __or__[ TAlong: StringableCollectionElement]( owned self, owned succ : TAlong) -> RunAlong[ Self, TAlong] : 
        return RunAlong( self, succ) 


@value
struct RunIt( StringableCollectionElement):
    var     _Runner : fn( mut maestro : Maestro)  escaping -> Bool 
    var     _Doc : String
    var     _JobId : UInt16  

    @always_inline
    fn __init__( out self) : 
        self._JobId = UInt16.MAX
        self._Doc = String()
        x = 0
        fn  default( mut maestro : Maestro) -> Bool:
            return x == 0
        self._Runner = default 

    @implicit
    fn __init__( out self, runner : fn( mut maestro : Maestro) escaping -> Bool) : 
        self._JobId = UInt16.MAX
        self._Doc = String()
        self._Runner = runner 
  
    fn __init__( out self, runner : fn( mut maestro : Maestro) escaping -> Bool, doc : String) : 
        self._JobId = UInt16.MAX
        self._Doc = doc
        self._Runner = runner 
 
    fn __del__( owned self): 
        m = Maestro()
        print( "RunIt: Del: ", self._Doc)
        pass
    @always_inline
    fn    Score(  self, mut maestro : Maestro) -> Bool:
        return self._Runner( maestro)

    @always_inline
    fn      SetJobId( mut self, jobId : UInt16) :
        self._JobId = jobId
    
    @always_inline
    fn __str__( self) -> String:
        str = "[ " + self._Doc + "]"
        return str
    
    @always_inline
    fn __rshift__[ TSucc: StringableCollectionElement]( owned self, owned succ : TSucc) -> RunAfter[ RunIt, TSucc] : 
        return RunAfter( self^, succ^) 

    @always_inline
    fn __or__[ TAlong: StringableCollectionElement]( owned self, owned along : TAlong) -> RunAlong[ RunIt, TAlong] : 
        return RunAlong( self^, along^) 

@value
struct Runner( StringableCollectionElement):
    var     _Runner : fn( mut maestro : Maestro)  escaping -> Bool 
    var     _Doc : String
    var     _JobId : UInt16  

    @staticmethod
    fn Default() -> fn( mut maestro : Maestro)  escaping -> Bool: 
        x = 0
        fn  default( mut maestro : Maestro) -> Bool:
            return x == 0
        return default
         
    @always_inline
    fn __init__( out self) : 
        self._JobId = UInt16.MAX
        self._Doc = String() 
        self._Runner = Self.Default() 

    @implicit
    fn __init__( out self, runner : fn( mut maestro : Maestro) escaping -> Bool) : 
        self._JobId = UInt16.MAX
        self._Doc = String()
        self._Runner = runner 
  
    fn __init__( out self, runner : fn( mut maestro : Maestro) escaping -> Bool, doc : String) : 
        self._JobId = UInt16.MAX
        self._Doc = doc
        self._Runner = runner 
 
    fn __del__( owned self): 
        m = Maestro()
        #print( "Runner: Del: ", self._Doc)
        pass 
    
    @always_inline
    fn    Score(  self, mut maestro : Maestro) -> Bool:
        return self._Runner( maestro)

    @always_inline
    fn      SetJobId( mut self, jobId : UInt16) :
        self._JobId = jobId
    
    @always_inline
    fn __str__( self) -> String:
        str = "[ " + str( self._JobId) + "]"
        return str

 #----------------------------------------------------------------------------------------------------------------------------------

struct Atelier:
    var     _StartCount: UInt32                         # Count of Processing Queue started, used for startup and shutdown 
    var     _SzSchedJob: Atm[ True, DType.uint32]       # Count of cumulative scheduled jobs in Works and Queues
    var     _SzQueue: Atm[ True, DType.uint32]     
    var     _Lock: SpinLock
    var     _LockedMark: UInt32
    var     _JobSilo: Silo[ UInt16, True]               # A Stack of free jobIds

    var     _JobBuff: Buff[ Runner]                     # Runner at the jobId
    var     _SzPreds: Buff[ UInt16]                     # Count of predessors for job at the jobId
    var     _SuccIds: Buff[ UInt16]                     # Successor job for the job at the jobId

    var     _Maestros: Buff[ Maestro]                   # All the Maestros

    @always_inline
    fn __init__( out self, mxQueue : UInt32 ) : 
        self._StartCount = UInt32( 0)
        self._SzSchedJob = UInt32( 0)
        self._SzQueue = UInt32( 0)
        self._LockedMark = UInt32.MAX 
        self._Lock = SpinLock()
        mx = UInt16.MAX.cast[ DType.uint32]()
        self._JobSilo = Silo[ UInt16, True]( mx, UInt16( 0))
        self._JobSilo.DoIndexSetup( True)
        self._JobBuff = Buff[ Runner]( mx, Runner()) 
        self._SzPreds = Buff[ UInt16]( mx, UInt16( 0))
        self._SuccIds = Buff[ UInt16]( mx, UInt16( 0))
        self._Maestros = Buff[ Maestro]( mxQueue, Maestro()) 
        var ind : UInt32 = 0
        for g in self._Maestros.Arr():
            g[].SetAtelier( ind, self)
            ind += 1
        pass
        
    fn __del__( owned self): 
        #print( "Atelier: Del ")
        pass
        
    fn Maestros( self) -> Arr[ Maestro, __origin_of( self._Maestros._DPtr)]:
        return self._Maestros.Arr()
    
    fn Honcho( self) -> UnsafePointer[ Maestro]:
        return self._Maestros.PtrAt( UInt32( 0))

    fn DoLaunch( self) -> Bool: 
        @parameter
        fn worker( ind: Int): 
            self._Maestros.PtrAt( UInt32( ind +1))[].ExecuteLoop()
        pass
        
        print( "DoLaunch")
        szWorker = self._Maestros.Size() -1
        if ( szWorker):
            parallelize[ worker]( int( szWorker))
        self._Maestros.PtrAt( UInt32( 0))[].ExecuteLoop()
        print( "DoLaunch Over")
        return True
     
    fn  Size( self) -> UInt32:
        return self._Maestros.Size()  
        
    fn  IsLocked( self, id: UInt32 ) -> Bool :
        return id > self._LockedMark
    
    fn  IncrSzSchedJob( mut self, inc : UInt32) -> UInt32:
        return self._SzSchedJob.Incr( inc) 

    fn  SuccIdAt( mut self, jobId: UInt16) -> UInt16:
        return self._SuccIds.PtrAt( jobId)[]

    fn  SetSuccIdAt( mut self, jobId: UInt16, succId: UInt16):
        self._SuccIds.PtrAt( jobId)[] = succId
     
    fn  IncrPredAt( mut self, jobId: UInt16) -> UInt16:
        self._SzPreds.PtrAt( jobId)[] += 1
        return self._SzPreds.PtrAt( jobId)[] 
 
    fn  DecrPredAt( mut self, jobId: UInt16) -> UInt16:
        self._SzPreds.PtrAt( jobId)[] -= 1
        return self._SzPreds.PtrAt( jobId)[] 
        

    fn  SetJobAt( mut self, jobId: UInt16, owned runner : Runner): 
        ly = self._JobBuff.PtrAt( jobId)
        ly[] = runner^
        ly[].SetJobId( jobId)
        pass 

    fn  JobAt( mut self, jobId: UInt16) -> UnsafePointer[ Runner]: 
        return self._JobBuff.PtrAt( jobId)
        
    fn  AllocJob( mut self) -> UInt16 :
        stk = self._JobSilo.Stack()
        if stk[].Size():
            return stk[].Pop()[]   
        return 0

    fn  AssignSucc( mut self, jobId : UInt16,   succId : UInt16):
        self.SetSuccIdAt( jobId, succId)
        _ = self.IncrPredAt( succId)

    fn ConstructJobAt( mut self, jobId : UInt16,   succId : UInt16,  owned runner : Runner):  
        self.SetJobAt( jobId, runner^) 
        self.AssignSucc( jobId, succId) 
        

    fn  AllocJobs( mut self, mut stk : Stk[ UInt16, MutableAnyOrigin, _]) -> Bool :
        freeJobs = self._JobSilo.Stack() 
        xSz = stk.Import( freeJobs[]) 
        return xSz != 0
        
    
    fn  FreeJobs( mut self, mut stk : Stk[ UInt16, MutableAnyOrigin, _]) -> Bool :
        freeJobs = self._JobSilo.Stack() 
        xSz = freeJobs[].Import( stk) 
        return xSz != 0
        

    fn  Dump( self): 
        pass
        
#----------------------------------------------------------------------------------------------------------------------------------

fn AtelierExample1():
    atelier = Atelier( 2)  
    g = Maestro()
    x = 10
    fn closure( mut maestro : Maestro) -> Bool:
        print( x)
        return True
    _ = g.PopJob() 
    atelier.SetJobAt( 1, closure)
    var id : UInt16  = 1
    job = atelier._JobBuff.PtrAt( id)
    _ = job[].Score( g)
    atelier.Dump()

#----------------------------------------------------------------------------------------------------------------------------------

fn AtelierExample() : 
    print( "AtelierExample")  
    atelier = Atelier( 4)  
    x = 10
    fn c1( mut maestro : Maestro) -> Bool:
        x += 1
        print( x)
        return True  
    maestro = atelier.Honcho()
    jId = UInt16( 0)
    jId = maestro[].Construct( jId, c1)
    jId = maestro[].Construct( jId, c1) 
    maestro[].EnqueueJob( jId)
    _ = atelier.DoLaunch()
    print( x)
    return 

#--------------------------------------------------------------------------------------------------------------------------------

import random

fn AtelierSortExample() : 
    print( "AtelierSortExample")  
    vec  = Buff[ Float32]( 800, 0) 
    arr = vec.Arr_()  
    for iter in arr: 
        iter[] = random.random_ui64( 13, 1139).__int__()
    arr.SwapAt( 3, 5)   

    @parameter
    fn Less( p: UInt32, q: UInt32, arr : Arr[ Float32, MutableAnyOrigin]) -> Bool: 
        return  arr.At( p) < arr.At( q) 
        
    @parameter
    fn Swap( p: UInt32, q: UInt32, arr : Arr[ Float32, MutableAnyOrigin]) -> None: 
        arr.SwapAt( p, q) 

    uSeg = USeg( 0, arr.Size()) 
    segEncap = uSeg.HeistQSorter[ Less, Swap]( arr)
    jId = UInt16( 0)
    atelier = Atelier( 4)  

    maestro = atelier.Honcho()
    jId = maestro[].Construct( jId, segEncap)
    maestro[].EnqueueJob( jId)
    _ = atelier.DoLaunch() 
    arr.Print()
 
#----------------------------------------------------------------------------------------------------------------------------------
 
fn AtelierComposeExample() : 
    print( "AtelierComposeExample")  
    
    x = 10
    fn c1( mut maestro : Maestro) -> Bool: 
        print( "a")
        x = 5
        return True  
     
    fn c2( mut maestro : Maestro) -> Bool: 
        print( "b")
        x = 3
        return True  

    atelier = Atelier( 4)  
    maestro = atelier.Honcho() 
    p = RunAfter( ( RunIt( c1, "6") >> RunAlong( RunAlong( RunAlong( RunIt( c2, "5"), RunIt( c2, "4")), RunIt( c2, "3")), RunIt( c1, "2"))), RunIt( c2, "1"))
    #p = (( RunIt( c1, "6") >> ( ( ( RunIt( c2, "5") | RunIt( c2, "5")) | RunIt( c2, "3")) | RunIt( c1, "2"))) >> RunIt( c2, "1"))
    print( str( p) )
    
    maestro = atelier.Honcho()
    maestro[].PostBefore( RunIt( c1, "x")._Runner)
    maestro[].PostBefore( RunIt( c2, "y")._Runner)
    _ = atelier.DoLaunch() 
    pass

#----------------------------------------------------------------------------------------------------------------------------------
