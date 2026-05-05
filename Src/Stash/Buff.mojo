# Buff.mojo -----------------------------------------------------------------------------------------------------------------------

from Stash import USeg
from Stash import Arr

#----------------------------------------------------------------------------------------------------------------------------------

struct Buff [ T: Copyable ]( Copyable ):
    
    comptime _UPtr = UnsafePointer[Self.T, MutExternalOrigin]

    var     _DPtr:  Self._UPtr
    var     _Size: UInt32
     
    def __init__(out self, sz: UInt32 = 4): 
        self._DPtr = alloc[Self.T]( Int( sz))
        self._Size = sz
        pass
  
    def __init__( out self, sz: UInt32, value: Self.T):   
        self._Size = sz
        self._DPtr = alloc[Self.T]( Int( sz))
        for i in USeg( sz):
            (self._DPtr + i).init_pointee_copy( value)

    def __init__(out self, *, deinit take: Self): 
        self._Size = take._Size
        self._DPtr = take._DPtr 

    def __del__(deinit self):
        self._DPtr.free() 
        pass

    def Arr( self) -> Arr[ Self.T, MutExternalOrigin]: 
        return Arr[ Self.T, MutExternalOrigin]( self._Size, self._DPtr)

