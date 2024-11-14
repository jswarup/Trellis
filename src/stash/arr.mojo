# arr.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct _ArrIter[
    is_mutable: Bool, //,
    T: CollectionElement,
    origin: Origin[is_mutable].type, 
]: 
    var     index: Int
    var     src: Arr[T, origin]

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
struct Arr[
    is_mutable: Bool, //,
    T: CollectionElement,
    origin: Origin[is_mutable].type,
](CollectionElementNew):

    var _data: UnsafePointer[T]
    var _len: Int

    @always_inline
    fn __init__(inout self, *, ptr: UnsafePointer[T], length: Int):
        self._data = ptr
        self._len = length

    @always_inline
    fn __init__(inout self, *, other: Self):
        self._data = other._data
        self._len = other._len

    @always_inline
    fn __init__(inout self, ref [origin]list: List[T, *_]):
        self._data = list.data
        self._len = len(list)

    fn SwapAt(inout self, i: Int, j: Int):
        if i != j:
            swap((self._data + i)[], (self._data + j)[])

    @always_inline
    fn __getitem__(self, idx: Int) -> ref [origin] T:
        var     offset = idx
        if offset < 0:
            offset += len(self)
        return self._data[offset] 

    @always_inline
    fn __iter__(self) -> _ArrIter[T, origin]: 
        return _ArrIter(0, self)
 
    @always_inline
    fn __len__(self) -> Int: 
        return self._len

    fn unsafe_ptr(self) -> UnsafePointer[T]:
        return self._data

    fn as_ref(self) -> Pointer[T, origin]:
        return Pointer[T, origin].address_of(self._data[0])

    @always_inline
    fn copy_from[
        origin: MutableOrigin, //
    ](self: Arr[T, origin], other: Arr[T, _]):
        debug_assert(len(self) == len(other), "Arrs must be of equal length")
        for i in range(len(self)):
            self[i] = other[i]

    fn __bool__(self) -> Bool:
        return len(self) > 0

    fn fill[origin: MutableOrigin, //](self: Arr[T, origin], value: T):
        for element in self:
            element[] = value
 
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
    
    fn __init__( inout self, size: Int, value: T):   
        self.size = size
        self.data = UnsafePointer[T].alloc( int(size))
        for i in range( 0, size):
            (self.data + i).init_pointee_copy(value)

    fn __del__(owned self):
        for i in range(self.size):
            (self.data + i).destroy_pointee()
        self.data.free()
     
    fn Arr(ref [_]self) -> Arr[T, __origin_of(self)]: 
        return Arr[T, __origin_of(self)](
            ptr=self.data, length=self.size
        )
 
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

fn main():  
    ArrExample()

#----------------------------------------------------------------------------------------------------------------------------------
