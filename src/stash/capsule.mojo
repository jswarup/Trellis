# capsule.mojo --------------------------------------------------------------------------------------------------------------------


from testing import assert_equal, assert_false, assert_true
from os import abort
from sys import alignof, sizeof
from sys.intrinsics import _type_is_eq
from memory import UnsafePointer
from python import Python

#----------------------------------------------------------------------------------------------------------------------------------

struct Capsule[*Ts: CollectionElement](
    CollectionElement,
    ExplicitlyCopyable,
): 
    # Fields
    alias   _sentinel: Int = -1
    alias   _mlir_type = __mlir_type[ `!kgen.variant<[rebind(:`, __type_of(Ts), ` `, Ts, `)]>` ]
    var     _impl: Self._mlir_type
 
    fn __init__(inout self, *, unsafe_uninitialized: ()): 
        self._impl = __mlir_attr[`#kgen.unknown : `, Self._mlir_type]

    fn __init__[T: CollectionElement](inout self, owned value: T): 
        self._impl = __mlir_attr[`#kgen.unknown : `, self._mlir_type]
        alias idx = Self._check[T]()
        self._get_discr() = idx
        self._get_ptr[T]().init_pointee_move(value^)

    fn __init__(inout self, *, other: Self): 
        self = Self(unsafe_uninitialized=())
        self._get_discr() = other._get_discr()

        @parameter
        for i in range(len(VariadicList(Ts))):
            alias T = Ts[i]
            if self._get_discr() == i:
                self._get_ptr[T]().init_pointee_move(other._get_ptr[T]()[])
                return

    fn __copyinit__(inout self, other: Self):  
        self = Self(other=other)

    fn __moveinit__(inout self, owned other: Self): 
        self._impl = __mlir_attr[`#kgen.unknown : `, self._mlir_type]
        self._get_discr() = other._get_discr()

        @parameter
        for i in range(len(VariadicList(Ts))):
            alias T = Ts[i]
            if self._get_discr() == i:
                # Calls the correct __moveinit__
                other._get_ptr[T]().move_pointee_into(self._get_ptr[T]())
                return

    fn __del__(owned self): 
        @parameter
        for i in range(len(VariadicList(Ts))):
            if self._get_discr() == i:
                self._get_ptr[Ts[i]]().destroy_pointee()
                return
 
    fn __getitem__[T: CollectionElement](ref [_]self: Self) -> ref [self] T: 
        if not self.isa[T]():
            abort("get: wrong variant type")

        return self.unsafe_get[T]()
  
    fn _get_ptr[T: CollectionElement](self) -> UnsafePointer[T]:
        alias idx = Self._check[T]()
        constrained[idx != Self._sentinel, "not a union element type"]()
        var ptr = UnsafePointer.address_of(self._impl).address
        var discr_ptr = __mlir_op.`pop.variant.bitcast`[
            _type = UnsafePointer[T]._mlir_type, index = idx.value
        ](ptr)
        return discr_ptr
 
    fn _get_discr(ref [_]self: Self) -> ref [self] UInt8:
        var ptr = UnsafePointer.address_of(self._impl).address
        var discr_ptr = __mlir_op.`pop.variant.discr_gep`[
            _type = __mlir_type.`!kgen.pointer<scalar<ui8>>`
        ](ptr)
        return UnsafePointer(discr_ptr).bitcast[UInt8]()[]
 
    fn take[T: CollectionElement](inout self) -> T: 
        if not self.isa[T]():
            abort("taking the wrong type!")

        return self.unsafe_take[T]()

    @always_inline
    fn unsafe_take[T: CollectionElement](inout self) -> T: 
        debug_assert(self.isa[T](), "taking wrong type")
        # don't call the variant's deleter later
        self._get_discr() = Self._sentinel
        return self._get_ptr[T]().take_pointee()

    @always_inline
    fn replace[
        Tin: CollectionElement, Tout: CollectionElement
    ](inout self, owned value: Tin) -> Tout: 
        if not self.isa[Tout]():
            abort("taking out the wrong type!")

        return self.unsafe_replace[Tin, Tout](value^)

    @always_inline
    fn unsafe_replace[
        Tin: CollectionElement, Tout: CollectionElement
    ](inout self, owned value: Tin) -> Tout: 
        debug_assert(self.isa[Tout](), "taking out the wrong type!")

        var x = self.unsafe_take[Tout]()
        self.set[Tin](value^)
        return x^

    fn set[T: CollectionElement](inout self, owned value: T): 
        self = Self(value^)

    fn isa[T: CollectionElement](self) -> Bool: 
        alias idx = Self._check[T]()
        return self._get_discr() == idx

    fn unsafe_get[T: CollectionElement](ref [_]self: Self) -> ref [self] T: 
        debug_assert(self.isa[T](), "get: wrong variant type")
        return self._get_ptr[T]()[]

    @staticmethod
    fn _check[T: CollectionElement]() -> Int:
        @parameter
        for i in range(len(VariadicList(Ts))):
            if _type_is_eq[Ts[i], T]():
                return i
        return Self._sentinel




def test_basic():
    alias IntOrString = Capsule[Int, String]
    var i = IntOrString(4)
    var s = IntOrString(String("4"))

    # isa
    assert_true(i.isa[Int]())
    assert_false(i.isa[String]())
    assert_true(s.isa[String]())
    assert_false(s.isa[Int]())

    # get
    assert_equal(4, i[Int])
    assert_equal("4", s[String])
    
    print( i[Int], s[String])
    
    # we don't test what happens when you `get` the wrong type.
    # have fun!

    # set
    i.set[String]("i")
    assert_false(i.isa[Int]())
    assert_true(i.isa[String]())
    assert_equal("i", i[String])

def main():
    test_basic()
