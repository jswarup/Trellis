// mod.rs ---------------------------------------------------------------------------------------------------------
#[cfg( feature = "tests")]
pub mod _tests;
pub mod exprrepos;
pub mod polyexpr;
pub mod powexpr;
pub mod prodexpr;
pub mod realexpr;
pub mod sumexpr;
pub mod termtree;
pub mod varexpr;
pub use	exprrepos::{ BaseExpr, ExprEntry, ExprRepos };
pub use	polyexpr::PolyExpr;
pub use	powexpr::PowExpr;
pub use	prodexpr::ProdExpr;
pub use	realexpr::RealExpr;
pub use	sumexpr::SumExpr;
pub use	termtree::{ AsTermNode, ITermNode, Term, TermBinNode };
pub use	varexpr::{ VarAttrib, VarExpr, VarKind };

//-------------------------------------------------------------------------------------------------
