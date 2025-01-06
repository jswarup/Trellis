# mule.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
import heist

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct Mule ( CollectionElement): 
    var     _Runner : String
    var     _ParMule : UnsafePointer[ Mule]
    var     _SuccMule : UnsafePointer[ Mule]

    fn __init__( out self):
        self._Runner = String()
        self._ParMule = UnsafePointer[ Mule]()
        self._SuccMule = UnsafePointer[ Mule]()

    fn __init__( out self, runner :String):
        self._Runner = runner
        self._ParMule = UnsafePointer[ Mule]()
        self._SuccMule = UnsafePointer[ Mule]()

    @always_inline
    fn __del__( owned self):         
        print( "Mule: Del ", self._Runner)
        if self._ParMule:
            self._ParMule.free()
        if self._SuccMule:
            self._SuccMule.free()
        pass

    fn __bool__(self) -> Bool:
        return self._Runner.__len__()

    fn __rshift__( owned self, owned other : Mule) -> Mule:      
        if self._SuccMule:
            self._SuccMule[] = self._SuccMule[].__rshift__( other^) 
        else:
            self._SuccMule = UnsafePointer[ Mule].alloc( 1)
            self._SuccMule.init_pointee_move( other^) 
        return self

    fn __or__( owned self, owned other : Mule) -> Mule:      
        if self._ParMule:
            self._ParMule[] = self._ParMule[].__or__( other^) 
        else:
            self._ParMule = UnsafePointer[ Mule].alloc( 1)
            self._ParMule.init_pointee_move( other^) 
        return self

    fn write_to[W: Writer](self, mut writer: W):
        writer.write( "(")
        writer.write( self._Runner)
        if self._ParMule:
            writer.write( " | ")
            self._ParMule[].write_to( writer)
        writer.write( ")")
    
        writer.write( " -> ")
        if self._SuccMule:
            self._SuccMule[].write_to( writer)

#----------------------------------------------------------------------------------------------------------------------------------
 
fn MuleExample(): 
    a  =  Mule( "a") >> ( Mule( "b")  | Mule( "c") | Mule( "d")) >> Mule( "e")
    print( String.write( a))
    pass


#----------------------------------------------------------------------------------------------------------------------------------

 

#----------------------------------------------------------------------------------------------------------------------------------
 