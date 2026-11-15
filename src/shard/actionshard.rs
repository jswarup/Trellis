//-- actionshard.rs -----------------------------------------------------------------------------------------------------------------
use	crate::{
    shard::
    { IGrammar, Parser },
    silo::Arr,
    stalks::UniNode,
};
use	std::fmt;

//---------------------------------------------------------------------------------------------------------------------------------

pub struct ActionOp< W>
{
    pub _Action: W,
}
impl< W> ActionOp< W>
{
    pub fn	New( action: W) -> Self
    {
        Self { _Action: action }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct ActionCtxOp< W>
{
    pub _Action: W,
}
impl< W> ActionCtxOp< W>
{
    pub fn	New( action: W) -> Self
    {
        Self { _Action: action }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub type ActionShard< C, W> = UniNode< C, ActionOp< W>>;
pub type ActionCtxShard< C, W> = UniNode< C, ActionCtxOp< W>>;

//---------------------------------------------------------------------------------------------------------------------------------

pub fn	Coerce< F>( f: F) -> F
where
    F: Fn( Arr< '_, u8>) -> bool,
{
    f
}

//---------------------------------------------------------------------------------------------------------------------------------

pub fn	CoerceCtx< Ctx, F>( f: F) -> F
where
    F: Fn( Arr< '_, u8>, &mut Ctx) -> bool,
{
    f
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< Ctx, C, W> IGrammar< Ctx> for UniNode< C, ActionOp< W>>
where
    C: IGrammar< Ctx>,
    W: Fn( Arr< '_, u8>) -> bool,
{
    fn	Match( &self, parser: &mut Parser, ctx: &mut Ctx) -> bool
    {
        let  	m = parser.CurrMark();
        let  	res = parser.ParseGrammar( &self._Child, ctx, m);
        if let  	Some( completedMark) = res {
            let  	arr = parser.InStream().BytesAt( m, completedMark - m);
            ( self._Op._Action)( arr)
        } else {
            false
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< Ctx, C, W> IGrammar< Ctx> for UniNode< C, ActionCtxOp< W>>
where
    C: IGrammar< Ctx>,
    W: Fn( Arr< '_, u8>, &mut Ctx) -> bool,
{
    fn	Match( &self, parser: &mut Parser, ctx: &mut Ctx) -> bool
    {
        let  	m = parser.CurrMark();
        let  	res = parser.ParseGrammar( &self._Child, ctx, m);
        if let  	Some( completedMark) = res {
            let  	arr = parser.InStream().BytesAt( m, completedMark - m);
            ( self._Op._Action)( arr, ctx)
        } else {
            false
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< C, W> fmt::Display for UniNode< C, ActionOp< W>> {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        write!( f, "Action")
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< C, W> fmt::Debug for UniNode< C, ActionOp< W>> {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        fmt::Display::fmt( self, f)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< C, W> fmt::Display for UniNode< C, ActionCtxOp< W>> {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        write!( f, "ActionCtx")
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< C, W> fmt::Debug for UniNode< C, ActionCtxOp< W>> {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        fmt::Display::fmt( self, f)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
