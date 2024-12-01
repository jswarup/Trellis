# caper.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from strand import Atm
from stash  import Arr, Buff, Stk, Silo
import heist

#----------------------------------------------------------------------------------------------------------------------------------
  
trait Runnable( CollectionElement):
    fn    Score( inout self, inout grifter:  Grifter) -> Bool:
        pass

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct  Runner[ T : Runnable] : 
    var    _Runner: UnsafePointer[ T]
 
    fn __init__( out self) : 
        self._Runner = UnsafePointer[ T].alloc(1)

    fn Set[ X : Runnable]( inout self, inout x: X) :  
        print( "Set")
        y = UnsafePointer.address_of( self._Runner).bitcast[ UnsafePointer[ X]]() 
        y[].init_pointee_move( x)  
        

    fn  DoRun( self, inout grifter:  Grifter) -> Bool:
        print( "DoRun")
        return True
        #return self._Runner[].Score( grifter)    

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Operator( Runnable) :
    fn __init__( out self) : 
        pass

    fn __copyinit__( out self, other: Self):
        print( "__copyinit__")
        pass
 
    fn __moveinit__( out self, owned other: Self):
        print( "__moveinit__")
        pass
 
    fn __del__(owned self):         
        print( "__del__")
        pass

    fn  Score( inout self, inout grifter:  Grifter) -> Bool:
        print( "a")
        return True

#----------------------------------------------------------------------------------------------------------------------------------

struct Caper:
    var     _StartCount: UInt32                         # Count of Processing Queue started, used for startup and shutdown 
    var     _SzSchedJob: UInt32                         # Count of cumulative scheduled jobs in Works and Queues
    var     _SzQueue: Atm[ True, DType.uint32]     
    var     _JobSilo: Silo[ UInt16]
    var     _JobBuff: Buff[ Runner[ Runnable], False]
    var     _SzPreds: Buff[ UInt16, False]
    var     _SuccIds: Buff[ UInt16, False]

    fn __init__( out self) :
        self._StartCount = 0
        self._SzSchedJob = 0
        self._SzQueue = UInt32( 0)
        mx = UInt16.MAX.cast[ DType.uint32]()
        self._JobSilo = Silo[ UInt16]( mx)
        self._JobBuff = Buff[ Runner[ Runnable], False]( mx, Runner[ Runnable]()) 
        self._SzPreds = Buff[ UInt16, False]( mx, UInt16( 0))
        self._SuccIds = Buff[ UInt16, False]( mx, UInt16( 0))
        pass
    
    fn  SuccIdAt( inout self, jobId: UInt16) -> UInt16:
        return self._SuccIds.PtrAt( jobId)[]

    fn  SetSuccIdAt( inout self, jobId: UInt16, succId: UInt16):
        self._SuccIds.PtrAt( jobId)[] = succId
    
    fn  SzPredAt( self, jobId: UInt16) -> UInt16:
        return self._SzPreds.PtrAt( jobId)[] 

    fn  IncrPredAt( inout self, jobId: UInt16):
        self._SzPreds.PtrAt( jobId)[] += 1
 
    fn  DecrPredAt( inout self, jobId: UInt16):
        self._SzPreds.PtrAt( jobId)[] -= 1

    fn  FillJobAt[ T :Runnable] (  inout self, jobId: UInt16, inout runner:  T ):        
        print( "FillJobAt")
        ly = self._JobBuff.PtrAt( jobId)
        ly[].Set( runner)
        pass

    fn  Dump( self): 
        pass

#----------------------------------------------------------------------------------------------------------------------------------

fn outer(b: Bool, x: String) -> fn() escaping -> String:
    fn closure() -> String:
        print(x) # 'x' is captured by calling String.__copyinit__
        return "Closure"

    fn bare_function()  -> String:
        print(x) # nothing is captured
        return "bare_function"

    if b:
        # closure can be safely returned because it owns its state
        return closure^

    # function pointers can be converted to runtime closures
    return bare_function

#----------------------------------------------------------------------------------------------------------------------------------

fn CaperExample1():
    func1 = outer( False, "False")
    func2 = outer( True, "True")
    print( func1())
    print( func2())


fn CaperExample():
    caper = Caper() 
    oper = Operator()
    caper.FillJobAt( 1, oper) 
    x = caper._JobBuff.PtrAt( UInt32( 1))
    g = Grifter()
    _ = x[].DoRun( g)
    caper.Dump()

#----------------------------------------------------------------------------------------------------------------------------------
