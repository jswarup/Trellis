// exprshard.rs -----------------------------------------------------------------------------------
use	crate::flux::FixedStream;
use	crate::fresco::{ ExprRepos, LogicOp, RealExpr };
use	crate::shard::{ IGrammar, Parser, WSpc };
use	core::cell::{ Cell, RefCell };
use	core::fmt;

//-------------------------------------------------------------------------------------------------

pub struct ExprShard< 'a>
{
    _Repos:     RefCell< &'a mut ExprRepos>,
    _RootToken: Cell< u32>,
    _ReuseVars: bool,
}

//-------------------------------------------------------------------------------------------------

impl< 'a> ExprShard< 'a>
{
    pub fn	New( repos: &'a mut ExprRepos) -> Self
    {
        Self {
            _Repos:     RefCell::new( repos),
            _RootToken: Cell::new( u32::MAX),
            _ReuseVars: true,
        }
    }

    pub fn	WithReuse( repos: &'a mut ExprRepos, reuseVars: bool) -> Self
    {
        Self {
            _Repos:     RefCell::new( repos),
            _RootToken: Cell::new( u32::MAX),
            _ReuseVars: reuseVars,
        }
    }

    pub fn	RootToken( &self) -> u32
    { self._RootToken.get() }

    pub fn	Parse( repos: &'a mut ExprRepos, text: &str) -> Result< u32, String>
    {
        let  	mut stream = FixedStream::from( text);
        let  	mut parser = Parser::New( &mut stream);
        let  	shard = Self::New( repos);
        if let  	Some( completedMark) = parser.ParseGrammar( &shard, 0)
        {
            if completedMark >= parser.InStream().Size()
            {
                Ok( shard.RootToken())
            }
            else
            {
                Err( format!( "Unparsed trailing characters at position {}", completedMark))
            }
        }
        else
        {
            Err( "Failed to parse expression".to_string())
        }
    }

    //---------------------------------------------------------------------------------------------

    fn	RealCreate( &self, val: f64) -> u32
    { self._Repos.borrow_mut().RealCreate( val) }

    fn	VarCreate( &self, name: String) -> u32
    {
        let  	reuse = self._ReuseVars;
        self._Repos.borrow_mut().VarCreate( name, reuse)
    }

    fn	AddCreate( &self, a: u32, b: u32) -> u32
    { self._Repos.borrow_mut().AddCreate( a, b) }

    fn	DiffCreate( &self, a: u32, b: u32) -> u32
    { self._Repos.borrow_mut().DiffCreate( a, b) }

    fn	MultCreate( &self, a: u32, b: u32) -> u32
    { self._Repos.borrow_mut().MultCreate( a, b) }

    fn	DivCreate( &self, a: u32, b: u32) -> u32
    { self._Repos.borrow_mut().DivCreate( a, b) }

    fn	PowCreate( &self, a: u32, b: u32) -> u32
    { self._Repos.borrow_mut().PowerCreate( a, b) }

    fn	LogicCreate( &self, op: LogicOp, a: u32, b: u32) -> u32
    { self._Repos.borrow_mut().LogicCreate( op, a, b) }

    fn	CondCreate( &self, cond: u32, ifTrue: u32, ifFalse: u32) -> u32
    { self._Repos.borrow_mut().CondCreate( cond, ifTrue, ifFalse) }

    fn	FuncCreate( &self, name: String, arg: u32) -> u32
    { self._Repos.borrow_mut().FuncCreate( name, arg) }

    //---------------------------------------------------------------------------------------------

    fn	SkipWs( parser: &mut Parser)
    {
        let  	_ = parser.ParseGrammar( &WSpc, parser.CurrMark());
    }

    fn	MatchExact( parser: &mut Parser, target: &str, caseInsensitive: bool) -> bool
    {
        let  	m = parser.CurrMark();
        let  	len = target.len() as u32;
        if m + len > parser.InStream().Size()
        {
            return false;
        }
        let  	bytes = parser.InStream().BytesAt( m, len);
        let  	slice: &[u8] = bytes.into();
        let  	targetBytes = target.as_bytes();
        let  	matched = if caseInsensitive
        {
            slice.eq_ignore_ascii_case( targetBytes)
        }
        else
        {
            slice == targetBytes
        };
        if matched
        {
            parser.SetCurrMark( m + len);
            true
        }
        else
        {
            false
        }
    }

    fn	MatchKeyword( parser: &mut Parser, kw: &str) -> bool
    {
        let  	origMark = parser.CurrMark();
        if !Self::MatchExact( parser, kw, true)
        {
            return false;
        }
        let  	m = parser.CurrMark();
        if m < parser.InStream().Size()
        {
            let  	nextByte = parser.GetAt( m);
            if Self::IsIdentChar( nextByte)
            {
                parser.SetCurrMark( origMark);
                return false;
            }
        }
        true
    }

    fn	MatchChar( parser: &mut Parser, ch: u8) -> bool
    {
        let  	m = parser.CurrMark();
        if m < parser.InStream().Size() && parser.GetAt( m) == ch
        {
            parser.SetCurrMark( m + 1);
            true
        }
        else
        {
            false
        }
    }

    fn	PeekChar( parser: &mut Parser) -> Option< u8>
    {
        let  	m = parser.CurrMark();
        if m < parser.InStream().Size()
        {
            Some( parser.GetAt( m))
        }
        else
        {
            None
        }
    }

    fn	IsIdentStart( b: u8) -> bool
    {
        b.is_ascii_alphabetic() || b == b'_'
    }

    fn	IsIdentChar( b: u8) -> bool
    {
        b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'@' || b == b'#' || b == b'$' || b == b'!'
    }

    fn	MatchNumber( &self, parser: &mut Parser) -> Option< u32>
    {
        let  	origMark = parser.CurrMark();
        let  	mut m = origMark;
        let  	size = parser.InStream().Size();
        if m >= size
        {
            return None;
        }
        let  	c0 = parser.GetAt( m);
        if !c0.is_ascii_digit() && c0 != b'.'
        {
            return None;
        }
        let  	mut hasDigits = false;
        while m < size && parser.GetAt( m).is_ascii_digit()
        {
            hasDigits = true;
            m += 1;
        }
        if m < size && parser.GetAt( m) == b'.'
            && ( hasDigits || ( m + 1 < size && parser.GetAt( m + 1).is_ascii_digit()))
        {
            m += 1;
            while m < size && parser.GetAt( m).is_ascii_digit()
            {
                hasDigits = true;
                m += 1;
            }
        }
        if !hasDigits
        {
            return None;
        }
        if m < size && ( parser.GetAt( m) == b'e' || parser.GetAt( m) == b'E')
        {
            let  	mut expM = m + 1;
            if expM < size && ( parser.GetAt( expM) == b'+' || parser.GetAt( expM) == b'-')
            {
                expM += 1;
            }
            while expM < size && parser.GetAt( expM).is_ascii_digit()
            {
                expM += 1;
            }
            m = expM;
        }
        let  	cleanStr = {
            let  	bytes = parser.InStream().BytesAt( origMark, m - origMark);
            let  	slice: &[u8] = bytes.into();
            let  	s = std::str::from_utf8( slice).ok()?;
            if s.ends_with( 'e') || s.ends_with( 'E')
            {
                s[..s.len() - 1].to_string()
            }
            else if s.ends_with( "e+") || s.ends_with( "e-") || s.ends_with( "E+") || s.ends_with( "E-")
            {
                s[..s.len() - 2].to_string()
            }
            else
            {
                s.to_string()
            }
        };
        let  	val = cleanStr.parse::< f64>().ok()?;
        parser.SetCurrMark( m);
        Some( self.RealCreate( val))
    }

    fn	MatchIdentStr( parser: &mut Parser) -> Option< String>
    {
        let  	origMark = parser.CurrMark();
        let  	size = parser.InStream().Size();
        if origMark >= size
        {
            return None;
        }
        let  	c0 = parser.GetAt( origMark);
        if !Self::IsIdentStart( c0)
        {
            return None;
        }
        let  	mut m = origMark + 1;
        while m < size && Self::IsIdentChar( parser.GetAt( m))
        {
            m += 1;
        }
        let  	s = {
            let  	bytes = parser.InStream().BytesAt( origMark, m - origMark);
            let  	slice: &[u8] = bytes.into();
            std::str::from_utf8( slice).ok()?.to_string()
        };
        if s.eq_ignore_ascii_case( "if")
            || s.eq_ignore_ascii_case( "else")
            || s.eq_ignore_ascii_case( "then")
            || s.eq_ignore_ascii_case( "and")
            || s.eq_ignore_ascii_case( "or")
        {
            return None;
        }
        parser.SetCurrMark( m);
        Some( s)
    }

    fn	MatchPrimary( &self, parser: &mut Parser) -> Option< u32>
    {
        Self::SkipWs( parser);
        let  	origMark = parser.CurrMark();

        // 1. Bracketed ( ... ) or { ... }
        if let  	Some( ch) = Self::PeekChar( parser)
            && ( ch == b'(' || ch == b'{')
        {
            let  	closeCh = if ch == b'(' { b')' } else { b'}' };
            let  	_ = Self::MatchChar( parser, ch);
            Self::SkipWs( parser);
            let  	inner = self.MatchExpr( parser)?;
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                return None;
            }
            return Some( inner);
        }

        // 2. Number
        if let  	Some( numNode) = self.MatchNumber( parser)
        {
            return Some( numNode);
        }

        // 3. Identifier: Function call or Variable
        if let  	Some( ident) = Self::MatchIdentStr( parser)
        {
            let  	afterIdentMark = parser.CurrMark();
            Self::SkipWs( parser);
            if Self::MatchChar( parser, b'(')
            {
                Self::SkipWs( parser);
                let  	argNode = self.MatchExpr( parser)?;
                Self::SkipWs( parser);
                if !Self::MatchChar( parser, b')')
                {
                    parser.SetCurrMark( origMark);
                    return None;
                }
                return Some( self.FuncCreate( ident, argNode));
            }
            else
            {
                parser.SetCurrMark( afterIdentMark);
                return Some( self.VarCreate( ident));
            }
        }

        parser.SetCurrMark( origMark);
        None
    }

    fn	MatchUnary( &self, parser: &mut Parser) -> Option< u32>
    {
        Self::SkipWs( parser);
        if Self::MatchChar( parser, b'+')
        {
            return self.MatchUnary( parser);
        }
        if Self::MatchChar( parser, b'-')
        {
            Self::SkipWs( parser);
            if let  	Some( numNode) = self.MatchNumber( parser)
            {
                let  	val = self._Repos.borrow().At::< RealExpr>( numNode).Value();
                return Some( self.RealCreate( -val));
            }
            let  	operand = self.MatchUnary( parser)?;
            let  	zeroNode = self.RealCreate( 0.0);
            return Some( self.DiffCreate( zeroNode, operand));
        }
        self.MatchPrimary( parser)
    }

    fn	MatchPow( &self, parser: &mut Parser) -> Option< u32>
    {
        let  	mut left = self.MatchUnary( parser)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            Self::SkipWs( parser);
            if Self::MatchChar( parser, b'^')
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchUnary( parser)
                {
                    left = self.PowCreate( left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                break;
            }
        }
        Some( left)
    }

    fn	MatchMulDiv( &self, parser: &mut Parser) -> Option< u32>
    {
        let  	mut left = self.MatchPow( parser)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            Self::SkipWs( parser);
            if Self::MatchChar( parser, b'*')
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchPow( parser)
                {
                    left = self.MultCreate( left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    break;
                }
            }
            else if Self::MatchChar( parser, b'/')
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchPow( parser)
                {
                    left = self.DivCreate( left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                break;
            }
        }
        Some( left)
    }

    fn	MatchAddSub( &self, parser: &mut Parser) -> Option< u32>
    {
        let  	mut left = self.MatchMulDiv( parser)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            Self::SkipWs( parser);
            if Self::MatchChar( parser, b'+')
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchMulDiv( parser)
                {
                    left = self.AddCreate( left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    break;
                }
            }
            else if Self::MatchChar( parser, b'-')
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchMulDiv( parser)
                {
                    left = self.DiffCreate( left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                break;
            }
        }
        Some( left)
    }

    fn	MatchComparison( &self, parser: &mut Parser) -> Option< u32>
    {
        let  	mut left = self.MatchAddSub( parser)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            Self::SkipWs( parser);
            let  	op = if Self::MatchExact( parser, "==", false)
            {
                Some( LogicOp::Eq)
            }
            else if Self::MatchExact( parser, "!=", false)
            {
                Some( LogicOp::Ne)
            }
            else if Self::MatchExact( parser, "<=", false)
            {
                Some( LogicOp::Le)
            }
            else if Self::MatchExact( parser, ">=", false)
            {
                Some( LogicOp::Ge)
            }
            else if Self::MatchChar( parser, b'<')
            {
                Some( LogicOp::Lt)
            }
            else if Self::MatchChar( parser, b'>')
            {
                Some( LogicOp::Gt)
            }
            else if Self::MatchChar( parser, b'=')
            {
                Some( LogicOp::Eq)
            }
            else
            {
                None
            };

            if let  	Some( logicOp) = op
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchAddSub( parser)
                {
                    left = self.LogicCreate( logicOp, left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                break;
            }
        }
        Some( left)
    }

    fn	MatchLogicalAnd( &self, parser: &mut Parser) -> Option< u32>
    {
        let  	mut left = self.MatchComparison( parser)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            Self::SkipWs( parser);
            if Self::MatchKeyword( parser, "and")
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchComparison( parser)
                {
                    left = self.LogicCreate( LogicOp::And, left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                break;
            }
        }
        Some( left)
    }

    fn	MatchLogicalOr( &self, parser: &mut Parser) -> Option< u32>
    {
        let  	mut left = self.MatchLogicalAnd( parser)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            Self::SkipWs( parser);
            if Self::MatchKeyword( parser, "or")
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchLogicalAnd( parser)
                {
                    left = self.LogicCreate( LogicOp::Or, left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                break;
            }
        }
        Some( left)
    }

    fn	MatchIf( &self, parser: &mut Parser) -> Option< u32>
    {
        let  	origMark = parser.CurrMark();
        Self::SkipWs( parser);
        if !Self::MatchKeyword( parser, "if")
        {
            parser.SetCurrMark( origMark);
            return None;
        }
        Self::SkipWs( parser);

        let  	cond = if let  	Some( ch) = Self::PeekChar( parser)
            && ( ch == b'(' || ch == b'{')
        {
            let  	closeCh = if ch == b'(' { b')' } else { b'}' };
            let  	_ = Self::MatchChar( parser, ch);
            Self::SkipWs( parser);
            let  	c = self.MatchExpr( parser)?;
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                return None;
            }
            c
        }
        else
        {
            self.MatchLogicalOr( parser)?
        };

        Self::SkipWs( parser);
        if Self::MatchKeyword( parser, "then")
        {
            Self::SkipWs( parser);
        }

        let  	ifTrue = if let  	Some( ch) = Self::PeekChar( parser)
            && ( ch == b'(' || ch == b'{')
        {
            let  	closeCh = if ch == b'(' { b')' } else { b'}' };
            let  	_ = Self::MatchChar( parser, ch);
            Self::SkipWs( parser);
            let  	t = self.MatchExpr( parser)?;
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                return None;
            }
            t
        }
        else
        {
            self.MatchExpr( parser)?
        };

        Self::SkipWs( parser);
        if !Self::MatchKeyword( parser, "else")
        {
            parser.SetCurrMark( origMark);
            return None;
        }
        Self::SkipWs( parser);

        let  	ifFalse = if let  	Some( ch) = Self::PeekChar( parser)
            && ( ch == b'(' || ch == b'{')
        {
            let  	closeCh = if ch == b'(' { b')' } else { b'}' };
            let  	_ = Self::MatchChar( parser, ch);
            Self::SkipWs( parser);
            let  	f = self.MatchExpr( parser)?;
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                return None;
            }
            f
        }
        else
        {
            self.MatchExpr( parser)?
        };

        Some( self.CondCreate( cond, ifTrue, ifFalse))
    }

    fn	MatchExpr( &self, parser: &mut Parser) -> Option< u32>
    {
        Self::SkipWs( parser);
        let  	saveMark = parser.CurrMark();
        if let  	Some( ifNode) = self.MatchIf( parser)
        {
            return Some( ifNode);
        }
        parser.SetCurrMark( saveMark);
        self.MatchLogicalOr( parser)
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a> IGrammar for ExprShard< 'a>
{
    fn	Match( &self, parser: &mut Parser) -> bool
    {
        Self::SkipWs( parser);
        if let  	Some( root) = self.MatchExpr( parser)
        {
            self._RootToken.set( root);
            Self::SkipWs( parser);
            true
        }
        else
        {
            false
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a> fmt::Display for ExprShard< 'a>
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        write!( f, "ExprShard")
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a> fmt::Debug for ExprShard< 'a>
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        write!( f, "ExprShard")
    }
}

//-------------------------------------------------------------------------------------------------
