# buff.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from strand import Atm
import stash

#----------------------------------------------------------------------------------------------------------------------------------
 
struct Buff[T: CollectionElement, is_atomic: Bool = False]( CollectionElement): 
    
    var     _DPtr: UnsafePointer[T] 
    var     _Size: Atm[ is_atomic, DType.uint32]
     
    #-----------------------------------------------------------------------------------------------------------------------------
    
    fn __init__( out self): 
        self._DPtr = UnsafePointer[T]()
        self._Size = UInt32( 0)
    
    fn __init__( out self, _Size: UInt32):   
        self._Size = _Size
        self._DPtr = UnsafePointer[ T].alloc( int( _Size))
        
    fn __init__( out self, _Size: UInt32, value: T):   
        self._Size = _Size
        self._DPtr = UnsafePointer[ T].alloc( int( _Size))
        for i in uSeg( 0, _Size):
            (self._DPtr + i).init_pointee_copy( value) 

    @always_inline
    fn __copyinit__( out self, existing: Self, /):
        self._DPtr = UnsafePointer[ T].alloc( int( existing._Size.Value())) 
        self._Size = existing._Size.Value()
        for i in uSeg( self._Size.Value()):
             (self._DPtr + i).init_pointee_copy( (existing._DPtr + i)[])

    @always_inline
    fn __moveinit__( out self, owned existing: Self, /):
        self._DPtr = existing._DPtr
        self._Size = existing._Size.Get()
        existing._DPtr = UnsafePointer[T]()
        existing._Size.Set( 0)

    fn __del__( owned self):
        print ( "vec: del")
        for i in uSeg( self._Size.Get()):
            (self._DPtr + i).destroy_pointee()
        self._DPtr.free() 

    #-----------------------------------------------------------------------------------------------------------------------------

    fn __len__( self) -> Int: 
        return int( self._Size.Value())

    #-----------------------------------------------------------------------------------------------------------------------------
    
    fn DataPtr( self) -> UnsafePointer[ T]: 
        return self._DPtr

    fn Arr( self) -> Arr[ T, __origin_of( self)]: 
        return Arr[ T, __origin_of( self)]( self._DPtr, self._Size.Value())
 
    fn Resize( inout self, nwSz: UInt32, value: T):
        var     dest = UnsafePointer[ T].alloc( int( nwSz))
        sz = min( self._Size.Value(), nwSz)
        for i in uSeg( sz):
            (self._DPtr + i).move_pointee_into( dest + i)

        if ( sz < self._Size.Value()):
            for i in uSeg( sz, self._Size.Value() -sz):
                (self._DPtr + i).destroy_pointee()
        
        if ( sz < nwSz):
            for i in uSeg( sz, nwSz -sz):
                (dest + i).init_pointee_copy( value)
        if self._DPtr:
            self._DPtr.free()
        self._DPtr = dest
        self._Size = nwSz

#----------------------------------------------------------------------------------------------------------------------------------
