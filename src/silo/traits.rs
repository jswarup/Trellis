// traits.rs -------------------------------------------------------------------------------------------------------
use crate::silo::seg::USeg;

//-------------------------------------------------------------------------------------------------

// IArr — zero-virtual trait for indexed contiguous array and buffer access.
// Renamed from IContiguous, modeled directly from Trellis silo/arr.h and traits.h.
pub trait IArr< T> {
    fn  AsSlice( &self) -> &[T];
    fn  Len( &self) -> u32;
    #[inline]
    fn  Size( &self) -> u32
    {
        self.Len()
    }
    #[inline]
    fn  IsEmpty( &self) -> bool
    {
        self.Len() == 0
    }
    #[inline]
    fn  USeg( &self) -> USeg
    {
        USeg::FromLen( self.Len())
    }
    #[inline]
    fn  Traverse< F: FnMut( &T)>( &self, mut f: F)
    {
        let  slice = self.AsSlice();
        self.USeg().Traverse( |i| f( &slice[i as usize]));
    }
    #[inline]
    fn  Span< F: FnMut( &T) -> bool>( &self, mut f: F) -> bool
    {
        let  slice = self.AsSlice();
        self.USeg().Span( |i| f( &slice[i as usize]))
    }
}

//-------------------------------------------------------------------------------------------------

// IArrMut — mutable indexed array and buffer interface.
pub trait IArrMut< T>: IArr< T> {
    fn  AsMutSlice( &mut self) -> &mut [T];
    #[inline]
    fn  TraverseMut< F: FnMut( &mut T)>( &mut self, mut f: F)
    {
        let  useg = self.USeg();
        let  slice = self.AsMutSlice();
        useg.Traverse( |i| f( &mut slice[i as usize]));
    }
}
impl< T> IArr< T> for [T]
{
    #[inline]
    fn  AsSlice( &self) -> &[T]
    {
        self
    }
    #[inline]
    fn  Len( &self) -> u32
    {
        self.len() as u32
    }
}
impl< T> IArrMut< T> for [T]
{
    #[inline]
    fn  AsMutSlice( &mut self) -> &mut [T]
    {
        self
    }
}
