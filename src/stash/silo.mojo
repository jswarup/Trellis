# silo.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from stash import Buff, Stk, Arr   

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Silo [ T: CollectionElement, is_atomic: Bool = False ] :  
    var     _Buff: Buff[ T]  
    var     _Stk: Stk[ T, MutableAnyOrigin, is_atomic] 
    
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( inout self, mx: UInt32):
        self._Buff = Buff[ T]( mx)
        arr = Arr[ T, MutableAnyOrigin]( self._Buff.DataPtr(), mx)
        self._Stk = Stk[ T, MutableAnyOrigin, is_atomic]( arr, 0)


    fn  AllocBulk( inout self, inout outSilo: Silo[  T]) ->UInt32:
        return outSilo._Stk.Import( self._Stk)

    fn  DoIndexSetup[ type: DType]( inout self : Silo[ Scalar[ type]], fullFlg: Bool  = False):     
        arr = self._Buff.Arr_()
        arr.DoInitIndicize()
        if fullFlg:
            self._Stk = Stk( arr, arr.Size())
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
