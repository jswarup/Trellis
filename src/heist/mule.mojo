# mule.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from stash import Buff, Silo, Stk
import heist

#----------------------------------------------------------------------------------------------------------------------------------

struct MuleContext ( Stringable):
    var     _Lev : UInt32

    @always_inline
    fn __init__( out self, lev : UInt32) : 
        self._Lev = lev

    fn __str__( self) -> String:
        str = " " * int( self._Lev)
        return str

#----------------------------------------------------------------------------------------------------------------------------------

trait MuleAble( StringableCollectionElement):
    fn  Sched( self, mut maestro : Maestro, ctxt : MuleContext) :
        pass

#----------------------------------------------------------------------------------------------------------------------------------
 
struct MuleAfter[ TLeft: MuleAble, TRight: MuleAble] ( MuleAble):   
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

    fn __rshift__[ TNext: MuleAble]( owned self, owned succ : TNext) -> MuleAfter[ Self, TNext] : 
        #print( "MuleAfter: __rshift__") 
        return MuleAfter( self^, succ^) 

    fn __or__[ TAlong: MuleAble]( owned self, owned succ : TAlong) -> MuleAlong[ Self, TAlong] : 
        #print( "MuleAfter: __or__") 
        return MuleAlong( self^, succ^) 

    fn Sched( self, mut maestro : Maestro, ctxt : MuleContext) :
        print( str( ctxt), "MuleAfter: Sched")  
        self._Left.Sched( maestro, ctxt)
        rCtxt = MuleContext( ctxt._Lev +1)
        self._Right.Sched( maestro, rCtxt)
        pass

#----------------------------------------------------------------------------------------------------------------------------------
 
struct MuleAlong[ TLeft: MuleAble, TRight: MuleAble] ( MuleAble):   
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

    fn __rshift__[ TNext: MuleAble]( owned self, owned succ : TNext) -> MuleAfter[ Self, TNext] : 
        #print( "MuleAlong: __rshift__") 
        return MuleAfter( self^, succ^) 

    fn __or__[ TAlong: MuleAble]( owned self, owned succ : TAlong) -> MuleAlong[ Self, TAlong] : 
        #print( "MuleAlong: __or__") 
        return MuleAlong( self^, succ^) 

    fn Sched( self, mut maestro : Maestro, ctxt : MuleContext) :
        print( str( ctxt), "MuleAlong: Sched")  
        self._Left.Sched( maestro, ctxt)
        self._Right.Sched( maestro, ctxt)
        pass

@value
struct Mule( MuleAble):
    var     _Runner : Runner  
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
        print( "Mule: Del: ", self._Doc)
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
    fn __rshift__[ TSucc: MuleAble]( owned self, owned succ : TSucc) -> MuleAfter[ Mule, TSucc] : 
        #print( "Mule: __rshift__")  
        return MuleAfter( self^, succ^) 

    @always_inline
    fn __or__[ TAlong: MuleAble]( owned self, owned along : TAlong) -> MuleAlong[ Mule, TAlong] : 
        #print( "Mule: __or__")  
        return MuleAlong( self^, along^) 

    fn Sched( self, mut maestro : Maestro, ctxt : MuleContext) :
        print( str( ctxt),  "Mule: Sched ", self._Doc)  
        pass

#----------------------------------------------------------------------------------------------------------------------------------
 
fn MuleExample(): 
    print( "MuleExample")  
    x = 10
    fn c1( mut maestro : Maestro) -> Bool: 
        print( "a")
        x = 5
        return True  
     
    fn c2( mut maestro : Maestro) -> Bool: 
        print( "b")
        x = 3
        return True    
    p =  Mule( c2, "6") >> ( Mule( c2, "5") | Mule( c2, "4") | Mule( c1, "2")) >> Mule( c2, "1")
    print( str( p) )
    atelier = Atelier( 4)  
    maestro = atelier.Honcho() 
    maestro[].Post( p)
    pass


#----------------------------------------------------------------------------------------------------------------------------------
 