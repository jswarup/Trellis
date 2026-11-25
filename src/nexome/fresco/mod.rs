// mod.rs ---------------------------------------------------------------------------------------------------------
#[cfg( feature = "tests")]
pub mod _tests;
pub mod condexpr;
pub mod diff;
pub mod eval;
pub mod exprrepos;
pub mod exprshard;
pub mod exprview;
pub mod funcexpr;
pub mod least_squares;
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
pub use	diff::{ DiffLimits, Differentiate, DifferentiateManyWithLimits, DifferentiateVector, DifferentiateWithLimits, Jacobian };
pub use	eval::{ EvalCounters, EvalError, EvalLimits, EvalOutcome };
pub use	exprrepos::{ BaseExpr, ExprCheckpoint, ExprEntry, ExprRepos, ExprTransaction };
pub use	exprshard::{ ExprGrammar, ExprParseCtx, ExprShard, PublishError, PublishLimits };
pub use exprview::{ExprReadError, ExprView, PolyView};
pub use	funcexpr::FuncExpr;
pub use	least_squares::{
    LeastSquaresCounters, LeastSquaresLimits, LeastSquaresOptions, LeastSquaresPrepError,
    LeastSquaresProblem, LeastSquaresResult, LeastSquaresSpec, LeastSquaresStatus,
};
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
