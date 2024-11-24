# stk.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
import stash

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Stk[ T: CollectionElement ]( CollectionElementNew):

    var     _Size: UInt32
    var     _Arr: Arr[ T]
    
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( out self,  arr: Arr[ T], size: UInt32 = 0):
        self._Arr = arr
        self._Size = size

    @always_inline
    fn __init__( out self, other: Self):
        self._Arr = other._Arr
        self._Size = other._Size 

    @always_inline
    fn __moveinit__( out self, owned other: Self, /):
        self._Arr = other._Arr
        self._Size =  other._Size  
        other._Arr.__init__()

    @always_inline
    fn __del__( owned self):    
        pass
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn Size( self) -> UInt32: 
        return self._Size 

    @always_inline
    fn SzVoid( self) -> UInt32: 
        return self._Arr.Size() -self._Size 
    
    @always_inline
    fn Arr( self) -> Arr[ T]: 
        print( "stk: Arr")
        return Arr[ T]( self._Arr._DArr, self._Size)
 
    @always_inline
    fn Top( inout self) -> T: 
        return self._Arr.__getitem__( self._Size -1)
    
    @always_inline
    fn Pop( inout self: Stk[ T])-> T: 
        self._Size -= 1
        return self._Arr.PtrAt( self._Size)[]
 
    @always_inline
    fn Push( inout self, x: T) -> UInt32: 
        self._Arr.PtrAt( self._Size)[] = x
        self._Size += 1
        return self._Size -1

    #-----------------------------------------------------------------------------------------------------------------------------
 
    @always_inline
    fn Import( inout self, inout stk : Stk[ T], maxMov: UInt32 = UInt32.MAX)   -> UInt32:              
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
