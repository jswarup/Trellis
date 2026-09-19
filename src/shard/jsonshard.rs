//-- jsonshard.rs -----------------------------------------------------------------------------------------------------------------
use	crate::{
    ShardTree,
    flux::FieldImp,
    shard::
    { IGrammar, Parser, Str, WSpc },
    silo::
    { Arr, Stash },
};
use	std::fmt;

//---------------------------------------------------------------------------------------------------------------------------------

pub fn	UnescapeJsonString( raw: &str) -> Option< String>
{
    let  	mut chars = raw.chars().peekable();
    let  	mut out = String::with_capacity( raw.len());
    while let  	Some( ch) = chars.next() {
        if ch == '\\' {
            let  	esc = chars.next()?;
            match esc {
                '"' => out.push( '"'),
                '\\' => out.push('\\'),
                '/' => out.push( '/'),
                'b' => out.push( '\x08'),
                'f' => out.push( '\x0c'),
                'n' => out.push( '\n'),
                'r' => out.push( '\r'),
                't' => out.push( '\t'),
                'u' => {
                    let  	mut hex_str = String::with_capacity( 4);
                    for _ in 0..4 {
                        let  	h = chars.next()?;
                        if !h.is_ascii_hexdigit() {
                            return None;
                        }
                        hex_str.push( h);
                    }
                    let  	code = u32::from_str_radix( &hex_str, 16).ok()?;
                    if ( 0xD800..=0xDBFF).contains( &code) {
                        // High surrogate pair
                        if chars.next()? != '\\' || chars.next()? != 'u' {
                            return None;
                        }
                        let  	mut low_hex = String::with_capacity( 4);
                        for _ in 0..4 {
                            let  	h = chars.next()?;
                            if !h.is_ascii_hexdigit() {
                                return None;
                            }
                            low_hex.push( h);
                        }
                        let  	low_code = u32::from_str_radix( &low_hex, 16).ok()?;
                        if !( 0xDC00..=0xDFFF).contains( &low_code) {
                            return None;
                        }
                        let  	scalar = 0x10000 + ( ( ( code - 0xD800) << 10) | ( low_code - 0xDC00));
                        let  	unicode_char = char::from_u32( scalar)?;
                        out.push( unicode_char);
                    } else if ( 0xDC00..=0xDFFF).contains( &code) {
                        return None;
                    } else {
                        let  	unicode_char = char::from_u32( code)?;
                        out.push( unicode_char);
                    }
                }
                _ => return None,
            }
        } else if ( ch as u32) < 0x20 {
            // Control characters must be escaped in JSON strings
            return None;
        } else {
            out.push( ch);
        }
    }
    Some( out)
}

//---------------------------------------------------------------------------------------------------------------------------------

fn	MatchJsonStr( parser: &mut Parser) -> bool
{
    let  	mark = parser.CurrMark();
    if !Str.Match( parser) {
        return false;
    }
    let  	endMark = parser.CurrMark();
    let  	slice = parser.InStream().BytesAt( mark, endMark - mark);
    let  	raw = unsafe { std::slice::from_raw_parts( slice.Data(), slice.Size() as usize) };
    let  	Ok( s) = std::str::from_utf8( raw) else {
        return false;
    };
    if s.len() < 2 || !s.starts_with( '"') || !s.ends_with( '"') {
        return false;
    }
    UnescapeJsonString( &s[1..s.len() - 1]).is_some()
}

//---------------------------------------------------------------------------------------------------------------------------------

