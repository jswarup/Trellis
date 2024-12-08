# atm.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer
from os import Atomic

#----------------------------------------------------------------------------------------------------------------------------------

struct Atm[ is_atomic: Bool, type: DType]:

    var     _Data: Atomic[ type]

    @always_inline
    fn __init__(out self, val: Scalar[type]):
        self._Data = val 

    @always_inline
    fn Value( self) -> Scalar[type]: 
        return self._Data.value

    @always_inline
    fn Get( self) -> Scalar[type]:
        return self._Data.value
        
    @always_inline
    fn Fetch( inout self) -> Scalar[type]:
        @parameter
        if is_atomic:
            ret = self._Data.load() 
        else:
            ret = self._Data.value
        return ret

    @always_inline
    fn Set( inout self, val: Scalar[type]) -> None:
        expected = self.Get()
        while not self.CompareExchange( expected, val):
            pass 
        return
        
    @always_inline
    fn CompareExchange( inout self, inout expected: Scalar[type], desired: Scalar[type] ) -> Bool:
        res = True
        @parameter
        if is_atomic :
            res = self._Data.compare_exchange_weak( expected, desired)
        else:
            self._Data.value = desired
        return res

    @always_inline
    fn Incr( inout self, rhs: Scalar[type]) -> Scalar[type]:
        @parameter 
        if is_atomic:
            ret = self._Data.fetch_add( rhs)
        else:
            ret = self._Data.value = self._Data.value + rhs
        return ret

    @always_inline
    fn Decr( inout self, rhs: Scalar[type]) -> Scalar[type]: 
        @parameter 
        if is_atomic:
            ret = self._Data.fetch_add( rhs)
        else:
            ret = self._Data.value = self._Data.value - rhs
        return ret
        
#----------------------------------------------------------------------------------------------------------------------------------

fn AtmExample():   
    var atm = Atm[ False]( 3.0)
    atm.Set( 13)
    x = atm.Get()
    print( x)

#----------------------------------------------------------------------------------------------------------------------------------
