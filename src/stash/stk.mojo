# stk.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
import stash

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Stk[ is_mutable: Bool, //, T: CollectionElement, origin: Origin[ is_mutable].type,]( CollectionElementNew):
    var     _Size: UInt32
    var     _Arr: Arr[T, origin]
    
    @always_inline
    fn __init__( inout self,  arr: Arr[T, origin], size: UInt32 = 0):
        self._Arr = arr
        self._Size = size

    @always_inline
    fn __init__( inout self, other: Self):
        self._Arr = other._Arr
        self._Size = other._Size 

    @always_inline
    fn __moveinit__( inout self, owned existing: Self, /):
        self._Arr = existing._Arr
        self._Size =  existing._Size  
    
    @always_inline
    fn Size( self) -> UInt32: 
        return self._Size 

    @always_inline
    fn SzVoid( self) -> UInt32: 
        return self._Arr.Size() -self._Size 
    
    @always_inline
    fn Arr( self) -> Arr[ T, origin]: 
        return Arr[T, origin]( self._Arr.unsafe_ptr(), self._Size)
 
    @always_inline
    fn Top( inout self) -> T: 
        return self._Arr.__getitem__( self._Size -1)
    
    @always_inline
    fn Pop[ origin: MutableOrigin, //]( inout self: Stk[T, origin])-> T: 
        self._Size -= 1
        return self._Arr.PtrAt( self._Size)[]
 
    @always_inline
    fn Push[ origin: MutableOrigin, //]( inout self: Stk[T, origin], x: T) -> UInt32: 
        self._Arr.PtrAt( self._Size)[] = x
        self._Size += 1
        return self._Size -1
 
    @always_inline
    fn Import[ origin: MutableOrigin, orig: MutableOrigin]( inout self: Stk[T, origin], inout stk: Stk[T, orig], maxMov: UInt32 = UInt32.MAX)   -> UInt32:              
        szCacheVoid = self.SzVoid()                                                                                
        szAlloc =  szCacheVoid if szCacheVoid < stk.Size() else stk.Size()
        if szAlloc > maxMov:
            szAlloc = maxMov 
        for i in uSeg( szAlloc):
            self._Arr.PtrAt( self._Size +i)[] = stk._Arr.PtrAt( stk._Size -szAlloc +i)[]
            
        self._Size += szAlloc
        stk._Size -= szAlloc
        return szAlloc

 #----------------------------------------------------------------------------------------------------------------------------------

fn StkExample():   
    vec  = VArr[ UInt32]( 7, 0) 
    arr = vec.Arr(); 
    i = 0
    for iter in arr:
        i += 1
        iter[] = i
    arr.SwapAt( 3, 5)  
    stk = Stk( arr, arr.Size())

    for i in uSeg( 4):
        x = stk.Pop() 
    
    for i in uSeg( 3):
        _ = stk.Push( i + 13)
    stk.Arr().Print()
    vec2  = VArr[ UInt32]( 100, 0) 
    stk2 = Stk( vec2.Arr())
    _ = stk2.Import( stk, 3)
    stk2.Arr().Print()

#----------------------------------------------------------------------------------------------------------------------------------

fn main():  
    StkExample()

#----------------------------------------------------------------------------------------------------------------------------------
