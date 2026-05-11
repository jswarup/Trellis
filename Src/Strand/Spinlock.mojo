# Spinlock.mojo ------------------------------------------------------------------------------------------------------------------------

from Strand import Atm 

#----------------------------------------------------------------------------------------------------------------------------------
 
struct Spinlock :  
    comptime   AtmFlag = Atm[ DType.int64]
    
    var     _Flag: Self.AtmFlag  

    def __init__( out self):
        self._Flag = Self.AtmFlag( 0)

    @always_inline
    def Lock( mut self):   
        var     expected  = Int64( 0)
        while not self._Flag.CompareExchange( expected, 1): 
            pass
        return

    @always_inline
    def Unlock(mut self): 
        self._Flag.Set( 0)
        return

#----------------------------------------------------------------------------------------------------------------------------------

struct Lockguard[ origin: MutOrigin]:
    
    comptime _SLockPtr =  Pointer[ Spinlock, Self.origin]

    var     lock : Self._SLockPtr

    def __init__( out self, ref [ Self.origin ] lock: Spinlock) :
        self.lock = Self._SLockPtr( to= lock)

    @no_inline
    def __enter__( self):
        self.lock[].Lock()

    @no_inline
    def __exit__( self):
        self.lock[].Unlock()

#----------------------------------------------------------------------------------------------------------------------------------
 

