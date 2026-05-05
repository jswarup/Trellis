
from Stash import * 
from Strand import *

#----------------------------------------------------------------------------------------------------------------------------------

def main():  
    var buff =  Buff[ UInt]( 20, 0) 
    var arr =  buff.Arr()
    for i in USeg( 20): 
        print( arr[ i])
    useg = USeg( 20)
    print( useg) 
    var     atm = Atm[ DType.uint32]( 10)     
    print( atm.Get()) 
    pass

