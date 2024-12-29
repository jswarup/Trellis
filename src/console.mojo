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
    #heist.mule.MuleExample() 
    heist.atelier.AtelierExample() 
    pass
    
fn main(): 
    AtmDemo()  
    ArrDemo()
    HeistDemo() 

