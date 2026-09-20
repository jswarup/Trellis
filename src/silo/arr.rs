// arr.rs ---------------------------------------------------------------------------------------------------------
use	crate::silo::cast::{ IArrExt, IConstPtrAtExt, IMutArrExt, IPtrAtExt };
use	crate::silo::traits::{ IArr, IArrMut };
use	crate::silo::useg::USeg;
use	std::marker::PhantomData;
use	std::ops::{ Index, IndexMut };
use	std::ptr;

//-------------------------------------------------------------------------------------------------
// Arr — non-owning, borrowed contiguous array view.
// Exactly 16 bytes (_Ptr and _Size), zero-virtual, trivially copyable.
// Modeled directly from Trellis silo/arr.h.
pub struct Arr< 'a, T> {
    _Ptr: *const T,
    _Size: u32,
    _marker: PhantomData< &'a T>,
}

//-------------------------------------------------------------------------------------------------

unsafe impl< 'a, T: Sync> Send for Arr<'a, T>
{ }
unsafe impl< 'a, T: Sync> Sync for Arr<'a, T>
{ }
impl< T> Copy for Arr< '_, T> {}

//-------------------------------------------------------------------------------------------------

impl< T> Clone for Arr< '_, T> {
    #[inline]
    fn	clone( &self) -> Self
    {
        *self
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> Arr<'a, T>
{

    //---------------------------------------------------------------------------------------------
    // Constructors & Factories
    #[inline]
    pub const fn	Empty() -> Self
    {
        Self {
            _Ptr: ptr::NonNull::dangling().as_ptr(),
            _Size: 0,
            _marker: PhantomData,
        }
    }
    #[inline]
    pub const fn	New( ptr: *const T, size: u32) -> Self
    {
        Self {
            _Ptr: ptr,
            _Size: size,
            _marker: PhantomData,
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> From<Arr<'a, T>> for &'a [T] {
    #[inline]
    fn	from( arr: Arr< 'a, T>) -> Self {
        if arr.IsEmpty() {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts( arr._Ptr, arr._Size as usize) }
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> From<MutArr<'a, T>> for &'a mut [T] {
    #[inline]
    fn	from( arr: MutArr< 'a, T>) -> Self {
        if arr.IsEmpty() {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut( arr._Ptr, arr._Size as usize) }
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> From<&'a [T]> for Arr< 'a, T> {
    #[inline]
    fn	from( slice: &'a [T]) -> Self {
        Self::New( 
            slice.as_ptr(),
            u32::try_from( slice.len()).expect( "Array too large"),
        )
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> From<&'a mut [T]> for MutArr< 'a, T> {
    #[inline]
    fn	from( slice: &'a mut [T]) -> Self {
        Self::New( 
            slice.as_mut_ptr(),
            u32::try_from( slice.len()).expect( "Array too large"),
        )
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T, const N: usize> From<&'a [T; N]> for Arr< 'a, T> {
    #[inline]
    fn	from( arr: &'a [T; N]) -> Self {
        Self::from( arr.as_slice())
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T, const N: usize> From<&'a mut [T; N]> for MutArr< 'a, T> {
    #[inline]
    fn	from( arr: &'a mut [T; N]) -> Self {
        Self::from( arr.as_mut_slice())
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> Arr<'a, T>
{

    //---------------------------------------------------------------------------------------------
    // Accessors & Queries
    #[inline]
    pub const fn	Size( &self) -> u32
    {
        self._Size
    }
    #[inline]
    pub const fn	Len( &self) -> u32
    {
        self._Size
    }
    #[inline]
    pub const fn	IsEmpty( &self) -> bool
    {
        self._Size == 0
    }
    #[inline]
    pub const fn	Data( &self) -> *const T
    {
        self._Ptr
    }
    #[inline]
    pub fn	First( &self) -> Option< &'a T> {
        self.Get( 0)
    }
    #[inline]
    pub fn	Last( &self) -> Option< &'a T> {
        self._Size.checked_sub( 1).and_then( |index| self.Get( index))
    }
    #[inline]
    pub fn	Get( &self, index: u32) -> Option< &'a T> {
        if index < self._Size {
            Some( self._Ptr.RefAt( index as usize))
        } else {
            None
        }
    }
    #[inline]
    pub fn	GetMut( &self, index: u32) -> Option< &'a mut T> {
        if index < self._Size {
            Some( self._Ptr.cast_mut().MutRefAt( index as usize))
        } else {
            None
        }
    }
    /// # Safety
    /// The storage must be writable and access must be coordinated with all other views.
    #[inline]
    pub unsafe fn	MutView( &self) -> MutArr< 'a, T> {
        MutArr::New( self._Ptr.cast_mut(), self._Size)
    }
    #[inline]
    pub const fn	USeg( &self) -> USeg
    {
        USeg::FromLen( self._Size)
    }

    //---------------------------------------------------------------------------------------------
    // Slicing
    #[inline]
    pub fn	LSnip( &self, count: u32) -> Self
    {
        self.Slice( count, self._Size)
    }
    #[inline]
    pub fn	RSnip( &self, count: u32) -> Self
    {
        self.Slice( 0, self._Size.saturating_sub( count))
    }
    #[inline]
    pub fn	Slice( &self, start: u32, count: u32) -> Self
    {
        let  	take = count.min( self._Size.saturating_sub( start));
        if take == 0 {
            return Self::Empty();
        }
        unsafe { Self::New( self._Ptr.add( start as usize), take) }
    }
}
impl< 'a, T> Default for Arr<'a, T>
{
    fn	default() -> Self
    {
        Self::Empty()
    }
}
impl< 'a, T> Index<u32> for Arr<'a, T>
{
    type Output = T;
    #[inline]
    fn	index( &self, index: u32) -> &Self::Output
    {
        self.Get( index).expect( "Index out of bounds")
    }
}
impl< 'a, T> IArr<T> for Arr<'a, T>
{
    #[inline]
    fn	Arr( &self) -> Arr< '_, T> {
        *self
    }
    #[inline]
    fn	Len( &self) -> u32
    {
        self._Size
    }
}

//-------------------------------------------------------------------------------------------------
// MutArr — mutable borrowed contiguous array view.
// Exactly 16 bytes (_Ptr and _Size), zero-virtual.
pub struct MutArr< 'a, T> {
    _Ptr: *mut T,
    _Size: u32,
    _marker: PhantomData< &'a mut T>,
}
unsafe impl< 'a, T: Send> Send for MutArr<'a, T>
{ }
unsafe impl< 'a, T: Sync> Sync for MutArr<'a, T>
{ }
impl< 'a, T> MutArr<'a, T>
{
    #[inline]
    pub const fn	Empty() -> Self
    {
        Self {
            _Ptr: ptr::NonNull::dangling().as_ptr(),
            _Size: 0,
            _marker: PhantomData,
        }
    }
    #[inline]
    pub const fn	New( ptr: *mut T, size: u32) -> Self
    {
        Self {
            _Ptr: ptr,
            _Size: size,
            _marker: PhantomData,
        }
    }
    #[inline]
    pub const fn	Size( &self) -> u32
    {
        self._Size
    }
    #[inline]
    pub const fn	Len( &self) -> u32
    {
        self._Size
    }
    #[inline]
    pub const fn	IsEmpty( &self) -> bool
    {
        self._Size == 0
    }
    #[inline]
    pub const fn	Data( &self) -> *mut T
    {
        self._Ptr
    }
    #[inline]
    pub fn	Get( &self, index: u32) -> Option< &'a T> {
        if index < self._Size {
            Some( self._Ptr.RefAt( index as usize))
        } else {
            None
        }
    }
    #[inline]
    pub fn	GetMut( &mut self, index: u32) -> Option< &'a mut T> {
        if index < self._Size {
            Some( self._Ptr.MutRefAt( index as usize))
        } else {
            None
        }
    }
    #[inline]
    pub fn	Arr( &self) -> Arr< '_, T> {
        Arr::New( self._Ptr, self._Size)
    }
    /// # Safety
    /// The caller must prevent conflicting access through the duplicated views.
    #[inline]
    pub unsafe fn	Alias( &self) -> Self
    {
        Self::New( self._Ptr, self._Size)
    }
    #[inline]
    pub fn	CopyFrom( &mut self, source: Arr< '_, T>)
    where
        T: Copy,
    {
        assert_eq!( self._Size, source.Len(), "Array sizes differ");
        if self._Size > 0 {
            unsafe {
                ptr::copy( source._Ptr, self._Ptr, self._Size as usize);
            }
        }
    }
    /// # Safety
    /// The slot must be uninitialized or its previous value must already have been moved out.
    #[inline]
    pub unsafe fn	WriteAt( &mut self, index: u32, value: T)
    {
        assert!( index < self._Size, "Index out of bounds");
        unsafe {
            self._Ptr.add( index as usize).write( value);
        }
    }
    /// # Safety
    /// The slot must be initialized. Afterward it must not be read or dropped until reinitialized.
    #[inline]
    pub unsafe fn	ReadAt( &mut self, index: u32) -> T
    {
        assert!( index < self._Size, "Index out of bounds");
        unsafe { self._Ptr.add( index as usize).read() }
    }
    /// # Safety
    /// Source elements must be initialized; destination slots must be uninitialized.
    /// The regions must not overlap. The caller must relinquish ownership of the source values.
    #[inline]
    pub unsafe fn	MoveFrom( &mut self, source: Arr< '_, T>) {
        assert!( source.Len() <= self._Size, "Destination too small");
        if !source.IsEmpty() {
            unsafe {
                ptr::copy_nonoverlapping( source._Ptr, self._Ptr, source.Len() as usize);
            }
        }
    }
    #[inline]
    pub const fn	USeg( &self) -> USeg
    {
        USeg::FromLen( self._Size)
    }
    #[inline]
    pub fn	QSort< F: FnMut( &T, &T) -> bool>( &mut self, mut less: F)
    {
        let  	data = self._Ptr;
        let  	size = self._Size;
        // USeg invokes these callbacks sequentially. Comparison references live only
        // for that callback; the exclusive view borrow covers every swap.
        self.USeg().QSort( 
            |a, b| {
                assert!( a < size && b < size, "Index out of bounds");
                unsafe { less( &*data.add( a as usize), &*data.add( b as usize)) }
            },
            |a, b| {
                assert!( a < size && b < size, "Index out of bounds");
                unsafe {
                    ptr::swap( data.add( a as usize), data.add( b as usize));
                }
            },
        );
    }
    #[inline]
    pub fn	Swap( &mut self, i: u32, j: u32)
    {
        assert!( i < self._Size && j < self._Size, "Index out of bounds");
        if i != j {
            std::mem::swap( 
                self._Ptr.MutRefAt( i as usize),
                self._Ptr.MutRefAt( j as usize),
            );
        }
    }
    #[inline]
    pub fn	SetAt( &mut self, k: u32, val: T)
    {
        assert!( k < self._Size, "Index out of bounds");
        *self._Ptr.MutRefAt( k as usize) = val;
    }
    #[inline]
    pub fn	SwapAt( &self, k: u32, val: &mut T)
    {
        assert!( k < self._Size, "Index out of bounds");
        std::mem::swap( self._Ptr.MutRefAt( k as usize), val);
    }
    #[inline]
    pub fn	LSnip( &mut self, count: u32) -> Self
    {
        self.Slice( count, self._Size)
    }
    #[inline]
    pub fn	RSnip( &mut self, count: u32) -> Self
    {
        self.Slice( 0, self._Size.saturating_sub( count))
    }
    #[inline]
    pub fn	Slice( &mut self, start: u32, count: u32) -> Self
    {
        let  	arr = self.Arr().Slice( start, count);
        Self::New( arr._Ptr.cast_mut(), arr._Size)
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> Default for MutArr<'a, T>
{
    fn	default() -> Self
    {
        Self::Empty()
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> Index<u32> for MutArr<'a, T>
{
    type Output = T;
    #[inline]
    fn	index( &self, index: u32) -> &Self::Output
    {
        self.Get( index).expect( "Index out of bounds")
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> IndexMut<u32> for MutArr<'a, T>
{
    #[inline]
    fn	index_mut( &mut self, index: u32) -> &mut Self::Output
    {
        self.GetMut( index).expect( "Index out of bounds")
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> IArr<T> for MutArr<'a, T>
{
    #[inline]
    fn	Arr( &self) -> Arr< '_, T> {
        self.Arr()
    }
    #[inline]
    fn	Len( &self) -> u32
    {
        self._Size
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a, T> IArrMut<T> for MutArr<'a, T>
{
    #[inline]
    fn	MutArr( &mut self) -> MutArr< '_, T> {
        MutArr::New( self._Ptr, self._Size)
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a> Arr<'a, u8>
{
    /// Read a native-endian value at an element index, without requiring alignment.
    /// # Safety
    /// The selected bytes must be a valid representation of U.
    #[inline]
    pub unsafe fn	ReadValue< U: Copy>( &self, index: u32) -> U
    {
        let  	offset = ValueOffset::< U>( index, self._Size);
        unsafe { self._Ptr.add( offset as usize).cast::< U>().read_unaligned() }
    }
    #[inline]
    pub fn	Str( &self) -> &'a str {
        let  	slice: &[u8] = ( *self).into();
        unsafe { std::str::from_utf8_unchecked( slice) }
    }
    #[inline]
    pub fn	AsMutSlice( &self) -> &'a mut [u8] {
        unsafe { self.MutView().into() }
    }
}

//-------------------------------------------------------------------------------------------------

impl MutArr< '_, u8> {
    /// Write a native-endian value at an element index, without requiring alignment.
    /// # Safety
    /// U must contain no uninitialized padding. The caller must prevent conflicting access.
    #[inline]
    pub unsafe fn	WriteValue< U: Copy>( &self, index: u32, value: U)
    {
        let  	offset = ValueOffset::< U>( index, self._Size);
        unsafe {
            self._Ptr
                .add( offset as usize)
                .cast::< U>()
                .write_unaligned( value);
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a> From<Arr<'a, u8>> for &'a str {
    #[inline]
    fn	from( arr: Arr< 'a, u8>) -> &'a str
    {
        arr.Str()
    }
}

//-------------------------------------------------------------------------------------------------
// Raw casts retain the caller's responsibility for valid representations and access.
impl< 'a, T: Copy> Arr<'a, T>
{
    #[inline]
    pub fn	CastArr( &self) -> Arr< 'a, u8> {
        self.CastArrFrom()
    }
    #[inline]
    pub fn	CastArrFrom< U: Copy>( &self) -> Arr< 'a, U> {
        let  	size = CastSize::< T, U>( self._Size);
        if size == 0 {
            return Arr::Empty();
        }
        let  	data = self._Ptr.cast::< U>();
        assert!( data.is_aligned(), "Arr pointer not aligned to target type");
        Arr::New( data, size)
    }
}

//-------------------------------------------------------------------------------------------------

impl< T: Copy> MutArr< '_, T> {
    #[inline]
    pub fn	CastMutArr< U: Copy>( &mut self) -> MutArr< '_, U> {
        let  	arr = self.Arr().CastArrFrom::< U>();
        MutArr::New( arr._Ptr.cast_mut(), arr._Size)
    }
}

//-------------------------------------------------------------------------------------------------

impl< T: Copy> IArrExt for Arr< '_, T> {
    #[inline]
    fn	CastArr( &self) -> Arr< '_, u8> {
        self.CastArr()
    }
    #[inline]
    fn	CastArrFrom< U: Copy>( &self) -> Arr< '_, U> {
        self.CastArrFrom()
    }
}

//-------------------------------------------------------------------------------------------------

impl< T: Copy> IMutArrExt for MutArr< '_, T> {
    #[inline]
    fn	CastMutArr< U: Copy>( &mut self) -> MutArr< '_, U> {
        self.CastMutArr()
    }
}

//-------------------------------------------------------------------------------------------------

#[inline]
fn	CastSize< T, U>( size: u32) -> u32
{
    let  	sourceWidth = u32::try_from( std::mem::size_of::< T>()).expect( "Source type too large");
    let  	targetWidth = u32::try_from( std::mem::size_of::< U>()).expect( "Target type too large");
    assert!( targetWidth > 0, "Cannot cast to ZST");
    let  	bytes = size
        .checked_mul( sourceWidth)
        .expect( "Array byte size overflow");
    assert_eq!( 
        bytes % targetWidth,
        0,
        "Arr size in bytes not aligned to target type"
    );
    bytes / targetWidth
}

//-------------------------------------------------------------------------------------------------

#[inline]
fn	ValueOffset< T>( index: u32, size: u32) -> u32
{
    let  	width = u32::try_from( std::mem::size_of::< T>()).expect( "Value too large");
    assert!( width > 0 && index < size / width, "Index out of bounds");
    index * width
}

//-------------------------------------------------------------------------------------------------
