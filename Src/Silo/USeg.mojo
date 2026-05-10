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
        self._Last = b + ( sz - 1 )

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
    def          LSnip( self, k : UInt32) -> Self:
        return USeg( self._First + k, self.Size() - k) 
        
    @always_inline
    def          RSnip( self, k : UInt32) -> Self:
        return USeg( self._First, self.Size() - k)

    @always_inline
    def Span[ Lambda: def( UInt32) -> Bool]( self, lm: Lambda) -> UInt32:
        var i: UInt32 = 0
        for _ in USeg( self.Size()):
            if not lm( self.First() + i):
                break
            i += 1
        return i
 


    def Partition[ LessAt: def( UInt32, UInt32) -> Bool, SwapAt: def( UInt32, UInt32)]( self, lessAt: LessAt, swapAt: SwapAt) -> UInt32:  
         
        mid = self.Mid()
        if lessAt( self._First, mid):                                        
            swapAt( self._First, mid)  
 
        pivot = self._First
        for i in self.LSnip( 1):
            if lessAt( i,  self._First):
                pivot += 1  
                swapAt( pivot, i)
                
        if lessAt( pivot, self._First):  
            swapAt( self._First, pivot) 
        return pivot

    def QSort[ LessAt: def( UInt32, UInt32) -> Bool, SwapAt: def( UInt32, UInt32)]( self, lessAt: LessAt, swapAt: SwapAt):  
          
        pivot = self.Partition( lessAt, swapAt) 

        # Recursively sort the two sub-arrays  
        useg = USeg( self._First, pivot - self._First);
        if ( useg.Size() > 1):
            useg.QSort( lessAt, swapAt)  
            
        useg = USeg( pivot + 1, self._Last - pivot);
        if ( useg.Size() > 1):
            useg.QSort( lessAt, swapAt) 
        
    
    def LowerBound[ LessTestAt: def( UInt32) -> Bool]( self, lessTestAt: LessTestAt, low : UInt32) -> UInt32: 
        hi = self.End()
        lo = low 
        while ( lo < hi)  : 
            mid = (lo + hi)/2;
            if ( lessTestAt( mid)):
                lo = mid + 1
            else:
                hi = mid
        return lo
    
    
    def LowerBound[ LessAt: def( UInt32, UInt32) -> Bool]( self, low : UInt32, x : UInt32, lessAt: LessAt) -> UInt32: 
        hi = self.End()
        lo = low 
        while ( lo < hi)  : 
            mid = (lo + hi)/2;
            if ( lessAt( mid, x)):
                lo = mid + 1
            else:
                hi = mid
        return lo

    def UpperBound[ LessAt: def( UInt32, UInt32) -> Bool]( self, low : UInt32, x : UInt32, lessAt: LessAt) -> UInt32: 
        hi = self.End()
        lo = low 
        while ( lo < hi)  : 
            mid = (lo + hi)/2;
            if ( lessAt( x, mid)):
                hi = mid
            else:
                lo = mid + 1
        return lo