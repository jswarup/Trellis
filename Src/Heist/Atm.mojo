# Atm.mojo -----------------------------------------------------------------------------------------------------------------------

from std.atomic import Atomic

#----------------------------------------------------------------------------------------------------------------------------------

struct Atm[ dtype: DType]( Movable, Copyable, ImplicitlyCopyable):
    var         _Data: Atomic[ Self.dtype]

    def __init__( out self, value: Scalar[ Self.dtype] = 0):
        self._Data = Atomic[ Self.dtype]( value)  
        
    def __init__(out self, *, copy: Self):
        self._Data = Atomic[ Self.dtype](copy._Data.load())

    def __init__(out self, *,  deinit take: Atm[ Self.dtype]): 
        self._Data =  Atomic[ Self.dtype]( take._Data.load()) 
        
    @always_inline
    def Get( self) -> Scalar[ Self.dtype]:
        return self._Data.load()

    @always_inline
    def Set( mut self, val: Scalar[ Self.dtype]) -> None:
        expected = self.Get()
        while not self.CompareExchange( expected, val):
            pass

    @always_inline
    def CompareExchange( mut self, mut expected: Scalar[ Self.dtype], desired: Scalar[ Self.dtype]) -> Bool:
        return self._Data.compare_exchange( expected, desired)  # return stored value before attempt

    @always_inline
    def Incr( mut self, rhs: Scalar[ Self.dtype]) -> Scalar[ Self.dtype]:
        ret = self._Data.fetch_add( rhs)
        ret += rhs
        return ret

#----------------------------------------------------------------------------------------------------------------------------------
 
struct Spinlock ( Copyable, ImplicitlyCopyable):  
    comptime   AtmFlag = Atm[ DType.int64]
    
    var     _Flag: Self.AtmFlag  

    def __init__( out self):
        self._Flag = Self.AtmFlag( 0) 
    
    def __init__( out self, *, copy: Self):
        self._Flag = Self.AtmFlag( copy._Flag.Get())  

    @always_inline
    def Lock( mut self):   
        var     expected  = Int64( 0)
        while not self._Flag.CompareExchange( expected, 1): 
            pass
        return

    @always_inline
    def Unlock(mut self): 
        self._Flag.Set( 0)
        return

#----------------------------------------------------------------------------------------------------------------------------------

struct Lockguard[ origin: MutOrigin]:
    
    comptime _SLockPtr =  Pointer[ Spinlock, Self.origin]

    var     lock : Self._SLockPtr

    def __init__( out self, ref [ Self.origin ] lock: Spinlock) :
        self.lock = Self._SLockPtr( to= lock)

    @no_inline
    def __enter__( self):
        self.lock[].Lock()

    @no_inline
    def __exit__( self):
        self.lock[].Unlock()

#----------------------------------------------------------------------------------------------------------------------------------
 
