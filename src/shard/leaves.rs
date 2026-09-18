//-- leaves.rs -------------------------------------------------------------------------------------------------------------------------

use crate::shard::Parser;

use crate::shard::{Charset, IGrammar};

//---------------------------------------------------------------------------------------------------------------------------------

//---------------------------------------------------------------------------------------------------------------------------------

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> IGrammar for &'a str {
    fn Match(&self, parser: &mut Parser) -> bool {
        (**self).Match(parser)
    }
}

impl<'a> IGrammar for &'a char {
    fn Match(&self, parser: &mut Parser) -> bool {
        (**self).Match(parser)
    }
}

impl<'a> IGrammar for &'a Charset {
    fn Match(&self, parser: &mut Parser) -> bool {
        (**self).Match(parser)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<F> IGrammar for F
where
    F: Fn(&mut Parser) -> bool,
{
    fn Match(&self, parser: &mut Parser) -> bool {
        self(parser)
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl IGrammar for Charset {
    fn Match(&self, parser: &mut Parser) -> bool {
        let mark = parser.CurrMark();
        if mark >= parser.InStream().Size() {
            return false;
        }
        let curr = parser.GetAt(mark);
        if self.Get(curr) {
            parser.SetCurrMark(mark + 1);
            true
        } else {
            false
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl IGrammar for char {
    fn Match(&self, parser: &mut Parser) -> bool {
        let mark = parser.CurrMark();
        if mark >= parser.InStream().Size() {
            return false;
        }
        let curr = parser.GetAt(mark);
        if curr == (*self as u8) {
            parser.SetCurrMark(mark + 1);
            true
        } else {
            false
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl IGrammar for str {
    fn Match(&self, parser: &mut Parser) -> bool {
        let mark = parser.CurrMark();
        let key = self.as_bytes();
        let mut currentMark = mark;

        for &b in key {
            let stream = parser.InStream();
            let curr = stream.At(currentMark);
            if curr != b {
                return false;
            }
            if let Some(next) = parser.Incr(currentMark) {
                currentMark = next;
            } else {
                return false;
            }
        }

        parser.SetCurrMark(currentMark);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct Str {}

#[allow(non_upper_case_globals)]
pub const Str: Str = Str {};

impl IGrammar for Str {
    fn Match(&self, parser: &mut Parser) -> bool {
        let mark = parser.CurrMark();
        let mut m = mark;
        let curr = parser.GetAt(m);
        if curr != (b'"' as u8) {
            return false;
        }

        if let Some(next) = parser.Incr(m) {
            m = next;
            let mut escape = false;
            loop {
                let c = parser.GetAt(m);
                if c == (0 as u8) && m >= parser.InStream().Size() {
                    return false;
                }

                if escape {
                    escape = false;
                } else if c == (b'\\' as u8) {
                    escape = true;
                } else if c == (b'"' as u8) {
                    if let Some(nxt) = parser.Incr(m) {
                        parser.SetCurrMark(nxt);
                        return true;
                    } else {
                        return false;
                    }
                }

                if let Some(nxt) = parser.Incr(m) {
                    m = nxt;
                } else {
                    return false;
                }
            }
        }
        false
    }
}
