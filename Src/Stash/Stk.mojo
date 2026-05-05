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
  