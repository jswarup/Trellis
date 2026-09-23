use	crate::silo::arr::{ Arr, MutArr };
use	crate::silo::buff::Buff;
use	crate::silo::stk::Stk;
use	crate::silo::useg::USeg;
use	std::alloc::{ Layout, alloc };
use	std::ops::{ Index, IndexMut };
use	std::sync::atomic::{ AtomicU32, Ordering };

//-------------------------------------------------------------------------------------------------
// Stash — dynamic array container for building up elements backed by an owning Buff.
// Exactly 24 bytes (_Buff is 16 bytes, _Sz is 4 bytes + 4 alignment), zero-virtual.
// Modeled directly from Trellis silo/stash.h and Kosh silo/stash.rs.
// Designed for local instantiation to build and fill elements, extracting into a Buff.
pub struct Stash< T>
{
    _Buff: Buff< T>,
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
            _Buff: Buff::New(),
            _Sz: AtomicU32::new( 0),
        }
    }
    pub fn	WithCapacity( capacity: u32) -> Self
    {
        Self {
            _Buff: Buff::WithCapacity( capacity),
            _Sz: AtomicU32::new( 0),
        }
    }
    pub fn	FromDispenser< F: FnMut( u32) -> T>( 
        capacity: u32, initial_size: u32, mut dispenser: F,
    ) -> Self
    {
        let  	cap = capacity.max( initial_size);
        let  	mut buff: Buff< T> = Buff::WithCapacity( cap);
        USeg::FromLen( initial_size).Traverse( |i| unsafe {
            buff.MutArr().WriteAt( i, dispenser( i));
        });
        Self {
            _Buff: buff,
            _Sz: AtomicU32::new( initial_size),
        }
    }

    //---------------------------------------------------------------------------------------------
    // Capacity & Growth
    fn	Allocate( capacity: u32) -> *mut T
    {
        let  	layout = Layout::array::< T>( capacity as usize).expect( "Capacity overflow");
        assert!( 
            layout.size() > 0,
            "Zero-sized types unsupported in raw Buff"
        );
        let  	ptr = unsafe { alloc( layout) as *mut T };
        if ptr.is_null() {
            std::alloc::handle_alloc_error( layout);
        }
        ptr
    }
    fn	ReplaceBuffer( &mut self, new_cap: u32)
    {
        let  	cur_sz = self.Size();
        assert!( 
            new_cap >= cur_sz,
            "New capacity cannot discard initialized elements"
        );
        let  	new_ptr = Self::Allocate( new_cap);
        let  	mut old_buff = self._Buff.Take();
        if cur_sz > 0 {
            unsafe {
                MutArr::New( new_ptr, new_cap).MoveFrom( old_buff.Arr().Slice( 0, cur_sz));
            }
        }
        old_buff.Destroy( 0);
        self._Buff = unsafe { Buff::FromRawParts( new_ptr, new_cap) };
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
            self._Buff.MutArr().WriteAt( cur_sz, val);
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
            unsafe { Some( self._Buff.MutArr().ReadAt( new_sz)) }
        }
    }
    #[allow( clippy::mut_from_ref)]                                    // Stash owns stable storage behind its atomic size.
    pub fn	TopMut( &self) -> Option< &mut T>
    {
        let  	cur_sz = self.Size();
        if cur_sz == 0 {
            None
        } else {
            unsafe { self.Arr().GetMut( cur_sz - 1) }
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
            self.Arr().Get( cur_sz - 1).copied()
        }
    }
    pub fn	PushX( &self, val: &mut T) -> bool
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
            return self._Buff.Take();
        }
        if cur_sz == 0 {
            self._Sz.store( 0, Ordering::Release);
            return Buff::New();
        }
        self.ReplaceBuffer( cur_sz);
        self._Sz.store( 0, Ordering::Release);
        self._Buff.Take()
    }
    #[inline]
    pub fn	IntoBuff( self) -> Buff< T>
    {
        self.ExtractBuff()
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
        self._Buff.Arr().RSnip( self._Buff.Cap() - self.Size())
    }
    #[inline]
    pub fn	MutArr( &mut self) -> MutArr< '_, T> {
        let  	sz = self.Size();
        unsafe { MutArr::New( self._Buff.MutArr().Data(), sz) }
    }
    #[inline]
    pub fn	Stk< 'a>(&'a self) -> Stk< 'a, T> {
        Stk::Create(
            &self._Sz,
            unsafe { self._Buff.Arr().MutView() },
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
        self.Arr().Get( index).unwrap()
    }
}
impl< T> IndexMut< u32> for Stash< T> {
    #[inline]
    fn	index_mut( &mut self, index: u32) -> &mut Self::Output
    {
        assert!( index < self.Size(), "Index out of bounds");
        unsafe { &mut *self._Buff.MutArr().Data().add( index as usize) }
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
        let  	cur_sz = self.Size();
        self._Buff.Destroy( cur_sz);
    }
}
