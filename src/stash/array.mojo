
from memory import Pointer, UnsafePointer, memcpy
from stash import USeg


@value
struct IArr[
    arr_mutability: Bool, //,
    T: CollectionElement,
    arr_origin: Origin[arr_mutability].type 
]:  
 
    var src: Pointer[ FArr[T], arr_origin]


    fn __init__(inout self, src: Pointer[ FArr[T], arr_origin]):  
        self.src = src 

    fn __iter__(self) -> Self:
        return self

    fn __next__(
        inout self,
    ) -> Pointer[T, arr_origin]:  
        return Pointer.address_of(self.src[][self.index - 1]) 
  
@value
struct _ArrIter[
    arr_mutability: Bool, //,
    T: CollectionElement,
    arr_origin: Origin[arr_mutability].type 
]:  

    var index: Int
    var src: Pointer[ FArr[T], arr_origin]


    fn __init__(inout self, index: Int, src: Pointer[ FArr[T], arr_origin]): 
        self.index = index
        self.src = src 

    fn __iter__(self) -> Self:
        return self

    fn __next__(
        inout self,
    ) -> Pointer[T, arr_origin]: 
        self.index += 1
        return Pointer.address_of(self.src[][self.index - 1]) 
 
    fn __has_next__(self) -> Bool:
        return self.__len__() > 0

    fn __len__(self) -> Int: 
        return len(self.src[]) - self.index
        
@value
struct FArr[T: CollectionElement](
    CollectionElement
): 
    var data: UnsafePointer[T] 
    var size: Int
     
    fn __init__(inout self): 
        self.data = UnsafePointer[T]()
        self.size = 0 
 

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
    

fn ArrExample():   
    vec  = FArr[ Int]( 7, 0) 
    origin_type = __origin_of( vec)
    i = 0
    for iter in vec:
        i += 1
        iter[] = i
    vec.SwapAt( 3, 5)
    for iter in vec:
        print( iter[])