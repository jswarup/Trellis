# stk.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from strand import Atm,SpinLock, LockGuard
import stash

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Stk[ is_mutable: Bool, //, T: CollectionElement, origin: Origin[is_mutable], is_atomic: Bool = False ]( CollectionElement):

    var     _Size: Atm[ is_atomic, DType.uint32]
    var     _Arr: Arr[ T, origin]
    
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( out self,  arr: Arr[ T, origin], size: UInt32 = 0):
        self._Arr = arr
        self._Size.__init__( size)

    @always_inline
    fn __init__( out self, other: Self):
        self._Arr = other._Arr
        self._Size.__init__( other._Size.Value())

    @always_inline
    fn __moveinit__( out self, owned other: Self, /):
        self._Arr = other._Arr
        self._Size.__init__( other._Size.Get())  
        other._Arr.__init__()

    @always_inline
    fn __copyinit__( out self, other: Self, /):
        self._Arr = other._Arr
        self._Size.__init__( other._Size.Get())  

    @always_inline
    fn __del__( owned self):    
        pass
    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn Size( mut self) -> UInt32: 
        return self._Size.Fetch()  

    @always_inline
    fn SzVoid( mut self) -> UInt32: 
        return self._Arr.Size() -self._Size.Fetch() 
    
    @always_inline
    fn Arr( mut self) -> Arr[ T, __origin_of( self)]: 
        print( "stk: Arr")
        return Arr[ T, __origin_of( self)]( self._Arr._DArr, self._Size.Get() )
 
    @always_inline
    fn Top( mut self) -> T: 
        return self._Arr.__getitem__( self._Size.Get()  -1)
    
    @always_inline
    fn Pop( mut self)-> UnsafePointer[ T]:
        ind = self._Size.Decr( 1);
        if (  ind != UInt32.MAX):
            return self._Arr.PtrAt( ind)  
        return UnsafePointer[ T]()
 
    @always_inline
    fn Pop( mut self, mut  slock : SpinLock)-> UnsafePointer[ T]:
        with LockGuard( slock): 
            top = self.Pop()
            if ( top):
                return top
            _ = self._Size.Incr( 1)
            return UnsafePointer[ T]()

    @always_inline
    fn Push( mut self, x: T) -> UInt32: 
        nwSz = self._Size.Incr( 1)
        self._Arr.PtrAt(  nwSz -1)[] = x
        return nwSz

    #-----------------------------------------------------------------------------------------------------------------------------
 
    @always_inline
    fn Import( mut self, mut  stk : Stk[ T, _, _], maxMov: UInt32 = UInt32.MAX)   -> UInt32:              
        szCacheVoid = self.SzVoid()                                                                                
        szAlloc =  szCacheVoid if szCacheVoid < stk.Size() else stk.Size()
        if szAlloc > maxMov:
            szAlloc = maxMov 
        for i in uSeg( szAlloc):
            self._Arr.PtrAt( self.Size() +i)[] = stk._Arr.PtrAt( stk.Size() -szAlloc +i)[]
            
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

    x = stk.Pop() 
    print( x[])

    arr.Print()
    for i in uSeg( 2):
        x = stk.Pop() 
        print( x[]) 
    
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
