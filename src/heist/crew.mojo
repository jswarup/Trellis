# crew.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import Pointer, UnsafePointer, memcpy
from algorithm import parallelize, vectorize
from stash import Buff
import heist

#----------------------------------------------------------------------------------------------------------------------------------

struct Crew:
    var     _Grifters: Buff[ Grifter]
    var     _Caper: Caper

    @always_inline
    fn __init__( out self, mxQueue : UInt32 ) : 
        self._Grifters = Buff[ Grifter]( mxQueue)
        self._Caper = Caper()
        var ind : UInt32 = 0
        for g in self._Grifters.Arr():
            g[].SetCrew( ind, self)
            ind += 1
        pass

    
    fn DoLaunch( self) -> Bool:
        @parameter
        fn worker( ind: Int):
            arr = self._Grifters.Arr()
            arr[ ind].ExecuteLoop()
            pass

        parallelize[ worker]( self._Grifters.__len__(), self._Grifters.__len__())
        return True
     
    fn  Size( self) -> UInt32:
        return self._Grifters.Size()

#----------------------------------------------------------------------------------------------------------------------------------

fn CrewExample() :
    crew = Crew( 4)   
    _ = crew.DoLaunch()
    return 

#----------------------------------------------------------------------------------------------------------------------------------
