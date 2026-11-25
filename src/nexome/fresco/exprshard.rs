// exprshard.rs -----------------------------------------------------------------------------------
use	crate::fresco::{ ExprRepos, LogicOp };
use	crate::shard::{ IGrammar, ParseError, Parser, WSpc };
use	crate::silo::{ Buff, Stash };
use	core::fmt;

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublishLimits
{
    pub _MaxDepth: u32,
    pub _MaxNodes: u32,
    pub _MaxEdges: u32,
}

impl Default for PublishLimits
{
    fn	default() -> Self
    {
        Self {
            _MaxDepth: ExprGrammar::K_MAX_EXPR_DEPTH,
            _MaxNodes: 100_000,
            _MaxEdges: 200_000,
        }
    }
}

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Debug, PartialEq, Eq)]
pub enum PublishError
{
    NoAcceptedExpression,
    InvalidStagedId( u32),
    CycleDetected( u32),
    InvalidArity( u32),
    DepthExceeded( u32),
    NodeBudgetExceeded( u32),
    EdgeBudgetExceeded( u32),
}

impl fmt::Display for PublishError
{
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        match self
        {
            Self::NoAcceptedExpression => write!( f, "No accepted expression to publish"),
            Self::InvalidStagedId( id) => write!( f, "Invalid staged expression ID {}", id),
            Self::CycleDetected( id) => write!( f, "Cycle detected in staged expression at ID {}", id),
            Self::InvalidArity( id) => write!( f, "Invalid operator arity in staged expression at ID {}", id),
            Self::DepthExceeded( max) => write!( f, "Staged expression depth exceeds limit of {}", max),
            Self::NodeBudgetExceeded( max) => write!( f, "Staged expression node count exceeds limit of {}", max),
            Self::EdgeBudgetExceeded( max) => write!( f, "Staged expression edge count exceeds limit of {}", max),
        }
    }
}

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

    #[inline]
    pub fn	RootToken( &self) -> u32
    {
        return self._RootToken;
    }

    #[inline]
    pub fn	StagedRoot( &self) -> u32
    { self._StagedRoot }

    #[inline]
    pub fn	IsPublished( &self) -> bool
    { self._IsPublished }

    pub fn	Publish( &mut self) -> Result< u32, PublishError>
    {
        return self.PublishWithLimits( &PublishLimits::default());
    }

    pub fn	PublishWithLimits( &mut self, limits: &PublishLimits) -> Result< u32, PublishError>
    {
        if self._IsPublished
        {
            return Ok( self._RootToken);
        }
        let  	postOrder = self.ValidateStaged( limits)?;
        let  	stagingCount = self._Staging.Size();
        let  	reuseVars = self._ReuseVars;
        let  	staging = &self._Staging;
        let  	stagedRoot = self._StagedRoot;

        let  	root = self._Repos.WithTransaction( |repos| {
            let  	mut finalIds = Buff::< u32>::FromDispenser( stagingCount, |_| u32::MAX);
            let  	count = postOrder.Size();
            let  	mut i = 0u32;
            while i < count
            {
                let  	localId = postOrder[i];
                let  	res = match &staging[localId]
                {
                    StagedNode::Real( val) => repos.RealCreate( *val),
                    StagedNode::Var( name) => repos.VarCreate( name.clone(), reuseVars),
                    StagedNode::Add( a, b) => repos.AddCreate( finalIds[*a], finalIds[*b]),
                    StagedNode::Diff( a, b) => repos.DiffCreate( finalIds[*a], finalIds[*b]),
                    StagedNode::Mult( a, b) => repos.MultCreate( finalIds[*a], finalIds[*b]),
                    StagedNode::Div( a, b) => repos.DivCreate( finalIds[*a], finalIds[*b]),
                    StagedNode::Power( a, b) => repos.PowerCreate( finalIds[*a], finalIds[*b]),
                    StagedNode::Logic( op, a, b) => repos.LogicCreate( *op, finalIds[*a], finalIds[*b]),
                    StagedNode::LogicUnary( op, a) => repos.LogicUnaryCreate( *op, finalIds[*a]),
                    StagedNode::Cond( c, t, f) => repos.CondCreate( finalIds[*c], finalIds[*t], finalIds[*f]),
                    StagedNode::Func( name, arg) => repos.FuncCreate( name.clone(), finalIds[*arg]),
                };
                finalIds[localId] = res;
                i += 1;
            }
            return Ok::< u32, PublishError>( finalIds[stagedRoot]);
        })?;

        self._RootToken = root;
        self._IsPublished = true;
        return Ok( root);
    }

    fn	ValidateStaged( &self, limits: &PublishLimits) -> Result< Buff< u32>, PublishError>
    {
        if self._StagedRoot == u32::MAX
        {
            return Err( PublishError::NoAcceptedExpression);
        }
        let  	stagingCount = self._Staging.Size();
        if self._StagedRoot >= stagingCount
        {
            return Err( PublishError::InvalidStagedId( self._StagedRoot));
        }

        let  	mut state = Buff::< u8>::FromDispenser( stagingCount, |_| 0);
        let  	mut postOrder = Stash::< u32>::WithCapacity( stagingCount);
        let  	mut stack = Stash::< ( u32, u32, bool)>::WithCapacity( 64);
        stack.Push( ( self._StagedRoot, 0, false));

        let  	mut nodesVisited = 0u32;
        let  	mut edgesTraversed = 0u32;

        while let Some( ( localId, depth, postVisit)) = stack.Pop()
        {
            if postVisit
            {
                state[localId] = 2;
                postOrder.Push( localId);
                continue;
            }

            if depth > limits._MaxDepth
            {
                return Err( PublishError::DepthExceeded( limits._MaxDepth));
            }
            if localId >= stagingCount
            {
                return Err( PublishError::InvalidStagedId( localId));
            }
            if state[localId] == 1
            {
                return Err( PublishError::CycleDetected( localId));
            }
            if state[localId] == 2
            {
                continue;
            }

            nodesVisited = nodesVisited.saturating_add( 1);
            if nodesVisited > limits._MaxNodes
            {
                return Err( PublishError::NodeBudgetExceeded( limits._MaxNodes));
            }

            state[localId] = 1;
            stack.Push( ( localId, depth, true));

            let  	children: [u32; 3];
            let  	childCount: u32;
            match &self._Staging[localId]
            {
                StagedNode::Real( _) | StagedNode::Var( _) =>
                {
                    children = [0, 0, 0];
                    childCount = 0;
                }
                StagedNode::Add( a, b)
                | StagedNode::Diff( a, b)
                | StagedNode::Mult( a, b)
                | StagedNode::Div( a, b)
                | StagedNode::Power( a, b) =>
                {
                    children = [*a, *b, 0];
                    childCount = 2;
                }
                StagedNode::Logic( op, a, b) =>
                {
                    if *op == LogicOp::Not
                    {
                        return Err( PublishError::InvalidArity( localId));
                    }
                    children = [*a, *b, 0];
                    childCount = 2;
                }
                StagedNode::LogicUnary( op, a) =>
                {
                    if *op != LogicOp::Not
                    {
                        return Err( PublishError::InvalidArity( localId));
                    }
                    children = [*a, 0, 0];
                    childCount = 1;
                }
                StagedNode::Func( _, a) =>
                {
                    children = [*a, 0, 0];
                    childCount = 1;
                }
                StagedNode::Cond( c, t, f) =>
                {
                    children = [*c, *t, *f];
                    childCount = 3;
                }
            }

            let  	mut idx = childCount;
            while idx > 0
            {
                idx -= 1;
                let  	childId = children[idx as usize];
                edgesTraversed = edgesTraversed.saturating_add( 1);
                if edgesTraversed > limits._MaxEdges
                {
                    return Err( PublishError::EdgeBudgetExceeded( limits._MaxEdges));
                }
                if childId >= stagingCount
                {
                    return Err( PublishError::InvalidStagedId( childId));
                }
                if state[childId] == 1
                {
                    return Err( PublishError::CycleDetected( childId));
                }
                stack.Push( ( childId, depth + 1, false));
            }
        }

        return Ok( postOrder.IntoBuff());
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
