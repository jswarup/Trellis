# USeg.mojo ------------------------------------------------------------------------------------------------------------------------

#----------------------------------------------------------------------------------------------------------------------------------


struct USeg( ImplicitlyCopyable, Iterable, Iterator, TrivialRegisterPassable, Writable):
    comptime Element = UInt32
    comptime IteratorType[ iterable_mut: Bool, //, iterable_origin: Origin[ mut=iterable_mut]]: Iterator = Self

    var _First: UInt32
    var _Last: UInt32

    @always_inline
    def __init__( out self):
        self._First = UInt32.MAX
        self._Last = UInt32.MAX - 1

    @always_inline
    def __init__( out self, sz: UInt32):
        self._First = 0
        self._Last = 0 + sz - 1

    @always_inline
    def __init__( out self, b: UInt32, sz: UInt32):
        self._First = b
        self._Last = b + sz - 1

    @always_inline
    def __len__( self) -> Int:
        return Int( self.Size())

    @always_inline
    def Begin( self) -> UInt32:
        return self._First

    @always_inline
    def End( self) -> UInt32:
        return self._Last + 1

    @always_inline
    def Size( self) -> UInt32:
        return self.End() - self.Begin()

    @always_inline
    def First( self) -> UInt32:
        return self._First

    @always_inline
    def Mid( self) -> UInt32:
        return ( self._First + self._Last) // 2

    @always_inline
    def Last( self) -> UInt32:
        return self._Last

    @always_inline
    def IsValid( self) -> Bool:
        return self._First != UInt32.MAX

    @always_inline
    def __getitem__( self, idx: UInt32) -> UInt32:
        return self._First + idx

    @always_inline
    def __iter__( self) -> Self:
        return self

    @always_inline
    def __has_next__( self) -> Bool:
        return self.First() < self.End()

    @always_inline
    def __next__( mut self) -> UInt32:
        var start = self._First
        self._First += 1
        return start

    @always_inline
    def __str__( self) -> String:
        return "[ " + String.write( self.First()) + ", " + String.write( self.Last()) + "]"

    @no_inline
    def write_to( self, mut writer: Some[ Writer]):
        return writer.write( "[ ", self.First(), ", ", self.Last(), "]")

    @always_inline
    def Span[ Lambda: def( UInt32) -> Bool]( self, lm: Lambda) -> UInt32:
        var i: UInt32 = 0
        for _ in USeg( self.Size()):
            if not lm( self.First() + i):
                break
            i += 1
        return i

    def QSortPartition[ LessAt: def( UInt32, UInt32) -> Bool, SwapAt: def( UInt32, UInt32)]
            ( self, lessAt: LessAt, swapAt: SwapAt) -> UInt32:
        var     i = self.First()
        var     j = self.Last()
        var     piv = self.Mid()
        while ( i < j):
            while (( i < piv) and lessAt( i, piv)):
                i += 1
            while (( piv < j) and lessAt( piv, j)):
                j -= 1
            if i < j:
                swapAt( i, j)
                i += 1
                j -= 1
        return i

    def QSort[ LessAt: def( UInt32, UInt32) -> Bool, SwapAt: def( UInt32, UInt32)]( self, lessAt: LessAt, swapAt: SwapAt):
        if self.Size() <= 1:
            return 
        var     piv = self.QSortPartition( lessAt, swapAt)
        USeg( self.First(), piv - self.First()).QSort( lessAt, swapAt)
        USeg( piv + 1, self.Last() - piv).QSort( lessAt, swapAt)
