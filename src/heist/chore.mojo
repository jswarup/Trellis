# chore.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from stash import Buff, Silo, Arr, Stk
import heist

#----------------------------------------------------------------------------------------------------------------------------------

struct ChoreContext ( Stringable):
    var     _Lev : UInt32
    var     _JobArr : Silo[ UInt16]                 

    @always_inline
    fn __init__( out self, lev : UInt32) : 
        self._JobArr = Silo[ UInt16]( 64, 0) 
        self._Lev = lev
    
    @always_inline
    fn SuccJobs( self) -> Stk[ UInt16, MutableAnyOrigin]: 
        return self._JobArr.Stack()[] 

    fn __str__( self) -> String:
        str = " " * int( self._Lev) + self.SuccJobs().Arr().__str__()
        return str

#----------------------------------------------------------------------------------------------------------------------------------

trait ChoreIfc( StringableCollectionElement):
    fn  Sched( mut self, mut maestro : Maestro, mut ctxt : ChoreContext) :
        pass
 
    fn  SchedBefore( mut self, mut maestro : Maestro, mut outJobs : Silo[ UInt16], succId : UInt16):
        pass

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct Chore( ChoreIfc):
    var     _Runner : Runner  
    var     _Doc : String
    var     _JobId : UInt16  

    @always_inline
    fn __init__( out self) : 
        self._JobId = UInt16.MAX
        self._Doc = String() 
        self._Runner = Runner.Default() 

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
        print( "Chore: Del: ", self._Doc)
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
    fn __rshift__[ TSucc: ChoreIfc]( owned self, owned succ : TSucc) -> ChoreAfter[ Chore, TSucc] : 
        #print( "Chore: __rshift__")  
        return ChoreAfter( self^, succ^) 

    @always_inline
    fn __or__[ TAlong: ChoreIfc]( owned self, owned along : TAlong) -> ChoreAlong[ Chore, TAlong] : 
        #print( "Chore: __or__")  
        return ChoreAlong( self^, along^) 

    fn Sched( mut self, mut maestro : Maestro, mut ctxt : ChoreContext) :
        jobId = maestro.AllocJob()
        maestro._Atelier[].SetJobAt( jobId, self._Runner^) 
        self._Runner = Runner.Default()
        _ = ctxt._JobArr.Push( jobId)
        print( "Chore: Sched ", jobId, self._Doc)  
        pass

    fn  SchedBefore( mut self, mut maestro : Maestro, mut outJobs : Silo[ UInt16], succId : UInt16):
        jobId = maestro.AllocJob()
        maestro._Atelier[].SetJobAt( jobId, self._Runner^) 
        self._Runner = Runner.Default()
        maestro._Atelier[].AssignSucc( jobId, succId) 
        _ = outJobs.Push( jobId)
        pass

#----------------------------------------------------------------------------------------------------------------------------------
 
struct ChoreAfter[ TLeft: ChoreIfc, TRight: ChoreIfc] ( ChoreIfc):   
    var     _Left : TLeft
    var     _Right : TRight

    fn __init__( out self, owned left : TLeft, owned right : TRight):  
        self._Left = left^ 
        self._Right = right^

    @always_inline
    fn __init__( out self, other: Self):
        self._Left = other._Left
        self._Right = other._Right 

    @always_inline
    fn __copyinit__( out self, other: Self, /):
        self._Left = other._Left
        self._Right = other._Right 

    @always_inline
    fn __moveinit__( out self, owned other: Self, /):
        self._Left = other._Left^
        self._Right = other._Right^ 

    fn __str__( self) -> String:
        str = "[ " + self._Left.__str__() + " >> " + self._Right.__str__() + "]"
        return str

    fn __rshift__[ TNext: ChoreIfc]( owned self, owned succ : TNext) -> ChoreAfter[ Self, TNext] : 
        #print( "ChoreAfter: __rshift__") 
        return ChoreAfter( self^, succ^) 

    fn __or__[ TAlong: ChoreIfc]( owned self, owned succ : TAlong) -> ChoreAlong[ Self, TAlong] : 
        #print( "ChoreAfter: __or__") 
        return ChoreAlong( self^, succ^) 

    fn Sched( mut self, mut maestro : Maestro, mut ctxt : ChoreContext) :
        self._Left.Sched( maestro, ctxt)
        rCtxt = ChoreContext( ctxt._Lev +1)
        self._Right.Sched( maestro, rCtxt)
        maestro.Dispatch( rCtxt.SuccJobs().Arr())
        print( str( rCtxt), "ChoreAfter: Sched", str( ctxt))  
        pass

    fn  SchedBefore( mut self, mut maestro : Maestro, mut outJobs : Silo[ UInt16], succId : UInt16):
        pass

