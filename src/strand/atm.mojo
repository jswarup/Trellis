# atm.mojo ------------------------------------------------------------------------------------------------------------------------

from os import Atomic

struct Atm[ type: DType, //, is_atomic: Bool = False]:
    var     _Data: Atomic[type]

    @always_inline
    fn __init__(out self, value: Scalar[type]):
        self._Data = value
    
    @always_inline
    fn Get( inout self) -> Scalar[type]:
        @parameter
        ret = self._Data.load() if is_atomic else self._Data.value
        return ret
    
    @always_inline
    fn Set( inout self, value: Scalar[type]) :
        @parameter
        if is_atomic :
            expected = self._Data.value
            while not self._Data.compare_exchange_weak( expected, value):
                pass
        else:
            self._Data.value = value
        return
#----------------------------------------------------------------------------------------------------------------------------------

fn AtmExample():   
    var atm = Atm[ False]( 3.0)
    atm.Set( 13)
    x = atm.Get()
    print( x)
