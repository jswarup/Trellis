# Arr.mojo -----------------------------------------------------------------------------------------------------------------------

from Stash import USeg 

#----------------------------------------------------------------------------------------------------------------------------------
 
struct Arr [ Mut: Bool, //,T: ImplicitlyCopyable,  origin: Origin[ mut = Mut]]( ImplicitlyCopyable, TrivialRegisterPassable ): 
    comptime _UPtr = UnsafePointer[Self.T, MutExternalOrigin]

    var     _DPtr: Self._UPtr
    var     _Size: UInt32
     
    def __init__(out self): 
        self._DPtr = Self._UPtr.unsafe_dangling()
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

        
 