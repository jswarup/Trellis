# Stk.mojo -----------------------------------------------------------------------------------------------------------------------

from Stash import USeg, Arr, Buff 
from Strand import Atm

#----------------------------------------------------------------------------------------------------------------------------------
 
struct Stk [ Mut: Bool, //,T: ImplicitlyCopyable,  origin: Origin[ mut =Mut]](  ):   
    
    comptime    _UPtr = UnsafePointer[Self.T, MutExternalOrigin]
    comptime    _Null = Self._UPtr.unsafe_dangling()

    var     _Arr: Arr[ Self.T, Self.origin]
    var     _Size: Atm[ DType.uint32]
     
    def __init__(out self, arr : Arr[ Self.T, Self.origin], sz: UInt32 = 0):  
        self._Size = Atm[ DType.uint32]( sz)
        self._Arr = arr
        pass
  
    @always_inline
    def __len__( self) -> UInt32:
        return self._Size.Get()

    @always_inline
    def Size( mut self) -> UInt32: 
        return self._Size.Get()  

    @always_inline
    def USeg( self) -> USeg: 
        return USeg( self._Size.Get())

    
    @always_inline
    def Top(  self) -> ref[Self.origin] Self.T: 
        return self._Arr[ self._Size.Get()  -1]
    
    @always_inline
    def PopPtr( mut self) -> Self._UPtr: 
        while True:
            var     sz =  self._Size.Get()
            if sz == 0:
                return Self._Null 
            if ( self._Size.CompareExchange( sz, sz -1)):
                return self._Arr.ObjPtrAt( sz -1) 
    
    @always_inline
    def Pop( mut self) -> ref[Self.origin] Self.T:              # Use with Caution : Single thread 
        return self.PopPtr()[]
  
    @always_inline
    def Push( mut self, val: Self.T) -> Bool: 
        var     sz =  self._Size.Get()
        if sz >= self._Arr._Size:
            return False
        if ( not self._Size.CompareExchange( sz, sz + 1)):
            return False
        self._Arr[ sz] = val 
        return True
   
    def Import( mut self, mut  stk : Stk[ Self.T, _], maxMov: UInt32 = UInt32.MAX)   -> UInt32:            
        while True:
            var         sz = self._Size.Get()            
            szCacheVoid = self._Arr.Size() -sz                                                                               
            szAlloc =  szCacheVoid if szCacheVoid < stk.Size() else stk.Size()
            if szAlloc > maxMov:
                szAlloc = maxMov  
            if ( szAlloc == 0):
                return 0
            if ( self._Size.CompareExchange( sz, sz + szAlloc)):
                break    
        var     sz = self._Size.Get()
        var     stkSz = stk._Size.Incr( -szAlloc) 
        for i in USeg( szAlloc):
            self._Arr[ sz -i -1] = stk._Arr[ stkSz +i]
        return szAlloc

    
    def Export( mut self, mut  stk : Stk[ Self.T, _], maxMov: UInt32 = UInt32.MAX)   -> UInt32:            
        while True:
            var         sz = stk._Size.Get()            
            szCacheVoid = stk._Arr.Size() -sz                                                                               
            szAlloc =  szCacheVoid if szCacheVoid < self.Size() else self.Size()
            if szAlloc > maxMov:
                szAlloc = maxMov  
            if ( szAlloc == 0):
                return 0
            if ( stk._Size.CompareExchange( sz, sz + szAlloc)):
                break    
        var     sz = stk._Size.Get()
        var     stkSz = self._Size.Incr( -szAlloc) 
        for i in USeg( szAlloc):
            stk._Arr[ sz -i -1] = self._Arr[ stkSz +i]
        return szAlloc
