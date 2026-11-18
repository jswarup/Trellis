// mod.rs ---------------------------------------------------------------------------------------------------------
#[cfg( feature = "tests")]
pub mod _tests;
pub mod condexpr;
pub mod diff;
pub mod eval;
pub mod exprrepos;
pub mod exprshard;
pub mod funcexpr;
pub mod logicexpr;
pub mod polyexpr;
pub mod powexpr;
pub mod prodexpr;
pub mod realexpr;
pub mod shape;
pub mod sumexpr;
pub mod tensor;
pub mod termtree;
pub mod varexpr;

pub use	condexpr::CondExpr;
pub use	diff::{ Differentiate, DifferentiateVector, Jacobian };
pub use	eval::EvalError;
pub use	exprrepos::{ BaseExpr, ExprCheckpoint, ExprEntry, ExprRepos };
pub use	exprshard::{ ExprGrammar, ExprParseCtx, ExprShard };
pub use	funcexpr::FuncExpr;
pub use	logicexpr::{ LogicExpr, LogicOp };
pub use	polyexpr::{ CanonicalPolynomial, Monomial, PolyExpr };
pub use	powexpr::PowExpr;
pub use	prodexpr::ProdExpr;
pub use	realexpr::RealExpr;
pub use	shape::{ K_MAX_TENSOR_RANK, ShapeError, TensorShape };
pub use	sumexpr::SumExpr;
pub use	tensor::{ TensorError, TensorExpr };
pub use	termtree::{ AsTermNode, ITermNode, Term, TermBinNode };
pub use	varexpr::{ VarAttrib, VarExpr, VarKind };

//-------------------------------------------------------------------------------------------------
