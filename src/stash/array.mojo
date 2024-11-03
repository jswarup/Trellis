


from sys.intrinsics import _type_is_eq

struct Array( Movable, Movable):
    data: __mlir_type[Float32]  # Defines an MLIR-backed vector of 32-bit floating points
    length: Int
