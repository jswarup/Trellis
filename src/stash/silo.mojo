# silo.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from strand import Atm, Spinlock
from stash import Buff, Stk, Arr 

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Silo[  is_mutable: Bool, //,  T: CollectionElement, origin: Origin[is_mutable].type, Mx: UInt32 ]:  
    var     _Lock: Spinlock
    var     _LockedMark: UInt32
    var     _Buff: Buff[ T, origin, True]
    var     _Stk : Stk[  T, origin, True]
    
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( out self):
        self._LockedMark = UInt32.MAX 
        self._Lock = Spinlock()
        self._Buff = Buff[ T, origin, True]( Mx)
        arr = self._Buff.Arr() 
        stk = Stk[ T, origin, True]( arr)
        self._Stk = stk

#----------------------------------------------------------------------------------------------------------------------------------

fn SiloExample():   
    silo = Silo[ UInt32, MutableAnyOrigin, 1024]()
    pass

#----------------------------------------------------------------------------------------------------------------------------------
