use	crate::silo::arr::{ Arr, MutArr };
use	crate::silo::buff::{ Buff, RawBuff };
use	crate::silo::stk::Stk;
use	crate::silo::useg::USeg;
use	std::ops::{ Index, IndexMut };
use	std::ptr;
use	std::sync::atomic::{ AtomicU32, Ordering };

//-------------------------------------------------------------------------------------------------
// Stash — dynamic array container for building up elements backed by an owning Buff.
// Exactly 24 bytes (_Buff is 16 bytes, _Sz is 4 bytes + 4 alignment), zero-virtual.
// Modeled directly from Trellis silo/stash.h and Kosh silo/stash.rs.
// Designed for local instantiation to build and fill elements, extracting into a Buff.
pub struct Stash< T>
{
    _Buff: RawBuff< T>,
    _Sz: AtomicU32,
}
unsafe impl< T: Send> Send for Stash< T> {}
unsafe impl< T: Sync> Sync for Stash< T> {}
impl< T> Stash< T>
{

    //---------------------------------------------------------------------------------------------
    // Constructors & Factories
    #[inline]
    pub const fn	New() -> Self
    {
        Self {
            _Buff: RawBuff::New(),
            _Sz: AtomicU32::new( 0),
        }
    }
    pub fn	WithCapacity( capacity: u32) -> Self
    {
        Self {
            _Buff: RawBuff::WithCapacity( capacity),
            _Sz: AtomicU32::new( 0),
        }
    }
    pub fn	FromDispenser< F: FnMut( u32) -> T>( 
        capacity: u32, initial_size: u32, mut dispenser: F,
    ) -> Self
    {
        let  	cap = capacity.max( initial_size);
        let  	raw: RawBuff< T> = RawBuff::WithCapacity( cap);
        struct DispenserGuard< T> {
            raw: RawBuff< T>,
            init: u32,
        }
        impl< T> Drop for DispenserGuard< T> {
            fn	drop( &mut self) {
                for i in 0..self.init {
                    unsafe {
                        ptr::drop_in_place( self.raw._Ptr.add( i as usize));
                    }
                }
            }
        }
        let  	mut guard = DispenserGuard {
            raw,
            init: 0,
        };
        for i in 0..initial_size {
            unsafe {
                ptr::write( guard.raw.Ptr().add( i as usize), dispenser( i));
            }
            guard.init += 1;
        }
        let  	( ptr, c) = guard.raw.Take().IntoRawParts();
        std::mem::forget( guard);
        Self {
            _Buff: unsafe { RawBuff::FromRawParts( ptr, c) },
            _Sz: AtomicU32::new( initial_size),
        }
    }

    //---------------------------------------------------------------------------------------------
    // Capacity & Growth
    fn	ReplaceBuffer( &mut self, new_cap: u32)
    {
        let  	cur_sz = self.Size();
        assert!( 
            new_cap >= cur_sz,
            "New capacity cannot discard initialized elements"
        );
        let  	mut new_raw = RawBuff::WithCapacity( new_cap);
        if cur_sz > 0 {
            unsafe {
                ptr::copy_nonoverlapping( self._Buff.Ptr(), new_raw.Ptr(), cur_sz as usize);
            }
        }
        self._Buff.Dealloc();
        self._Buff = new_raw.Take();
    }
    fn	grow( &mut self)
    {
        let  	cur_cap = self._Buff.Cap();
        let  	new_cap = if cur_cap == 0 {
            4
        } else {
            cur_cap.checked_mul( 2).expect( "Capacity overflow")
        };
        self.ReplaceBuffer( new_cap);
    }
    pub fn	Reserve( &mut self, new_cap: u32)
    {
        if new_cap > self._Buff.Cap() {
            self.ReplaceBuffer( new_cap);
        }
    }

