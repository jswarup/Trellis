# crew.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from algorithm import parallelize, vectorize
from stash import Buff
import heist

#----------------------------------------------------------------------------------------------------------------------------------

struct Crew:
    var     _Abettors: Buff[ Abettor]
    var     _Caper: Caper

    @always_inline
    fn __init__( out self, mxQueue : UInt32 ) : 
        self._Abettors = Buff[ Abettor]( mxQueue)
        self._Caper = Caper()
        var ind : UInt32 = 0
        for g in self._Abettors.Arr():
            g[].SetCrew( ind, self)
            ind += 1
        pass

    
    fn DoLaunch( self) -> Bool:
        @parameter
        fn worker( ind: Int):
            arr = self._Abettors.Arr()
            arr[ ind].ExecuteLoop()
            pass

        parallelize[ worker]( self._Abettors.__len__(), self._Abettors.__len__())
        return True
     
    fn  Size( self) -> UInt32:
        return self._Abettors.Size()

#----------------------------------------------------------------------------------------------------------------------------------

fn CrewExample() :
    crew = Crew( 4)   
    _ = crew.DoLaunch()
    return 

#----------------------------------------------------------------------------------------------------------------------------------
