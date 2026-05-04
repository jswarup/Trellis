# Arr.mojo -----------------------------------------------------------------------------------------------------------------------

from Stash import USeg 

#----------------------------------------------------------------------------------------------------------------------------------

struct Arr [ T: Copyable ]( ImplicitlyCopyable, TrivialRegisterPassable ):
    
    comptime _UPtr = UnsafePointer[Self.T, MutExternalOrigin]

    var     _DPtr: Self._UPtr
    var     _Size: UInt32
     
    def __init__(out self): 
        self._DPtr =  alloc[Self.T]( Int( 0))
        self._Size = 0
        pass
  
    def __init__( out self, sz: UInt32, uPtr: Self._UPtr):   
        self._Size = sz
        self._DPtr = uPtr 
         
