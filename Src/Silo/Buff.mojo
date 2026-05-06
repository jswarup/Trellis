# Buff.mojo -----------------------------------------------------------------------------------------------------------------------

from Silo import USeg, Arr

#----------------------------------------------------------------------------------------------------------------------------------

struct Buff [ T: ImplicitlyCopyable ]( Copyable ):
    
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

    def Arr( self) -> Arr[ Self.T, origin_of(self)]: 
        return Arr[ Self.T, origin_of(self)]( self._Size, self._DPtr) 

    def Resize( mut self, nwSz: UInt32, value: Self.T):
        olDPtr = self._DPtr
        olSz = self._Size
        self._DPtr = alloc[Self.T]( Int( nwSz)) 
        self._Size = nwSz
        sz = min( olSz, nwSz)
        for i in USeg( sz):
            (self._DPtr + i).init_pointee_move_from( olDPtr + i)

        if ( sz < olSz):
            for i in USeg( sz, olSz -sz):
                (olDPtr + i).destroy_pointee()
        
        if ( sz < nwSz):
            for i in USeg( sz, nwSz -sz):
                (self._DPtr + i).init_pointee_copy( value)
        if sz:
            olDPtr.free()

    @staticmethod
    def  Test():
        var     b = Buff[ UInt32]( 4, 42)
        print( b.Arr())
        b.Resize( 6, 99)
        print( b.Arr())
        b.Resize( 5, 0)
        print( b.Arr())
        