fn	MatchJsonNumber( parser: &mut Parser) -> bool
{
    let  	origMark = parser.CurrMark();
    let  	mut m = origMark;
    // Optional minus
    if parser.GetAt( m) == b'-' {
        let  	Some( nextM) = parser.Incr( m) else {
            return false;
        };
        m = nextM;
    }
    // Integer part: either '0' or ('1'..='9' followed by '0'..='9'*)
    let  	firstDigit = parser.GetAt( m);
    if firstDigit == b'0' {
        let  	Some( nextM) = parser.Incr( m) else {
            parser.SetCurrMark( m + 1);
            return true;
        };
        m = nextM;
        // Leading zero followed by another digit is illegal in JSON
        let  	nextChar = parser.GetAt( m);
        if nextChar >= b'0' && nextChar <= b'9' {
            return false;
        }
    } else if firstDigit >= b'1' && firstDigit <= b'9' {
        let  	Some( nextM) = parser.Incr( m) else {
            parser.SetCurrMark( m + 1);
            return true;
        };
        m = nextM;
        while parser.GetAt( m) >= b'0' && parser.GetAt( m) <= b'9' {
            if let  	Some( nextM) = parser.Incr( m) {
                m = nextM;
            } else {
                break;
            }
        }
    } else {
        return false;
    }
    // Optional fraction part: '.' followed by 1+ digits
    if parser.GetAt( m) == b'.' {
        let  	Some( nextM) = parser.Incr( m) else {
            return false;
        };
        m = nextM;
        let  	mut fracDigits = 0u32;
        while parser.GetAt( m) >= b'0' && parser.GetAt( m) <= b'9' {
            fracDigits += 1;
            if let  	Some( nextM) = parser.Incr( m) {
                m = nextM;
            } else {
                break;
            }
        }
        if fracDigits == 0 {
            return false;
        }
    }
    // Optional exponent part: 'e' or 'E' [+-] 1+ digits
    let  	expChar = parser.GetAt( m);
    if expChar == b'e' || expChar == b'E' {
        let  	Some( nextM) = parser.Incr( m) else {
            return false;
        };
        m = nextM;
        let  	sign = parser.GetAt( m);
        if sign == b'+' || sign == b'-' {
            let  	Some( nextM) = parser.Incr( m) else {
                return false;
            };
            m = nextM;
        }
        let  	mut expDigits = 0u32;
        while parser.GetAt( m) >= b'0' && parser.GetAt( m) <= b'9' {
            expDigits += 1;
            if let  	Some( nextM) = parser.Incr( m) {
                m = nextM;
            } else {
                break;
            }
        }
        if expDigits == 0 {
            return false;
        }
    }
    parser.SetCurrMark( m);
    true
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct Json< 'a> {
    _ImpStash: Stash< FieldImp< 'a>>,
}
pub type JSon< 'a> = Json<'a>;

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a> Json<'a>
{
    fn	post_value( &self, target: &mut FieldImp< 'a>, input: &str) -> bool {
        match target {
            FieldImp::U64( dst) => {
                if let  	Ok( v) = input.parse::< u64>() {
                    **dst = v;
                    true
                } else {
                    false
                }
            }
            FieldImp::F64( dst) => {
                if let  	Ok( v) = input.parse::< f64>() {
                    **dst = v;
                    true
                } else {
                    false
                }
            }
            FieldImp::Bool( dst) => {
                if let  	Ok( v) = input.parse::< bool>() {
                    **dst = v;
                    true
                } else {
                    false
                }
            }
            FieldImp::String( dst) => {
                **dst = input.to_string();
                true
            }
            FieldImp::Str( dst) => {
                let  	leaked: &'a str = Box::leak(input.to_string().into_boxed_str());
                **dst = leaked;
                true
            }
            FieldImp::FluxSink( flx) => {
                if let  	Ok( v) = input.parse::< u64>() {
                    let  	mut temp = v;
                    flx.FromFieldImp( FieldImp::U64( &mut temp))
                } else if let  	Ok( v) = input.parse::< f64>() {
                    let  	mut temp = v;
                    flx.FromFieldImp( FieldImp::F64( &mut temp))
                } else if let  	Ok( v) = input.parse::< bool>() {
                    let  	mut temp = v;
                    flx.FromFieldImp( FieldImp::Bool( &mut temp))
                } else {
                    let  	mut temp = input.to_string();
                    flx.FromFieldImp( FieldImp::String( &mut temp))
                }
            }
            _ => false,
        }
    }
    fn	post_string( &self, target: &mut FieldImp< 'a>, input: &str) -> bool {
        match target {
            FieldImp::String( dst) => {
                **dst = input.to_string();
                true
            }
            FieldImp::Str( dst) => {
                let  	leaked: &'a str = Box::leak(input.to_string().into_boxed_str());
                **dst = leaked;
                true
            }
            FieldImp::FluxSink( flx) => {
                let  	mut temp = input.to_string();
                flx.FromFieldImp( FieldImp::String( &mut temp))
            }
            _ => false,
        }
    }
    pub fn	New( mut docImp: FieldImp< 'a>) -> Self {
        let  	mut json = Self {
            _ImpStash: Stash::FromDispenser( 32_u32, 0_u32, |_| FieldImp::Null),
        };
        json._ImpStash.Push( std::mem::take( &mut docImp));
        json
    }
    fn	MatchObject( &self, parser: &mut Parser) -> bool
    {
        let  	mut unescapedKey = String::new();
        let  	objectName = |arr: Arr< u8>| {
            let  	s = match std::str::from_utf8( unsafe {
                std::slice::from_raw_parts( arr.Data(), arr.Size() as usize)
            })
            {
                Ok( s) => s,
                Err( _) => return false,
            };
            if s.len() < 2 || !s.starts_with( '"') || !s.ends_with( '"') {
                return false;
            }
            let  	key = &s[1..s.len() - 1];
            let  	Some( unescaped) = UnescapeJsonString( key) else {
                return false;
            };
            unescapedKey = unescaped;
            let  	mut child = FieldImp::Null;
            let  	mut found = false;
            if let  	Some( top) = self._ImpStash.TopMut() {
                top.Resolve();
                match top {
                    FieldImp::Obj( cb) => {
                        found = cb( &unescapedKey, &mut child);
                    }
                    FieldImp::Null => {
                        found = true;
                    }
                    _ => {}
                }
            }
            if !found {
                return false;
            }
            self._ImpStash.Stk().PushX( &mut child);
            true
        };
        let  	mut valStr = String::new();
        let  	mut isStringVal = false;
        let  	objectValue = |arr: Arr< u8>| {
            let  	s = std::str::from_utf8( unsafe {
                std::slice::from_raw_parts( arr.Data(), arr.Size() as usize)
            })
            .unwrap();
            if s.len() >= 2 && s.starts_with( '"') && s.ends_with( '"') {
                isStringVal = true;
                if let  	Some( unescaped) = UnescapeJsonString( &s[1..s.len() - 1]) {
                    valStr = unescaped;
                    true
                } else {
                    false
                }
            } else {
                isStringVal = false;
                valStr = s.to_string();
                true
            }
        };
        let  	objShard = ShardTree!( Str[ objectName] < ?WSpc < ':' < ?WSpc < ( |p: &mut Parser| self.MatchValue( p) )[ objectValue]);
        let  	Some( newM) = parser.ParseGrammar( &objShard, parser.CurrMark()) else {
            return false;
        };
        parser.SetCurrMark( newM);
        let  	mut posted = true;
        if let  	Some( topImp) = self._ImpStash.TopMut() {
            topImp.Resolve();
            if !matches!( topImp, FieldImp::Null | FieldImp::Arr( _) | FieldImp::Obj( _)) {
                let  	mut topVal = std::mem::replace( topImp, FieldImp::Null);
                if isStringVal {
                    posted = self.post_string( &mut topVal, &valStr);
                } else {
                    posted = self.post_value( &mut topVal, &valStr);
                }
                *topImp = topVal;
            }
        }
        let  	mut temp = FieldImp::Null;
        self._ImpStash.Stk().Pop( &mut temp);
        posted
    }
    fn	MatchValue( &self, parser: &mut Parser) -> bool
    {
        let  	objShard = ShardTree!( '{' < ?( ?WSpc < ( |p: &mut Parser| self.MatchObject( p) ) < *( ?WSpc < ',' < ?WSpc < ( |p: &mut Parser| self.MatchObject( p) )) ) < ?WSpc < '}');
        let  	arrElement = |p: &mut Parser| {
            let  	mut child = FieldImp::Null;
            let  	mut accepted = false;
            if let  	Some( top) = self._ImpStash.TopMut() {
                top.Resolve();
                match top {
                    FieldImp::Arr( cb) => {
                        accepted = cb( &mut child);
                    }
                    FieldImp::Null => {
                        accepted = true;
                    }
                    _ => {}
                }
            }
            if !accepted {
                return false;
            }
            self._ImpStash.Stk().PushX( &mut child);
            let  	elemValue = |arr: Arr< u8>| {
                let  	s = std::str::from_utf8( unsafe {
                    std::slice::from_raw_parts( arr.Data(), arr.Size() as usize)
                })
                .unwrap();
                let  	( valStr, isStr) = if s.len() >= 2 && s.starts_with( '"') && s.ends_with( '"') {
                    if let  	Some( unescaped) = UnescapeJsonString( &s[1..s.len() - 1]) {
                        ( unescaped, true)
                    } else {
                        return false;
                    }
                } else {
                    ( s.to_string(), false)
                };
                let  	mut posted = true;
                if let  	Some( topImp) = self._ImpStash.TopMut() {
                    topImp.Resolve();
                    if !matches!( topImp, FieldImp::Null | FieldImp::Arr( _) | FieldImp::Obj( _)) {
                        let  	mut topVal = std::mem::replace( topImp, FieldImp::Null);
                        if isStr {
                            posted = self.post_string( &mut topVal, &valStr);
                        } else {
                            posted = self.post_value( &mut topVal, &valStr);
                        }
                        *topImp = topVal;
                    }
                }
                posted
            };
            let  	elemShard = ShardTree!( 
                ( MatchJsonStr | "true" | "false" | "null" | MatchJsonNumber)[elemValue]
                    | ( |p: &mut Parser| self.MatchValue( p))
            );
            let  	res = p.ParseGrammar( &elemShard, p.CurrMark());
            let  	mut temp = FieldImp::Null;
            self._ImpStash.Stk().Pop( &mut temp);
            res.is_some()
        };
        let  	arrShard = ShardTree!( '[' < ?( ?WSpc < ( arrElement ) < *( ?WSpc < ',' < ?WSpc < ( arrElement )) ) < ?WSpc < ']');
        let  	keyShard = ShardTree!( MatchJsonStr | "true" | "false" | "null" | MatchJsonNumber);
        let  	valShard = ShardTree!( keyShard | arrShard | objShard);
        let  	Some( newM) = parser.ParseGrammar( &valShard, parser.CurrMark()) else {
            return false;
        };
        parser.SetCurrMark( newM);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a> IGrammar for Json<'a>
{
    fn	Match( &self, parser: &mut Parser) -> bool
    {
        let  	m = parser.CurrMark();
        let  	jsonhard = ShardTree!( ?WSpc < ( |p: &mut Parser| self.MatchValue( p) ) < ?WSpc);
        let  	Some( newM) = parser.ParseGrammar( &jsonhard, m) else {
            return false;
        };
        parser.SetCurrMark( newM);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a> fmt::Display for Json<'a>
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        write!( f, "Json")
    }
}
impl< 'a> fmt::Debug for Json<'a>
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        write!( f, "Json")
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
