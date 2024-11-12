


struct FArr[T: CollectionElement]( CollectionElement, CollectionElementNew, Sized, Boolable):
    var     _Arr: UnsafePointer[ T]
    var     _Size: UInt32

