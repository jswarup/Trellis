# Stash.mojo -----------------------------------------------------------------------------------------------------------------------

from Silo import Stk, Arr, Buff, USeg
from Strand import Atm

#----------------------------------------------------------------------------------------------------------------------------------

struct Stash[ T: ImplicitlyCopyable, origin: Origin = MutAnyOrigin]( Movable):
    var _Buff: Buff[ Self.T, Self.origin]
    var _Stk: Stk[ Self.T, Self.origin]

    def __init__( out self):
        self._Buff = Buff[ Self.T, Self.origin]()
        self._Stk = Stk[ Self.T, Self.origin]()
        pass

    def __init__( out self, szCap: UInt32, ):
        self._Buff = Buff[ Self.T, Self.origin]( szCap)
        var arr = self._Buff.Arr()
        self._Stk = Stk[ Self.T, Self.origin]( arr, 0)
        pass 
          

    def __init__( out self, *, deinit take: Self):
        self._Buff = take._Buff^ 
        self._Stk = take._Stk^ 

    @always_inline
    def Size( self) -> UInt32:
        return self._Stk.Size()

    @always_inline
    def Arr( self) -> Arr[ Self.T, Self.origin]:
        return self._Stk.Arr()

    @always_inline
    def Push( mut self, val: Self.T) -> Bool:
        return self._Stk.Push( val) 

    def  DoIndexSetup[ dt: DType]( mut self : Stash[ Scalar[ dt], Self.origin], fullFlg: Bool  = False) -> ref [self] Stash[ Scalar[ dt], Self.origin]:     
        arr = self._Buff.Arr()
        arr.DoIndicize() 
        self._Stk = Stk[ Scalar[ dt],  Self.origin] ( arr, arr.Size() if fullFlg else 0)
        return self 
    
    def  XferOutBulk( mut self, mut  stash: Stash[ Self.T, _]) ->UInt32:
        return self._Stk.Export( stash._Stk)

    def  XferInBulk( mut self, mut  stash: Stash[ Self.T, _]) ->UInt32:
        return self._Stk.Import( stash._Stk)


#----------------------------------------------------------------------------------------------------------------------------------
