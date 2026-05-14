# Stash.mojo -----------------------------------------------------------------------------------------------------------------------

from Silo import Stk, Arr, Buff, USeg
from Strand import Atm

#----------------------------------------------------------------------------------------------------------------------------------

struct Stash[ T: ImplicitlyCopyable, origin: Origin = MutAnyOrigin]( Movable, Copyable, ImplicitlyCopyable):
    var _Buff: Buff[ Self.T, Self.origin]
    var _Stk: Stk[ Self.T, Self.origin]
 

    def __init__( out self, szCap: UInt32, fill: Self.T, szStk: UInt32 = 0):
        self._Buff = Buff[ Self.T, Self.origin]( szCap, fill)
        var arr = self._Buff.Arr()
        self._Stk = Stk[ Self.T, Self.origin]( arr, szStk)
        pass  

    @always_inline
    def Size( self) -> UInt32:
        return self._Stk.Size()

    @always_inline
    def Stk( mut self) -> Pointer[Stk[ Self.T, Self.origin], origin_of( self._Stk)]:
        return Pointer(to = self._Stk) 
 
    def  DoIndexSetup[ dt: DType]( mut self : Stash[ Scalar[ dt], Self.origin], fullFlg: Bool  = False) -> ref [self] Stash[ Scalar[ dt], Self.origin]:     
        arr = self._Buff.Arr()
        arr.DoIndicize() 
        self._Stk = Stk[ Scalar[ dt],  Self.origin] ( arr, arr.Size() if fullFlg else 0)
        return self 
    
    def  XferOutBulk( mut self, mut  stash: Stash[ Self.T, _]) -> UInt32:
        return self._Stk.Export( stash._Stk)

    def  XferInBulk( mut self, mut  stash: Stash[ Self.T, _]) -> UInt32:
        return self._Stk.Import( stash._Stk)


#----------------------------------------------------------------------------------------------------------------------------------
