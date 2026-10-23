// exprshard.rs -----------------------------------------------------------------------------------
use	crate::flux::FixedStream;
use	crate::fresco::{ ExprRepos, LogicOp };
use	crate::shard::{ IGrammar, Parser, WSpc };
use	crate::silo::{ Buff, Stash, cast::IConstPtrMutRefExt };
use	core::fmt;

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Debug)]
enum StagedNode
{
    Real( f64),
    Var( String),
    Add( u32, u32),
    Diff( u32, u32),
    Mult( u32, u32),
    Div( u32, u32),
    Power( u32, u32),
    Logic( LogicOp, u32, u32),
    LogicUnary( LogicOp, u32),
    Cond( u32, u32, u32),
    Func( String, u32),
}

//-------------------------------------------------------------------------------------------------

pub struct ExprShard< 'a>
{
    _Repos:       &'a mut ExprRepos,
    _Staging:     Stash< StagedNode>,
    _StagedRoot:  u32,
    _RootToken:   u32,
    _ReuseVars:   bool,
    _IsPublished: bool,
}

//-------------------------------------------------------------------------------------------------

impl< 'a> ExprShard< 'a>
{
    pub const K_MAX_EXPR_DEPTH: u32 = 256;

    pub fn	New( repos: &'a mut ExprRepos) -> Self
    {
        Self {
            _Repos:       repos,
            _Staging:     Stash::New(),
            _StagedRoot:  u32::MAX,
            _RootToken:   u32::MAX,
            _ReuseVars:   true,
            _IsPublished: false,
        }
    }

    pub fn	WithReuse( repos: &'a mut ExprRepos, reuseVars: bool) -> Self
    {
        Self {
            _Repos:       repos,
            _Staging:     Stash::New(),
            _StagedRoot:  u32::MAX,
            _RootToken:   u32::MAX,
            _ReuseVars:   reuseVars,
            _IsPublished: false,
        }
    }

    pub fn	RootToken( &self) -> u32
    {
        if !self._IsPublished && self._StagedRoot != u32::MAX
        {
            let  	this: &mut Self = ( self as *const Self).MutRef();
            let  	_ = this.Publish();
        }
        self._RootToken
    }

    pub fn	StagedRoot( &self) -> u32
    { self._StagedRoot }

    pub fn	IsPublished( &self) -> bool
    { self._IsPublished }

    pub fn	Publish( &mut self) -> Result< u32, &'static str>
    {
        if self._IsPublished
        {
            return Ok( self._RootToken);
        }
        if self._StagedRoot == u32::MAX
        {
            return Err( "No accepted expression to publish");
        }
        let  	stagingCount = self._Staging.Size();
        let  	mut finalIds = Buff::< u32>::FromDispenser( stagingCount, |_| u32::MAX);
        let  	root = self.PublishSubtree( self._StagedRoot, &mut finalIds);
        self._RootToken = root;
        self._IsPublished = true;
        Ok( root)
    }

    fn	PublishSubtree( &mut self, localId: u32, finalIds: &mut Buff< u32>) -> u32
    {
        if localId >= self._Staging.Size()
        {
            panic!( "Staged node ID out of bounds");
        }
        if finalIds[localId] != u32::MAX
        {
            return finalIds[localId];
        }
        let  	node = self._Staging[localId].clone();
        let  	res = match node
        {
            StagedNode::Real( val) => self._Repos.RealCreate( val),
            StagedNode::Var( name) => self._Repos.VarCreate( name, self._ReuseVars),
            StagedNode::Add( a, b) =>
            {
                let  	fa = self.PublishSubtree( a, finalIds);
                let  	fb = self.PublishSubtree( b, finalIds);
                self._Repos.AddCreate( fa, fb)
            }
            StagedNode::Diff( a, b) =>
            {
                let  	fa = self.PublishSubtree( a, finalIds);
                let  	fb = self.PublishSubtree( b, finalIds);
                self._Repos.DiffCreate( fa, fb)
            }
            StagedNode::Mult( a, b) =>
            {
                let  	fa = self.PublishSubtree( a, finalIds);
                let  	fb = self.PublishSubtree( b, finalIds);
                self._Repos.MultCreate( fa, fb)
            }
            StagedNode::Div( a, b) =>
            {
                let  	fa = self.PublishSubtree( a, finalIds);
                let  	fb = self.PublishSubtree( b, finalIds);
                self._Repos.DivCreate( fa, fb)
            }
            StagedNode::Power( a, b) =>
            {
                let  	fa = self.PublishSubtree( a, finalIds);
                let  	fb = self.PublishSubtree( b, finalIds);
                self._Repos.PowerCreate( fa, fb)
            }
            StagedNode::Logic( op, a, b) =>
            {
                let  	fa = self.PublishSubtree( a, finalIds);
                let  	fb = self.PublishSubtree( b, finalIds);
                self._Repos.LogicCreate( op, fa, fb)
            }
            StagedNode::LogicUnary( op, a) =>
            {
                let  	fa = self.PublishSubtree( a, finalIds);
                self._Repos.LogicUnaryCreate( op, fa)
            }
            StagedNode::Cond( cond, ifTrue, ifFalse) =>
            {
                let  	fc = self.PublishSubtree( cond, finalIds);
                let  	ft = self.PublishSubtree( ifTrue, finalIds);
                let  	ff = self.PublishSubtree( ifFalse, finalIds);
                self._Repos.CondCreate( fc, ft, ff)
            }
            StagedNode::Func( name, arg) =>
            {
                let  	fa = self.PublishSubtree( arg, finalIds);
                self._Repos.FuncCreate( name, fa)
            }
        };
        finalIds[localId] = res;
        res
    }

    pub fn	Parse( repos: &'a mut ExprRepos, text: &str) -> Result< u32, String>
    {
        let  	mut stream = FixedStream::from( text);
        let  	mut parser = Parser::New( &mut stream);
        let  	mut shard = Self::New( repos);
        if let  	Some( completedMark) = parser.ParseGrammar( &shard, 0)
        {
            if completedMark >= parser.InStream().Size()
            {
                shard.Publish().map_err( |e| e.to_string())
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

    fn	RealCreate( &mut self, val: f64) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Real( val));
        idx
    }

    fn	VarCreate( &mut self, name: String) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Var( name));
        idx
    }

    fn	AddCreate( &mut self, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Add( a, b));
        idx
    }

    fn	DiffCreate( &mut self, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Diff( a, b));
        idx
    }

