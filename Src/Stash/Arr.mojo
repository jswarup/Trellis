# Arr.mojo -----------------------------------------------------------------------------------------------------------------------

from Stash import USeg 

#----------------------------------------------------------------------------------------------------------------------------------
 
struct Arr [ Mut: Bool, //,T: ImplicitlyCopyable,  origin: Origin[ mut = Mut]]( ImplicitlyCopyable, Iterable, Iterator, TrivialRegisterPassable ): 
    comptime    _UPtr = UnsafePointer[Self.T, MutExternalOrigin]
    comptime    _Null = Self._UPtr.unsafe_dangling()
    comptime    Element = Self.T
    
    comptime IteratorType[
        iterable_mut: Bool, //, iterable_origin: Origin[mut=iterable_mut]
    ]: Iterator = Self
    
    var     _DPtr: Self._UPtr
    var     _Size: UInt32
     
    def __init__(out self): 
        self._DPtr = Self._Null
        self._Size = 0
        pass
  
    def __init__( out self, sz: UInt32, uPtr: Self._UPtr):   
        self._Size = sz
        self._DPtr = uPtr  
    
    @always_inline
    def __getitem__(self, idx: UInt32) ->  ref[Self.origin] Self.T:
        return self._DPtr[ idx]

    @always_inline
    def __setitem__(self, idx: UInt32, val: Self.T):
        self._DPtr[idx].__del__()
        self._DPtr[ idx] = val

    @always_inline
    def __len__(self) -> UInt32:
        return self._Size
 
    @always_inline
    def __iter__(self) -> Self:
        return self
 
    @always_inline
    def __has_next__( self) -> Bool:
        return self._Size > 0

    @always_inline
    def __next__(mut self) -> ref[Self.origin] Self.T:
        var startPtr = self._DPtr 
        self._DPtr += 1
        self._Size -= 1
        return startPtr[0]

    
    @always_inline
    def Size( mut self) -> UInt32: 
        return self._Size

    @always_inline
    def USeg( self) -> USeg: 
        return USeg( self._Size)
    
    @always_inline
    def ObjPtrAt( self, idx: UInt32) -> Self._UPtr: 
        if ( idx >= self._Size):
            return Self._Null
        return self._DPtr + idx