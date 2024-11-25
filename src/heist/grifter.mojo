# grifter.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
import heist, stash

#----------------------------------------------------------------------------------------------------------------------------------


struct Grifter [ is_mutable: Bool, //, T: CollectionElement, origin: Origin[is_mutable].type ]:
    @always_inline
    fn __init__( out self): 
        pass