    //---------------------------------------------------------------------------------------------
    // Dynamic Insertion & Extraction
    pub fn	Push( &mut self, val: T)
    {
        self.PushBack( val);
    }
    pub fn	PushBack( &mut self, val: T)
    {
        let  	cur_sz = self.Size();
        if cur_sz >= self._Buff.Cap() {
            self.grow();
        }
        unsafe {
            ptr::write( self._Buff.Ptr().add( cur_sz as usize), val);
        }
        self._Sz.store( cur_sz + 1, Ordering::Release);
    }
    pub fn	Pop( &mut self) -> Option< T>
    {
        let  	cur_sz = self.Size();
        if cur_sz == 0 {
            None
        } else {
            let  	new_sz = cur_sz - 1;
            self._Sz.store( new_sz, Ordering::Release);
            unsafe { Some( ptr::read( self._Buff.Ptr().add( new_sz as usize))) }
        }
    }
    #[allow( clippy::mut_from_ref)]                                    // Stash owns stable storage behind its atomic size.
    pub fn	TopMut( &self) -> Option< &mut T>
    {
        let  	cur_sz = self.Size();
        if cur_sz == 0 {
            None
        } else {
            unsafe { Some( &mut *self._Buff.Ptr().add( ( cur_sz - 1) as usize)) }
        }
    }
    #[inline]
    pub fn	Top( &self) -> Option< T>
    where
        T: Copy,
    {
        let  	cur_sz = self.Size();
        if cur_sz == 0 {
            None
        } else {
            unsafe { Some( *self._Buff.Ptr().add( ( cur_sz - 1) as usize)) }
        }
    }
    pub fn	PushX( &self, val: &mut T) -> bool
    where
        T: Default,
    {
        self.Stk().PushX( val)
    }
    pub fn	Clear( &mut self)
    {
        while self.Pop().is_some() {}
    }
    pub fn	Resize< F: FnMut( u32) -> T>( &mut self, new_size: u32, mut dispenser: F)
    {
        let  	cur_sz = self.Size();
        if new_size < cur_sz {
            for _ in new_size..cur_sz {
                self.Pop();
            }
        } else if new_size > cur_sz {
            self.Reserve( new_size);
            for i in cur_sz..new_size {
                self.Push( dispenser( i));
            }
        }
    }
    pub fn	ExtractBuff( mut self) -> Buff< T>
    {
        let  	cur_sz = self.Size();
        let  	cur_cap = self._Buff.Cap();
        if cur_sz == cur_cap {
            self._Sz.store( 0, Ordering::Release);
            let  	( ptr, cap) = self._Buff.Take().IntoRawParts();
            return unsafe { Buff::FromRawParts( ptr, cap) };
        }
        if cur_sz == 0 {
            self._Sz.store( 0, Ordering::Release);
            return Buff::New();
        }
        self.ReplaceBuffer( cur_sz);
        self._Sz.store( 0, Ordering::Release);
        let  	( ptr, cap) = self._Buff.Take().IntoRawParts();
        unsafe { Buff::FromRawParts( ptr, cap) }
    }
    #[inline]
    pub fn	IntoBuff( self) -> Buff< T>
    {
        self.ExtractBuff()
    }
    /// Extracts accumulated elements into a compact `Buff<T>` and leaves this `Stash`
    /// in an empty, valid, and reusable state without cloning elements.
    #[inline]
    pub fn	TakeBuff( &mut self) -> Buff< T>
    {
        if self.Size() == 0 {
            return Buff::New();
        }
        let  	old = std::mem::take( self);
        old.ExtractBuff()
    }
    #[inline]
    pub fn	ToBuff( &self) -> Buff< T>
    where
        T: Clone,
    {
        Buff::FromArr( self.Arr())
    }

