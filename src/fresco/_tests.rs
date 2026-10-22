// _tests.rs ------------------------------------------------------------------------------------------------------
use	crate::fresco::{
    CondExpr, ExprRepos, FuncExpr, ITermNode, LogicExpr, LogicOp, PowExpr, ProdExpr, RealExpr, SumExpr, VarAttrib,
    VarExpr,
};
use	crate::flux::IFluxExportSource;
use	crate::silo::USeg;
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

//-------------------------------------------------------------------------------------------------

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

//-------------------------------------------------------------------------------------------------

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

//-------------------------------------------------------------------------------------------------

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

jeeves_test!( Fresco, ExprShardParsesElmerCppExpressions, |ctx| {
    let  	testStrings = [
        "3.287e-6*8.428031e/0.1",
        "(176.082676677973*16.666666667/1e+29*7.59276562252209e+29*1.25472101253359*1.05010931826836*10)*0.9360247712085881/100*6.9276553029*0.9952257667",
        "(176.082676677973*16.666666667/10000000000000000000000000000000*7.59276562252209e+29*1.25472101253359*1.05010931826836*10)*0.9360247712085881/100*6.9276553029*0.9952257667",
        "10000000000000000000000000000000",
        "if ((XX>=YY) > (XX<=ZZ)) (YY) else (XX)",
        "((XX>=YY) > (XX<=ZZ))",
        "if (XX<YY)  (0) else (if ((XX>=YY) and (XX<=ZZ)) YY else XX)",
        "if ( this > that) ( doThis) else ( doThat)",
        "5.0",
        "5.0 * x* y + 2 *x^5",
        "if ( this) ( doThis) else ( doThat)",
        "if ((VFact*150) > 1)  (0.1) else (1)",
        "if ((RFact*150)> 1)(if (RFact>0)(RFact*150)else (1))else (1)",
        "if ((fip1) > 0) (if ((fip1*200)>2) (fip1*200) else (2)) else (1)",
        "if((VFact*150)> 1) (0.6)else (1)",
        "if((VFact*150)> 1)(if(VFact>0)(VFact*150)else (1))else (1)",
        "if((VFact*120)> 1)(if(VFact>0)(VFact*120)else (1))else (1)",
        "if (X<T) (0) else (if ((X>=T) and (X<=M)) ((1/(M-T))*(((X-T)^3)+(X-T))) else ((((1/(M-T))*(((M-T)^3)+(X-T)))*M)/X))",
        "((if(esw=1)((((((lecdi*100*lcyt)/hdac)))/tp53))else((((((lecdi*lcyt)/hdac)))/tp53))))",
        "2 +4 -5*X^-2 +8 -7 +9 /X^3*X -5 *X^3 -7",
        "log(hep.conc/hep.x)",
        "log( x)",
        "1+(ifn.conc/ki_pias)",
        "(((((((((((((((((((((((((((((((((((((((((((((((((((((((ksumo*s100)*icf)*tchf)*vhl)*map7)*rps)*nlk)*cdk)*rb1)*mta1)*usp7)/cops5)*usp42)*fhit)/rbbp6)*phf20)*rybp)/aurka)/aurkb)*pten)*gtse1)*uchl1)*atf3)*prkdc)*pin1)*numb)/ptk2)*hexim1)/nr3c1)*tsc22)/mif)/nedd8)/s100b)*btk)*pml)*no_n)*rps14)*rpl26)*hey1)*rps3)*rpl22)*rpl11)*npm1)*rpl5)*rrna)/psme3)/ube2)*abl1)/cdk5)*lats2)/csnk))/rnf2)*usf1)/grk5)",
        "((((nfkb+1.0E)-20)/(((((nfkb+fos_jun)+nfe2)+tfap)+1.0E)-20))*100)",
        "(kcat*pi3k.conc)*ifng.vcyt",
        "if (X<T) (X) else (T)",
        "(((((((1E-8)*28232636.928289104/1.12)*3.04*1.01*1.07*1.83063494099437)*0.930242843470181*1.02*1.02)*0.8367964753841665/1.288470e*1.05)*1)*0.9912028347)*0.9764265867514584",
        "{((if (mut.kras>0) (0.05/tora/adag) else (1)))}",
        "((dusp6+1e-18)/(ppp2+dusp6+1e-18))*100",
        "((((1/(Z-Y))*(((Z-Y)^3)+(Z-Y)))*Z)/X)",
        "(5.0)",
        "((a*150)> 1)",
    ];

    USeg::FromLen( testStrings.len() as u32).Traverse( |idx| {
        let  	exprStr = testStrings[idx as usize];
        let  	mut repos = ExprRepos::New();
        let  	res = repos.ParseExpr( exprStr);
        jeeves_assert!( ctx, res.is_ok(), format!( "Failed parsing: {}", exprStr));
        let  	token = res.unwrap();
        jeeves_assert!( ctx, token != u32::MAX);
    });
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, ExprShardAstVerification, |ctx| {
    let  	mut repos = ExprRepos::New();

    // 1. Conditional parsing
    let  	condTok = repos.ParseExpr( "if (X < T) (X) else (T)").unwrap();
    let  	condExpr = repos.At::< CondExpr>( condTok);
    let  	cmpExpr = repos.At::< LogicExpr>( condExpr.Cond());
    jeeves_assert_eq!( ctx, cmpExpr.Op(), LogicOp::Lt);
    jeeves_assert_eq!( ctx, repos.VarNameAt( repos.At::< VarExpr>( condExpr.IfTrue()).VarIndex()), "X");
    jeeves_assert_eq!( ctx, repos.VarNameAt( repos.At::< VarExpr>( condExpr.IfFalse()).VarIndex()), "T");

    // 2. Logic AND / OR
    let  	andTok = repos.ParseExpr( "A and B").unwrap();
    let  	andExpr = repos.At::< LogicExpr>( andTok);
    jeeves_assert_eq!( ctx, andExpr.Op(), LogicOp::And);

    let  	orTok = repos.ParseExpr( "A or B").unwrap();
    let  	orExpr = repos.At::< LogicExpr>( orTok);
    jeeves_assert_eq!( ctx, orExpr.Op(), LogicOp::Or);

    // 3. Comparison equality with = and ==
    let  	eqTok1 = repos.ParseExpr( "X == Y").unwrap();
    jeeves_assert_eq!( ctx, repos.At::< LogicExpr>( eqTok1).Op(), LogicOp::Eq);
    let  	eqTok2 = repos.ParseExpr( "X = Y").unwrap();
    jeeves_assert_eq!( ctx, repos.At::< LogicExpr>( eqTok2).Op(), LogicOp::Eq);

    // 4. Function call
    let  	fnTok = repos.ParseExpr( "log( x)").unwrap();
    let  	fnExpr = repos.At::< FuncExpr>( fnTok);
    jeeves_assert_eq!( ctx, fnExpr.Name(), "log");
    jeeves_assert_eq!( ctx, repos.VarNameAt( repos.At::< VarExpr>( fnExpr.Arg()).VarIndex()), "x");

    // 5. Scientific notation with trailing e
    let  	trailTok = repos.ParseExpr( "8.428031e").unwrap();
    let  	realExpr = repos.At::< RealExpr>( trailTok);
    jeeves_assert!( ctx, ( realExpr.Value() - 8.428031).abs() < 1e-6);

    // 6. Large number
    let  	largeTok = repos.ParseExpr( "10000000000000000000000000000000").unwrap();
    let  	largeReal = repos.At::< RealExpr>( largeTok);
    jeeves_assert!( ctx, largeReal.Value() > 1e30);

    // 7. Serialization includes new node types
    let  	json = format!( "{}", &repos as &dyn IFluxExportSource);
    jeeves_assert!( ctx, json.contains( "\"Type\": \"CondExpr\""));
    jeeves_assert!( ctx, json.contains( "\"Type\": \"LogicExpr\""));
    jeeves_assert!( ctx, json.contains( "\"Type\": \"FuncExpr\""));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, NonAssociativeLoweringParityPropagation, |ctx| {
    let  	mut repos = ExprRepos::New();

    // 1. Additive parity: a - (b - c) => a + c - b
    let  	tree1 = TermTree!( "a" - ( "b" - "c"));
    let  	sumTok1 = repos.PostTermTree( &tree1);
    let  	sumExpr1 = repos.At::< SumExpr>( sumTok1);
    jeeves_assert_eq!( ctx, sumExpr1.Poly().CommonSize(), 2);
    jeeves_assert_eq!( ctx, sumExpr1.Poly().SizeChild(), 3);
    let  	c0 = repos.At::< VarExpr>( sumExpr1.Poly().Child( 0)).VarIndex();
    let  	c1 = repos.At::< VarExpr>( sumExpr1.Poly().Child( 1)).VarIndex();
    let  	c2 = repos.At::< VarExpr>( sumExpr1.Poly().Child( 2)).VarIndex();
    jeeves_assert_eq!( ctx, repos.VarNameAt( c0), "a");
    jeeves_assert_eq!( ctx, repos.VarNameAt( c1), "c");
    jeeves_assert_eq!( ctx, repos.VarNameAt( c2), "b");

    // 2. Additive parity: (a - b) - c => a - b - c
    let  	tree2 = TermTree!( ( "a" - "b") - "c");
    let  	sumTok2 = repos.PostTermTree( &tree2);
    let  	sumExpr2 = repos.At::< SumExpr>( sumTok2);
    jeeves_assert_eq!( ctx, sumExpr2.Poly().CommonSize(), 1);
    jeeves_assert_eq!( ctx, sumExpr2.Poly().SizeChild(), 3);
    let  	d0 = repos.At::< VarExpr>( sumExpr2.Poly().Child( 0)).VarIndex();
    let  	d1 = repos.At::< VarExpr>( sumExpr2.Poly().Child( 1)).VarIndex();
    let  	d2 = repos.At::< VarExpr>( sumExpr2.Poly().Child( 2)).VarIndex();
    jeeves_assert_eq!( ctx, repos.VarNameAt( d0), "a");
    jeeves_assert_eq!( ctx, repos.VarNameAt( d1), "b");
    jeeves_assert_eq!( ctx, repos.VarNameAt( d2), "c");

    // 3. Multiplicative parity: a / (b / c) => (a * c) / b
    let  	ptree1 = TermTree!( "a" / ( "b" / "c"));
    let  	prodTok1 = repos.PostTermTree( &ptree1);
    let  	prodExpr1 = repos.At::< ProdExpr>( prodTok1);
    jeeves_assert_eq!( ctx, prodExpr1.Poly().CommonSize(), 2);
    jeeves_assert_eq!( ctx, prodExpr1.Poly().SizeChild(), 3);
    let  	pc0 = repos.At::< VarExpr>( prodExpr1.Poly().Child( 0)).VarIndex();
    let  	pc1 = repos.At::< VarExpr>( prodExpr1.Poly().Child( 1)).VarIndex();
    let  	pc2 = repos.At::< VarExpr>( prodExpr1.Poly().Child( 2)).VarIndex();
    jeeves_assert_eq!( ctx, repos.VarNameAt( pc0), "a");
    jeeves_assert_eq!( ctx, repos.VarNameAt( pc1), "c");
    jeeves_assert_eq!( ctx, repos.VarNameAt( pc2), "b");

    // 4. Multiplicative parity: (a / b) / c => a / (b * c)
    let  	ptree2 = TermTree!( ( "a" / "b") / "c");
    let  	prodTok2 = repos.PostTermTree( &ptree2);
    let  	prodExpr2 = repos.At::< ProdExpr>( prodTok2);
    jeeves_assert_eq!( ctx, prodExpr2.Poly().CommonSize(), 1);
    jeeves_assert_eq!( ctx, prodExpr2.Poly().SizeChild(), 3);

    // 5. Exponentiation strictly binary: (a ^ b) ^ c
    let  	powTree = TermTree!( ( "a" ^ "b") ^ "c");
    let  	powTok = repos.PostTermTree( &powTree);
    let  	powExpr = repos.At::< PowExpr>( powTok);
    jeeves_assert_eq!( ctx, powExpr.Poly().SizeChild(), 2);
    let  	leftPow = repos.At::< PowExpr>( powExpr.Poly().Child( 0));
    jeeves_assert_eq!( ctx, leftPow.Poly().SizeChild(), 2);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, UnaryNotAndInequalityLexing, |ctx| {
    let  	mut repos = ExprRepos::New();

    // 1. Whitespace independence of !=
    let  	neTok1 = repos.ParseExpr( "x!=y").unwrap();
    let  	neTok2 = repos.ParseExpr( "x != y").unwrap();
    let  	expr1 = repos.At::< LogicExpr>( neTok1);
    let  	expr2 = repos.At::< LogicExpr>( neTok2);
    jeeves_assert_eq!( ctx, expr1.Op(), LogicOp::Ne);
    jeeves_assert_eq!( ctx, expr2.Op(), LogicOp::Ne);

    // 2. Unary prefix ! and not
    let  	notTok1 = repos.ParseExpr( "!x").unwrap();
    let  	notTok2 = repos.ParseExpr( "not x").unwrap();
    let  	n1 = repos.At::< LogicExpr>( notTok1);
    let  	n2 = repos.At::< LogicExpr>( notTok2);
    jeeves_assert_eq!( ctx, n1.Op(), LogicOp::Not);
    jeeves_assert_eq!( ctx, n2.Op(), LogicOp::Not);
    jeeves_assert!( ctx, n1.IsUnary());
    jeeves_assert!( ctx, n2.IsUnary());

    // 3. Compound negation: !(a > b)
    let  	compNotTok = repos.ParseExpr( "!(a > b)").unwrap();
    let  	compNot = repos.At::< LogicExpr>( compNotTok);
    jeeves_assert_eq!( ctx, compNot.Op(), LogicOp::Not);
    let  	innerCmp = repos.At::< LogicExpr>( compNot.Left());
    jeeves_assert_eq!( ctx, innerCmp.Op(), LogicOp::Gt);

    // 4. Identifier special characters excluding !
    let  	specialTok = repos.ParseExpr( "var.sub + ifn@1 + flag# - cost$").unwrap();
    jeeves_assert!( ctx, specialTok != u32::MAX);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, TransactionalRollbackOnSyntaxErrors, |ctx| {
    let  	mut repos = ExprRepos::New();
    let  	_ = repos.ParseExpr( "x + y").unwrap();
    let  	validExprs = repos.Size();
    let  	validVars = repos.SzVar();

    // Syntax errors must leave zero residue in repos
    jeeves_assert!( ctx, repos.ParseExpr( "x + ").is_err());
    jeeves_assert_eq!( ctx, repos.Size(), validExprs);
    jeeves_assert_eq!( ctx, repos.SzVar(), validVars);

    jeeves_assert!( ctx, repos.ParseExpr( "x y").is_err());
    jeeves_assert_eq!( ctx, repos.Size(), validExprs);
    jeeves_assert_eq!( ctx, repos.SzVar(), validVars);

    jeeves_assert!( ctx, repos.ParseExpr( "if (a) then").is_err());
    jeeves_assert_eq!( ctx, repos.Size(), validExprs);
    jeeves_assert_eq!( ctx, repos.SzVar(), validVars);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, VarAttribSentinelAndLiteralNegation, |ctx| {
    let  	mut repos = ExprRepos::New();

    // Literal negation folding (1 node only, zero garbage nodes)
    let  	negTok = repos.ParseExpr( "-42.5").unwrap();
    jeeves_assert_eq!( ctx, repos.Size(), 1);
    let  	real = repos.At::< RealExpr>( negTok);
    jeeves_assert_eq!( ctx, real.Value(), -42.5);

    // VarAttrib sentinel verification
    let  	attr = VarAttrib::New( "testVar".to_string());
    jeeves_assert!( ctx, attr.IsIndependent());
    jeeves_assert!( ctx, !attr.IsDependent());
    jeeves_assert!( ctx, !attr.IsAggregate());
    jeeves_assert_eq!( ctx, attr.DepExpr(), u32::MAX);
    jeeves_assert_eq!( ctx, attr.AggrIndex(), u32::MAX);
});

//-------------------------------------------------------------------------------------------------
