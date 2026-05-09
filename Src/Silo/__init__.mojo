from .USeg import *
from .Arr import *
from .Buff import *
from .Stk import *
from .Stash import *


def BuffTest():
    print( "BuffTest:")
    var b = Buff[ UInt32]( 4, 42)
    print( b.Arr())
    b.Resize( 6, 99)
    b.Resize( 5, 0)
    var a = b.Arr()
    a.Reverse()
    print( b.Arr())

    a.DoIndicize()
    a.Reverse()
    print( a)

    def Less( a: UInt32, b: UInt32) -> Bool:
        return a < b

    def Swap( a: UInt32, b: UInt32) -> None:
        pass

    a.QSort( 0, a.Size() - 1, Less, Swap)
    print( a)


def StkTest():
    print( "StkTest:")
    var b0 = Buff[ UInt32]( 22)
    var a0 = b0.Arr()
    var stk0 = Stk( a0, 0)

    for x in USeg( b0.Size()):
        _ = stk0.Push( b0.Size() - x)

    var b1 = Buff[ UInt32]( 12)
    var a1 = b1.Arr()
    a1.DoIndicize( 113)
    a1.Reverse()
    var stk1 = Stk( a1, 5)

    print( stk0.Arr(), stk1.Arr())
    _ = stk1.Import( stk0, 4)
    print( stk0.Arr(), stk1.Arr())
    _ = stk0.Export( stk1, 5)
    print( stk0.Arr(), stk1.Arr())
    _ = stk0.Import( stk1, 6)
    print( stk0.Arr(), stk1.Arr())
    _ = stk1.Import( stk0, 7)
    print( stk0.Arr(), stk1.Arr())
    _ = stk0.Export( stk1, 8)
    print( stk0.Arr(), stk1.Arr())
    _ = stk1.Export( stk0, 20)
    print( stk0.Arr(), stk1.Arr()) 


def USegTest():
    print( "USegTest:")
    var useg0 = USeg( 4, 9)
    print( useg0)

    def Write( x: UInt32) -> Bool:
        print( x, end=" ")
        return True

    var span = useg0.Span( Write)
    print( span)

    
    def lessAt( x: UInt32, y: UInt32) -> Bool:
        if ( x == y):
            print( x, end=" ")
        return x < y

    def swapAt( x: UInt32, y: UInt32) -> None:
        pass

    useg0.QSort( lessAt, swapAt)
    print( useg0)


def StashTest():
    print( "StashTest:")
    var stash = Stash[ UInt32]( 20)

def ClosureTest()  raises: 
      var a, b, c, d = 1, 2, 3, 4
      var x = "hello"

      # Legacy closure: no capture list. Cannot capture variables.
      def hello():
          print("hi")

      # Unified closure with no captures (stateless). Stateless closures
      # lift to top-level functions and can be passed as FFI callbacks.
      def add_one(n: Int) {} -> Int:
          return n + 1

      # Unified closure with explicit captures and a default capturing
      # convention:
      def my_fn() {mut a, b, c^, read}:
          # capture:
          # `a` by mut reference
          # `b` by immut reference
          # `c` by moving
          # `d` by immut reference (the default `read` convention)
        a =+ ( b + c +d)

      # Unified closure that captures `x` by ref (carries an
      # origin-mutability parameter):
      def show_x() {ref x}:
          print(x)

      # Function effects come before the capture list. The calling context
      # must handle errors raised from a `raises` closure.
      def fallible() raises {}:
          raise Error("nope")

      # Closures are invoked like ordinary functions:
      hello()
      print(add_one(41))
      my_fn()
      show_x()
      try:
          fallible()
      except e:
          print(e)

      # The `thin` function effect declares a function pointer type
      # (distinct from a closure trait). Stateless closures and top-level
      # functions satisfy `thin`:
      var fn_ptr: def(Int) thin -> Int = add_one
      print(fn_ptr(99))

def SiloTest(): 
    try:
        ClosureTest()
    except e:
        print(e) 

    BuffTest()
    #StkTest()
    #USegTest()
    #StashTest()
