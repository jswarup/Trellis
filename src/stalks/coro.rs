//-- coro.rs -------------------------------------------------------------------------------------------------------------------------
use	corosensei::{ Coroutine, CoroutineResult, Yielder };

//---------------------------------------------------------------------------------------------------------------------------------

/// Represents the result of a coroutine resuming.
pub enum CoroRes< Y, R>
{
    Yield( Y),
    Done( R),
}

//---------------------------------------------------------------------------------------------------------------------------------

/// Trait for interacting with a coroutine.
pub trait ICoro< In, Yield, Out>
{
    fn	Resume( &mut self, input: In) -> CoroRes< Yield, Out>;
    fn	IsDone( &self) -> bool;
}

//---------------------------------------------------------------------------------------------------------------------------------

/// Ergonomic wrapper around a corosensei Yielder.
pub struct CoroYielder< 'a, In, Yield>
{
    _Yielder: &'a Yielder< In, Yield>,
}

impl< 'a, In, Yield> CoroYielder< 'a, In, Yield>
{
    pub fn	Suspend( &self, val: Yield) -> In
    {
        self._Yielder.suspend( val)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

/// A fast stackful coroutine instance that implements `ICoro`.
pub struct Coro<In: 'static, Yield: 'static, Out: 'static> {
    _Coro: Coroutine<In, Yield, Out>,
    _IsDone: bool,
}

unsafe impl<In: Send + 'static, Yield: Send + 'static, Out: Send + 'static> Send for Coro<In, Yield, Out> {}
unsafe impl<In: Sync + 'static, Yield: Sync + 'static, Out: Sync + 'static> Sync for Coro<In, Yield, Out> {}

impl<In: 'static, Yield: 'static, Out: 'static> Coro<In, Yield, Out> {
    pub fn New<F>(f: F) -> Self
    where
        F: FnOnce(CoroYielder<'_, In, Yield>, In) -> Out + Send + 'static,
    {
        Self {
            _Coro: Coroutine::new(move |yielder, input| {
                let y = CoroYielder { _Yielder: yielder };
                f(y, input)
            }),
            _IsDone: false,
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<In: 'static, Yield: 'static, Out: 'static> ICoro<In, Yield, Out> for Coro<In, Yield, Out> {
    fn	Resume( &mut self, input: In) -> CoroRes< Yield, Out>
    {
        if self._IsDone {
            panic!( "Coroutine is already done");
        }

        match self._Coro.resume( input) {
            CoroutineResult::Yield( y) => CoroRes::Yield( y),
            CoroutineResult::Return( r) => {
                self._IsDone = true;
                CoroRes::Done( r)
            }
        }
    }

    fn	IsDone( &self) -> bool
    {
        self._IsDone
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[macro_export]
macro_rules! Coro {
    ( |$yielder:ident, $input:ident| $body:expr ) => {
        $crate::stalks::Coro::New( move |$yielder, $input| $body )
    };
    ( |$yielder:ident| $body:expr ) => {
        $crate::stalks::Coro::New( move |$yielder, _| $body )
    };
}

//---------------------------------------------------------------------------------------------------------------------------------
