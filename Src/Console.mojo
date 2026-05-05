
from Stash import * 
from Strand import *

#----------------------------------------------------------------------------------------------------------------------------------

def main():  
    var buff =  Buff[ UInt]( 20, 0) 
    var arr =  buff.Arr()
    var stk = Stk( arr, 0)
    for i in USeg( 20): 
        _ = stk.Push( UInt( i)) 
        
    for i in USeg( 20): 
        print( arr[ i])
    useg = USeg( 20)
    print( useg) 
    var     atm = Atm( UInt( 10))     
    print( atm.Get()) 
    pass

