# silo.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from strand import Atm, Spinlock
from stash import Buff, Stk, Arr   

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Silo[   T: CollectionElement] :  
    var     _Lock: Spinlock
    var     _LockedMark: UInt32
    var     _Buff: Buff[ T, True]
    
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( inout self, Mx: UInt32 ):
        self._LockedMark = UInt32.MAX 
        self._Lock = Spinlock()
        self._Buff = Buff[ T, True]( Mx)  
        

    fn DoInit[ IntAssign : fn( inout x: T, ind: UInt32) capturing-> None ]( inout self: Silo[ T]) ->Stk[  T, True, __origin_of( self._Buff)]:
        arr = self._Buff.Arr() 
        stk = Stk[  T, True, __origin_of( self._Buff)]( arr) 
        for i in uSeg( self._Buff.Size()):
            IntAssign( arr[ i], i)
        return stk

    fn  IsLocked( self, id: UInt32 ) -> Bool :
        return id > self._LockedMark

    fn  AllocBulk( inout self, inout alStk: Stk[  T, _, MutableAnyOrigin]) ->UInt32:
        return 0 #return alStk.Import( self._Stk)
    
#----------------------------------------------------------------------------------------------------------------------------------

fn SiloExample():   
    silo = Silo[ UInt16]( 1024)
    @parameter
    fn IntAssign( inout x: UInt16, ind: UInt32):
        x = ind.cast[ DType.uint16]()
    stk = silo.DoInit[ IntAssign]()
    print( stk.Size())
    pass

#----------------------------------------------------------------------------------------------------------------------------------
