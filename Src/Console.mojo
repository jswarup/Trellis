
from Stash import * 
from Strand import *

#----------------------------------------------------------------------------------------------------------------------------------

def main():  
    var buff =  Buff[ UInt]( 20, 0) 
    var arr =  buff.Arr()
    var stk = Stk( arr, 0)
    for i in arr.USeg(): 
        _ = stk.Push( UInt( i)) 

    stk = Stk( arr, 0)    
    for x in arr: 
        _ = stk.Push( 20 -x) 
    for i in arr.USeg(): 
        print( arr[ i])
    
    for i in stk.USeg(): 
        print( stk.Pop())
    useg = USeg( 20)
    print( useg) 
    var     atm = Atm( UInt( 10))     
    print( atm.Get()) 
    pass

