# useg.mojo ------------------------------------------------------------------------------------------------------------------------
 
#----------------------------------------------------------------------------------------------------------------------------------

struct USeg ( ImplicitlyCopyable, Iterable, Iterator, TrivialRegisterPassable): 
    
    comptime Element = UInt32
    comptime IteratorType[
        iterable_mut: Bool, //, iterable_origin: Origin[mut=iterable_mut]
    ]: Iterator = Self

    var     _First: UInt32
    var     _Last: UInt32
    
    @always_inline
    def __init__( out self ):
        self._First = UInt32.MAX
        self._Last = UInt32.MAX -1 
    
    @always_inline
    def __init__( out self, sz :UInt32):
        self._First = 0
        self._Last = 0 + sz -1 
    
    @always_inline
    def __init__( out self, b :UInt32, sz :UInt32):
        self._First = b
        self._Last = b + sz -1  

    @always_inline
    def __len__( self) -> Int:
        return  Int( self._Last -self._First +1)

    @always_inline
    def __getitem__( self, idx: UInt32) -> UInt32: 
        return self._First + idx
 
    @always_inline
    def __iter__(self) -> Self:
        return self
 
    @always_inline
    def __has_next__( self) -> Bool:
        return self._Last >= self._First

    @always_inline
    def __next__(mut self) -> UInt32:
        var start = self._First
        self._First += 1
        return start 
    
     
    