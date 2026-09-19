// termtree.rs ----------------------------------------------------------------------------------------------------
use	crate::stalks::{ BinNode, BinOp };
use	std::fmt;

//-------------------------------------------------------------------------------------------------

#[derive( Clone, Debug)]
pub enum Term {
    Null,
    String( String),
    Real( f64),
}
impl Default for Term {
    fn	default() -> Self
    {
        Self::Null
    }
}
impl fmt::Display for Term {
    fn	fmt( &self, formatter: &mut fmt::Formatter< '_>) -> fmt::Result {
        match self {
            Self::Null => write!( formatter, "Term(Null)"),
            Self::String( value) => write!( formatter, "Term({value})"),
            Self::Real( value) => write!( formatter, "Term({value})"),
        }
    }
}
impl From< char> for Term {
    fn	from( value: char) -> Self
    {
        Self::String( value.to_string())
    }
}
impl From< String> for Term {
    fn	from( value: String) -> Self
    {
        Self::String( value)
    }
}
impl From< &str> for Term {
    fn	from( value: &str) -> Self
    {
        Self::String( value.to_string())
    }
}
impl From< f64> for Term {
    fn	from( value: f64) -> Self
    {
        Self::Real( value)
    }
}

//-------------------------------------------------------------------------------------------------

pub trait ITermNode {
    fn	ChildrenCount( &self) -> u32;
    fn	Child( &self, index: u32) -> &dyn ITermNode;
    fn	Op( &self) -> BinOp;
    fn	AsLeaf( &self) -> &Term;
}
impl ITermNode for Term {
    fn	ChildrenCount( &self) -> u32
    {
        0
    }
    fn	Child( &self, _index: u32) -> &dyn ITermNode
    {
        panic!( "Leaf has no children")
    }
    fn	Op( &self) -> BinOp
    {
        BinOp::None
    }
    fn	AsLeaf( &self) -> &Term
    {
        self
    }
}
impl< T: ITermNode + ?Sized> ITermNode for &T {
    fn	ChildrenCount( &self) -> u32
    {
        ( **self).ChildrenCount()
    }
    fn	Child( &self, index: u32) -> &dyn ITermNode
    {
        ( **self).Child( index)
    }
    fn	Op( &self) -> BinOp
    {
        ( **self).Op()
    }
    fn	AsLeaf( &self) -> &Term
    {
        ( **self).AsLeaf()
    }
}
pub type TermBinNode< L, R> = BinNode< L, R>;
impl< L: ITermNode, R: ITermNode> ITermNode for BinNode< L, R> {
    fn	ChildrenCount( &self) -> u32
    {
        2
    }
    fn	Child( &self, index: u32) -> &dyn ITermNode
    {
        match index {
            0 => &self._Left,
            1 => &self._Right,
            _ => panic!( "Term child index out of bounds"),
        }
    }
    fn	Op( &self) -> BinOp
    {
        self._Op
    }
    fn	AsLeaf( &self) -> &Term
    {
        static NULL_TERM: Term = Term::Null;
        &NULL_TERM
    }
}
pub trait AsTermNode {
    type Node: ITermNode;
    fn	AsTermNode( self) -> Self::Node;
}
impl< T: ITermNode> AsTermNode for T {
    type Node = T;
    fn	AsTermNode( self) -> Self::Node
    {
        self
    }
}
impl AsTermNode for char {
    type Node = Term;
    fn	AsTermNode( self) -> Self::Node
    {
        Term::from( self)
    }
}
impl AsTermNode for &str {
    type Node = Term;
    fn	AsTermNode( self) -> Self::Node
    {
        Term::from( self)
    }
}
impl AsTermNode for String {
    type Node = Term;
    fn	AsTermNode( self) -> Self::Node
    {
        Term::from( self)
    }
}
impl AsTermNode for f64 {
    type Node = Term;
    fn	AsTermNode( self) -> Self::Node
    {
        Term::from( self)
    }
}
#[macro_export]
macro_rules! TermTree {
    ( @leaf $( $leaf:tt )+) => {{
        use $crate::fresco::AsTermNode;
        ($( $leaf )+).AsTermNode()
    }};
    ($( $tree:tt )+) => {
        $crate::NodeTree!( @parse TermTree, $( $tree )+)
    };
}

//-------------------------------------------------------------------------------------------------
