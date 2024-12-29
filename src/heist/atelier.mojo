# atelier.mojo ------------------------------------------------------------------------------------------------------------------------

from memory import UnsafePointer, memcpy 
from strand import Atm, SpinLock
from stash  import Arr, Buff, Stk, Silo
import heist

#----------------------------------------------------------------------------------------------------------------------------------
  

#----------------------------------------------------------------------------------------------------------------------------------

struct Atelier:

    fn __init__( out self) :
        pass
        

#----------------------------------------------------------------------------------------------------------------------------------

fn outer(b: Bool, x: String) -> fn() escaping -> String:
    fn closure() -> String:
        print(x) # 'x' is captured by calling String.__copyinit__
        return "Closure"

    fn bare_function()  -> String:
        print(x) # nothing is captured
        return "bare_function"

    if b:
        # closure can be safely returned because it owns its state
        return closure^

    # function pointers can be converted to runtime closures
    return bare_function

#----------------------------------------------------------------------------------------------------------------------------------

fn AtelierExample1():
    func1 = outer( False, "False")
    func2 = outer( True, "True")
    print( func1())
    print( func2())



#----------------------------------------------------------------------------------------------------------------------------------
