// exprshard.rs -----------------------------------------------------------------------------------
use	crate::fresco::{ ExprRepos, LogicOp };
use	crate::shard::{ IGrammar, ParseError, Parser, WSpc };
use	crate::silo::{ Buff, Stash };
use	core::fmt;

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Debug)]
pub enum StagedNode
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

pub struct ExprParseCtx< 'a>
{
    pub _Repos:       &'a mut ExprRepos,
    pub _Staging:     Stash< StagedNode>,
    pub _StagedRoot:  u32,
    pub _RootToken:   u32,
    pub _ReuseVars:   bool,
    pub _IsPublished: bool,
}

//-------------------------------------------------------------------------------------------------

impl< 'a> ExprParseCtx< 'a>
{
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

    pub fn	RootToken( &mut self) -> u32
    {
        if !self._IsPublished && self._StagedRoot != u32::MAX
        {
            let  	_ = self.Publish();
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

    pub fn	RealCreate( &mut self, val: f64) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Real( val));
        idx
    }

    pub fn	VarCreate( &mut self, name: String) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Var( name));
        idx
    }

    pub fn	AddCreate( &mut self, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Add( a, b));
        idx
    }

    pub fn	DiffCreate( &mut self, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Diff( a, b));
        idx
    }

    pub fn	MultCreate( &mut self, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Mult( a, b));
        idx
    }

    pub fn	DivCreate( &mut self, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Div( a, b));
        idx
    }

    pub fn	PowCreate( &mut self, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Power( a, b));
        idx
    }

    pub fn	LogicCreate( &mut self, op: LogicOp, a: u32, b: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Logic( op, a, b));
        idx
    }

    pub fn	LogicUnaryCreate( &mut self, op: LogicOp, a: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::LogicUnary( op, a));
        idx
    }

    pub fn	CondCreate( &mut self, cond: u32, ifTrue: u32, ifFalse: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Cond( cond, ifTrue, ifFalse));
        idx
    }

    pub fn	FuncCreate( &mut self, name: String, arg: u32) -> u32
    {
        let  	idx = self._Staging.Size();
        self._Staging.Push( StagedNode::Func( name, arg));
        idx
    }

    pub fn	Transaction< T, F>( &mut self, parser: &mut Parser, op: F) -> Option< T>
    where
        F: FnOnce( &mut Self, &mut Parser) -> Option< T>,
    {
        let  	mark = parser.CurrMark();
        let  	staged = self._Staging.Size();
        let  	res = op( self, parser);
        if res.is_none() {
            parser.SetCurrMark( mark);
            self._Staging.Resize( staged, |_| StagedNode::Real( 0.0));
        }
        res
    }
}

//-------------------------------------------------------------------------------------------------

pub struct ExprGrammar;
pub type ExprShard = ExprGrammar;

impl ExprGrammar
{
    pub const K_MAX_EXPR_DEPTH: u32 = 256;

    pub fn	Parse( repos: &mut ExprRepos, text: &str) -> Result< u32, String>
    {
        let  	mut ctx = ExprParseCtx::New( repos);
        match ExprGrammar.ParseStr( text, &mut ctx)
        {
            Ok( _) => ctx.Publish().map_err( |e| e.to_string()),
            Err( ParseError::TrailingGarbage { mark, .. }) =>
            {
                Err( format!( "Unparsed trailing characters at position {}", mark))
            }
            Err( _) => Err( "Failed to parse expression".to_string()),
        }
    }


    //---------------------------------------------------------------------------------------------

