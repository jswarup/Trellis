//-- numbers.rs -----------------------------------------------------------------------------------------------------------------------

use std::fmt;

use crate::shard::{IGrammar, Parser};

//---------------------------------------------------------------------------------------------------------------------------------

macro_rules! ImplNumberShard {
    ( $shard:ident, $cnst:ident, $label:literal ) => {
        pub struct $shard;
        pub const $cnst: $shard = $shard;

        impl fmt::Display for $shard {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{ }", $label)
            }
        }
        impl fmt::Debug for $shard {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{ }", $label)
            }
        }
    };
}

//---------------------------------------------------------------------------------------------------------------------------------

fn MatchSign(parser: &mut Parser, m: u32) -> u32 {
    let curr = parser.GetAt(m);
    if curr == (b'-' as u8) || curr == (b'+' as u8) {
        parser.Incr(m).unwrap_or(m)
    } else {
        m
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

fn MatchDecDigits(parser: &mut Parser, mut m: u32) -> (u32, bool) {
    let mut matched = false;
    loop {
        let curr = parser.GetAt(m);
        if curr >= (b'0' as u8) && curr <= (b'9' as u8) {
            matched = true;
            if let Some(nextM) = parser.Incr(m) {
                m = nextM;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    (m, matched)
}

//---------------------------------------------------------------------------------------------------------------------------------

fn MatchHexDigits(parser: &mut Parser, mut m: u32) -> (u32, bool) {
    let mut matched = false;
    loop {
        let curr = parser.GetAt(m);
        if (curr >= (b'0' as u8) && curr <= (b'9' as u8))
            || (curr >= (b'a' as u8) && curr <= (b'f' as u8))
            || (curr >= (b'A' as u8) && curr <= (b'F' as u8))
        {
            matched = true;
            if let Some(nextM) = parser.Incr(m) {
                m = nextM;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    (m, matched)
}

//---------------------------------------------------------------------------------------------------------------------------------

fn MatchHexPrefix(parser: &mut Parser, m: u32) -> Option<u32> {
    if parser.GetAt(m) != (b'0' as u8) {
        return None;
    }
    let m = parser.Incr(m)?;
    let curr = parser.GetAt(m);
    if curr != (b'x' as u8) && curr != (b'X' as u8) {
        return None;
    }
    parser.Incr(m)
}

//---------------------------------------------------------------------------------------------------------------------------------

ImplNumberShard!(UIntShard, UInt, "UInt");

//---------------------------------------------------------------------------------------------------------------------------------

impl IGrammar for UIntShard {
    fn Match(&self, parser: &mut Parser) -> bool {
        let origMark = parser.CurrMark();
        let (m, matched) = MatchDecDigits(parser, origMark);
        if !matched {
            return false;
        }
        let _bytes = parser.InStream().BytesAt(origMark, (m - origMark as u32));
        parser.SetCurrMark(m);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

ImplNumberShard!(IntShard, Int, "Int");

//---------------------------------------------------------------------------------------------------------------------------------

impl IGrammar for IntShard {
    fn Match(&self, parser: &mut Parser) -> bool {
        let origMark = parser.CurrMark();
        let mSign = MatchSign(parser, origMark);
        let (m, matched) = MatchDecDigits(parser, mSign);
        if !matched {
            return false;
        }
        parser.SetCurrMark(m);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

ImplNumberShard!(HexShard, Hex, "Hex");

//---------------------------------------------------------------------------------------------------------------------------------

impl IGrammar for HexShard {
    fn Match(&self, parser: &mut Parser) -> bool {
        let origMark = parser.CurrMark();
        let m = MatchSign(parser, origMark);
        // Advance past optional 0x/0X prefix
        let mDigits = MatchHexPrefix(parser, m).unwrap_or(m);
        let (m, matched) = MatchHexDigits(parser, mDigits);
        if !matched {
            return false;
        }
        parser.SetCurrMark(m);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

ImplNumberShard!(RealShard, Real, "Real");

//---------------------------------------------------------------------------------------------------------------------------------

impl IGrammar for RealShard {
    fn Match(&self, parser: &mut Parser) -> bool {
        let origMark = parser.CurrMark();
        let mut m = MatchSign(parser, origMark);
        let mut matchedDigits = false;
        let (nextM, d) = MatchDecDigits(parser, m);
        if d {
            m = nextM;
            matchedDigits = true;
        }
        if parser.GetAt(m) == (b'.' as u8) {
            if let Some(nextM) = parser.Incr(m) {
                m = nextM;
                let (nextM, d) = MatchDecDigits(parser, m);
                if d {
                    m = nextM;
                    matchedDigits = true;
                }
            }
        }
        if !matchedDigits {
            return false;
        }
        // Optional exponent
        let curr = parser.GetAt(m);
        if curr == (b'e' as u8) || curr == (b'E' as u8) {
            if let Some(nextM) = parser.Incr(m) {
                m = nextM;
                let curr = parser.GetAt(m);
                if curr == (b'-' as u8) || curr == (b'+' as u8) {
                    if let Some(nextM) = parser.Incr(m) {
                        m = nextM;
                    }
                }
                let (nextM, matched) = MatchDecDigits(parser, m);
                if !matched {
                    return false;
                }
                m = nextM;
            }
        }
        parser.SetCurrMark(m);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
