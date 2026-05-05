# Atm.mojo -----------------------------------------------------------------------------------------------------------------------

from std.atomic import Atomic

#----------------------------------------------------------------------------------------------------------------------------------
 

#----------------------------------------------------------------------------------------------------------------------------------
 
struct Atm[ dtype: DType]:  
    var     _Data: Atomic[ Self.dtype]  

    def __init__(out self, value: Scalar[Self.dtype]):
        self._Data = Atomic[ Self.dtype]( value)
     
    
    @always_inline
    def Get( self) -> Scalar[ Self.dtype]:
        return self._Data.load()   

    @always_inline
    def  Set( mut self, val: Scalar[Self.dtype]) -> None:
        expected = self.Get()
        while not self.CompareExchange( expected, val):  
        
    @always_inline
    def  CompareExchange( mut self, mut  expected: Scalar[Self.dtype], desired: Scalar[Self.dtype] ) -> Bool:
        res =  self._Data.compare_exchange( expected, desired) 
        return res

    @always_inline
    def  Incr( mut self, rhs: Scalar[Self.dtype]) -> Scalar[Self.dtype]: 
        ret = self._Data.fetch_add( rhs) 
        ret += rhs 
        return ret
 
        