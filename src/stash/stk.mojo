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
    fn __init__( out self,  owned arr: Arr[ T, origin], size: UInt32 = 0):
        self._Arr = arr
        self._Size = size

    @always_inline
    fn __init__( out self, other: Self):
        self._Arr = other._Arr
        self._Size = other._Size

    @always_inline
    fn __moveinit__( out self, owned other: Self, /):
        self._Arr = other._Arr
        self._Size = other._Size
        other._Arr = Arr[ T, origin]()
        self._Size = UInt32( 0)

    @always_inline
    fn __copyinit__( out self, other: Self, /):
        self._Arr = other._Arr
        self._Size = other._Size

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
    fn Arr( self) -> Arr[ T, __origin_of( self)]: 
        #print( "stk: Arr")
        return Arr[ T, __origin_of( self)]( self._Arr._DArr, self._Size.Get() )
 
    @always_inline
    fn Top( mut self) -> T: 
        return self._Arr.__getitem__( self._Size.Get()  -1)
    
    @always_inline
    fn Pop( mut self)->   T:
        ind = self._Size.Incr( -1)
        if (  ind != UInt32.MAX):
            return self._Arr.At( ind) 
        return UnsafePointer[ T]()[]
  
    @always_inline
    fn Push( mut self, x: T) -> UInt32: 
        ind = self._Size.Incr( 1)
        self._Arr.PtrAt(  ind -1)[] = x
        return ind
    
    fn  Clip( mut self,  clipSz : UInt32) :  
        _ = self._Size.Incr( -clipSz)

    #-----------------------------------------------------------------------------------------------------------------------------
 
    @always_inline
    fn Import( mut self, mut  stk : Stk[ T, _, _], maxMov: UInt32 = UInt32.MAX)   -> UInt32:              
        szCacheVoid = self.SzVoid()                                                                                
        szAlloc =  szCacheVoid if szCacheVoid < stk.Size() else stk.Size()
        if szAlloc > maxMov:
            szAlloc = maxMov 
        for i in USeg( szAlloc):
            self._Arr.PtrAt( self.Size() +i)[] = stk._Arr.PtrAt( stk.Size() -szAlloc +i)[]
            
        _ = self._Size.Incr( szAlloc)
        _ = stk._Size.Incr( -szAlloc)
        return szAlloc

    #-----------------------------------------------------------------------------------------------------------------------------
    
    fn Print[ T: StringableCollectionElement](  self: Stk[ T, _, _], endStr: StringLiteral = "\n" ) -> None: 
        sz = self._Size.Value()
        print( "[ ", sz, end =": ")  
        for i in USeg( sz): 
            print( str( self._Arr.PtrAt( i)[]), end =" ") 
        print("] ", end=endStr) 

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
    print( x)

    arr.Print()
    for i in USeg( 2):
        x = stk.Pop() 
        print( x) 
    
    for i in USeg( 3):
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
