# crew.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy
from algorithm import parallelize, vectorize
from stash import Buff, Arr
import heist

#----------------------------------------------------------------------------------------------------------------------------------

struct Crew:
    var     _Abettors: Buff[ Abettor]
    var     _Atelier: Atelier

    @always_inline
    fn __init__( out self, mxQueue : UInt32 ) : 
        self._Abettors = Buff[ Abettor]( mxQueue)
        self._Atelier = Atelier()
        var ind : UInt32 = 0
        for g in self._Abettors.Arr():
            g[].SetCrew( ind, self)
            ind += 1
        pass
        
    fn Abettors( self) -> Arr[ Abettor, MutableAnyOrigin]:
        return self._Abettors.Arr_()
    
    fn DoLaunch( self) -> Bool:
        abettors = self.Abettors()
        @parameter
        fn worker( ind: Int):
            abettors[ ind].ExecuteLoop()
            pass

        parallelize[ worker]( abettors.__len__(), abettors.__len__())
        return True
     
    fn  Size( self) -> UInt32:
        return self._Abettors.Size()
    
    
  

#----------------------------------------------------------------------------------------------------------------------------------

fn CrewExample() :
    crew = Crew( 4) 
    x = 10
    fn closure() -> Bool:
        print( x)
        return True  
    abettor = crew.Abettors().PtrAt( 0)
    #abettor.Allocate();
    #abettor[].EnqueueJob( closure)
    _ = crew.DoLaunch()
    
    return 

#----------------------------------------------------------------------------------------------------------------------------------
