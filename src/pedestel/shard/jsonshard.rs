//-- jsonshard.rs -----------------------------------------------------------------------------------------------------------------
use	crate::{
    ShardTree,
    flux::FieldImp,
    shard::
    { IGrammar, Parser, Str, WSpc },
    silo::
    { Stash },
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
    if !Str.Match( parser, &mut ()) {
        return false;
    }
    let  	endMark = parser.CurrMark();
    let  	slice = parser.InStream().BytesAt( mark, endMark - mark);
    let  	raw = slice.into();
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
        if nextChar.is_ascii_digit() {
            return false;
        }
    } else if ( b'1'..=b'9').contains( &firstDigit) {
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

pub struct JsonParseCtx< 'a> {
    pub _ImpStash: Stash< FieldImp< 'a>>,
}
pub type Json< 'a> = JsonParseCtx<'a>;
pub type JSon< 'a> = JsonParseCtx<'a>;

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a> JsonParseCtx<'a>
{
    pub fn	New( mut docImp: FieldImp< 'a>) -> Self {
        let  	mut stash = Stash::FromDispenser( 32_u32, 0_u32, |_| FieldImp::Null);
        stash.Push( std::mem::take( &mut docImp));
        Self { _ImpStash: stash }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct JsonGrammar;

impl JsonGrammar
{
    fn	post_value< 'a>( target: &mut FieldImp< 'a>, input: &str) -> bool {
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
            FieldImp::Str( _dst) => {
                // Reject transient parser string into long-lived &'a str without leaking.
                false
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
                    let  	mut temp: &str = input;
                    if flx.FromFieldImp( FieldImp::Str( &mut temp)) {
                        true
                    } else {
                        let  	mut temp_str = input.to_string();
                        flx.FromFieldImp( FieldImp::String( &mut temp_str))
                    }
                }
            }
            FieldImp::BoxedSink( flx) => {
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
                    let  	mut temp: &str = input;
                    if flx.FromFieldImp( FieldImp::Str( &mut temp)) {
                        true
                    } else {
                        let  	mut temp_str = input.to_string();
                        flx.FromFieldImp( FieldImp::String( &mut temp_str))
                    }
                }
            }
            FieldImp::ExpectedType( expected) => *expected == input,
            _ => false,
        }
    }

    fn	post_string< 'a>( target: &mut FieldImp< 'a>, input: &str) -> bool {
        match target {
            FieldImp::ExpectedType( expected) => *expected == input,
            FieldImp::String( dst) => {
                **dst = input.to_string();
                true
            }
            FieldImp::Str( _dst) => {
                // Reject transient parser string into long-lived &'a str without leaking.
                false
            }
            FieldImp::FluxSink( flx) => {
                let  	mut temp: &str = input;
                if flx.FromFieldImp( FieldImp::Str( &mut temp)) {
                    true
                } else {
                    let  	mut temp_str = input.to_string();
                    flx.FromFieldImp( FieldImp::String( &mut temp_str))
                }
            }
            FieldImp::BoxedSink( flx) => {
                let  	mut temp: &str = input;
                if flx.FromFieldImp( FieldImp::Str( &mut temp)) {
                    true
                } else {
                    let  	mut temp_str = input.to_string();
                    flx.FromFieldImp( FieldImp::String( &mut temp_str))
                }
            }
            _ => false,
        }
    }
    fn	MatchObject< 'a>( &self, parser: &mut Parser, ctx: &mut JsonParseCtx< 'a>) -> bool
    {
        let  	keyMark = parser.CurrMark();
        if parser.Parse( &Str, keyMark).is_none() {
            return false;
        }
        let  	keyEnd = parser.CurrMark();
        let  	keyBytes = parser.InStream().BytesAt( keyMark, keyEnd - keyMark);
        let  	s = match std::str::from_utf8( keyBytes.into()) {
            Ok( s) => s,
            Err( _) => return false,
        };
        if s.len() < 2 || !s.starts_with( '"') || !s.ends_with( '"') {
            return false;
        }
        let  	key = &s[1..s.len() - 1];
        let  	Some( unescapedKey) = UnescapeJsonString( key) else {
            return false;
        };
        let  	mut child = FieldImp::Null;
        let  	mut found = false;
        if let  	Some( top) = ctx._ImpStash.TopMut() {
            top.Resolve();
            match top {
                FieldImp::Obj( cb) | FieldImp::ObjValidated( cb, _) => {
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
        ctx._ImpStash.Stk().PushX( &mut child);
        let  	sepGrammar = ShardTree!( ?WSpc < ':' < ?WSpc);
        if parser.Parse( &sepGrammar, parser.CurrMark()).is_none() {
            let  	mut temp = FieldImp::Null;
            ctx._ImpStash.Stk().Pop( &mut temp);
            return false;
        }
        let  	valMark = parser.CurrMark();
        if !self.MatchValue( parser, ctx) {
            let  	mut temp = FieldImp::Null;
            ctx._ImpStash.Stk().Pop( &mut temp);
            return false;
        }
        let  	valEnd = parser.CurrMark();
        let  	valBytes = parser.InStream().BytesAt( valMark, valEnd - valMark);
        let  	valStrRaw = std::str::from_utf8( valBytes.into()).unwrap_or( "");
        let  	( valStr, isStringVal) = if valStrRaw.len() >= 2 && valStrRaw.starts_with( '"') && valStrRaw.ends_with( '"') {
            if let  	Some( unescaped) = UnescapeJsonString( &valStrRaw[1..valStrRaw.len() - 1]) {
                ( unescaped, true)
            } else {
                let  	mut temp = FieldImp::Null;
                ctx._ImpStash.Stk().Pop( &mut temp);
                return false;
            }
        } else {
            ( valStrRaw.trim().to_string(), false)
        };
        let  	mut posted = true;
        if let  	Some( topImp) = ctx._ImpStash.TopMut() {
            topImp.Resolve();
            if !matches!( topImp, FieldImp::Null | FieldImp::Arr( _) | FieldImp::Obj( _) | FieldImp::ObjValidated( _, _)) {
                let  	mut topVal = std::mem::replace( topImp, FieldImp::Null);
                if isStringVal {
                    posted = Self::post_string( &mut topVal, &valStr);
                } else {
                    posted = Self::post_value( &mut topVal, &valStr);
                }
                *topImp = topVal;
            }
        }
        let  	mut temp = FieldImp::Null;
        ctx._ImpStash.Stk().Pop( &mut temp);
        posted
    }

    fn	MatchJsonObject< 'a>( &self, parser: &mut Parser, ctx: &mut JsonParseCtx< 'a>) -> bool
    {
        let  	m = parser.CurrMark();
        let  	Some( nextM) = parser.Incr( m) else { return false; };
        parser.SetCurrMark( nextM);
        let  	_ = parser.Parse( &WSpc, parser.CurrMark());
        if parser.GetAt( parser.CurrMark()) == b'}' {
            let  	Some( endM) = parser.Incr( parser.CurrMark()) else { return false; };
            parser.SetCurrMark( endM);
            if let  	Some( top) = ctx._ImpStash.TopMut() {
                top.Resolve();
                return top.Validate();
            }
            return true;
        }
        loop {
            let  	_ = parser.Parse( &WSpc, parser.CurrMark());
            if !self.MatchObject( parser, ctx) {
                return false;
            }
            let  	_ = parser.Parse( &WSpc, parser.CurrMark());
            let  	c = parser.GetAt( parser.CurrMark());
            if c == b',' {
                let  	Some( nextM) = parser.Incr( parser.CurrMark()) else { return false; };
                parser.SetCurrMark( nextM);
            } else if c == b'}' {
                let  	Some( nextM) = parser.Incr( parser.CurrMark()) else { return false; };
                parser.SetCurrMark( nextM);
                break;
            } else {
                return false;
            }
        }
        if let  	Some( top) = ctx._ImpStash.TopMut() {
            top.Resolve();
            return top.Validate();
        }
        true
    }

    fn	MatchJsonArray< 'a>( &self, parser: &mut Parser, ctx: &mut JsonParseCtx< 'a>) -> bool
    {
        let  	m = parser.CurrMark();
        let  	Some( nextM) = parser.Incr( m) else { return false; };
        parser.SetCurrMark( nextM);
        let  	_ = parser.Parse( &WSpc, parser.CurrMark());
        if parser.GetAt( parser.CurrMark()) == b']' {
            let  	Some( endM) = parser.Incr( parser.CurrMark()) else { return false; };
            parser.SetCurrMark( endM);
            return true;
        }
        loop {
            let  	_ = parser.Parse( &WSpc, parser.CurrMark());
            let  	mut child = FieldImp::Null;
            let  	mut accepted = false;
            if let  	Some( top) = ctx._ImpStash.TopMut() {
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
            ctx._ImpStash.Stk().PushX( &mut child);
            let  	elemValMark = parser.CurrMark();
            if !self.MatchValue( parser, ctx) {
                let  	mut temp = FieldImp::Null;
                ctx._ImpStash.Stk().Pop( &mut temp);
                return false;
            }
            let  	elemValEnd = parser.CurrMark();
            let  	elemBytes = parser.InStream().BytesAt( elemValMark, elemValEnd - elemValMark);
            let  	s = std::str::from_utf8( elemBytes.into()).unwrap_or( "");
            let  	( valStr, isStr) = if s.len() >= 2 && s.starts_with( '"') && s.ends_with( '"') {
                if let  	Some( unescaped) = UnescapeJsonString( &s[1..s.len() - 1]) {
                    ( unescaped, true)
                } else {
                    let  	mut temp = FieldImp::Null;
                    ctx._ImpStash.Stk().Pop( &mut temp);
                    return false;
                }
            } else {
                ( s.trim().to_string(), false)
            };
            let  	mut posted = true;
            if let  	Some( topImp) = ctx._ImpStash.TopMut() {
                topImp.Resolve();
                if !matches!( topImp, FieldImp::Null | FieldImp::Arr( _) | FieldImp::Obj( _) | FieldImp::ObjValidated( _, _)) {
                    let  	mut topVal = std::mem::replace( topImp, FieldImp::Null);
                    if isStr {
                        posted = Self::post_string( &mut topVal, &valStr);
                    } else {
                        posted = Self::post_value( &mut topVal, &valStr);
                    }
                    *topImp = topVal;
                }
            }
            let  	mut temp = FieldImp::Null;
            ctx._ImpStash.Stk().Pop( &mut temp);
            if !posted {
                return false;
            }
            let  	_ = parser.Parse( &WSpc, parser.CurrMark());
            let  	c = parser.GetAt( parser.CurrMark());
            if c == b',' {
                let  	Some( nextM) = parser.Incr( parser.CurrMark()) else { return false; };
                parser.SetCurrMark( nextM);
            } else if c == b']' {
                let  	Some( nextM) = parser.Incr( parser.CurrMark()) else { return false; };
                parser.SetCurrMark( nextM);
                break;
            } else {
                return false;
            }
        }
        true
    }

    fn	MatchJsonPrimitive< 'a>( &self, parser: &mut Parser, ctx: &mut JsonParseCtx< 'a>) -> bool
    {
        let  	m = parser.CurrMark();
        let  	primitiveGrammar = ShardTree!( MatchJsonStr | "true" | "false" | "null" | MatchJsonNumber);
        let  	Some( newM) = parser.Parse( &primitiveGrammar, m) else {
            return false;
        };
        let  	bytes = parser.InStream().BytesAt( m, newM - m);
        let  	s = std::str::from_utf8( bytes.into()).unwrap_or( "");
        let  	( valStr, isStr) = if s.len() >= 2 && s.starts_with( '"') && s.ends_with( '"') {
            if let  	Some( unescaped) = UnescapeJsonString( &s[1..s.len() - 1]) {
                ( unescaped, true)
            } else {
                return false;
            }
        } else {
            ( s.trim().to_string(), false)
        };
        if ctx._ImpStash.Size() == 1
            && let  	Some( topImp) = ctx._ImpStash.TopMut()
        {
            topImp.Resolve();
            if !matches!( topImp, FieldImp::Null | FieldImp::Arr( _) | FieldImp::Obj( _) | FieldImp::ObjValidated( _, _)) {
                let  	mut topVal = std::mem::replace( topImp, FieldImp::Null);
                let  	posted = if isStr {
                    Self::post_string( &mut topVal, &valStr)
                } else {
                    Self::post_value( &mut topVal, &valStr)
                };
                *topImp = topVal;
                if !posted {
                    return false;
                }
            }
        }
        true
    }

    fn	MatchValue< 'a>( &self, parser: &mut Parser, ctx: &mut JsonParseCtx< 'a>) -> bool
    {
        let  	m = parser.CurrMark();
        let  	curr = parser.GetAt( m);
        if curr == b'{' {
            self.MatchJsonObject( parser, ctx)
        } else if curr == b'[' {
            self.MatchJsonArray( parser, ctx)
        } else {
            self.MatchJsonPrimitive( parser, ctx)
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a> IGrammar< JsonParseCtx< 'a>> for JsonGrammar
{
    fn	Match( &self, parser: &mut Parser, ctx: &mut JsonParseCtx< 'a>) -> bool
    {
        let  	_ = parser.Parse( &WSpc, parser.CurrMark());
        let  	matched = self.MatchValue( parser, ctx);
        if matched {
            let  	_ = parser.Parse( &WSpc, parser.CurrMark());
            true
        } else {
            false
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl fmt::Display for JsonGrammar
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        write!( f, "Json")
    }
}
impl fmt::Debug for JsonGrammar
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result {
        write!( f, "Json")
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
