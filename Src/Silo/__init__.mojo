
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
    var     a0 = b0.Arr()
    var     stk0 = Stk( a0, 0)

    for x in USeg( b0.Size()): 
        _ = stk0.Push( b0.Size() -x)   

    var     b1 = Buff[ UInt32]( 12) 
    var     a1 = b1.Arr()
    var     stk1 = Stk( a1, 0)

    print( stk0.Arr(), stk1.Arr())
    _ = stk1.Import( stk0, 4)
    print( stk0.Arr(), stk1.Arr())
    _ = stk0.Export( stk1, 5)
    print( stk0.Arr(), stk1.Arr())
    _ = stk0.Import( stk1, 6)
    print( stk0.Arr(), stk1.Arr())
    _ = stk1.Import( stk0, 7)
    print( stk0.Arr(), stk1.Arr())
    _ = stk0.Export( stk1, 8)
    print( stk0.Arr(), stk1.Arr())
    _ = stk1.Export( stk0, 20)
    print( stk0.Arr(), stk1.Arr())
    #print( stk0.Arr(), stk1.Arr()) 


def SiloTest():
    #BuffTest() 
    StkTest()
    
