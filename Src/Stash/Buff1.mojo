# Buff.mojo -----------------------------------------------------------------------------------------------------------------------
   
#----------------------------------------------------------------------------------------------------------------------------------

struct Buff[T: Copyable]: 

    comptime _UnsafePointerType = UnsafePointer[Self.T, MutExternalOrigin] 
    var _data: Self._UnsafePointerType
    var _size: Int
    var _capacity: Int

    # 1. CONSTRUCTOR
    # 'out self' indicates this method is responsible for fully initializing 
    # the uninitialized instance before it returns.
    def __init__(out self, capacity: Int = 4):
        self._size = 0
        self._capacity = capacity
        # Allocate raw memory to hold 'capacity' number of elements
        self._data = _UnsafePointerType.alloc(self._capacity)

    # 2. DESTRUCTOR
    # Any type that allocates memory must deallocate it in its destructor 
    # to avoid memory leaks. Mojo guarantees this is called promptly.
    def __del__(deinit self):
        # First, cleanly destroy all initialized elements
        for i in range(self._size):
            (self._data + i).destroy_pointee()
        # Finally, free the raw memory buffer
        self._data.free()

    # 3. APPEND
    # 'mut self' provides a mutable reference, allowing us to modify 
    # the struct's internal state (size, capacity, and data pointer).
    def append(mut self, value: T):
        if self._size == self._capacity:
            self.reserve(self._capacity * 2)
        
        # Initialize the raw memory at the current end index with the new value
        (self._data + self._size).init_pointee_copy(value)
        self._size += 1

    # 4. RESERVE (Dynamic Resizing)
    def reserve(mut self, new_capacity: Int):
        if new_capacity <= self._capacity:
            return
        
        # Allocate a larger block of memory
        var new_data = Self._UnsafePointerType.alloc(new_capacity)
        
        # Move existing elements to the new memory block
        for i in range(self._size):
            (new_data + i).init_pointee_move((self._data + i).take_pointee())
            
        # Free the old, smaller memory block
        self._data.free()
        
        # Update struct fields to point to the new block
        self._data = new_data
        self._capacity = new_capacity

    # 5. ELEMENT ACCESS & UTILITIES
    # The default 'self' behaves like 'read self', meaning it gets an immutable reference.
    def __getitem__(self, index: Int) -> T:
        return self._data[index]
        
    def size(self) -> Int:
        return self._size
        
    def capacity(self) -> Int:
        return self._capacity
 

def main():
    var buf = Buff[Int]()
    buf.append(10)
    buf.append(20)

    print(buf[0])
    print(buf.size())
