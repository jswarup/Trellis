# Stk.mojo -----------------------------------------------------------------------------------------------------------------------

from Stash import USeg, Arr, Buff 
from Strand import Atm

#----------------------------------------------------------------------------------------------------------------------------------
 
struct Stk [ Mut: Bool, //,T: ImplicitlyCopyable,  origin: Origin[ mut =Mut]](  ):   
    
    var     _Arr: Arr[ Self.T, Self.origin]
    var     _Size: Atm[ DType.uint32]
     
    def __init__(out self, arr : Arr[ Self.T, Self.origin], sz: UInt32 = 0):  
        self._Size = Atm[ DType.uint32]( sz)
        self._Arr = arr
        pass
  
    def Push( mut self, val: Self.T) -> Bool: 
        if self._Size.Get() >= self._Arr._Size:
            return False
        self._Arr[ self._Size.Get()] = val
        self._Size.Set( self._Size.Get() + 1)
        return True

    @always_inline
    def __len__( self) -> UInt32:
        return self._Size.Get()

    @always_inline
    def USeg( self) -> USeg: 
        return USeg( self._Size.Get())

    @always_inline
    def Size( mut self) -> UInt32: 
        return self._Size.Get()  

    @always_inline
    def SzVoid( mut self) -> UInt32: 
        return self._Arr.Size() -self._Size.Get() 