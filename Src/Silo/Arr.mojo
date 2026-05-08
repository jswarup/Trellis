# Arr.mojo -----------------------------------------------------------------------------------------------------------------------

from Silo import USeg

#----------------------------------------------------------------------------------------------------------------------------------


struct Arr[ T: ImplicitlyCopyable, origin: Origin = MutAnyOrigin]( 
    ImplicitlyCopyable, Iterable, Iterator, TrivialRegisterPassable, Writable
):
    comptime _UPtr = UnsafePointer[ Self.T, MutExternalOrigin]
    comptime _Null = Self._UPtr.unsafe_dangling()
    comptime Element = Self.T

    comptime IteratorType[ iterable_mut: Bool, //, iterable_origin: Origin[ mut=iterable_mut]]: Iterator = Self

    var _DPtr: Self._UPtr
    var _Size: UInt32

    def __init__( out self):
        self._DPtr = Self._Null
        self._Size = 0
        pass

    def __init__( out self, uPtr: Self._UPtr, sz: UInt32):
        self._Size = sz
        self._DPtr = uPtr

    @always_inline
    def __getitem__( self, idx: UInt32) -> ref[ Self.origin] Self.T:
        return self._DPtr[ idx]

    @always_inline
    def __setitem__( self, idx: UInt32, val: Self.T):
        self._DPtr[ idx].__del__()
        self._DPtr[ idx] = val

    @always_inline
    def __len__( self) -> UInt32:
        return self._Size

    @always_inline
    def __iter__( self) -> Self:
        return self

    @always_inline
    def __has_next__( self) -> Bool:
        return self._Size > 0

    @always_inline
    def __next__( mut self) -> ref[ Self.origin] Self.T:
        return self.Next()

    @always_inline
    def IsValid( self) -> Bool:
        return ( self._DPtr != Self._Null) and ( self._Size > 0)

    @always_inline
    def Size( self) -> UInt32:
        return self._Size

    @always_inline
    def USeg( self) -> USeg:
        return USeg( self._Size)

    @always_inline
    def PtrAt( self, idx: UInt32) -> Self._UPtr:
        return self._DPtr + idx

    @always_inline
    def At( self, idx: UInt32) -> ref[ Self.origin] Self.T:
        return self._DPtr[ idx]

    @always_inline
    def SetAt( self, idx: UInt32, val: Self.T):
        self._DPtr[ idx].__del__()
        self._DPtr[ idx] = val

    @always_inline
    def Subset( self, useg: USeg) -> Arr[ Self.T, Self.origin]:
        return Arr[ Self.T, Self.origin]( self._DPtr + useg.First(), useg.Size())

    @always_inline
    def DoIndicize[ dt: DType]( ref self: Arr[ Scalar[ dt], _], b: UInt32 = 0):
        for i in USeg( self._Size):
            self._DPtr[ i] = Scalar[ dt]( i + b)

    @always_inline
    def Next( mut self) -> ref[ Self.origin] Self.T:
        var startPtr = self._DPtr
        self._DPtr += 1
        self._Size -= 1
        return startPtr[ 0]

    @no_inline
    def write_to( self, mut writer: Some[ Writer]):
        writer.write( "[ ")
        comptime if conforms_to( self.T, Writable):
            for i in self.USeg():
                writer.write( " ", self._DPtr[ i])
        else:
            writer.write( "#", self._Size)
        return writer.write( "]")

    def Reverse( mut self):
        for i in USeg( self._Size / 2):
            var tmp = self._DPtr[ i]
            self._DPtr[ i] = self._DPtr[ self._Size - 1 - i]
            self._DPtr[ self._Size - 1 - i] = tmp
        return

    def SwapAt( self, a: UInt32, b: UInt32):
        if a != b:
            ( self._DPtr + a).swap_pointees( self._DPtr + b)

    def QSort[ Less: def( Self.T, Self.T) -> Bool]( self, less: Less):
        var useg = self.USeg()

        def lessIdxAt( a: UInt32, b: UInt32) -> Bool:
            #return less( self._DPtr[ a], self._DPtr[ b])
            return True

        def swapIdxAt( a: UInt32, b: UInt32):
            #self.SwapAt( a, b)
            pass

        useg.QSort( lessIdxAt, swapIdxAt)