    //---------------------------------------------------------------------------------------------
    // Accessors & Queries
    #[inline]
    pub fn	Size( &self) -> u32
    {
        self._Sz.load( Ordering::Acquire)
    }
    #[inline]
    pub fn	Len( &self) -> u32
    {
        self.Size()
    }
    #[inline]
    pub fn	Capacity( &self) -> u32
    {
        self._Buff.Cap()
    }
    #[inline]
    pub fn	IsEmpty( &self) -> bool
    {
        self.Size() == 0
    }
    #[inline]
    pub fn	Arr( &self) -> Arr< '_, T> {
        let  	sz = self.Size();
        if sz == 0 {
            Arr::Empty()
        } else {
            Arr::New( self._Buff.Ptr(), sz)
        }
    }
    #[inline]
    pub fn	MutArr( &mut self) -> MutArr< '_, T> {
        let  	sz = self.Size();
        if sz == 0 {
            MutArr::Empty()
        } else {
            unsafe { MutArr::New( self._Buff.Ptr(), sz) }
        }
    }
    #[inline]
    pub fn	Stk< 'a>(&'a self) -> Stk< 'a, T> {
        Stk::Create(
            &self._Sz,
            unsafe { MutArr::New( self._Buff.Ptr(), self._Buff.Cap()) },
        )
    }
    #[inline]
    pub fn	USeg( &self) -> USeg
    {
        USeg::FromLen( self.Size())
    }
}
impl< T> Default for Stash< T> {
    fn	default() -> Self
    {
        Self::New()
    }
}
impl< T: Clone> Clone for Stash< T> {
    fn	clone( &self) -> Self
    {
        let  	sz = self.Size();
        let  	mut new_stash = Self::WithCapacity( sz);
        let  	arr = self.Arr();
        for i in 0..sz {
            new_stash.Push( arr.Get( i).unwrap().clone());
        }
        new_stash
    }
}
impl< T> Index< u32> for Stash< T> {
    type Output = T;
    #[inline]
    fn	index( &self, index: u32) -> &Self::Output
    {
        assert!( index < self.Size(), "Index out of bounds");
        unsafe { &*self._Buff.Ptr().add( index as usize) }
    }
}
impl< T> IndexMut< u32> for Stash< T> {
    #[inline]
    fn	index_mut( &mut self, index: u32) -> &mut Self::Output
    {
        assert!( index < self.Size(), "Index out of bounds");
        unsafe { &mut *self._Buff.Ptr().add( index as usize) }
    }
}

//-------------------------------------------------------------------------------------------------

#[macro_export]
macro_rules! Stash {
    ( @__ $acc:ident, $exp:expr; for $item:pat in $iter:expr; if $cond:expr) => ( 
        for $item in $iter {
            if $cond {
                $acc.Push($exp);
            }
        }
    );
    ( @__ $acc:ident, $exp:expr; for $item:pat in $iter:expr) => ( 
        for $item in $iter {
            $acc.Push($exp);
        }
    );
    ( @__ $acc:ident, $exp:expr; for $item:pat in $iter:expr; if $cond:expr; $($tail:tt)+) => ( 
        for $item in $iter {
            if $cond {
                $crate::Stash![@__ $acc, $exp; $($tail)+];
            }
        }
    );
    ( @__ $acc:ident, $exp:expr; for $item:pat in $iter:expr; $($tail:tt)+) => ( 
        for $item in $iter {
            $crate::Stash![@__ $acc, $exp; $($tail)+];
        }
    );
    ($exp:expr; $($tail:tt)+) => ( {
        let  	mut ret = $crate::silo::stash::Stash::New();
        $crate::Stash![@__ ret, $exp; $($tail)+];
        ret
    });
    () => {
        $crate::silo::stash::Stash::New()
    };
    ( $( $x:expr ),* ) => {
        {
            let  	mut temp = $crate::silo::stash::Stash::New();
            $( 
                temp.Push($x);
            )*
            temp
        }
    };
    ( $( $x:expr ),+ , ) => {
        $crate::Stash![ $( $x ),* ]
    };
}
impl< T> Drop for Stash< T> {
    fn	drop( &mut self)
    {
        struct StashDropGuard< T> {
            ptr: *mut T,
            remaining: u32,
        }
        impl< T> Drop for StashDropGuard< T> {
            fn	drop( &mut self) {
                while self.remaining > 0 {
                    self.remaining -= 1;
                    unsafe {
                        ptr::drop_in_place( self.ptr.add( self.remaining as usize));
                    }
                }
            }
        }
        let  	mut guard = StashDropGuard {
            ptr: self._Buff.Ptr(),
            remaining: self.Size(),
        };
        while guard.remaining > 0 {
            guard.remaining -= 1;
            unsafe {
                ptr::drop_in_place( guard.ptr.add( guard.remaining as usize));
            }
        }
    }
}
