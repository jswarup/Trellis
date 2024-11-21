import stash
import demo

fn PtrDemo():    
    demo.ptrexh.PointerExample()
    demo.ptrexh.UnSafePtrExample()
    demo.ptrexh.StructPtrExample()
    demo.ptrexh.PtrArithExample()

fn TypeDemo():    
    demo.typeexh.TypeExample();

fn ArrDemo():   
    #stash.arr.ArrExample()
    #stash.arr.ArrSortExample() 
    stash.stk.StkExample()

fn main():   
    ArrDemo() 
