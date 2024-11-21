# arr.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
import stash

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Arr[
    is_mutable: Bool, //,
    T: CollectionElement,
    origin: Origin[is_mutable].type,
]( CollectionElementNew):

    var     _DArr: UnsafePointer[ T]
    var     _Size: UInt32

    @always_inline
    fn __init__( inout self, ptr: UnsafePointer[ T], length: UInt32):
        self._DArr = ptr
        self._Size = length 

    @always_inline
    fn __init__( inout self, other: Self):
        self._DArr = other._DArr
        self._Size = other._Size 

    @always_inline
    fn __init__( inout self, ref [ origin] list: Arr[ T, *_]):
        self._DArr = list._DArr
        self._Size = len(list) 

    @always_inline
    fn __del__( owned self):         
        pass

    @always_inline
    fn __getitem__( self, idx: UInt32) -> ref [origin] T:  
        return self._DArr[idx] 

    @always_inline
    fn __iter__( self) -> Arr[T, origin]: 
        return Arr[T, origin]( self._DArr, self._Size)  
 
    @always_inline
    fn __has_next__(self) -> Bool:
        return self.__len__() > 0

    @always_inline
    fn __next__(
        inout self,
    ) -> Pointer[T, origin]: 
        ptr = Pointer[T, origin].address_of(self._DArr[0])
        self._DArr += 1
        self._Size -= 1
        return ptr

    @always_inline
    fn __len__(self) -> Int: 
        return int( self._Size)

    @always_inline
    fn PtrAt[ origin: MutableOrigin, // ]( self: Arr[ T, origin], k: UInt32) -> Pointer[T, origin]:
        return Pointer[T, origin].address_of(self._DArr[ k])

    @always_inline
    fn unsafe_ptr(self) -> UnsafePointer[T]:
        return self._DArr

    @always_inline
    fn as_ref(self) -> Pointer[T, origin]:
        return Pointer[T, origin].address_of(self._DArr[0])

    @always_inline
    fn __bool__(self) -> Bool:
        return len(self) > 0

    @always_inline
    fn copy_from[
        origin: MutableOrigin, //
    ](self: Arr[T, origin], other: Arr[T, _]): 
        for i in uSeg( len(self)):
            self[i] = other[i]

    @always_inline
    fn __copyinit__( inout self, existing: Self, /):
        self._DArr = existing._DArr
        self._Size =  existing._Size 

    @always_inline
    fn __moveinit__( inout self, owned existing: Self, /):
        self._DArr = existing._DArr
        self._Size =  existing._Size 
        existing._DArr = UnsafePointer[T]()
        existing._Size = 0;
        
    @always_inline
    fn fill[ origin: MutableOrigin, //]( self: Arr[T, origin], value: T):
        for element in self:
            element[] = value
    
    fn Arr(ref [_] self) -> Arr[ T, __origin_of( self)]: 
        return Arr[T, __origin_of( self)]( self._DArr, self._Size)

    @always_inline
    fn Size( self) -> UInt32: 
        return self._Size

    @always_inline
    fn SwapAt( self, i: UInt32, j: UInt32):
        if i != j:
            swap( self._DArr[ i], self._DArr[ j])
            
    fn Subset[ origin: MutableOrigin, //]( ref [_] self: Arr[T, origin], useg: USeg) -> Arr[T, origin]:
        return Arr[ T, origin]( self._DArr + useg.First(),  useg.Size())

    fn DoQSort[ Less: fn( r: T, s: T) capturing -> Bool]( self)-> None: 
        @parameter
        fn less( p: UInt32, q: UInt32) -> Bool:
            return Less( self._DArr[ p], self._DArr[ q])    
        @parameter
        fn swap( p: UInt32, q: UInt32) -> None: 
            self.SwapAt( p, q) 
        USeg( 0, self.Size()).QSort[ less, swap]()
        
     
    fn  BinarySearch[ Lower: Bool, Less: fn( r: T, s: T) capturing -> Bool]( self, target : T,  start : UInt32 = 0)-> UInt32:
        @parameter
        fn less( p: UInt32) -> Bool:
            if ( Lower):
                return Less( self._DArr[ p], target)  
            return not Less( target, self._DArr[ p]);  
        return uSeg( start, self._Size - start).BinarySearch[ less]() 

    fn  PlayEquivalence[ Less: fn( r: T, s: T) capturing -> Bool, Play: fn( useg: USeg) capturing -> Bool, ]( self) ->None:
        var lo: UInt32 = 0
        while ( lo < self.Size()) :
            var hi: UInt32 = self.BinarySearch[ False, Less]( self._DArr[ lo], lo)
            res = Play( uSeg( lo, hi -lo))
            if not res:
                return
            lo = hi;
        return 

    fn Print[ T: StringableCollectionElement] (  self: Arr[ T, origin], endStr: StringLiteral = "\n" ) -> None: 
        print( "[ ", self.Size(), end =": ") 
        for iter in self:
            print( str( iter[]), end =" ") 
        print("] ", end=endStr) 

   
#----------------------------------------------------------------------------------------------------------------------------------

fn ArrExample():   
    vec  = Buff[ UInt32]( 7, 0) 
    arr = vec.Arr(); 
    i = 0
    for iter in arr:
        i += 1
        iter[] = i
    arr.SwapAt( 3, 5) 
    for iter in arr:
        print( iter[]) 

#----------------------------------------------------------------------------------------------------------------------------------

import random

fn ArrSortExample():   
    vec  = Buff[ Float32]( 80, 0) 
    arr = vec.Arr()  
    for iter in arr: 
        iter[] = int( random.random_ui64( 13, 113))
    arr.SwapAt( 3, 5)  
    arr.Print()
    vec.Resize( 100, 30)
    arr = vec.Arr() 
    @parameter
    fn less(lhs: Float32, rhs: Float32) -> Bool:  
        return lhs < rhs

    arr.DoQSort[ less]()
    arr.Print()
    
    res = arr.BinarySearch[ False, less]( 89)

    @parameter
    fn play( useg : USeg) -> Bool:
        arr.Subset( useg).Print()
        return True

    arr.PlayEquivalence[ less, play]()
    print( res)
    
#----------------------------------------------------------------------------------------------------------------------------------

fn main():  
    ArrSortExample()

#----------------------------------------------------------------------------------------------------------------------------------
