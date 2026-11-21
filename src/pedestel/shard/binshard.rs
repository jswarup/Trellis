//-- binshard.rs -----------------------------------------------------------------------------------------------------------------------
use	crate::shard::Parser;
use	crate::{
    shard::IGrammar,
    stalks::
    { BinNode, BinOp },
};

//---------------------------------------------------------------------------------------------------------------------------------

pub type BinShard< L, R> = BinNode< L, R>;

//---------------------------------------------------------------------------------------------------------------------------------

impl< Ctx, L, R> IGrammar< Ctx> for BinNode< L, R>
where
    L: IGrammar< Ctx>,
    R: IGrammar< Ctx>,
{
    fn	Match( &self, parser: &mut Parser, ctx: &mut Ctx) -> bool
    {
        match self._Op {
            BinOp::Bor => {
                let  	m1 = parser.CurrMark();
                let  	leftRes = parser.ParseGrammar( &self._Left, ctx, m1);
                if leftRes.is_some() {
                    return true;
                }
                let  	m2 = parser.CurrMark();
                parser.ParseGrammar( &self._Right, ctx, m2).is_some()
            }
            BinOp::Less => {
                let  	m1 = parser.CurrMark();
                let  	leftRes = parser.ParseGrammar( &self._Left, ctx, m1);
                if let  	Some( newM) = leftRes {
                    if parser.ParseGrammar( &self._Right, ctx, newM).is_some() {
                        return true;
                    }
                    parser.SetCurrMark( m1);
                }
                false
            }
            _ => panic!( "Unsupported operator in BinShard Match"),
        }
    }
}
