# stk.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from strand import Atm
import stash

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Stk[ is_mutable: Bool, //, T: CollectionElement, origin: Origin[is_mutable], is_atomic: Bool = False ]( CollectionElementNew):

    var     _Size: Atm[ is_atomic, DType.uint32]
    var     _Arr: Arr[ T, origin]
    
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( out self,  arr: Arr[ T, origin], size: UInt32 = 0):
        self._Arr = arr
        self._Size = size

    @always_inline
    fn __init__( out self, other: Self):
        self._Arr = other._Arr
        self._Size = other._Size.Value() 

    @always_inline
    fn __moveinit__( out self, owned other: Self, /):
        self._Arr = other._Arr
        self._Size =  other._Size.Get()  
        other._Arr.__init__()

    @always_inline
    fn __del__( owned self):    
        pass
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn Size( inout self) -> UInt32: 
        return self._Size.Get()  

    @always_inline
    fn SzVoid( inout self) -> UInt32: 
        return self._Arr.Size() -self._Size.Get() 
    
    @always_inline
    fn Arr( inout self) -> Arr[ T, __origin_of( self)]: 
        print( "stk: Arr")
        return Arr[ T, __origin_of( self)]( self._Arr._DArr, self._Size.Get() )
 
    @always_inline
    fn Top( inout self) -> T: 
        return self._Arr.__getitem__( self._Size.Get()  -1)
    
    @always_inline
    fn Pop( inout self: Stk[ T])-> T: 
        _ = self._Size.Decr( 1)
        return self._Arr.PtrAt( self._Size.Get() )[]
 
    @always_inline
    fn Push( inout self, x: T) -> UInt32: 
        self._Arr.PtrAt( self._Size.Get() )[] = x
        return  self._Size.Incr( 1) -1

    #-----------------------------------------------------------------------------------------------------------------------------
 
    @always_inline
    fn Import( inout self, inout stk : Stk[ T], maxMov: UInt32 = UInt32.MAX)   -> UInt32:              
        szCacheVoid = self.SzVoid()                                                                                
        szAlloc =  szCacheVoid if szCacheVoid < stk.Size() else stk.Size()
        if szAlloc > maxMov:
            szAlloc = maxMov 
        for i in uSeg( szAlloc):
            self._Arr.PtrAt( self._Size.Get() +i)[] = stk._Arr.PtrAt( stk._Size.Get() -szAlloc +i)[]
            
        _ = self._Size.Incr( szAlloc)
        _ = stk._Size.Decr( szAlloc)
        return szAlloc

#----------------------------------------------------------------------------------------------------------------------------------

fn StkExample():   
    vec  = Buff[ UInt32]( 7, 0) 
    arr = vec.Arr(); 
    i = 0
    for iter in arr:
        i += 1
        iter[] = i
    arr.SwapAt( 3, 5)  
    stk = Stk( arr, arr.Size())

    for i in uSeg( 2):
        x = stk.Pop() 
        print( x)
    
    for i in uSeg( 3):
        _ = stk.Push( i + 13)
    vec2  = Buff[ UInt32]( 100, 0) 
    vec2.Arr().Print()
    stk2 = Stk( vec2.Arr(), 5)
    stk2.Arr().Print()
    _ = stk2.Import( stk, 3)
    stk2.Arr().Print()

#----------------------------------------------------------------------------------------------------------------------------------

fn main():  
    StkExample()

#----------------------------------------------------------------------------------------------------------------------------------
