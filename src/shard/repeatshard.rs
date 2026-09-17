//-- repeatshard.rs -----------------------------------------------------------------------------------------------------------------

use crate::shard::Parser;
use crate::{shard::IGrammar, silo::USeg, stalks::UniNode};
use std::fmt;

//---------------------------------------------------------------------------------------------------------------------------------

pub type RepeatShard<C> = UniNode<C, USeg>;

//---------------------------------------------------------------------------------------------------------------------------------

impl<C> IGrammar for UniNode<C, USeg>
where
    C: IGrammar,
{
    fn Match(&self, parser: &mut Parser) -> bool {
        let mut count = (0 as u32);
        let first = self._Op.First();
        let last = if self._Op.IsEmpty() {
            u32::MAX
        } else {
            self._Op.Last()
        };

        let mut m = parser.CurrMark();

        while count < last {
            let res = parser.ParseGrammar(&self._Child, m);
            if let Some(newM) = res {
                if newM == m {
                    count += (1 as u32);
                    break;
                }
                m = newM;
                count += (1 as u32);
            } else {
                break;
            }
        }

        if count >= first {
            parser.SetCurrMark(m);
            true
        } else {
            false
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<C> fmt::Display for UniNode<C, USeg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Repeat( {:?})", self._Op)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<C> fmt::Debug for UniNode<C, USeg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
