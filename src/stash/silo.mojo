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
    fn __init__( mut self, mx: UInt32, value: T):
        self._Buff = Buff[ T]( mx, value)
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
        self._Stk = Stk[ T, MutableAnyOrigin, is_atomic]( Arr[ T, MutableAnyOrigin]( self._Buff.DataPtr(), self._Buff.Size()), other._Stk.Size())
        
    fn __del__( owned self): 
        #print( "Silo: Del ")
        pass
        
    fn  AllocBulk( mut self, mut  outSilo: Silo[  T]) ->UInt32:
        return outSilo._Stk.Import( self._Stk)

    fn  DoIndexSetup[ type: DType]( mut self : Silo[ Scalar[ type], is_atomic], fullFlg: Bool  = False):     
        arr = self._Buff.Arr_()
        arr.DoInitIndicize()
        if fullFlg:
            self._Stk = Stk[ Scalar[ type], MutableAnyOrigin,  is_atomic] ( arr, arr.Size())
        pass
  
    fn  Stack( ref [_]  self) -> Pointer[ Stk[ T, MutableAnyOrigin, is_atomic], __origin_of( self._Stk)]  :
        return Pointer.address_of( self._Stk)
          

    @always_inline
    fn Push( mut self, x: T)-> UInt32:
        return self._Stk.Push( x)
         
        
#----------------------------------------------------------------------------------------------------------------------------------

fn SiloExample():  
    print( "SiloExample")  
    silo = Silo[ UInt16]( 113, 0)
    silo.DoIndexSetup( True)
    silo2 = Silo[ UInt16]( 83, 0) 
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
