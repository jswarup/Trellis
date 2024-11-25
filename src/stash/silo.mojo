# silo.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from strand import Atm, SpinLock
from stash import Buff, Stk, Arr   

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Silo [ T: CollectionElement] :  
    var     _Lock: SpinLock
    var     _LockedMark: UInt32
    var     _Buff: Buff[ T, True] 
    var     _Arr: Arr[ T, MutableAnyOrigin] 
    var     _Stk: Stk[ T, MutableAnyOrigin] 
    
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( inout self, mx: UInt32):
        self._LockedMark = UInt32.MAX 
        self._Lock = SpinLock()
        self._Buff = Buff[ T, True]( mx)
        self._Arr.DoInit( self._Buff.DataPtr(), mx)
        self._Stk = Stk( self._Arr, 0)

    fn  IsLocked( self, id: UInt32 ) -> Bool :
        return id > self._LockedMark

    fn  AllocBulk( inout self, inout outSilo: Silo[  T]) ->UInt32:
        return outSilo._Stk.Import( self._Stk)

    fn  DoIndexSetup[ type: DType]( inout self : Silo[ Scalar[ type]], fullFlg: Bool  = False):        
        self._Arr.DoInitIndicize()
        if fullFlg:
            self._Stk = Stk( self._Arr, self._Arr.Size())
        pass

#----------------------------------------------------------------------------------------------------------------------------------

fn SiloExample():  
    print( "SiloExample")  
    silo = Silo[ UInt16]( 113)
    silo.DoIndexSetup( True)
    silo2 = Silo[ UInt16]( 83) 
    num = silo.AllocBulk( silo2)
    print( num)
    silo._Stk.Arr().Print()
    @parameter
    fn IntAssign( inout x: UInt16, ind: UInt32):
        x = ind.cast[ DType.uint16]()
    #stk = silo.DoInit[ IntAssign]( 10)
    #print( stk.Size())
    pass

#----------------------------------------------------------------------------------------------------------------------------------
