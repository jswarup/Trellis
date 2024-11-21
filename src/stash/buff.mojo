# buff.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
import stash

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct Buff[T: CollectionElement](
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
