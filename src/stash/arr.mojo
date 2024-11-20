# arr.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
import stash

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct _ArrIter[
    is_mutable: Bool, //,
    T: CollectionElement,
    origin: Origin[is_mutable].type, 
]: 
    var     index: UInt32
    var     src: Arr[T, origin]

    fn __init__(inout self,  arr: Arr[T, origin]): 
        self.index = 0
        self.src = arr
        #print( "init Iter:")

    fn __del__(owned self):        
        #print( "delete Iter:")
        pass
        
    @always_inline 
    fn __iter__(self) -> Self:
        return self

    @always_inline
    fn __next__(
        inout self,
    ) -> Pointer[T, origin]:
        @parameter 
        self.index += 1
        return Pointer.address_of(self.src[self.index - 1]) 

    @always_inline
    fn __has_next__(self) -> Bool:
        return self.__len__() > 0

    @always_inline
    fn __len__(self) -> Int:
        return int( len(self.src) - self.index)

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
    fn __init__( inout self, ref [ origin]list: Arr[ T, *_]):
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
            
    fn Subset[ origin: MutableOrigin, //]( self: Arr[T, origin], useg: USeg) -> Arr[T, origin]:
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

@value
struct VArr[T: CollectionElement](
    CollectionElement
): 
    var _DPtr: UnsafePointer[T] 
    var _Size: UInt32
     
    fn __init__(inout self): 
        self._DPtr = UnsafePointer[T]()
        self._Size = 0  
    
    fn __init__( inout self, _Size: UInt32, value: T):   
        self._Size = _Size
        self._DPtr = UnsafePointer[ T].alloc( int( _Size))
        for i in uSeg( 0, _Size):
            (self._DPtr + i).init_pointee_copy( value) 

    fn __del__( owned self):
        for i in uSeg( self._Size):
            (self._DPtr + i).destroy_pointee()
        self._DPtr.free() 
     
    fn Arr(ref [_] self) -> Arr[ T, __origin_of( self)]: 
        return Arr[T, __origin_of( self)]( self._DPtr, self._Size)
 
    fn __len__( self) -> UInt32: 
        return self._Size

    fn Resize( inout self, nwSz: UInt32, value: T):
        var     dest = UnsafePointer[ T].alloc( int( nwSz))
        sz = min( self._Size, nwSz)
        for i in uSeg( sz):
            (self._DPtr + i).move_pointee_into( dest + i)

        if ( sz < self._Size):
            for i in uSeg( sz, self._Size -sz):
                (self._DPtr + i).destroy_pointee()
        
        if ( sz < nwSz):
            for i in uSeg( sz, nwSz -sz):
                (dest + i).init_pointee_copy( value)
        if self._DPtr:
            self._DPtr.free()
        self._DPtr = dest
        self._Size = nwSz
   
#----------------------------------------------------------------------------------------------------------------------------------

fn ArrExample():   
    vec  = VArr[ UInt32]( 7, 0) 
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
    vec  = VArr[ Float32]( 80, 0) 
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
