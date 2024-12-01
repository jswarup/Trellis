
from memory import UnsafePointer, memcpy 
 
fn PointerExample(): 
    var x: Int = 42 
    var ptr = UnsafePointer[Int].address_of(x) 
    value = ptr[]
    print(value)  # Prints 42 
    ptr[] = 100
    print(x)  # Prints 100  
 
fn UnSafePtrExample():
    ptr = UnsafePointer[Int].alloc(1)
    ptr[] = 42
    
    value = ptr[]
    print("Value stored:", value)  # Prints: Value stored: 42
    ptr.free()

fn PtrArithExample():
    ptr = UnsafePointer[Int].alloc(3)

    ptr[ 0] = 10            # First element
    ptr[ 1] = 20            # Second element
    ptr[ 2] = 30            # Third element
    
    for i in range(3):
        value = ptr[ i]
        print("Value at index", i, ":", value)
    
    ptr.free()

@value
struct Point:
    var x: Int
    var y: Int

fn StructPtrExample():
    ptr = UnsafePointer[Point].alloc(1)
    point = Point( 5, 10)
    
    ptr[ 0] = point
    
    # Read the struct
    loaded_point = ptr[ 0]
    print("Point coordinates: (", loaded_point.x, ",", loaded_point.y, ")")
    
    # Clean up
    ptr.free()
