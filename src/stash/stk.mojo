# stk.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
import stash

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Stk[ origin: Origin[ True].type,//, T: CollectionElement]( CollectionElementNew):
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
    fn Pop( inout self) -> T: 
        self._Size -= 1
        return self._Arr.PtrAt( self._Size)[]
 
    @always_inline
    fn Push( inout self, x: T) -> UInt32: 
        self._Arr.PtrAt( self._Size)[] = x
        self._Size += 1
        return self._Size -1

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
        print( x)
    
    for i in uSeg( 3):
        _ = stk.Push( i + 13)
    stk.Arr().Print()

#----------------------------------------------------------------------------------------------------------------------------------

fn main():  
    StkExample()

#----------------------------------------------------------------------------------------------------------------------------------
