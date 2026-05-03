# Buff.mojo -----------------------------------------------------------------------------------------------------------------------
   
#----------------------------------------------------------------------------------------------------------------------------------

struct Buff [ T: Copyable ]:
    
    var     _DPtr:  UnsafePointer[Self.T, MutExternalOrigin]
    var     _Size: UInt32
     
    def __init__(out self, sz: UInt32 = 4): 
        self._DPtr = alloc[Self.T]( Int( sz))
        self._Size = sz
        pass
  
    def __init__( out self, sz: UInt32, value: Self.T):   
        self._Size = sz
        self._DPtr = alloc[Self.T]( Int( sz))
        #for i in USeg( 0, sz):
        #    (self._DPtr + i).init_pointee_copy( value) 

    def __del__(deinit self):
        self._DPtr.free() 
        pass


