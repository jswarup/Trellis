# silo.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from strand import Atm, Spinlock
from stash import Buff, Stk, Arr   

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Silo[  is_mutable: Bool, //,  T: CollectionElement,  origin: Origin[is_mutable].type] :  
    var     _Lock: Spinlock
    var     _LockedMark: UInt32
    var     _Buff: Buff[ T, origin, True]
    var     _Stk : Stk[  T, origin, True]
    
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( out self, Mx: UInt32 ):
        self._LockedMark = UInt32.MAX 
        self._Lock = Spinlock()
        self._Buff = Buff[ T, origin, True]( Mx)
        arr = self._Buff.Arr() 
        stk = Stk[ T, origin, True]( arr)
        self._Stk = stk

    fn DoInit[ IntAssign : fn( inout x: T, ind: UInt32) capturing-> None ]( inout self: Silo[ T, MutableAnyOrigin]) ->None:
        arr = self._Buff.Arr() 
        for i in uSeg( self._Buff.Size()):
            IntAssign( arr[ i], i)
        pass

    fn  IsLocked( self, id: UInt32 ) -> Bool :
        return id > self._LockedMark

    fn  AllocBulk( inout self, inout alStk: Stk[  T, MutableAnyOrigin, _]) ->UInt32:
        return 0 #return alStk.Import( self._Stk)
    
#----------------------------------------------------------------------------------------------------------------------------------

fn SiloExample():   
    silo = Silo[ UInt16, MutableAnyOrigin]( 1024)
    @parameter
    fn IntAssign( inout x: UInt16, ind: UInt32):
        x = ind.cast[ DType.uint16]()
    silo.DoInit[ IntAssign]()
    print( silo._Buff.Size())
    pass

#----------------------------------------------------------------------------------------------------------------------------------
