# Buff.mojo -----------------------------------------------------------------------------------------------------------------------

from Silo import USeg, Arr

#----------------------------------------------------------------------------------------------------------------------------------


struct Buff[ T: ImplicitlyCopyable, origin: Origin = MutAnyOrigin]( Copyable, Movable, ImplicitlyCopyable):
    comptime _UPtr = UnsafePointer[ Self.T, MutExternalOrigin]

    var _DPtr: Self._UPtr
    var _Size: UInt32

    def __init__( out self, sz: UInt32 = 4):
        self._DPtr = alloc[ Self.T]( Int( sz))
        self._Size = sz
        pass

    def __init__( out self, sz: UInt32, value: Self.T):
        self._Size = sz
        self._DPtr = alloc[ Self.T]( Int( sz))
        for i in USeg( sz):
            ( self._DPtr + i).init_pointee_copy( value)

    def __init__( out self, *, copy: Self):
        self._Size = copy._Size 
        copy._DPtr.free()
        self._DPtr = alloc[ Self.T]( Int( copy._Size))
        for i in USeg( copy._Size):
            ( self._DPtr + i).init_pointee_copy( copy._DPtr[ i])

    def __init__( out self, *, deinit take: Self):
        self._Size = take._Size
        self._DPtr = take._DPtr

    def __del__( deinit self):
        self._DPtr.free()
        pass

    @always_inline
    def Size( self) -> UInt32:
        return self._Size

    @always_inline
    def Arr( self) -> Arr[ Self.T, Self.origin]:
        return Arr[ Self.T, Self.origin]( self._DPtr, self._Size)

    @always_inline
    def Resize( mut self, nwSz: UInt32, value: Self.T):
        olDPtr = self._DPtr
        olSz = self._Size
        self._DPtr = alloc[ Self.T]( Int( nwSz))
        self._Size = nwSz
        sz = min( olSz, nwSz)
        for i in USeg( sz):
            ( self._DPtr + i).init_pointee_move_from( olDPtr + i)

        if sz < olSz:
            for i in USeg( sz, olSz - sz):
                ( olDPtr + i).destroy_pointee()

        if sz < nwSz:
            for i in USeg( sz, nwSz - sz):
                ( self._DPtr + i).init_pointee_copy( value)
        if sz:
            olDPtr.free()

    def Reserve( mut self, nwSz: UInt32):
        if nwSz > self._Size:
            self.Resize( nwSz, Self.T())