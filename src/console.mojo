from stash import USeg, FArr


    
fn main():   
    vec  = FArr[ Int]( 7, 0) 
    origin_type = __origin_of( vec)
    i = 0
    for iter in vec:
        i += 1
        iter[] = i
    vec.SwapAt( 3, 5)
    for iter in vec:
        print( iter[])
