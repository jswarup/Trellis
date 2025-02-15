# arr.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
import stash

#----------------------------------------------------------------------------------------------------------------------------------
  
struct Arr[ is_mutable: Bool, //, T: CollectionElement, origin: Origin[is_mutable]  = MutableAnyOrigin]( CollectionElement):

    var     _DArr: UnsafePointer[ T]
    var     _Size: UInt32

    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __init__( out self):
        self._DArr = UnsafePointer[T]()
        self._Size = 0;

    @always_inline
    fn __init__( out self,  ptr: UnsafePointer[ T], length: UInt32):
        self._DArr = ptr
        self._Size = length 

    @always_inline
    fn __init__( out self, other: Self):
        self._DArr = other._DArr
        self._Size = other._Size 

    @always_inline
    fn __copyinit__( out self, other: Self, /):
        self._DArr = other._DArr
        self._Size =  other._Size 

    @always_inline
    fn __moveinit__( out self, owned other: Self, /):
        self._DArr = other._DArr
        self._Size =  other._Size 
        other._DArr = UnsafePointer[T]()
        other._Size = 0;

    @always_inline
    fn __del__( owned self):         
        #print( "Arr: Del ")
        pass

    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __len__( self) -> Int: 
        return int( self._Size)

    @always_inline
    fn __getitem__( self, idx: UInt32) -> ref [__origin_of( self)] T:  
        return self._DArr[idx] 

    @always_inline
    fn __iter__( ref [_] self) -> Self:
        return self
 
    @always_inline
    fn __has_next__( self) -> Bool:
        return self.__len__() > 0

    @always_inline
    fn __next__(
        mut self,
    ) -> UnsafePointer[ T]: 
        ptr = UnsafePointer[ T].address_of(self._DArr[0])
        self._DArr += 1
        self._Size -= 1
        return ptr

    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn __bool__( self) -> Bool:
        return self.Size() > 0

    #-----------------------------------------------------------------------------------------------------------------------------

    @always_inline
    fn DoInit( mut self,  ptr: UnsafePointer[ T], length: UInt32) -> None:
        self._DArr = ptr
        self._Size = length 

    fn DoSetup[ Map : fn( ind: UInt32) capturing-> T ]( mut self) :
        for i in USeg( self.Size()):
            (self._DArr + i).init_pointee_copy( Map( i))  
    
    fn DoInitIndicize[ type: DType]( mut self : Arr[ Scalar[ type], _]) -> None:
        @parameter
        fn  index( ind: UInt32) -> SIMD[ type, size=1]:
            return ind.cast[ type]()
        self.DoSetup[ index]()

    @always_inline
    fn Size( self) -> UInt32: 
        return self._Size
 
    fn Subset( ref [_] self, useg: USeg) -> Arr[ T, __origin_of( self)]:
        return Arr[ T, __origin_of( self)]( self._DArr + useg.First(),  useg.Size())

    fn Advance( ref [_] self, k : UInt32) -> Arr[ T, __origin_of( self)]:
        return Arr[ T, __origin_of( self)]( self._DArr + k,  self._Size -k)
    
    fn Shorten( ref [_] self, k : UInt32) -> Arr[ T, __origin_of( self)]:
        return Arr[ T, __origin_of( self)]( self._DArr,  self._Size -k)
    
    @always_inline
    fn PtrAt( ref [_] self, k: UInt32) -> UnsafePointer[ T]:
        return UnsafePointer[ T].address_of(self._DArr[ k]) 
    
    @always_inline
    fn At( ref [_] self, k: UInt32) -> ref [self] T:
        return UnsafePointer[ T].address_of(self._DArr[ k])[]

    @always_inline
    fn First( ref [_] self) -> ref [self] T:
        return UnsafePointer[ T].address_of(self._DArr[ 0])[]

    @always_inline
    fn Last( ref [_] self) -> ref [self] T:
        return UnsafePointer[ T].address_of(self._DArr[ self._Size -1])[]

    @always_inline
    fn  Assign[ origin: MutableOrigin, // ]( mut self: Arr[T, origin], other: Arr[T, _]): 
        for i in USeg( self.Size()):
            self.PtrAt( i)[] = other[i] 

    @always_inline
    fn SwapAt( self, i: UInt32, j: UInt32): 
        swap( self._DArr[ i], self._DArr[ j])
           
    #-----------------------------------------------------------------------------------------------------------------------------
      
    fn DoValuate[ Valuate: fn( k: UInt32) capturing -> T] ( mut self : Arr[ T, MutableAnyOrigin]):   
        for i in USeg( self.Size() ):
            self.PtrAt( i)[] =  Valuate( i)

    fn DoAll(  self, callee : fn( t : T) escaping -> None):   
        for i in USeg( self.Size()):
            callee( self.At( i))
        return 

    @always_inline
    fn Fill[ origin: MutableOrigin, //]( self: Arr[T, origin], value: T):
        for element in self:
            element[] = value
    
    #-----------------------------------------------------------------------------------------------------------------------------
    
    fn DoQSort[ Less: fn( a: T, b: T) capturing -> Bool]( self)-> None: 
        @parameter
        fn less( p: UInt32, q: UInt32, self : Self) -> Bool:
            return Less( self._DArr[ p], self._DArr[ q])    
        @parameter
        fn swap( p: UInt32, q: UInt32, self : Self) -> None: 
            self.SwapAt( p, q) 
        USeg( 0, self.Size()).QSort[ less, swap]( self)
        
    #-----------------------------------------------------------------------------------------------------------------------------
     
    fn  BinarySearch[ Lower: Bool, Less: fn( r: T, s: T) capturing -> Bool]( self, target : T,  start : UInt32 = 0)-> UInt32:
        @parameter
        fn less( p: UInt32) -> Bool:
            if ( Lower):
                return Less( self._DArr[ p], target)  
            return not Less( target, self._DArr[ p]);  
        return USeg( start, self._Size - start).BinarySearch[ less]() 

    #-----------------------------------------------------------------------------------------------------------------------------
    
    fn  PlayEquivalence[ Less: fn( r: T, s: T) capturing -> Bool, Play: fn( useg: USeg) capturing -> Bool, ]( self) ->None:
        var lo: UInt32 = 0
        while ( lo < self.Size()) :
            var hi: UInt32 = self.BinarySearch[ False, Less]( self._DArr[ lo], lo)
            res = Play( USeg( lo, hi -lo))
            if not res:
                return
            lo = hi;
        return 

    #-----------------------------------------------------------------------------------------------------------------------------
    
    fn __str__[ T: StringableCollectionElement](  self: Arr[ T, _]) -> String:  
        str = String( "")
        for i in USeg( self.Size()):
            str += self._DArr[ i].__str__()
            str += " "
        return str

    #-----------------------------------------------------------------------------------------------------------------------------
    
    fn Print[ T: StringableCollectionElement] (  self: Arr[ T, _], endStr: StringLiteral = "\n" ) -> None: 
        print( "[ ", self.Size(), end =": ")  
        for i in USeg( self.Size()):
            print( str( self._DArr[ i]), end =" ") 
        print("] ", end=endStr) 

    
#----------------------------------------------------------------------------------------------------------------------------------

import random

fn ArrSortExample():   
    print( "ArrSortExample") 
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
       # arr.Subset( useg).Print()
        return True

    arr.PlayEquivalence[ less, play]()
    print( res)
    
#----------------------------------------------------------------------------------------------------------------------------------

fn main():  
    ArrSortExample()

#----------------------------------------------------------------------------------------------------------------------------------
