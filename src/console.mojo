import stash
import demo
import strand
import heist


fn PtrDemo():    
    demo.ptrexh.PointerExample()
    demo.ptrexh.UnSafePtrExample()
    demo.ptrexh.StructPtrExample()
    demo.ptrexh.PtrArithExample()

fn TypeDemo():    
    demo.typeexh.TypeExample();

fn ArrDemo():    
    stash.arr.ArrSortExample() 
    stash.stk.StkExample()
    stash.silo.SiloExample()
    pass

fn AtmDemo():
    strand.atm.AtmExample()
    strand.spinlock.SpinLockExample()

fn HeistDemo(): 
    heist.mule.MuleExample() 
    return
    heist.atelier.AtelierSortExample()
    heist.atelier.AtelierComposeExample()
    heist.atelier.AtelierExample() 
    pass
     


fn main(): 
    AtmDemo()  
    ArrDemo()
    HeistDemo() 
    pass

#----------------------------------------------------------------------------------------------------------------------------------

@value
struct Pair[ TLeft: StringableCollectionElement, TRight: StringableCollectionElement] ( StringableCollectionElement):   
    var     _Left : TLeft
    var     _Right : TRight

    fn __init__( out self, owned left : TLeft, owned right : TRight):  
        self._Left = left 
        self._Right = right

    fn __str__( self) -> String:
        str = "[ " + self._Left.__str__() + ", " + self._Right.__str__() + "]"
        return str
 

#----------------------------------------------------------------------------------------------------------------------------------
   