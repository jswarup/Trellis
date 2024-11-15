
from sys.intrinsics import _type_is_eq
from utils._visualizers import lldb_formatter_wrapping_type
 

struct USeg :
    var _First: Int
    var _Last: Int
    
    fn __init__( inout self ):
        self._First = Int.MAX
        self._Last = Int.MAX -1
    
    fn __init__( inout self, sz :Int):
        self._First = 0
        self._Last = 0 + sz -1
    
    fn __init__( inout self, b :Int, sz :Int):
        self._First = b
        self._Last = b + sz -1

    fn __copyinit__( inout self, other: Self):
        self._First =  other._First
        self._Last = other._Last
 
    fn  First( self) -> Int:
        return self._First

    fn  Mid( self) -> Int:
        return ( self._First +self._Last) //2

    fn  Last( self) -> Int:
        return self._Last

    fn  End( self) -> Int:
        return self._Last +1

    fn  Size( self) -> Int:
        return self.End() -self.First()

    fn  IsValid( self) -> Bool:
        return self._First != Int.MAX
    
    fn __repr__(self) -> String:
        return "[ " + repr( self.First()) + ", " + repr( self.Size()) + "]"
    
    fn  Traverse[ Lambda: fn( k: Int) capturing [_]-> None]( self):
        for i in self:
            Lambda( i)

    fn  RevTraverse[ Lambda: fn( k: Int) capturing [_]-> None]( self):
        for i in range( self.First(), self.End()):
            Lambda( self.Last() -1) 

    fn __iter__(self) -> Self:
        return self
 
    fn __next__(inout self) -> Int:
        var start = self._First
        self._First += 1
        return start
  
    fn __has_next__(self) -> Bool:
        return self.Size() > 0
 
    fn __len__(self) -> Int:
        return self.Size()
 
    fn __getitem__(self, idx: Int) -> Int:
        debug_assert(idx < self.Size(), "index out of range")
        return self._First + idx
 
    fn  Bound[ Low: Bool, Less: fn[ Low : Bool]( p: Int) capturing [_]-> Bool]( self) -> Int: 
        l = self.First()
        h = self.End()  
        while ( l < h):
            mid =  (l + h)//2 
            if Less[ Low]( mid):
                l = mid + 1
            else:
                h = mid 
        return l
    
    fn   QSortPartition[ Less: fn( p: Int, q: Int) capturing [_]-> Bool, Swap: fn( p: Int, q: Int) capturing [_]-> None]( owned self ) -> Int:
        piv = self.Mid()
        while True:
            while not Less(piv, self._First) and (self._First < piv):
                self._First += 1
            while ( not Less( self._Last, piv) and ( self._Last > piv)):
                self._Last -= 1
            if ( self._First == piv and self._Last == piv):
                return piv
            Swap( self._First, self._Last)
            if ( self._First == piv):
                piv = self._Last
            elif ( self._Last == piv):
                piv = self._First  

    fn QSort[ Less: fn( p: Int, q: Int) capturing [_]-> Bool, Swap: fn( p: Int, q: Int) capturing [_]-> None]( owned self ) -> None:
        first = self.First()
        last = self.Last()
        priv  = self.QSortPartition[ Less, Swap]()
        if ( first < priv):
            USeg( first, priv).QSort[ Less, Swap]()
        if ( ++priv < last):
            USeg( priv, last).QSort[ Less, Swap]()  


fn main():  
    var     uSeg = USeg( 0, 1)  
    var     vSeg = uSeg;
    @parameter
    fn  trial( k: Int)  -> None:     
        print( repr( vSeg))

    uSeg.Traverse[ trial]()
