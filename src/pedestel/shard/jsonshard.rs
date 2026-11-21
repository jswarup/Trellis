//-- jsonshard.rs -----------------------------------------------------------------------------------------------------------------
use	crate::{
    ShardTree,
    flux::FieldImp,
    shard::
    { IGrammar, Parser, Str, WSpc },
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

/// Owns one import descriptor, consumed on success or failure.
pub struct JsonParseCtx<'a>
{
    _Root: FieldImp<'a>,
}
pub type Json< 'a> = JsonParseCtx<'a>;
pub type JSon< 'a> = JsonParseCtx<'a>;

//---------------------------------------------------------------------------------------------------------------------------------

impl<'a> JsonParseCtx<'a>
{
    pub fn New(docImp: FieldImp<'a>) -> Self { return Self { _Root: docImp }; }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct JsonGrammar;

const K_MAX_JSON_DEPTH: u32 = 128;

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
    fn MatchChild(&self, parser: &mut Parser, mut child: FieldImp<'_>, depth: u32) -> bool
    {
        if !self.MatchValue(parser, &mut child, depth) {
            return false;
        }
        return child.Commit().is_ok();
    }

    fn MatchObject(&self, parser: &mut Parser, target: &mut FieldImp<'_>, depth: u32) -> bool
    {
        let keyMark = parser.CurrMark();
        if parser.Parse(&Str, keyMark).is_none() {
            return false;
        }
        let keyEnd = parser.CurrMark();
        let keyBytes = parser.InStream().BytesAt(keyMark, keyEnd - keyMark);
        let Ok(rawKey) = std::str::from_utf8(keyBytes.into()) else {
            return false;
        };
        if rawKey.len() < 2 || !rawKey.starts_with('"') || !rawKey.ends_with('"') {
            return false;
        }
        let Some(key) = UnescapeJsonString(&rawKey[1..rawKey.len() - 1]) else {
            return false;
        };
        let sepGrammar = ShardTree!( ?WSpc < ':' < ?WSpc);
        if parser.Parse( &sepGrammar, parser.CurrMark()).is_none() {
            return false;
        }
        let child = match target {
            FieldImp::Obj(importer) => {
                match importer.Field(&key) {
                    Ok(child) => child,
                    Err(_) => return false,
                }
            }
            FieldImp::Null => FieldImp::Null,
            _ => return false,
        };
        // The child is consumed before another borrow of the parent is possible.
        return self.MatchChild(parser, child, depth + 1);
    }

    fn MatchJsonObject(&self, parser: &mut Parser, target: &mut FieldImp<'_>, depth: u32) -> bool
    {
        if !matches!(target, FieldImp::Null | FieldImp::Obj(_)) {
            return false;
        }
        let mark = parser.CurrMark();
        let Some(next) = parser.Incr(mark) else {
            return false;
        };
        parser.SetCurrMark(next);
        let _ = parser.Parse(&WSpc, parser.CurrMark());
        if parser.GetAt( parser.CurrMark()) == b'}' {
            let Some( end) = parser.Incr( parser.CurrMark()) else { return false; };
            parser.SetCurrMark( end);
            return true;
        }
        loop {
            let _ = parser.Parse(&WSpc, parser.CurrMark());
            if !self.MatchObject(parser, target, depth) {
                return false;
            }
            let _ = parser.Parse(&WSpc, parser.CurrMark());
            let current = parser.GetAt(parser.CurrMark());
            let Some(next) = parser.Incr(parser.CurrMark()) else {
                return false;
            };
            if current == b'}' {
                parser.SetCurrMark(next);
                return true;
            }
            if current != b',' {
                return false;
            }
            parser.SetCurrMark(next);
        }
    }

    fn MatchJsonArray(&self, parser: &mut Parser, target: &mut FieldImp<'_>, depth: u32) -> bool
    {
        if !matches!(target, FieldImp::Null | FieldImp::Arr(_)) {
            return false;
        }
        let mark = parser.CurrMark();
        let Some(next) = parser.Incr(mark) else {
            return false;
        };
        parser.SetCurrMark(next);
        let _ = parser.Parse(&WSpc, parser.CurrMark());
        if parser.GetAt( parser.CurrMark()) == b']' {
            let Some( end) = parser.Incr( parser.CurrMark()) else { return false; };
            parser.SetCurrMark( end);
            return true;
        }
        loop {
            let _ = parser.Parse(&WSpc, parser.CurrMark());
            let child = match target {
                FieldImp::Arr(importer) => {
                    match importer.Next() {
                        Ok(child) => child,
                        Err(_) => return false,
                    }
                }
                FieldImp::Null => FieldImp::Null,
                _ => return false,
            };
            if !self.MatchChild(parser, child, depth + 1) {
                return false;
            }
            let _ = parser.Parse(&WSpc, parser.CurrMark());
            let current = parser.GetAt(parser.CurrMark());
            let Some(next) = parser.Incr(parser.CurrMark()) else {
                return false;
            };
            if current == b']' {
                parser.SetCurrMark(next);
                return true;
            }
            if current != b',' {
                return false;
            }
            parser.SetCurrMark(next);
        }
    }

    fn MatchJsonPrimitive(&self, parser: &mut Parser, target: &mut FieldImp<'_>) -> bool
    {
        let mark = parser.CurrMark();
        let grammar = ShardTree!(MatchJsonStr | "true" | "false" | "null" | MatchJsonNumber);
        let Some(end) = parser.Parse(&grammar, mark) else {
            return false;
        };
        let bytes = parser.InStream().BytesAt(mark, end - mark);
        let Ok(raw) = std::str::from_utf8(bytes.into()) else {
            return false;
        };
        if matches!(target, FieldImp::Null) {
            return true;
        }
        if raw == "null" {
            return match target {
                FieldImp::FluxSink(sink) => sink.TryFromFieldImp(FieldImp::Null).is_ok(),
                FieldImp::BoxedSink(sink) => sink.TryFromFieldImp(FieldImp::Null).is_ok(),
                _ => false,
            };
        }
        if raw.starts_with('"') && raw.ends_with('"') {
            let Some(text) = UnescapeJsonString(&raw[1..raw.len() - 1]) else {
                return false;
            };
            return Self::post_string(target, &text);
        }
        return Self::post_value(target, raw);
    }

    fn MatchValue(&self, parser: &mut Parser, target: &mut FieldImp<'_>, depth: u32) -> bool
    {
        if depth >= K_MAX_JSON_DEPTH {
            return false;
        }
        target.Resolve();
        return match parser.GetAt(parser.CurrMark()) {
            b'{' => self.MatchJsonObject(parser, target, depth),
            b'[' => self.MatchJsonArray(parser, target, depth),
            _ => self.MatchJsonPrimitive(parser, target),
        };
    }

    fn MatchRoot(&self, parser: &mut Parser, ctx: &mut JsonParseCtx<'_>, completeInput: bool)
                 -> bool
    {
        let mut root = std::mem::take(&mut ctx._Root);
        let _ = parser.Parse(&WSpc, parser.CurrMark());
        if !self.MatchValue(parser, &mut root, 0) {
            return false;
        }
        let _ = parser.Parse(&WSpc, parser.CurrMark());
        // Complete-document entry points report trailing data without publishing staging.
        if completeInput && parser.CurrMark() != parser.InStream().Size() {
            return true;
        }
        return root.Commit().is_ok();
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

struct JsonDocumentGrammar;

impl<'a> IGrammar<JsonParseCtx<'a>> for JsonDocumentGrammar
{
    fn Match(&self, parser: &mut Parser, ctx: &mut JsonParseCtx<'a>) -> bool
    {
        return JsonGrammar.MatchRoot(parser, ctx, true);
    }
}

impl<'a> IGrammar<JsonParseCtx<'a>> for JsonGrammar
{
    fn Match(&self, parser: &mut Parser, ctx: &mut JsonParseCtx<'a>) -> bool
    {
        return self.MatchRoot(parser, ctx, false);
    }

    fn ParseStr(&self, input: &str, ctx: &mut JsonParseCtx<'a>)
                -> Result<u32, crate::shard::ParseError>
    {
        return JsonDocumentGrammar.ParseStr(input, ctx);
    }

    fn ParseStream(&self, stream: &mut dyn crate::flux::IStream, ctx: &mut JsonParseCtx<'a>)
                   -> Result<u32, crate::shard::ParseError>
    {
        return JsonDocumentGrammar.ParseStream(stream, ctx);
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
