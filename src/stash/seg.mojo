# seg.mojo ------------------------------------------------------------------------------------------------------------------------

from sys.intrinsics import _type_is_eq
from utils._visualizers import lldb_formatter_wrapping_type 

#----------------------------------------------------------------------------------------------------------------------------------

struct USeg ( CollectionElement): 
    
    var _First: UInt32
    var _Last: UInt32
    
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( out self ):
        self._First = UInt32.MAX
        self._Last = UInt32.MAX -1 
    
    @always_inline
    fn __init__( out self, sz :UInt32):
        self._First = 0
        self._Last = 0 + sz -1 
    
    @always_inline
    fn __init__( out self, b :UInt32, sz :UInt32):
        self._First = b
        self._Last = b + sz -1 

    @always_inline
    fn __copyinit__( out self, other: Self, /):
        self._First =  other._First
        self._Last = other._Last  
 
    @always_inline
    fn __moveinit__( out self, owned other: Self):
        self._First =  other._First
        self._Last = other._Last
        other = USeg()
 
    @always_inline
    fn __del__(owned self):         
        pass

    #-----------------------------------------------------------------------------------------------------------------------------
    
    @always_inline
    fn __len__( self) -> Int:
        return int( self.Size())

    @always_inline
    fn __getitem__( self, idx: UInt32) -> UInt32: 
        return self._First + idx
 
    @always_inline
    fn __iter__(self) -> Self:
        return self
 
    @always_inline
    fn __has_next__( self) -> Bool:
        return self.Size() > 0

    @always_inline
    fn __next__(mut self) -> UInt32:
        var start = self._First
        self._First += 1
        return start 
    
    #-----------------------------------------------------------------------------------------------------------------------------

    fn __repr__(self) -> String:
        return "[ " + repr( self.First()) + ", " + repr( self.Last()) + "]"

    #-----------------------------------------------------------------------------------------------------------------------------
    
    @always_inline
    fn  First( self) -> UInt32:
        return self._First

    @always_inline
    fn  Mid( self) -> UInt32:
        return ( self._First +self._Last) //2

    @always_inline
    fn  Last( self) -> UInt32:
        return self._Last

    @always_inline
    fn  End( self) -> UInt32:
        return self._Last +1

    @always_inline
    fn  Size( self) -> UInt32:
        return self.End() -self.First()

    @always_inline
    fn  IsValid( self) -> Bool:
        return self._First != UInt32.MAX 
    
    fn  Traverse[ Lambda: fn( k: UInt32) capturing [_]-> None]( self):
        for i in self:
            Lambda( i)

    fn  RevTraverse[ Lambda: fn( k: UInt32) capturing [_]-> None]( self):
        for i in range( self.First(), self.End()):
            Lambda( self.Last() -1) 
    
    #-----------------------------------------------------------------------------------------------------------------------------

    fn  BinarySearch[  Less: fn( p: UInt32) capturing -> Bool]( self) -> UInt32: 
        l = self.First()
        h = self.End()  
        while ( l < h):
            mid =  (l + h)//2 
            if Less( mid):
                l = mid + 1
            else:
                h = mid 
        return l
    
    #-----------------------------------------------------------------------------------------------------------------------------

    fn   QSortPartition[ Less: fn( p: UInt32, q: UInt32) capturing -> Bool, Swap: fn( p: UInt32, q: UInt32) capturing -> None]( owned self ) -> UInt32:
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
                
    #----------------------------------------------------------------------------------------------------------------------------- 

    fn QSort[ Less: fn( p: UInt32, q: UInt32) capturing -> Bool, Swap: fn( p: UInt32, q: UInt32) capturing -> None]( self ) -> None: 
        list = List[ USeg]()
        list.append( self) 
        while list.__len__() :
            seg = list.pop() 
            piv  = seg.QSortPartition[ Less, Swap]()
            fSz = piv -seg._First +1 
            if ( fSz > 1):
                list.append( USeg( seg._First, fSz))
            piv += 1
            sSz = seg._Last -piv +1
            if ( sSz > 1 ):
                list.append( USeg( piv, sSz)) 

#----------------------------------------------------------------------------------------------------------------------------------

@always_inline
fn uSeg( sz: UInt32) -> USeg:
    return USeg( sz)

#----------------------------------------------------------------------------------------------------------------------------------

@always_inline
fn uSeg( b: UInt32, sz :UInt32) -> USeg:
    return USeg( b, sz)

#----------------------------------------------------------------------------------------------------------------------------------

fn main():  
    var     uSeg = USeg( 0, 1)  
    var     vSeg = uSeg;
    @parameter
    fn  trial( k: UInt32)  -> None:     
        print( repr( vSeg))

    uSeg.Traverse[ trial]()

#----------------------------------------------------------------------------------------------------------------------------------
