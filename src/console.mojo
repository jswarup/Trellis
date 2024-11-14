from stash import USeg, FArr


    
fn main():   
    vec  = FArr[ Int]( 54, 0) 
    origin_type = __origin_of( vec)
    for i in vec:
        i[] = 20
    for i in vec:
        print( i[])
