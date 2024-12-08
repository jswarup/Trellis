# grifter.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
import heist, stash

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct Grifter( CollectionElement):
    var     _Index: UInt32
    var     _Crew: UnsafePointer[ Crew]

    @always_inline
    fn __init__( out self) : 
        self._Crew = UnsafePointer[ Crew]()
        self._Index = UInt32.MAX
        pass

    fn SetCrew( inout self, ind : UInt32, crew: Crew):
        self._Index = ind
        self._Crew = UnsafePointer[ Crew].address_of( crew)
        pass

    fn ExecuteLoop( self) :
        print( self._Index, ": Done")
        pass
