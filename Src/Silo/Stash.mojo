# Stash.mojo -----------------------------------------------------------------------------------------------------------------------

from Silo import Stk, Arr, Buff, USeg
from Strand import Atm

#----------------------------------------------------------------------------------------------------------------------------------

struct Stash [ T: ImplicitlyCopyable, origin: Origin = MutAnyOrigin](  ):

    var     _Buff : Buff[ Self.T, Self.origin]
    var     _Stk: Stk[ Self.T, Self.origin] 
     
    def __init__(out self):  
        self._Buff = Buff[ Self.T, Self.origin]()
        self._Stk = Stk[ Self.T, Self.origin]()
        pass

    def __init__(out self, szCap: UInt32):  
        self._Buff = Buff[ Self.T, Self.origin]( szCap)
        var     arr = self._Buff.Arr()
        self._Stk = Stk[ Self.T, Self.origin]( arr, 0)
        pass

    @always_inline
    def Size( mut self) -> UInt32: 
        return self._Stk.Size() 

    @always_inline
    def Arr( self) -> Arr[ Self.T, Self.origin]: 
        return self._Stk.Arr() 

    @always_inline
    def Push( mut self, val: Self.T) -> Bool: 
        return self._Stk.Push( val)

#----------------------------------------------------------------------------------------------------------------------------------
 