

fn get_person_info() -> (StringLiteral, Int, Bool):
    name = "Alice"
    age = 30
    is_student = False
    return (name, age, is_student)
 
fn TypeExample(): 
    tup = get_person_info()
    print( len( tup)) 
    pass