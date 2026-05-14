# Console.mojo ------------------------------------------------------------------------------------------------------------------------

from Silo import *
from Strand import *

#----------------------------------------------------------------------------------------------------------------------------------


def main():
    TestSilo()
    TestStrand()

    pass


#----------------------------------------------------------------------------------------------------------------------------------


def main1():
    var buff = Buff[ UInt32]( 20, UInt32( 0))
    var arr = buff.Arr()
    var stk = Stk( arr, 0)
    for i in arr.USeg():
        _ = stk.Push( UInt32( i))

    stk = Stk( arr, 0)
    for x in arr:
        _ = stk.Push( 20 - x[])
    print( arr)

    for _ in stk.USeg():
        print( stk.Pop())
    useg = USeg( 20)
    print( useg)
    var atm = Atm( UInt32( 10))
    print( atm.Get())
    pass
