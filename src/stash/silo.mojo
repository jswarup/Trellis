# silo.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from stash import Buff, Stk, Arr   
from strand import SpinLock, LockGuard

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Silo [ T: CollectionElement, is_atomic: Bool = False ] ( CollectionElement):  
    var     _Buff: Buff[ T]  
    var     _Stk: Stk[ T, MutableAnyOrigin, is_atomic] 
    
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( mut self, mx: UInt32):
        self._Buff = Buff[ T]( mx)
        arr = Arr[ T, MutableAnyOrigin]( self._Buff.DataPtr(), self._Buff.Size())
        self._Stk = Stk[ T, MutableAnyOrigin, is_atomic]( arr, 0) 

    @always_inline
    fn __copyinit__( out self, other: Self): 
        self._Buff = other._Buff
        arr = Arr[ T, MutableAnyOrigin]( self._Buff.DataPtr(), self._Buff.Size())
        self._Stk = Stk[ T, MutableAnyOrigin, is_atomic]( arr, 0) 
        
    @always_inline
    fn __moveinit__( out self, owned other: Self, /): 
        self._Buff = other._Buff^
        arr = Arr[ T, MutableAnyOrigin]( self._Buff.DataPtr(), self._Buff.Size())
        self._Stk = Stk[ T, MutableAnyOrigin, is_atomic]( arr, 0)
        
    fn  AllocBulk( mut self, mut  outSilo: Silo[  T]) ->UInt32:
        return outSilo._Stk.Import( self._Stk)

    fn  DoIndexSetup[ type: DType]( mut self : Silo[ Scalar[ type]], fullFlg: Bool  = False):     
        arr = self._Buff.Arr_()
        arr.DoInitIndicize()
        if fullFlg:
            self._Stk = Stk( arr, arr.Size())
        pass
  
    fn  Stack( ref [_]  self) -> Stk[ T, MutableAnyOrigin, is_atomic]  :
        return self._Stk
      
    @always_inline
    fn Pop( mut self)-> UnsafePointer[ T]:
        return self._Stk.Pop()
         
    @always_inline
    fn Push( mut self, x: T)-> UInt32:
        return self._Stk.Push( x)
         
        
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
    fn IntAssign( mut  x: UInt16, ind: UInt32):
        x = ind.cast[ DType.uint16]()
    #stk = silo.DoInit[ IntAssign]( 10)
    #print( stk.Size())
    pass

#----------------------------------------------------------------------------------------------------------------------------------
