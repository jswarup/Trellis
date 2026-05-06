
from .USeg import *  
from .Arr import * 
from .Buff import * 
from .Stk import * 
 
def  BuffTest():
    print( "BuffTest:")
    var     b = Buff[ UInt32]( 4, 42)
    print( b.Arr())
    b.Resize( 6, 99)
    var     a = b.Arr()
    a.Reverse()
    print( b.Arr())
    b.Resize( 5, 0)
    print( b.Arr())
    

def  StkTest():
    print( "StkTest:")
    var     b0 = Buff[ UInt32]( 22) 
    var     stk0 = Stk( b0.Arr(), 0)
      
    for x in USeg( b0.Size()): 
        _ = stk0.Push( b0.Size() -x)  
    print( b0.Arr())

    var     b1 = Buff[ UInt32]( 12) 
    var     stk1 = Stk( b1.Arr(), 0)
    _ = stk1.Import( stk0, 2)
    print( stk0.Arr(), stk1.Arr()) 
    #_ = stk1.Export( stk0, 2)
    print( stk0.Arr(), stk1.Arr()) 

def SiloTest():
    BuffTest() 
    StkTest()
    
