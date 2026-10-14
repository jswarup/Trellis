// _tests.rs ------------------------------------------------------------------------------------------------------
use	crate::fresco::{ ExprRepos, ITermNode, PowExpr, ProdExpr, SumExpr, VarExpr };
use	crate::flux::IFluxExportSource;
use	crate::{ TermTree, jeeves_assert, jeeves_assert_eq, jeeves_test };

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, ExprReposStoresExpressions, |ctx| {
    let  	mut repos = ExprRepos::New();
    let  	variable = repos.VarCreate( "test".to_string(), false);
    let  	real = repos.RealCreate( 2.0);
    let  	sum = repos.AddCreate( variable, real);
    jeeves_assert_eq!( ctx, repos.SzVar(), 1);
    jeeves_assert_eq!( ctx, repos.Size(), 3);
    jeeves_assert_eq!( ctx, repos.VarNameAt( 0u32), "test");
    jeeves_assert_eq!( ctx, repos.At::< VarExpr>( variable).VarIndex(), 0);
    jeeves_assert_eq!( ctx, repos.At::< SumExpr>( sum).Poly().SizeChild(), 2);
});
jeeves_test!( Fresco, TermTreeLowersAssociativeSum, |ctx| {
    let  	tree = TermTree!( "a" + "b" + "c");
    let  	mut repos = ExprRepos::New();
    let  	root = repos.PostTermTree( &tree);
    let  	expression = repos.At::< SumExpr>( root);
    jeeves_assert_eq!( ctx, tree.ChildrenCount(), 2);
    jeeves_assert_eq!( ctx, expression.Poly().SizeChild(), 3);
    jeeves_assert_eq!( ctx, expression.Poly().CommonSize(), 3);
    jeeves_assert!( ctx, !expression.Poly().IsFlip( 0));
});
jeeves_test!( Fresco, LowersAllTermOperatorsAndExportsTypedExpressions, |ctx| {
    let  	mut repos = ExprRepos::New();
    let  	diff = repos.PostTermTree( &TermTree!( "a" - "b"));
    let  	div = repos.PostTermTree( &TermTree!( "c" / "d"));
    let  	pow = repos.PostTermTree( &TermTree!( "e" ^ "f"));
    let  	json = format!( "{}", &repos as &dyn IFluxExportSource);
    jeeves_assert_eq!( ctx, repos.At::< SumExpr>( diff).Poly().CommonSize(), 1);
    jeeves_assert_eq!( ctx, repos.At::< ProdExpr>( div).Poly().CommonSize(), 1);
    jeeves_assert_eq!( ctx, repos.At::< PowExpr>( pow).Poly().CommonSize(), 1);
    jeeves_assert!( ctx, json.contains( "\"Type\": \"PowExpr\""));
    jeeves_assert!( ctx, json.contains( "\"Type\": \"VarExpr\""));
});

jeeves_test!( Fresco, VarCreateIdentifierReuseAndSerialization, |ctx| {
    let  	mut repos = ExprRepos::New();
    let  	x0 = repos.VarCreate( "x".to_string(), false);
    let  	x1 = repos.VarCreate( "x".to_string(), true);
    let  	x2 = repos.VarCreate( "x".to_string(), false);
    let  	y0 = repos.VarCreate( "y".to_string(), true);
    let  	y1 = repos.VarCreate( "y".to_string(), true);

    jeeves_assert_eq!( ctx, repos.SzVar(), 3);
    jeeves_assert_eq!( ctx, repos.Size(), 5);
    jeeves_assert_eq!( ctx, repos.At::< VarExpr>( x0).VarIndex(), 0);
    jeeves_assert_eq!( ctx, repos.At::< VarExpr>( x1).VarIndex(), 0);
    jeeves_assert_eq!( ctx, repos.At::< VarExpr>( x2).VarIndex(), 1);
    jeeves_assert_eq!( ctx, repos.At::< VarExpr>( y0).VarIndex(), 2);
    jeeves_assert_eq!( ctx, repos.At::< VarExpr>( y1).VarIndex(), 2);

    let  	json = format!( "{}", &repos as &dyn IFluxExportSource);
    jeeves_assert!( ctx, json.contains( "\"_VarIndex\": 0"));
    jeeves_assert!( ctx, json.contains( "\"_VarIndex\": 1"));
    jeeves_assert!( ctx, json.contains( "\"_VarIndex\": 2"));
});

//-------------------------------------------------------------------------------------------------
