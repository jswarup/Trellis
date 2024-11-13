
from memory import Pointer, UnsafePointer, memcpy
from stash import USeg


@value
struct _ArrIter[
    list_mutability: Bool, //,
    T: CollectionElement,
    list_origin: Origin[list_mutability].type,
    forward: Bool = True,
]: 
    alias list_type = FArr[T]

    var index: Int
    var src: Pointer[Self.list_type, list_origin]

    fn __iter__(self) -> Self:
        return self

    fn __next__(
        inout self,
    ) -> Pointer[T, list_origin]:
        @parameter
        if forward:
            self.index += 1
            return Pointer.address_of(self.src[][self.index - 1])
        else:
            self.index -= 1
            return Pointer.address_of(self.src[][self.index])

    @always_inline
    fn __has_next__(self) -> Bool:
        return self.__len__() > 0

    fn __len__(self) -> Int:
        @parameter
        if forward:
            return len(self.src[]) - self.index
        else:
            return self.index
 
@value
struct FArr[T: CollectionElement](
    CollectionElement
): 
    var data: UnsafePointer[T] 
    var size: Int
     
    fn __init__(inout self): 
        self.data = UnsafePointer[T]()
        self.size = 0 
 
    fn __init__(inout self, data: UnsafePointer[T], size: Int): 
        self.data = data
        self.size = size 

    fn SwapAt(inout self, i: Int, j: Int):
        if i != j:
            swap((self.data + i)[], (self.data + j)[])
 
    fn __len__(self) -> Int: 
        return self.size
 
    fn __getitem__(ref [_]self, idx: Int) -> ref [self] T: 
        var normalized_idx = idx
 
        if normalized_idx < 0:
            normalized_idx += len(self)

        return (self.data + normalized_idx)[]
    
    fn __iter__(
        ref [_]self: Self,
    ) -> _ArrIter[T, __origin_of(self)]: 
        return _ArrIter(0, Pointer.address_of(self))
     
 

    fn __init__( inout self, size: Int, value: T):   
        self.size = size
        self.data = UnsafePointer[T].alloc( int(size))
        for i in range( 0, size):
            (self.data + i).init_pointee_copy(value)

    fn __del__(owned self):
        for i in range(self.size):
            (self.data + i).destroy_pointee()
        self.data.free()
     
    
fn main():   
    vec  = FArr[ Int]( 54, 0) 
    origin_type = __origin_of( vec)
    for i in vec:
        i[] = 20
    for i in vec:
        print( i[])
        
    

    
    
