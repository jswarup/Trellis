# mule.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
import heist

#----------------------------------------------------------------------------------------------------------------------------------

struct Mule ( CollectionElement): 
    var     _Runner : Runner 
    
    fn  Follow[ U : Muload, V : Muload]( self, u: U) -> V: 
        pass
    
    fn  Alter[ U : Muload, V : Muload]( self, u : U) -> V: 
        pass

#----------------------------------------------------------------------------------------------------------------------------------

struct Mule[ T: Muload] :
    var     _Value: T

    fn __init__( out self, value : T):
        self._Value = value

    fn __rshift__[ U: Muload,]( self, other : Mule[ U]) -> Mule[ V]:      
        v = self._Value.Follow( other._Value)
        return v

    fn __or__[ U: Muload]( self, other : Mule[ U]) -> Mule[ V]:      
        return self._Value.Alter( other._Value)     


#----------------------------------------------------------------------------------------------------------------------------------

struct IntML( Muload):
    var     _Value : Int32

    fn __init__( out self, value : Int32):
        self._Value = value
    
    fn  Follow[ U: Muload]( self, m : IntML) -> IntML: 
        return IntML( self._Value -m._Value)
    
    fn  Alter( self, m : IntML) -> IntML: 
        return IntML( self._Value +m._Value)

#----------------------------------------------------------------------------------------------------------------------------------

fn MuleExample() : 
    a = Mule( IntML( 10))
    b = Mule( IntML( 4))  

    print( (a >> 1)._Value)  # Right shift by 1: 0101 (5)

    print( (a | b)._Value)  # Bitwise OR: 1110 (14)
 