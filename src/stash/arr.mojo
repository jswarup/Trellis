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
    fn __len__(self) -> UInt32:
        return len(self.src) - self.index


#----------------------------------------------------------------------------------------------------------------------------------

 
struct Arr[
    is_mutable: Bool, //,
    T: CollectionElement,
    origin: Origin[is_mutable].type,
](CollectionElementNew):

    var     _DArr: UnsafePointer[T]
    var     _Len: UInt32

    @always_inline
    fn __init__(inout self, ptr: UnsafePointer[T], length: UInt32):
        self._DArr = ptr
        self._Len = length
        #print( "init  Arr:", self.unsafe_ptr())

    @always_inline
    fn __init__(inout self, other: Self):
        self._DArr = other._DArr
        self._Len = other._Len
        #print( "init  Arr:", self.unsafe_ptr())

    @always_inline
    fn __init__(inout self, ref [origin]list: Arr[T, *_]):
        self._DArr = list._DArr
        self._Len = len(list)
        #print( "init Arr:", self.unsafe_ptr(), list.unsafe_ptr())

    fn __del__(owned self):        
        #print( "delete Arr:", self.unsafe_ptr())
        pass

    @always_inline
    fn Size(self) -> UInt32: 
        return self._Len

    fn SwapAt(inout self, i: UInt32, j: UInt32):
        if i != j:
            swap((self._DArr + i)[], (self._DArr + j)[])

    @always_inline
    fn __getitem__(self, idx: UInt32) -> ref [origin] T: 
        debug_assert( idx < self._Len)
        return self._DArr[idx] 

    @always_inline
    fn __iter__(self) -> _ArrIter[T, origin]: 
        return _ArrIter( self)
 
    @always_inline
    fn __len__(self) -> Int: 
        return int( self._Len)

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
        fn less( p: UInt32, q: UInt32) -> Bool:
            res = Less( self._DArr[ p], self._DArr[ q])  
            return res 
        
        @parameter
        fn swap( p: UInt32, q: UInt32) -> None: 
            self.SwapAt( p, q)
        
        uSeg = USeg( 0, self.Size())
        uSeg.QSort[ less, swap]()
        
    fn Print[ T: StringableCollectionElement] (  self : Arr[ T, origin] ) -> None: 
        print( "[ ", self.Size(), end =": ") 
        for iter in self:
            print( str( iter[]), end =" ") 
        print("] ") 

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct FArr[T: CollectionElement](
    CollectionElement
): 
    var _DPtr: UnsafePointer[T] 
    var _Size: UInt32
     
    fn __init__(inout self): 
        self._DPtr = UnsafePointer[T]()
        self._Size = 0 
        #print( "init FArr:", self._Size)
    
    fn __init__( inout self, _Size: UInt32, value: T):   
        self._Size = _Size
        self._DPtr = UnsafePointer[T].alloc( int(_Size))
        for i in range( 0, _Size):
            (self._DPtr + i).init_pointee_copy(value)
        #print( "init FArr:", self._Size)

    fn __del__(owned self):
        for i in range(self._Size):
            (self._DPtr + i).destroy_pointee()
        self._DPtr.free()
        #print( "delete FArr:", self._DPtr)
     
    fn Arr(ref [_] self) -> Arr[T, __origin_of(self)]: 
        return Arr[T, __origin_of(self)]( self._DPtr, self._Size)
 
    fn __len__(self) -> UInt32: 
        return self._Size

    
    fn Resize(inout self, nwSz: UInt32, value: T):
        var     dest = UnsafePointer[T].alloc( nwSz)
        sz = min( self._Size, nwSz)
        for i in range( sz):
            (self._DPtr + i).move_pointee_into(dest + i)

        if ( nwSz < self._Size)
            for i in range(self._Size):
                (self._DPtr + i).destroy_pointee()
        
        _move_pointee_into_many_elements[hint_trivial_type](
            dest=new_data,
            src=self._DPtr,
            _Size=self._Size,
        )

        if self._DPtr:
            self._DPtr.free()
        self._DPtr = new_data
        self.capacity = new_capacity
   
#----------------------------------------------------------------------------------------------------------------------------------

fn ArrExample():   
    vec  = FArr[ UInt32]( 7, 0) 
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
    arr.Print()
    
    @parameter
    fn less(lhs: Float32, rhs: Float32) -> Bool:  
        return lhs < rhs

    arr.DoQSort[ less]()
    arr.Print()
    
#----------------------------------------------------------------------------------------------------------------------------------

fn main():  
    ArrSortExample()

#----------------------------------------------------------------------------------------------------------------------------------
