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

pub struct JSon<'a> {
    pub _ImpStash: Stash<FieldImp<'a>>,
}

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> JSon<'a> {
    pub fn New(mut docImp: FieldImp<'a>) -> Self {
        let mut json = Self {
            _ImpStash: Stash::FromDispenser(32_u32, 0_u32, |_| FieldImp::Null),
        };
        json._ImpStash.Push(std::mem::take(&mut docImp));
        json
    }

    fn MatchObject(&self, parser: &mut Parser) -> bool {
        let mut objArr = Arr::<u8>::Empty();
        let objectName = |arr: Arr<u8>| {
            objArr = unsafe { std::mem::transmute(arr) };
            let mut child = FieldImp::Null;
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
                    cb(key, &mut child);
                }
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

        if let Some(topImp) = self._ImpStash.TopMut() {
            topImp.Resolve();
            if !matches!(topImp, FieldImp::Null) {
                let topVal = std::mem::replace(topImp, FieldImp::Null);
                topVal.PostParsed(valStr);
            }
        }

        let mut temp = FieldImp::Null;
        self._ImpStash.Stk().Pop(&mut temp);

        true
    }

    fn MatchValue(&self, parser: &mut Parser) -> bool {
        let objShard = ShardTree!( '{' < *(?WSpc < ( |p: &mut Parser| self.MatchObject(p) ) < ? ( ',' < ?WSpc)) < ?WSpc < '}');

        let arrElement = |p: &mut Parser| {
            let checkEnd = ShardTree!( ?WSpc < ']' );
            if p.ParseGrammar(&checkEnd, p.CurrMark()).is_some() {
                return false;
            }

            let mut child = FieldImp::Null;
            if let Some(top) = self._ImpStash.TopMut() {
                top.Resolve();
                if let FieldImp::Arr(cb) = top {
                    cb(&mut child);
                }
            }
            self._ImpStash.Stk().PushX(&mut child);

            let elemValue = |mut arr: Arr<u8>| {
                println!(
                    "DEBUG ELEM MATCHED: {:?}",
                    std::str::from_utf8(unsafe {
                        std::slice::from_raw_parts(arr.Data(), arr.Size() as usize)
                    })
                    .unwrap()
                );
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
                if let Some(topImp) = self._ImpStash.TopMut() {
                    topImp.Resolve();
                    if !matches!(topImp, FieldImp::Null) {
                        let topVal = std::mem::replace(topImp, FieldImp::Null);
                        topVal.PostParsed(s);
                    }
                }
                true
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

impl<'a> IGrammar for JSon<'a> {
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

impl<'a> fmt::Display for JSon<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Json")
    }
}
impl<'a> fmt::Debug for JSon<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Json")
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