#----------------------------------------------------------------------------------------------------------------------------------
 
struct ChoreAlong[ TLeft: ChoreIfc, TRight: ChoreIfc] ( ChoreIfc):   
    var     _Left : TLeft
    var     _Right : TRight
 
    @always_inline
    fn __init__( out self, owned left : TLeft, owned right : TRight):  
        self._Left = left^
        self._Right = right^

    @always_inline
    fn __init__( out self, other: Self):
        self._Left = other._Left
        self._Right = other._Right 

    @always_inline
    fn __copyinit__( out self, other: Self, /):
        self._Left = other._Left
        self._Right = other._Right 

    @always_inline
    fn __moveinit__( out self, owned other: Self, /):
        self._Left = other._Left^
        self._Right = other._Right^ 

    @always_inline
    fn __str__( self) -> String:
        str = "[ " + self._Left.__str__() + " | " + self._Right.__str__() + "]"
        return str

    fn __rshift__[ TNext: ChoreIfc]( owned self, owned succ : TNext) -> ChoreAfter[ Self, TNext] : 
        #print( "ChoreAlong: __rshift__") 
        return ChoreAfter( self^, succ^) 

    fn __or__[ TAlong: ChoreIfc]( owned self, owned succ : TAlong) -> ChoreAlong[ Self, TAlong] : 
        #print( "ChoreAlong: __or__") 
        return ChoreAlong( self^, succ^) 

    fn Sched( mut self, mut maestro : Maestro, mut ctxt : ChoreContext) :
        self._Left.Sched( maestro, ctxt)
        rCtxt = ChoreContext( ctxt._Lev +1)
        self._Right.Sched( maestro, rCtxt)
        #print( str( ctxt), "ChoreAlong: Sched")  
        retJobs = ctxt.SuccJobs()
        subJobs = rCtxt.SuccJobs()
        _ = retJobs.Import( subJobs)
        pass

    fn  SchedBefore( mut self, mut maestro : Maestro, mut outJobs : Silo[ UInt16], succId : UInt16):
        pass


#----------------------------------------------------------------------------------------------------------------------------------
 
fn ChoreExample(): 
    print( "ChoreExample")  
    x = 10
    fn c1( mut maestro : Maestro) -> Bool: 
        print( "a")
        x = 5
        return True  
     
    fn c2( mut maestro : Maestro) -> Bool: 
        print( "b")
        x = 3
        return True    
    #p =  Chore( c2, "6") >> ( Chore( c2, "5") | ( Chore( c2, "4") >> Chore( c1, "3")) | Chore( c1, "2") ) >> ( Chore( c2, "1b") | Chore( c2, "1a"))
    p = Chore( c2, "1")  # >> Chore( c2, "2") >> Chore( c2, "3")  >> Chore( c2, "4")  
    #p = Chore( c1, "6") >> Chore( c2, "6");
    print( str( p) )
    atelier = Atelier(1)  
    maestro = atelier.Honcho() 
    maestro[].Post( p)
    _ = atelier.DoLaunch() 
    pass


#----------------------------------------------------------------------------------------------------------------------------------
