//-- actionshard.rs -----------------------------------------------------------------------------------------------------------------

use std::fmt;

use crate::{
    shard::{IGrammar, Parser},
    silo::{Arr, cast::IConstPtrMutRefExt},
    stalks::UniNode,
};

//---------------------------------------------------------------------------------------------------------------------------------

pub struct ActionOp<W> {
    _Action: W,
}

impl<W> ActionOp<W> {
    pub fn New(action: W) -> Self {
        Self { _Action: action }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub type ActionShard<C, W> = UniNode<C, ActionOp<W>>;

//---------------------------------------------------------------------------------------------------------------------------------

pub trait INotify {
    fn DoNotify(&mut self, matched: Arr<'_, u8>) -> bool;
}

impl<F> INotify for F
where
    F: for<'a> FnMut(Arr<'a, u8>) -> bool,
{
    fn DoNotify(&mut self, matched: Arr<'_, u8>) -> bool {
        self(matched)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub fn Coerce<F>(f: F) -> F
where
    F: INotify,
{
    f
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<C, W> IGrammar for UniNode<C, ActionOp<W>>
where
    C: IGrammar,
    W: INotify,
{
    fn Match(&self, parser: &mut Parser) -> bool {
        let m = parser.CurrMark();
        let res = parser.ParseGrammar(&self._Child, m);

        if let Some(completedMark) = res {
            let actionPtr = &self._Op._Action as *const W;
            let actionMut = actionPtr.MutRef();
            let arr = parser.InStream().BytesAt(m, completedMark - m);
            actionMut.DoNotify(arr)
        } else {
            false
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<C, W> fmt::Display for UniNode<C, ActionOp<W>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Action")
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<C, W> fmt::Debug for UniNode<C, ActionOp<W>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