    fn	SkipWs( parser: &mut Parser)
    {
        let  	_ = parser.Parse( &WSpc, parser.CurrMark());
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

    fn	MatchNumber( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>) -> Option< u32>
    {
        Self::MatchRawNumber( parser).map( |val| ctx.RealCreate( val))
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

    fn	MatchPrimary( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>, depth: u32) -> Option< u32>
    {
        Self::SkipWs( parser);
        let  	origMark = parser.CurrMark();
        let  	origStaged = ctx._Staging.Size();

        // 1. Bracketed ( ... ) or { ... }
        if let  	Some( ch) = Self::PeekChar( parser)
            && ( ch == b'(' || ch == b'{')
        {
            let  	closeCh = if ch == b'(' { b')' } else { b'}' };
            let  	_ = Self::MatchChar( parser, ch);
            Self::SkipWs( parser);
            let  	inner = match self.MatchExpr( parser, ctx, depth + 1)
            {
                Some( node) => node,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            };
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                return None;
            }
            return Some( inner);
        }

        // 2. Number
        if let  	Some( numNode) = self.MatchNumber( parser, ctx)
        {
            return Some( numNode);
        }

        // 3. Identifier: Function call or Variable
        if let  	Some( ident) = Self::MatchIdentStr( parser)
        {
            let  	afterIdentMark = parser.CurrMark();
            let  	afterIdentStaged = ctx._Staging.Size();
            Self::SkipWs( parser);
            if Self::MatchChar( parser, b'(')
            {
                Self::SkipWs( parser);
                let  	argNode = match self.MatchExpr( parser, ctx, depth + 1)
                {
                    Some( node) => node,
                    None =>
                    {
                        parser.SetCurrMark( origMark);
                        ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                        return None;
                    }
                };
                Self::SkipWs( parser);
                if !Self::MatchChar( parser, b')')
                {
                    parser.SetCurrMark( origMark);
                    ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
                return Some( ctx.FuncCreate( ident, argNode));
            }
            else
            {
                parser.SetCurrMark( afterIdentMark);
                ctx._Staging.Resize( afterIdentStaged, |_| StagedNode::Real( 0.0));
                return Some( ctx.VarCreate( ident));
            }
        }

        parser.SetCurrMark( origMark);
        ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
        None
    }

    fn	MatchUnary( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>, depth: u32) -> Option< u32>
    {
        if depth >= Self::K_MAX_EXPR_DEPTH
        {
            return None;
        }
        Self::SkipWs( parser);
        if Self::MatchChar( parser, b'+')
        {
            self.MatchUnary( parser, ctx, depth + 1)
        }
        else if Self::MatchChar( parser, b'-')
        {
            let  	saveMark = parser.CurrMark();
            let  	saveStaged = ctx._Staging.Size();
            Self::SkipWs( parser);
            if let  	Some( val) = Self::MatchRawNumber( parser)
            {
                Some( ctx.RealCreate( -val))
            }
            else if let  	Some( operand) = self.MatchUnary( parser, ctx, depth + 1)
            {
                let  	zeroNode = ctx.RealCreate( 0.0);
                Some( ctx.DiffCreate( zeroNode, operand))
            }
            else
            {
                parser.SetCurrMark( saveMark);
                ctx._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                None
            }
        }
        else if Self::MatchChar( parser, b'!') || Self::MatchKeyword( parser, "not")
        {
            let  	saveMark = parser.CurrMark();
            let  	saveStaged = ctx._Staging.Size();
            Self::SkipWs( parser);
            if let  	Some( operand) = self.MatchUnary( parser, ctx, depth + 1)
            {
                Some( ctx.LogicUnaryCreate( LogicOp::Not, operand))
            }
            else
            {
                parser.SetCurrMark( saveMark);
                ctx._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
                None
            }
        }
        else
        {
            self.MatchPrimary( parser, ctx, depth)
        }
    }

    fn	MatchPow( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchUnary( parser, ctx, depth)?;
        loop
        {
            let  	next = ctx.Transaction( parser, |ctx, parser| {
                Self::SkipWs( parser);
                if Self::MatchChar( parser, b'^')
                {
                    Self::SkipWs( parser);
                    self.MatchUnary( parser, ctx, depth)
                }
                else
                {
                    None
                }
            });
            if let  	Some( right) = next {
                left = ctx.PowCreate( left, right);
            } else {
                break;
            }
        }
        Some( left)
    }

    fn	MatchMulDiv( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchPow( parser, ctx, depth)?;
        loop
        {
            let  	next = ctx.Transaction( parser, |ctx, parser| {
                Self::SkipWs( parser);
                if Self::MatchChar( parser, b'*')
                {
                    Self::SkipWs( parser);
                    self.MatchPow( parser, ctx, depth).map( |r| (true, r))
                }
                else if Self::MatchChar( parser, b'/')
                {
                    Self::SkipWs( parser);
                    self.MatchPow( parser, ctx, depth).map( |r| (false, r))
                }
                else
                {
                    None
                }
            });
            match next {
                Some( (true, right)) => left = ctx.MultCreate( left, right),
                Some( (false, right)) => left = ctx.DivCreate( left, right),
                None => break,
            }
        }
        Some( left)
    }

    fn	MatchAddSub( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchMulDiv( parser, ctx, depth)?;
        loop
        {
            let  	next = ctx.Transaction( parser, |ctx, parser| {
                Self::SkipWs( parser);
                if Self::MatchChar( parser, b'+')
                {
                    Self::SkipWs( parser);
                    self.MatchMulDiv( parser, ctx, depth).map( |r| (true, r))
                }
                else if Self::MatchChar( parser, b'-')
                {
                    Self::SkipWs( parser);
                    self.MatchMulDiv( parser, ctx, depth).map( |r| (false, r))
                }
                else
                {
                    None
                }
            });
            match next {
                Some( (true, right)) => left = ctx.AddCreate( left, right),
                Some( (false, right)) => left = ctx.DiffCreate( left, right),
                None => break,
            }
        }
        Some( left)
    }

    fn	MatchComparison( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchAddSub( parser, ctx, depth)?;
        loop
        {
            let  	next = ctx.Transaction( parser, |ctx, parser| {
                Self::SkipWs( parser);
                let  	op = if Self::MatchExact( parser, "==", false) {
                    Some( LogicOp::Eq)
                } else if Self::MatchExact( parser, "!=", false) {
                    Some( LogicOp::Ne)
                } else if Self::MatchExact( parser, "<=", false) {
                    Some( LogicOp::Le)
                } else if Self::MatchExact( parser, ">=", false) {
                    Some( LogicOp::Ge)
                } else if Self::MatchChar( parser, b'<') {
                    Some( LogicOp::Lt)
                } else if Self::MatchChar( parser, b'>') {
                    Some( LogicOp::Gt)
                } else if Self::MatchChar( parser, b'=') {
                    Some( LogicOp::Eq)
                } else {
                    None
                }?;
                Self::SkipWs( parser);
                let  	right = self.MatchAddSub( parser, ctx, depth)?;
                Some( (op, right))
            });
            if let  	Some( (logicOp, right)) = next {
                left = ctx.LogicCreate( logicOp, left, right);
            } else {
                break;
            }
        }
        Some( left)
    }

    fn	MatchLogicalAnd( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchComparison( parser, ctx, depth)?;
        loop
        {
            let  	next = ctx.Transaction( parser, |ctx, parser| {
                Self::SkipWs( parser);
                if Self::MatchKeyword( parser, "and")
                {
                    Self::SkipWs( parser);
                    self.MatchComparison( parser, ctx, depth)
                }
                else
                {
                    None
                }
            });
            if let  	Some( right) = next {
                left = ctx.LogicCreate( LogicOp::And, left, right);
            } else {
                break;
            }
        }
        Some( left)
    }

    fn	MatchLogicalOr( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>, depth: u32) -> Option< u32>
    {
        let  	mut left = self.MatchLogicalAnd( parser, ctx, depth)?;
        loop
        {
            let  	next = ctx.Transaction( parser, |ctx, parser| {
                Self::SkipWs( parser);
                if Self::MatchKeyword( parser, "or")
                {
                    Self::SkipWs( parser);
                    self.MatchLogicalAnd( parser, ctx, depth)
                }
                else
                {
                    None
                }
            });
            if let  	Some( right) = next {
                left = ctx.LogicCreate( LogicOp::Or, left, right);
            } else {
                break;
            }
        }
        Some( left)
    }

    fn	MatchIf( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>, depth: u32) -> Option< u32>
    {
        let  	origMark = parser.CurrMark();
        let  	origStaged = ctx._Staging.Size();
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
            let  	c = match self.MatchExpr( parser, ctx, depth + 1)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            };
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                return None;
            }
            c
        }
        else
        {
            match self.MatchLogicalOr( parser, ctx, depth)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
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
            let  	t = match self.MatchExpr( parser, ctx, depth + 1)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            };
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                return None;
            }
            t
        }
        else
        {
            match self.MatchExpr( parser, ctx, depth + 1)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            }
        };

        Self::SkipWs( parser);
        if !Self::MatchKeyword( parser, "else")
        {
            parser.SetCurrMark( origMark);
            ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
            return None;
        }
        Self::SkipWs( parser);

        let  	ifFalse = if let  	Some( ch) = Self::PeekChar( parser)
            && ( ch == b'(' || ch == b'{')
        {
            let  	closeCh = if ch == b'(' { b')' } else { b'}' };
            let  	_ = Self::MatchChar( parser, ch);
            Self::SkipWs( parser);
            let  	f = match self.MatchExpr( parser, ctx, depth + 1)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            };
            Self::SkipWs( parser);
            if !Self::MatchChar( parser, closeCh)
            {
                parser.SetCurrMark( origMark);
                ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                return None;
            }
            f
        }
        else
        {
            match self.MatchExpr( parser, ctx, depth + 1)
            {
                Some( n) => n,
                None =>
                {
                    parser.SetCurrMark( origMark);
                    ctx._Staging.Resize( origStaged, |_| StagedNode::Real( 0.0));
                    return None;
                }
            }
        };

        Some( ctx.CondCreate( cond, ifTrue, ifFalse))
    }

    pub fn	MatchExpr( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< '_>, depth: u32) -> Option< u32>
    {
        if depth >= Self::K_MAX_EXPR_DEPTH
        {
            return None;
        }
        Self::SkipWs( parser);
        let  	saveMark = parser.CurrMark();
        let  	saveStaged = ctx._Staging.Size();
        if let  	Some( ifNode) = self.MatchIf( parser, ctx, depth)
        {
            Some( ifNode)
        }
        else
        {
            parser.SetCurrMark( saveMark);
            ctx._Staging.Resize( saveStaged, |_| StagedNode::Real( 0.0));
            self.MatchLogicalOr( parser, ctx, depth)
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl< 'a> IGrammar< ExprParseCtx< 'a>> for ExprGrammar
{
    fn	Match( &self, parser: &mut Parser, ctx: &mut ExprParseCtx< 'a>) -> bool
    {
        let  	origStreamMark = parser.CurrMark();
        let  	origStagedCount = ctx._Staging.Size();
        Self::SkipWs( parser);
        if let  	Some( root) = self.MatchExpr( parser, ctx, 0)
        {
            ctx._StagedRoot = root;
            Self::SkipWs( parser);
            true
        }
        else
        {
            ctx._Staging.Resize( origStagedCount, |_| StagedNode::Real( 0.0));
            ctx._StagedRoot = u32::MAX;
            ctx._RootToken = u32::MAX;
            parser.SetCurrMark( origStreamMark);
            false
        }
    }
}

//-------------------------------------------------------------------------------------------------

impl fmt::Display for ExprGrammar
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        write!( f, "ExprGrammar")
    }
}

//-------------------------------------------------------------------------------------------------

impl fmt::Debug for ExprGrammar
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        write!( f, "ExprGrammar")
    }
}

//-------------------------------------------------------------------------------------------------
