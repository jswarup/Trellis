//-- shardtree.rs -------------------------------------------------------------------------------------------------------------------
//---------------------------------------------------------------------------------------------------------------------------------

#[macro_export]
macro_rules! ShardTree {
    // Leaf / node creation calls
    ( @leaf [ $s:literal ] ) => {
        <$crate::shard::Charset>::from( $s.as_bytes() )
    };
    ( @leaf $l:literal ) => {
        $l
    };
    ( @leaf ( $( $inner:tt )+ ) ) => {
        $crate::ShardTree!( $( $inner )+ )
    };
    ( @leaf Str ) => {
        $crate::shard::Str
    };
    ( @leaf $l:ident ) => {
        $l
    };
    ( @leaf $leaf:expr ) => {
        $leaf
    };
    ( @action_expr $child:expr, $work:expr ) => {
        $crate::stalks::UniNode {
            _Child: $child,
            _Op: $crate::shard::actionshard::ActionOp::New( $work ),
        }
    };
    ( @action $child:expr, $p:ident, $( $body:tt )+ ) => {
        $crate::stalks::UniNode {
            _Child: $child,
            _Op: $crate::shard::actionshard::ActionOp::New( $crate::shard::actionshard::Coerce( | $p: $crate::silo::Arr< '_, u8> | { $( $body )+ } ) ),
        }
    };
    ( @action_ctx_typed $child:expr, $p:ident, $ctx:ident, $ctx_ty:ident, $( $body:tt )+ ) => {
        $crate::stalks::UniNode {
            _Child: $child,
            _Op: $crate::shard::actionshard::ActionCtxOp::New( $crate::shard::actionshard::CoerceCtx( | $p: $crate::silo::Arr< '_, u8>, $ctx: &mut $ctx_ty | { $( $body )+ } ) ),
        }
    };
    ( @action_ctx $child:expr, $p:ident, $ctx:ident, $( $body:tt )+ ) => {
        $crate::stalks::UniNode {
            _Child: $child,
            _Op: $crate::shard::actionshard::ActionCtxOp::New( $crate::shard::actionshard::CoerceCtx( | $p: $crate::silo::Arr< '_, u8>, $ctx: &mut _ | { $( $body )+ } ) ),
        }
    };
    ( @repeat $child:expr, $min:expr ) => {
        $crate::stalks::UniNode {
            _Child: $child,
            _Op: $crate::silo::USeg::NewInf( $min ),
        }
    };
    ( @optional $child:expr ) => {
        $crate::stalks::UniNode {
            _Child: $child,
            _Op: $crate::silo::USeg::New( 0, 1 ),
        }
    };
    // Delegate recursive parsing to NodeTree
    ( $( $tt:tt )+ ) => {
        $crate::NodeTree!( @parse ShardTree, $( $tt )+ )
    };
}
