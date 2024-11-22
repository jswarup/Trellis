# atm.mojo ------------------------------------------------------------------------------------------------------------------------

from os import Atomic

#----------------------------------------------------------------------------------------------------------------------------------

struct Atm[ is_atomic: Bool, type: DType]:
    var     _Data: Atomic[ type]

    @always_inline
    fn __init__(out self, value: Scalar[type]):
        self._Data = value
    
    @always_inline
    fn Get( inout self) -> Scalar[type]:
        @parameter
        ret = self._Data.load() if is_atomic else self._Data.value
        return ret
    
    @always_inline
    fn Set( inout self, value: Scalar[type]) -> None:
        expected = self.Get()
        while not self.CompareExchange( expected, value):
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


#----------------------------------------------------------------------------------------------------------------------------------

fn AtmExample():   
    var atm = Atm[ False]( 3.0)
    atm.Set( 13)
    x = atm.Get()
    print( x)

#----------------------------------------------------------------------------------------------------------------------------------
