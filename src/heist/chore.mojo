# chore.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from stash import Buff, Silo, Arr, Stk, USeg
from heist import Maestro, Atelier, Runner

#----------------------------------------------------------------------------------------------------------------------------------

trait ChoreIfc( WritableCollectionElement):  
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
        return self._Runner.Score( maestro)

    @always_inline
    fn      SetJobId( mut self, jobId : UInt16) :
        self._JobId = jobId
    
    @always_inline
    fn  write_to[W: Writer](self, mut writer: W):
        writer.write("[ "  + "]")
    
    @always_inline
    fn __rshift__[ TSucc: ChoreIfc]( owned self, owned succ : TSucc) -> ChoreAfter[ Chore, TSucc] : 
        #print( "Chore: __rshift__")  
        return ChoreAfter( self^, succ^) 

    @always_inline
    fn __or__[ TAlong: ChoreIfc]( owned self, owned along : TAlong) -> ChoreAlong[ Chore, TAlong] : 
        #print( "Chore: __or__")  
        return ChoreAlong( self^, along^) 
 
    fn  SchedBefore( mut self, mut maestro : Maestro, mut outJobs : Silo[ UInt16], succId : UInt16):
        jobId = maestro.Construct( succId, self._Runner^) 
        self._Runner = Runner.Default() 
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

    fn  write_to[W: Writer](self, mut writer: W):
        writer.write( "[ " + String( self._Left) + " >> " + String( self._Right) + "]")
    
    fn __rshift__[ TNext: ChoreIfc]( owned self, owned succ : TNext) -> ChoreAfter[ Self, TNext] : 
        #print( "ChoreAfter: __rshift__") 
        return ChoreAfter( self^, succ^) 

    fn __or__[ TAlong: ChoreIfc]( owned self, owned succ : TAlong) -> ChoreAlong[ Self, TAlong] : 
        #print( "ChoreAfter: __or__") 
        return ChoreAlong( self^, succ^)  

    fn  SchedBefore( mut self, mut maestro : Maestro, mut outJobs : Silo[ UInt16], succId : UInt16):
        stk = outJobs.Stack()
        inSz = stk[].Size()
        self._Right.SchedBefore( maestro, outJobs, succId)
        
        rSz = stk[].Size() -inSz
        if ( rSz == 1): 
            self._Left.SchedBefore( maestro, outJobs, stk[].Pop()) 
            return   
        
        buf = Buff[ UInt16]( stk[].Arr().Subset( USeg( inSz, rSz)))
        stk[].Clip( rSz)
        fn  EnqueJobs( mut maestro : Maestro) -> Bool: 
            arr = buf.Arr() 
            for i in USeg( buf.Size()): 
                maestro.EnqueueJob( arr.At( i))  
            return True     
        jobId = maestro.Construct( succId, EnqueJobs) 
        self._Left.SchedBefore( maestro, outJobs, jobId) 
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
    fn  write_to[W: Writer](self, mut writer: W):
        writer.write( "[ " + String( self._Left) + " | " + String( self._Right) + "]")

    fn __rshift__[ TNext: ChoreIfc]( owned self, owned succ : TNext) -> ChoreAfter[ Self, TNext] : 
        #print( "ChoreAlong: __rshift__") 
        return ChoreAfter( self^, succ^) 

    fn __or__[ TAlong: ChoreIfc]( owned self, owned succ : TAlong) -> ChoreAlong[ Self, TAlong] : 
        #print( "ChoreAlong: __or__") 
        return ChoreAlong( self^, succ^)  
        
    fn  SchedBefore( mut self, mut maestro : Maestro, mut outJobs : Silo[ UInt16], succId : UInt16):        
        self._Right.SchedBefore( maestro, outJobs, succId) 
        self._Left.SchedBefore( maestro, outJobs, succId) 
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
        print( "b", x)
        x = 3
        return True    
    
    fn c3( mut maestro : Maestro) -> Bool: 
        print( "c")
        x = 3
        return True    

    #p =  Chore( c2, "6") >> ( Chore( c2, "5") | ( Chore( c2, "4") >> Chore( c1, "3")) | Chore( c1, "2") ) >> ( Chore( c2, "1b") | Chore( c2, "1a"))
    #p = Chore( c2, "1")  # >> Chore( c2, "2") >> Chore( c2, "3")  >> Chore( c2, "4")  
    q = Chore() >> Chore() | Chore() | Chore() >> Chore() >> Chore() | Chore() | Chore() >> Chore();
    print( q)
    
    p = Chore( c1, "8") >> ( Chore( c3, "6")  | Chore( c2, "7") )  >> Chore( c1, "8")  
    
    print( String( p) )
    atelier = Atelier(1)  
    maestro = atelier.Honcho() 
    maestro[].Post( p)
    _ = atelier.DoLaunch() 
    pass


#----------------------------------------------------------------------------------------------------------------------------------
