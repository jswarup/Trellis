
from sys.intrinsics import _type_is_eq
from utils._visualizers import lldb_formatter_wrapping_type
 

struct USeg :
    var _First: UInt32
    var _Last: UInt32
    
    fn __init__( inout self ):
        self._First = -1
        self._Last = -1
    
    fn __init__( inout self, sz :UInt32):
        self._First = 0
        self._Last = 0 + sz -1
    
    fn __init__( inout self, b :UInt32, sz :UInt32):
        self._First = b
        self._Last = b + sz -1

    fn __copyinit__( inout self, other: Self):
        self._First =  other._First
        self._Last = other._Last

    fn  First( self) -> UInt32:
        return self._First

    fn  Last( self) -> UInt32:
        return self._Last

    fn  End( self) -> UInt32:
        return self._Last

    fn  Size( self) -> UInt32:
        return self._Last +1

    fn  IsValid( self) -> Bool:
        return ( self._First != -1) and ( self._Last != 0)
    
    fn __repr__(self) -> String:
        return "[ " + repr( self.First()) + ", " + repr( self.Size()) + "]"
    
    fn  Traverse[ Lambda: fn[ *Ts : AnyType]( k: UInt32, *args: *Ts) -> None, *Ts: AnyType]( self, *args: *Ts):
        for i in range( self.First(), self.End()):
            Lambda( i, args)
        return
    
fn main(): 
    var     uSeg = USeg( 0, 5)  
    fn  trial[ *Ts : AnyType]( k: UInt32, *args: *Ts): 
        a = len( args)
        a += args[ 0].End() 
        print( k + a + )
    uSeg.Traverse[ trial]( uSeg)

    

