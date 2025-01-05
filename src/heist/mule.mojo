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
        pass

    fn __bool__(self) -> Bool:
        return self._Runner.__len__()

    fn __lshift__( owned self, owned other : Mule) -> Mule:      
        if self._SuccMule:
            _ = self._SuccMule[].__lshift__( other)
            pass
        else:
            self._SuccMule = UnsafePointer[ Mule].address_of( other)
            pass
        return self

    fn __or__( self, other : Mule) -> Mule:      
        return self

    fn write_to[W: Writer](self, mut writer: W):
        writer.write( self._Runner)
        writer.write( "->")
        if self._SuccMule:
            self._SuccMule[].write_to( writer)

#----------------------------------------------------------------------------------------------------------------------------------
 
fn MuleExample():
    a = Mule( "a")
    a  =  a << Mule( "b") 
    print( String.write( a))
    pass


#----------------------------------------------------------------------------------------------------------------------------------

 

#----------------------------------------------------------------------------------------------------------------------------------
 