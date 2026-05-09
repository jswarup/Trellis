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
        return 

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

    def Reverse( ref self):
        for i in USeg( self._Size / 2):
            var tmp = self._DPtr[ i]
            self._DPtr[ i] = self._DPtr[ self._Size - 1 - i]
            self._DPtr[ self._Size - 1 - i] = tmp
        return

    def SwapAt( self, a: UInt32, b: UInt32):
        if a != b:
            ( self._DPtr + a).swap_pointees( self._DPtr + b)
 
    def Partition[ Less: def( Self.T, Self.T) -> Bool, Swap: def( UInt32, UInt32)]
            (self, low: UInt32, high: UInt32,  less: Less, swap: Swap) -> UInt32: 
        
        pivot = self.At( high)                                              # Choose the rightmost element as pivot 
        i = low - 1                                                         # Pointer for the greater element   
        for j in range(low, high):                                          # Traverse through all elements and compare them with the pivot
            if less( self.At( j), pivot):
                i = i + 1 
                self.SwapAt( i, j)                                          # Swap elements
                swap( i, j)
                 
        if less( pivot, self.At( i + 1)):
            self.SwapAt( i + 1, high)                                           # Swap the pivot element with the greater element specified by i
            swap( i + 1, high)
        
        
        return i + 1                                                        # Return the position from where partition is done

    def QSort[ Less: def( Self.T, Self.T) -> Bool, Swap: def( UInt32, UInt32)](self, low: UInt32, high: UInt32,  less: Less, swap: Swap):   

        if not low < high:
            return

        pivot_index = self.Partition( low, high, less, swap)                # Find the partition index   
        if low < ( pivot_index):
            self.QSort( low, pivot_index - 1, less, swap)                       # Recursively sort elements before and after partition
        if ( pivot_index +1) < high:
            self.QSort( pivot_index + 1, high, less, swap) 