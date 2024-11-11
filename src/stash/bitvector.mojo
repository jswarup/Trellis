


struct BitVector:
    var value: Int  # Underlying integer value to represent the bit vector

    
    fn __init__(inout self, v : Int):
        self.value = v
         

    # Overload the right shift operator (>>)
    fn __rshift__(self, shift_amount: Int) -> Self:
        return BitVector(self.value >> shift_amount)

    # Overload the logical OR operator (||)
    fn __or__(self, other: Self) -> Self:
        return BitVector(self.value | other.value)

    # Overload the logical NOT operator (!)
    fn __not__(self) -> Self:
        return BitVector(~self.value)  # Bitwise NOT

    # Define a string representation for easy printing
    fn __str__(self) -> String:
        return  str( self.value)

def main():
    # Create instances of BitVector
    bv1 = BitVector(0b1010)  # Binary: 1010
    bv2 = BitVector(0b1100)  # Binary: 1100

    # Right shift operator
    shifted_bv = bv1 >> 1
    print( str( shifted_bv))  # Output: BitVector(5), because 0b1010 >> 1 is 0b0101 (5 in decimal)

    # Logical OR operator
    var or_bv = bv1 || bv2
    print(or_bv)  # Output: BitVector(14), because 0b1010 | 0b1100 is 0b1110 (14 in decimal)

    # Logical NOT operator
    not_bv = !bv1
    print(not_bv)  # Output: BitVector(-11), bitwise NOT inverts all bits of 0b1010
