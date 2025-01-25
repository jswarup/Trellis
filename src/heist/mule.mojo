# mule.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
import heist

 
#----------------------------------------------------------------------------------------------------------------------------------

@value
struct MuleAfter[ TLeft: StringableCollectionElement, TRight: StringableCollectionElement] ( StringableCollectionElement):   
    var     _Left : TLeft
    var     _Right : TRight

    fn __init__( out self, owned left : TLeft, owned right : TRight):  
        self._Left = left^ 
        self._Right = right^

    fn __str__( self) -> String:
        str = "[ " + self._Left.__str__() + " >> " + self._Right.__str__() + "]"
        return str

    fn __rshift__[ TNext: StringableCollectionElement]( owned self, owned succ : TNext) -> MuleAfter[ Self, TNext] : 
        return MuleAfter( self, succ) 

    fn __or__[ TAlong: StringableCollectionElement]( owned self, owned succ : TAlong) -> MuleAlong[ Self, TAlong] : 
        return MuleAlong( self, succ) 

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct MuleAlong[ TLeft: StringableCollectionElement, TRight: StringableCollectionElement] ( StringableCollectionElement):   
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

    fn __rshift__[ TNext: StringableCollectionElement]( owned self, owned succ : TNext) -> MuleAfter[ Self, TNext] : 
        return MuleAfter( self, succ) 

    fn __or__[ TAlong: StringableCollectionElement]( owned self, owned succ : TAlong) -> MuleAlong[ Self, TAlong] : 
        return MuleAlong( self, succ) 


@value
struct Mule( StringableCollectionElement):
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
    fn __rshift__[ TSucc: StringableCollectionElement]( owned self, owned succ : TSucc) -> MuleAfter[ Mule, TSucc] : 
        return MuleAfter( self^, succ^) 

    @always_inline
    fn __or__[ TAlong: StringableCollectionElement]( owned self, owned along : TAlong) -> MuleAlong[ Mule, TAlong] : 
        return MuleAlong( self^, along^) 


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
    p = MuleAfter( ( Mule( c1, "6") >> MuleAlong( MuleAlong( MuleAlong( Mule( c2, "5"), Mule( c2, "4")), Mule( c2, "3")), Mule( c1, "2"))), Mule( c2, "1"))
    #p = (( Mule( c1, "6") >> ( ( ( Mule( c2, "5") | Mule( c2, "5")) | Mule( c2, "3")) | Mule( c1, "2"))) >> Mule( c2, "1"))
    print( str( p) )
    pass


#----------------------------------------------------------------------------------------------------------------------------------
 