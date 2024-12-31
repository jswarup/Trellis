# buff.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from strand import Atm
import stash

trait BuffElement( Defaultable, CollectionElement):
    pass

#----------------------------------------------------------------------------------------------------------------------------------
 
struct Buff[T: CollectionElement]( CollectionElement): 
    
    var     _DPtr: UnsafePointer[T] 
    var     _Size: UInt32
     
    #-----------------------------------------------------------------------------------------------------------------------------
    
    fn __init__( out self): 
        self._DPtr = UnsafePointer[T]()
        self._Size = UInt32( 0) 
        
    fn __init__( out self, sz: UInt32, value: T):   
        self._Size = sz
        self._DPtr = UnsafePointer[ T].alloc( int( sz))
        for i in uSeg( 0, sz):
            (self._DPtr + i).init_pointee_copy( value) 

    @always_inline
    fn __init__( out self, other: Self):
        self._DPtr = UnsafePointer[ T].alloc( int( other.Size())) 
        self._Size = other.Size()
        for i in uSeg( self.Size()):
             (self._DPtr + i).init_pointee_copy( (other._DPtr + i)[])

    @always_inline
    fn __moveinit__( out self, owned existing: Self, /):
        self._DPtr = existing._DPtr
        self._Size = existing.Size()
        existing._DPtr = UnsafePointer[T]()
        existing._Size = UInt32( 0)

    @always_inline
    fn __copyinit__( out self, existing: Self, /):
        self._DPtr = UnsafePointer[ T].alloc( int( existing.Size())) 
        self._Size = existing.Size()
        for i in uSeg( self.Size()):
             (self._DPtr + i).init_pointee_copy( (existing._DPtr + i)[])

    fn __del__( owned self): 
        print( "Buff: Del ", self._DPtr)
        for i in uSeg( self.Size()):
            (self._DPtr + i).destroy_pointee()
        self._DPtr.free() 

    #-----------------------------------------------------------------------------------------------------------------------------

    fn __len__( self) -> Int: 
        return int( self.Size())

    #-----------------------------------------------------------------------------------------------------------------------------
    
    @always_inline
    fn Size(  self) -> UInt32: 
        return self._Size

    fn DataPtr( self) -> UnsafePointer[ T]: 
        return self._DPtr

    @always_inline 
    fn PtrAt[ type: DType]( ref [_] self, k: Scalar[ type]) -> UnsafePointer[ T]:
        return UnsafePointer[ T].address_of(self._DPtr[ k])

    fn Arr( self) -> Arr[ T, __origin_of( self._DPtr)]: 
        return Arr[ T, __origin_of( self._DPtr)]( self._DPtr, self.Size())
 
    fn Arr_( self) -> Arr[ T, MutableAnyOrigin]: 
        return Arr[ T, MutableAnyOrigin]( self._DPtr, self.Size())
 
    fn Resize( mut self, nwSz: UInt32, value: T):
        olDPtr = self._DPtr
        olSz = self._Size
        self._DPtr = UnsafePointer[ T].alloc( int( nwSz)) 
        self._Size = nwSz
        sz = min( olSz, nwSz)
        for i in uSeg( sz):
            (olDPtr + i).move_pointee_into( self._DPtr + i)

        if ( sz < olSz):
            for i in uSeg( sz, olSz -sz):
                (olDPtr + i).destroy_pointee()
        
        if ( sz < nwSz):
            for i in uSeg( sz, nwSz -sz):
                (self._DPtr + i).init_pointee_copy( value)
        if olDPtr:
            olDPtr.free()

#----------------------------------------------------------------------------------------------------------------------------------
