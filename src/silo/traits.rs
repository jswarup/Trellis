// traits.rs -------------------------------------------------------------------------------------------------------
use	crate::silo::arr::{ Arr, MutArr };
use	crate::silo::useg::USeg;

//-------------------------------------------------------------------------------------------------
// IArr — zero-virtual trait for indexed contiguous array and buffer access.
// Renamed from IContiguous, modeled directly from Trellis silo/arr.h and traits.h.
pub trait IArr< T> {
    fn	Arr( &self) -> Arr< '_, T>;
    fn	Len( &self) -> u32;
    #[inline]
    fn	Size( &self) -> u32
    {
        self.Len()
    }
    #[inline]
    fn	IsEmpty( &self) -> bool
    {
        self.Len() == 0
    }
    #[inline]
    fn	USeg( &self) -> USeg
    {
        USeg::FromLen( self.Len())
    }
    #[inline]
    fn	Traverse< F: FnMut( &T)>( &self, mut f: F)
    {
        let  	arr = self.Arr();
        self.USeg().Traverse( |i| f( &arr[i]));
    }
    #[inline]
    fn	Span< F: FnMut( &T) -> bool>( &self, mut f: F) -> bool
    {
        let  	arr = self.Arr();
        self.USeg().Span( |i| f( &arr[i]))
    }
    #[inline]
    fn	TraverseRev< F: FnMut( &T)>( &self, mut f: F)
    {
        let  	arr = self.Arr();
        self.USeg().TraverseRev( |i| f( &arr[i]));
    }
}

//-------------------------------------------------------------------------------------------------
// IArrMut — mutable indexed array and buffer interface.
pub trait IArrMut< T>: IArr< T> {
    fn	MutArr( &mut self) -> MutArr< '_, T>;
    #[inline]
    fn	TraverseMut< F: FnMut( &mut T)>( &mut self, mut f: F)
    {
        let  	useg = self.USeg();
        let  	mut arr = self.MutArr();
        useg.Traverse( |i| f( &mut arr[i]));
    }
    #[inline]
    fn	TraverseRevMut< F: FnMut( &mut T)>( &mut self, mut f: F)
    {
        let  	useg = self.USeg();
        let  	mut arr = self.MutArr();
        useg.TraverseRev( |i| f( &mut arr[i]));
    }
}
