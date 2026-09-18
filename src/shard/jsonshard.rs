//-- jsonshard.rs -----------------------------------------------------------------------------------------------------------------

use crate::shard::numbers::Real;
use crate::{
    ShardTree,
    flux::FieldImp,
    shard::{IGrammar, Parser, WSpc},
    silo::{Arr, Stash},
};
use std::fmt;

//---------------------------------------------------------------------------------------------------------------------------------

pub struct Json<'a> {
    _ImpStash: Stash<FieldImp<'a>>,
}

pub type JSon<'a> = Json<'a>;

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> Json<'a> {
    pub fn New(mut docImp: FieldImp<'a>) -> Self {
        let mut json = Self {
            _ImpStash: Stash::FromDispenser(32_u32, 0_u32, |_| FieldImp::Null),
        };
        json._ImpStash.Push(std::mem::take(&mut docImp));
        json
    }

    fn MatchObject(&self, parser: &mut Parser) -> bool {
        let objectName = |arr: Arr<u8>| {
            let mut child = FieldImp::Null;
            let mut found = false;
            if let Some(top) = self._ImpStash.TopMut() {
                top.Resolve();
                if let FieldImp::Obj(cb) = top {
                    let mut key = std::str::from_utf8(unsafe {
                        std::slice::from_raw_parts(arr.Data(), arr.Size() as usize)
                    })
                    .unwrap();
                    if key.starts_with('"') && key.ends_with('"') {
                        key = &key[1..key.len() - 1];
                    }
                    found = cb(key, &mut child);
                }
            }
            if !found {
                return false;
            }
            self._ImpStash.Stk().PushX(&mut child);
            true
        };
        let mut valStr = "";
        let objectValue = |mut arr: Arr<u8>| {
            if (arr.Size() >= 2)
                && (*arr.First().unwrap() == b'"')
                && (*arr.Last().unwrap() == b'"')
            {
                arr = arr.LSnip(1).RSnip(1);
            }
            valStr = std::str::from_utf8(unsafe {
                std::slice::from_raw_parts(arr.Data(), arr.Size() as usize)
            })
            .unwrap();
            true
        };
        let objShard = ShardTree!( Str[ objectName] < ?WSpc < ':' < ?WSpc < ( |p: &mut Parser| self.MatchValue( p) )[ objectValue]);

        let Some(newM) = parser.ParseGrammar(&objShard, parser.CurrMark()) else {
            return false;
        };
        parser.SetCurrMark(newM);

        let mut posted = true;
        if let Some(topImp) = self._ImpStash.TopMut() {
            topImp.Resolve();
            if !matches!(topImp, FieldImp::Null) {
                let topVal = std::mem::replace(topImp, FieldImp::Null);
                posted = topVal.PostParsed(valStr);
            }
        }

        let mut temp = FieldImp::Null;
        self._ImpStash.Stk().Pop(&mut temp);

        posted
    }

    fn MatchValue(&self, parser: &mut Parser) -> bool {
        let objShard = ShardTree!( '{' < *(?WSpc < ( |p: &mut Parser| self.MatchObject(p) ) < ? ( ',' < ?WSpc)) < ?WSpc < '}');

        let arrElement = |p: &mut Parser| {
            let checkEnd = ShardTree!( ?WSpc < ']' );
            if p.ParseGrammar(&checkEnd, p.CurrMark()).is_some() {
                return false;
            }

            let mut child = FieldImp::Null;
            let mut accepted = false;
            if let Some(top) = self._ImpStash.TopMut() {
                top.Resolve();
                if let FieldImp::Arr(cb) = top {
                    accepted = cb(&mut child);
                }
            }
            if !accepted {
                return false;
            }
            self._ImpStash.Stk().PushX(&mut child);

            let elemValue = |mut arr: Arr<u8>| {
                if (arr.Size() >= 2)
                    && (*arr.First().unwrap() == b'"')
                    && (*arr.Last().unwrap() == b'"')
                {
                    arr = arr.LSnip(1).RSnip(1);
                }
                let s = std::str::from_utf8(unsafe {
                    std::slice::from_raw_parts(arr.Data(), arr.Size() as usize)
                })
                .unwrap();
                let mut posted = true;
                if let Some(topImp) = self._ImpStash.TopMut() {
                    topImp.Resolve();
                    if !matches!(topImp, FieldImp::Null) {
                        let topVal = std::mem::replace(topImp, FieldImp::Null);
                        posted = topVal.PostParsed(s);
                    }
                }
                posted
            };

            let elemShard = ShardTree!(
                (Str | "true" | "false" | "null" | Real)[elemValue]
                    | (|p: &mut Parser| self.MatchValue(p))
            );
            let res = p.ParseGrammar(&elemShard, p.CurrMark());

            let mut temp = FieldImp::Null;
            self._ImpStash.Stk().Pop(&mut temp);
            res.is_some()
        };

        let arrShard =
            ShardTree!( '[' < *(?WSpc < ( arrElement ) < ? ( ',' < ?WSpc)) < ?WSpc < ']');
        let keyShard = ShardTree!(Str | "true" | "false" | "null" | Real);
        let valShard = ShardTree!(keyShard | arrShard | objShard);

        let Some(newM) = parser.ParseGrammar(&valShard, parser.CurrMark()) else {
            return false;
        };
        parser.SetCurrMark(newM);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> IGrammar for Json<'a> {
    fn Match(&self, parser: &mut Parser) -> bool {
        let m = parser.CurrMark();
        let jsonhard = ShardTree!( ?WSpc < ( |p: &mut Parser| self.MatchValue(p) ) < ?WSpc);
        let Some(newM) = parser.ParseGrammar(&jsonhard, m) else {
            return false;
        };
        parser.SetCurrMark(newM);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> fmt::Display for Json<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Json")
    }
}
impl<'a> fmt::Debug for Json<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Json")
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
