# Arr.mojo -----------------------------------------------------------------------------------------------------------------------
   
from Stash import *

#----------------------------------------------------------------------------------------------------------------------------------

struct Arr [ T: ImplicitlyCopyable, Iterable, Iterator, TrivialRegisterPassable): 
    var     _DPtr:  UnsafePointer[ Self.T, MutExternalOrigin]
    var     _Size: UInt32

    def __init__(out self):  
        self._Size = 0
        self._DPtr = alloc[Self.T]( 0)
        pass
   