    fn	MultCreate( &mut self, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Mult( a, b));
        idx
    }

    fn	DivCreate( &mut self, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Div( a, b));
        idx
    }

    fn	PowCreate( &mut self, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Power( a, b));
        idx
    }

    fn	LogicCreate( &mut self, op: LogicOp, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Logic( op, a, b));
        idx
    }

    fn	LogicUnaryCreate( &mut self, op: LogicOp, a: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::LogicUnary( op, a));
        idx
    }

    fn	CondCreate( &mut self, cond: u32, ifTrue: u32, ifFalse: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Cond( cond, ifTrue, ifFalse));
        idx
    }

    fn	FuncCreate( &mut self, name: String, arg: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Func( name, arg));
        idx
    }

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
        b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'@' || b == b'#' || b == b'$'
    }

    fn	MatchRawNumber( parser: &mut Parser) -> Option< f64>
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
        Some( val)
    }

    fn	MatchNumber( &mut self, parser: &mut Parser) -> Option< u32>
    {
        Self::MatchRawNumber( parser).map( |val| self.RealCreate( val))
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

    fn	MatchPrimary( &mut self, parser: &mut Parser, depth: u32) -> Option< u32>
    {
        Self::SkipWs( parser);
        let  	origMark = parser.CurrMark();
        let  	origStaged = self._Staging.Size();

        // 1. Bracketed ( ... ) or { ... }
        if let  	Some( ch) = Self::PeekChar( parser)
            && ( ch == b'(' || ch == b'{')
        {
            let  	closeCh = if ch == b'(' { b')' } else { b'}' };
            let  	_ = Self::MatchChar( parser, ch);
            Self::SkipWs( parser);
            let  	inner = match self.MatchExpr( parser, depth + 1)
            {
                Some( node) => node,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            };
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
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
            let  	afterIdentStaged = self._Staging.Size();
            Self::SkipWs( parser);
            if Self::MatchChar( parser, b'(')
            {
                Self::SkipWs( parser);
                let  	argNode = match self.MatchExpr( parser, depth + 1)
                {
                    Some( node) => node,
                    None =>
                    {
                        parser.SetCurrMark( origMark);
                        self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                        return None;
                    }
                };
                Self::SkipWs( parser);
                if !Self::MatchChar( parser, b')')
                {
                    parser.SetCurrMark( origMark);
                    self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
                return Some( self.FuncCreate( ident, argNode));
            }
            else
            {
                parser.SetCurrMark( afterIdentMark);
                self._Staging.Resize( afterIdentStaged, |_| StagedNode::Real( 0.0));
                return Some( self.VarCreate( ident));
            }
        }

        parser.SetCurrMark( origMark);
        self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
        None
    }

    fn	MatchUnary( &mut self, parser: &mut Parser, depth: u32) -> Option< u32>
    {
        if depth >= Self::K_MAX_EXPR_DEPTH
        {
            return None;
        }
        Self::SkipWs( parser);
        if Self::MatchChar( parser, b'+')
        {
            self.MatchUnary( parser, depth + 1)
        }
        else if Self::MatchChar( parser, b'-')
        {
            let  	saveMark = parser.CurrMark();
            let  	saveStaged = self._Staging.Size();
            Self::SkipWs( parser);
            if let  	Some( val) = Self::MatchRawNumber( parser)
            {
                Some( self.RealCreate( -val))
            }
            else if let  	Some( operand) = self.MatchUnary( parser, depth + 1)
            {
                let  	zeroNode = self.RealCreate( 0.0);
                Some( self.DiffCreate( zeroNode, operand))
            }
            else
            {
                parser.SetCurrMark( saveMark);
                self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                None
            }
        }
        else if Self::MatchChar( parser, b'!') || Self::MatchKeyword( parser, "not")
        {
            let  	saveMark = parser.CurrMark();
            let  	saveStaged = self._Staging.Size();
            Self::SkipWs( parser);
            if let  	Some( operand) = self.MatchUnary( parser, depth + 1)
            {
                Some( self.LogicUnaryCreate( LogicOp::Not, operand))
            }
            else
            {
                parser.SetCurrMark( saveMark);
                self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                None
            }
        }
        else
        {
            self.MatchPrimary( parser, depth)
        }
    }

    fn	MatchPow( &mut self, parser: &mut Parser, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchUnary( parser, depth)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            let  	saveStaged = self._Staging.Size();
            Self::SkipWs( parser);
            if Self::MatchChar( parser, b'^')
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchUnary( parser, depth)
                {
                    left = self.PowCreate( left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                break;
            }
        }
        Some( left)
    }

    fn	MatchMulDiv( &mut self, parser: &mut Parser, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchPow( parser, depth)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            let  	saveStaged = self._Staging.Size();
            Self::SkipWs( parser);
            if Self::MatchChar( parser, b'*')
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchPow( parser, depth)
                {
                    left = self.MultCreate( left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                    break;
                }
            }
            else if Self::MatchChar( parser, b'/')
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchPow( parser, depth)
                {
                    left = self.DivCreate( left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                break;
            }
        }
        Some( left)
    }

    fn	MatchAddSub( &mut self, parser: &mut Parser, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchMulDiv( parser, depth)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            let  	saveStaged = self._Staging.Size();
            Self::SkipWs( parser);
            if Self::MatchChar( parser, b'+')
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchMulDiv( parser, depth)
                {
                    left = self.AddCreate( left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                    break;
                }
            }
            else if Self::MatchChar( parser, b'-')
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchMulDiv( parser, depth)
                {
                    left = self.DiffCreate( left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                break;
            }
        }
        Some( left)
    }

    fn	MatchComparison( &mut self, parser: &mut Parser, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchAddSub( parser, depth)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            let  	saveStaged = self._Staging.Size();
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
                if let  	Some( right) = self.MatchAddSub( parser, depth)
                {
                    left = self.LogicCreate( logicOp, left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                break;
            }
        }
        Some( left)
    }

    fn	MatchLogicalAnd( &mut self, parser: &mut Parser, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchComparison( parser, depth)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            let  	saveStaged = self._Staging.Size();
            Self::SkipWs( parser);
            if Self::MatchKeyword( parser, "and")
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchComparison( parser, depth)
                {
                    left = self.LogicCreate( LogicOp::And, left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                break;
            }
        }
        Some( left)
    }

    fn	MatchLogicalOr( &mut self, parser: &mut Parser, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchLogicalAnd( parser, depth)?;
        loop
        {
            let  	saveMark = parser.CurrMark();
            let  	saveStaged = self._Staging.Size();
            Self::SkipWs( parser);
            if Self::MatchKeyword( parser, "or")
            {
                Self::SkipWs( parser);
                if let  	Some( right) = self.MatchLogicalAnd( parser, depth)
                {
                    left = self.LogicCreate( LogicOp::Or, left, right);
                }
                else
                {
                    parser.SetCurrMark( saveMark);
                    self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                    break;
                }
            }
            else
            {
                parser.SetCurrMark( saveMark);
                self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                break;
            }
        }
        Some( left)
    }

    fn	MatchIf( &mut self, parser: &mut Parser, depth: u32) -> Option< u32>
    {
        let  	origMark = parser.CurrMark();
        let  	origStaged = self._Staging.Size();
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
            let  	c = match self.MatchExpr( parser, depth + 1)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            };
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                return None;
            }
            c
        }
        else
        {
            match self.MatchLogicalOr( parser, depth)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            }
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
            let  	t = match self.MatchExpr( parser, depth + 1)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            };
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                return None;
            }
            t
        }
        else
        {
            match self.MatchExpr( parser, depth + 1)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            }
        };

        Self::SkipWs( parser);
        if !Self::MatchKeyword( parser, "else")
        {
            parser.SetCurrMark( origMark);
            self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
            return None;
        }
        Self::SkipWs( parser);

        let  	ifFalse = if let  	Some( ch) = Self::PeekChar( parser)
            && ( ch == b'(' || ch == b'{')
        {
            let  	closeCh = if ch == b'(' { b')' } else { b'}' };
            let  	_ = Self::MatchChar( parser, ch);
            Self::SkipWs( parser);
            let  	f = match self.MatchExpr( parser, depth + 1)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            };
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                return None;
            }
            f
        }
        else
        {
            match self.MatchExpr( parser, depth + 1)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    self._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            }
        };

        Some( self.CondCreate( cond, ifTrue, ifFalse))
    }

    pub fn	MatchExpr( &mut self, parser: &mut Parser, depth: u32) -> Option< u32>
    {
        if depth >= Self::K_MAX_EXPR_DEPTH
        {
            return None;
        }
        Self::SkipWs( parser);
        let  	saveMark = parser.CurrMark();
        let  	saveStaged = self._Staging.Size();
        if let  	Some( ifNode) = self.MatchIf( parser, depth)
        {
            Some( ifNode)
        }
        else
        {
            parser.SetCurrMark( saveMark);
            self._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
            self.MatchLogicalOr( parser, depth)
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a> IGrammar for ExprShard< 'a>
{
    fn	Match( &self, parser: &mut Parser) -> bool
    {
        let  	this: &mut Self = ( self as *const Self).MutRef();
        let  	origStreamMark = parser.CurrMark();
        let  	origStagedCount = this._Staging.Size();
        Self::SkipWs( parser);
        if let  	Some( root) = this.MatchExpr( parser, 0)
        {
            this._StagedRoot = root;
            Self::SkipWs( parser);
            true
        }
        else
        {
            this._Staging.Resize( origStagedCount, |_| StagedNode::Real( 0.0));
            this._StagedRoot = u32::MAX;
            this._RootToken = u32::MAX;
            parser.SetCurrMark( origStreamMark);
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
