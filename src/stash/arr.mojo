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
    var     index: Int
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
        return len(self.src) - self.index

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct ArrW[type: CollectionElement](CollectionElement):
    var data: type

    fn __init__(inout self, *, other: Self):
        self.data = other.data

#----------------------------------------------------------------------------------------------------------------------------------

 
struct Arr[
    is_mutable: Bool, //,
    T: CollectionElement,
    origin: Origin[is_mutable].type,
](CollectionElementNew):

    var     _DArr: UnsafePointer[T]
    var     _Len: Int

    @always_inline
    fn __init__(inout self, ptr: UnsafePointer[T], length: Int):
        self._DArr = ptr
        self._Len = length
        #print( "init  Arr:", self.unsafe_ptr())

    @always_inline
    fn __init__(inout self, other: Self):
        self._DArr = other._DArr
        self._Len = other._Len
        #print( "init  Arr:", self.unsafe_ptr())

    @always_inline
    fn __init__(inout self, ref [origin]list: List[T, *_]):
        self._DArr = list.data
        self._Len = len(list)
        #print( "init Arr:", self.unsafe_ptr(), list.unsafe_ptr())

    fn __del__(owned self):        
        #print( "delete Arr:", self.unsafe_ptr())
        pass

    @always_inline
    fn Size(self) -> Int: 
        return self._Len

    fn SwapAt(inout self, i: Int, j: Int):
        if i != j:
            swap((self._DArr + i)[], (self._DArr + j)[])

    @always_inline
    fn __getitem__(self, idx: Int) -> ref [origin] T: 
        debug_assert( idx < self._Len)
        return self._DArr[idx] 

    @always_inline
    fn __iter__(self) -> _ArrIter[T, origin]: 
        return _ArrIter( self)
 
    @always_inline
    fn __len__(self) -> Int: 
        return self._Len

    fn unsafe_ptr(self) -> UnsafePointer[T]:
        return self._DArr

    fn as_ref(self) -> Pointer[T, origin]:
        return Pointer[T, origin].address_of(self._DArr[0])

    fn __bool__(self) -> Bool:
        return len(self) > 0

    @always_inline
    fn copy_from[
        origin: MutableOrigin, //
    ](self: Arr[T, origin], other: Arr[T, _]):
        debug_assert(len(self) == len(other), "Arrs must be of equal length")
        for i in range(len(self)):
            self[i] = other[i]


    fn __copyinit__(inout self, existing: Self, /):
        self._DArr = existing._DArr
        self._Len =  existing._Len
        #print( "copyinit  Arr:", self.unsafe_ptr(), existing.unsafe_ptr())

    fn __moveinit__(inout self, owned existing: Self, /):
        self._DArr = existing._DArr
        self._Len =  existing._Len
        #print( "movinit  Arr:", self.unsafe_ptr(), existing.unsafe_ptr())


    fn fill[origin: MutableOrigin, //](self: Arr[T, origin], value: T):
        for element in self:
            element[] = value
  
    fn DoQSort[  Less: fn( r: T, s: T) capturing -> Bool]( inout self)-> None: 
        @parameter
        fn less( p: Int, q: Int) -> Bool:
            res = Less( self._DArr[ p], self._DArr[ q])  
            return res 
        
        @parameter
        fn swap( p: Int, q: Int) -> None: 
            self.SwapAt( p, q)
        
        uSeg = USeg( 0, self.Size())
        uSeg.QSort[ less, swap]()
        

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct FArr[T: CollectionElement](
    CollectionElement
): 
    var data: UnsafePointer[T] 
    var size: Int
     
    fn __init__(inout self): 
        self.data = UnsafePointer[T]()
        self.size = 0 
        #print( "init FArr:", self.size)
    
    fn __init__( inout self, size: Int, value: T):   
        self.size = size
        self.data = UnsafePointer[T].alloc( int(size))
        for i in range( 0, size):
            (self.data + i).init_pointee_copy(value)
        #print( "init FArr:", self.size)

    fn __del__(owned self):
        for i in range(self.size):
            (self.data + i).destroy_pointee()
        self.data.free()
        #print( "delete FArr:", self.data)
     
    fn Arr(ref [_] self) -> Arr[T, __origin_of(self)]: 
        return Arr[T, __origin_of(self)]( self.data, self.size)
 
    fn __len__(self) -> Int: 
        return self.size
   
#----------------------------------------------------------------------------------------------------------------------------------

fn ArrExample():   
    vec  = FArr[ Int]( 7, 0) 
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
    vec  = FArr[ Float32]( 100, 0) 
    arr = vec.Arr();  
    for iter in arr: 
        iter[] = int( random.random_ui64( 13, 113))
    arr.SwapAt( 3, 5)  
    #for iter in arr:
    #    print( iter[]) 
    @parameter
    fn less(lhs: Float32, rhs: Float32) -> Bool:  
        return lhs < rhs

    arr.DoQSort[ less]()
    for iter in arr:
        print( iter[]) 

#----------------------------------------------------------------------------------------------------------------------------------

fn main():  
    ArrSortExample()

#----------------------------------------------------------------------------------------------------------------------------------
