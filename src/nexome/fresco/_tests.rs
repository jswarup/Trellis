// _tests.rs ------------------------------------------------------------------------------------------------------
use crate::flux::{FieldImp, FixedStream, IFluxExportSource};
use crate::fresco::{
    CondExpr, DifferentiateVector, EvalError, ExprGrammar,
    ExprParseCtx, ExprRepos, FuncExpr, ITermNode, Jacobian, LogicExpr, LogicOp, PowExpr,
    ProdExpr, RealExpr, ShapeError, SumExpr, TensorExpr, TensorShape, VarAttrib, VarExpr, VarKind,
};
use crate::shard::{JsonGrammar, JsonParseCtx, Parser};
use crate::silo::{Arr, USeg};
use crate::{TermTree, jeeves_assert, jeeves_assert_eq, jeeves_test};

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, ExprReposStoresExpressions, |ctx| {
    let mut repos = ExprRepos::New();
    let variable = repos.VarCreate("test".to_string(), false);
    let real = repos.RealCreate(2.0);
    let sum = repos.AddCreate(variable, real);
    jeeves_assert_eq!(ctx, repos.SzVar(), 1);
    jeeves_assert_eq!(ctx, repos.Size(), 3);
    jeeves_assert_eq!(ctx, repos.VarNameAt(0u32), "test");
    jeeves_assert_eq!(ctx, repos.At::<VarExpr>(variable).VarIndex(), 0);
    jeeves_assert_eq!(ctx, repos.At::<SumExpr>(sum).Poly().SizeChild(), 2);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, TermTreeLowersAssociativeSum, |ctx| {
    let tree = TermTree!("a" + "b" + "c");
    let mut repos = ExprRepos::New();
    let root = repos.PostTermTree(&tree);
    let expression = repos.At::<SumExpr>(root);
    jeeves_assert_eq!(ctx, tree.ChildrenCount(), 2);
    jeeves_assert_eq!(ctx, expression.Poly().SizeChild(), 3);
    jeeves_assert_eq!(ctx, expression.Poly().CommonSize(), 3);
    jeeves_assert!(ctx, !expression.Poly().IsFlip(0));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco,
             LowersAllTermOperatorsAndExportsTypedExpressions,
             |ctx| {
                 let mut repos = ExprRepos::New();
                 let diff = repos.PostTermTree(&TermTree!("a" - "b"));
                 let div = repos.PostTermTree(&TermTree!("c" / "d"));
                 let pow = repos.PostTermTree(&TermTree!("e" ^ "f"));
                 let json = format!("{}", &repos as &dyn IFluxExportSource);
                 jeeves_assert_eq!(ctx, repos.At::<SumExpr>(diff).Poly().CommonSize(), 1);
                 jeeves_assert_eq!(ctx, repos.At::<ProdExpr>(div).Poly().CommonSize(), 1);
                 jeeves_assert_eq!(ctx, repos.At::<PowExpr>(pow).Poly().CommonSize(), 1);
                 jeeves_assert!(ctx, json.contains("\"Type\": \"PowExpr\""));
                 jeeves_assert!(ctx, json.contains("\"Type\": \"VarExpr\""));
             });

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, VarCreateIdentifierReuseAndSerialization, |ctx| {
    let mut repos = ExprRepos::New();
    let x0 = repos.VarCreate("x".to_string(), false);
    let x1 = repos.VarCreate("x".to_string(), true);
    let x2 = repos.VarCreate("x".to_string(), false);
    let y0 = repos.VarCreate("y".to_string(), true);
    let y1 = repos.VarCreate("y".to_string(), true);

    jeeves_assert_eq!(ctx, repos.SzVar(), 3);
    jeeves_assert_eq!(ctx, repos.Size(), 5);
    jeeves_assert_eq!(ctx, repos.At::<VarExpr>(x0).VarIndex(), 0);
    jeeves_assert_eq!(ctx, repos.At::<VarExpr>(x1).VarIndex(), 0);
    jeeves_assert_eq!(ctx, repos.At::<VarExpr>(x2).VarIndex(), 1);
    jeeves_assert_eq!(ctx, repos.At::<VarExpr>(y0).VarIndex(), 2);
    jeeves_assert_eq!(ctx, repos.At::<VarExpr>(y1).VarIndex(), 2);

    let json = format!("{}", &repos as &dyn IFluxExportSource);
    jeeves_assert!(ctx, json.contains("\"_VarIndex\": 0"));
    jeeves_assert!(ctx, json.contains("\"_VarIndex\": 1"));
    jeeves_assert!(ctx, json.contains("\"_VarIndex\": 2"));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, ExprShardParsesElmerCppExpressions, |ctx| {
    let testStrings = ["3.287e-6*8.428031e/0.1",
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
                       "((a*150)> 1)"];

    USeg::FromLen(testStrings.len() as u32).Traverse(|idx| {
                                               let exprStr = testStrings[idx as usize];
                                               let mut repos = ExprRepos::New();
                                               let res = repos.ParseExpr(exprStr);
                                               jeeves_assert!(ctx,
                                                              res.is_ok(),
                                                              format!("Failed parsing: {}",
                                                                      exprStr));
                                               let token = res.unwrap();
                                               jeeves_assert!(ctx, token != u32::MAX);
                                           });
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, ExprShardAstVerification, |ctx| {
    let mut repos = ExprRepos::New();

    // 1. Conditional parsing
    let condTok = repos.ParseExpr("if (X < T) (X) else (T)").unwrap();
    let condExpr = repos.At::<CondExpr>(condTok);
    let cmpExpr = repos.At::<LogicExpr>(condExpr.Cond());
    jeeves_assert_eq!(ctx, cmpExpr.Op(), LogicOp::Lt);
    jeeves_assert_eq!(ctx,
                      repos.VarNameAt(repos.At::<VarExpr>(condExpr.IfTrue()).VarIndex()),
                      "X");
    jeeves_assert_eq!(ctx,
                      repos.VarNameAt(repos.At::<VarExpr>(condExpr.IfFalse()).VarIndex()),
                      "T");

    // 2. Logic AND / OR
    let andTok = repos.ParseExpr("A and B").unwrap();
    let andExpr = repos.At::<LogicExpr>(andTok);
    jeeves_assert_eq!(ctx, andExpr.Op(), LogicOp::And);

    let orTok = repos.ParseExpr("A or B").unwrap();
    let orExpr = repos.At::<LogicExpr>(orTok);
    jeeves_assert_eq!(ctx, orExpr.Op(), LogicOp::Or);

    // 3. Comparison equality with = and ==
    let eqTok1 = repos.ParseExpr("X == Y").unwrap();
    jeeves_assert_eq!(ctx, repos.At::<LogicExpr>(eqTok1).Op(), LogicOp::Eq);
    let eqTok2 = repos.ParseExpr("X = Y").unwrap();
    jeeves_assert_eq!(ctx, repos.At::<LogicExpr>(eqTok2).Op(), LogicOp::Eq);

    // 4. Function call
    let fnTok = repos.ParseExpr("log( x)").unwrap();
    let fnExpr = repos.At::<FuncExpr>(fnTok);
    jeeves_assert_eq!(ctx, fnExpr.Name(), "log");
    jeeves_assert_eq!(ctx,
                      repos.VarNameAt(repos.At::<VarExpr>(fnExpr.Arg()).VarIndex()),
                      "x");

    // 5. Scientific notation with trailing e
    let trailTok = repos.ParseExpr("8.428031e").unwrap();
    let realExpr = repos.At::<RealExpr>(trailTok);
    jeeves_assert!(ctx, (realExpr.Value() - 8.428031).abs() < 1e-6);

    // 6. Large number
    let largeTok = repos.ParseExpr("10000000000000000000000000000000").unwrap();
    let largeReal = repos.At::<RealExpr>(largeTok);
    jeeves_assert!(ctx, largeReal.Value() > 1e30);

    // 7. Serialization includes new node types
    let json = format!("{}", &repos as &dyn IFluxExportSource);
    jeeves_assert!(ctx, json.contains("\"Type\": \"CondExpr\""));
    jeeves_assert!(ctx, json.contains("\"Type\": \"LogicExpr\""));
    jeeves_assert!(ctx, json.contains("\"Type\": \"FuncExpr\""));
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, NonAssociativeLoweringParityPropagation, |ctx| {
    let mut repos = ExprRepos::New();

    // 1. Additive parity: a - (b - c) => a + c - b
    let tree1 = TermTree!("a" - ("b" - "c"));
    let sumTok1 = repos.PostTermTree(&tree1);
    let sumExpr1 = repos.At::<SumExpr>(sumTok1);
    jeeves_assert_eq!(ctx, sumExpr1.Poly().CommonSize(), 2);
    jeeves_assert_eq!(ctx, sumExpr1.Poly().SizeChild(), 3);
    let c0 = repos.At::<VarExpr>(sumExpr1.Poly().Child(0)).VarIndex();
    let c1 = repos.At::<VarExpr>(sumExpr1.Poly().Child(1)).VarIndex();
    let c2 = repos.At::<VarExpr>(sumExpr1.Poly().Child(2)).VarIndex();
    jeeves_assert_eq!(ctx, repos.VarNameAt(c0), "a");
    jeeves_assert_eq!(ctx, repos.VarNameAt(c1), "c");
    jeeves_assert_eq!(ctx, repos.VarNameAt(c2), "b");

    // 2. Additive parity: (a - b) - c => a - b - c
    let tree2 = TermTree!(("a" - "b") - "c");
    let sumTok2 = repos.PostTermTree(&tree2);
    let sumExpr2 = repos.At::<SumExpr>(sumTok2);
    jeeves_assert_eq!(ctx, sumExpr2.Poly().CommonSize(), 1);
    jeeves_assert_eq!(ctx, sumExpr2.Poly().SizeChild(), 3);
    let d0 = repos.At::<VarExpr>(sumExpr2.Poly().Child(0)).VarIndex();
    let d1 = repos.At::<VarExpr>(sumExpr2.Poly().Child(1)).VarIndex();
    let d2 = repos.At::<VarExpr>(sumExpr2.Poly().Child(2)).VarIndex();
    jeeves_assert_eq!(ctx, repos.VarNameAt(d0), "a");
    jeeves_assert_eq!(ctx, repos.VarNameAt(d1), "b");
    jeeves_assert_eq!(ctx, repos.VarNameAt(d2), "c");

    // 3. Multiplicative parity: a / (b / c) => (a * c) / b
    let ptree1 = TermTree!("a" / ("b" / "c"));
    let prodTok1 = repos.PostTermTree(&ptree1);
    let prodExpr1 = repos.At::<ProdExpr>(prodTok1);
    jeeves_assert_eq!(ctx, prodExpr1.Poly().CommonSize(), 2);
    jeeves_assert_eq!(ctx, prodExpr1.Poly().SizeChild(), 3);
    let pc0 = repos.At::<VarExpr>(prodExpr1.Poly().Child(0)).VarIndex();
    let pc1 = repos.At::<VarExpr>(prodExpr1.Poly().Child(1)).VarIndex();
    let pc2 = repos.At::<VarExpr>(prodExpr1.Poly().Child(2)).VarIndex();
    jeeves_assert_eq!(ctx, repos.VarNameAt(pc0), "a");
    jeeves_assert_eq!(ctx, repos.VarNameAt(pc1), "c");
    jeeves_assert_eq!(ctx, repos.VarNameAt(pc2), "b");

    // 4. Multiplicative parity: (a / b) / c => a / (b * c)
    let ptree2 = TermTree!(("a" / "b") / "c");
    let prodTok2 = repos.PostTermTree(&ptree2);
    let prodExpr2 = repos.At::<ProdExpr>(prodTok2);
    jeeves_assert_eq!(ctx, prodExpr2.Poly().CommonSize(), 1);
    jeeves_assert_eq!(ctx, prodExpr2.Poly().SizeChild(), 3);

    // 5. Exponentiation strictly binary: (a ^ b) ^ c
    let powTree = TermTree!(("a" ^ "b") ^ "c");
    let powTok = repos.PostTermTree(&powTree);
    let powExpr = repos.At::<PowExpr>(powTok);
    jeeves_assert_eq!(ctx, powExpr.Poly().SizeChild(), 2);
    let leftPow = repos.At::<PowExpr>(powExpr.Poly().Child(0));
    jeeves_assert_eq!(ctx, leftPow.Poly().SizeChild(), 2);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, UnaryNotAndInequalityLexing, |ctx| {
    let mut repos = ExprRepos::New();

    // 1. Whitespace independence of !=
    let neTok1 = repos.ParseExpr("x!=y").unwrap();
    let neTok2 = repos.ParseExpr("x != y").unwrap();
    let expr1 = repos.At::<LogicExpr>(neTok1);
    let expr2 = repos.At::<LogicExpr>(neTok2);
    jeeves_assert_eq!(ctx, expr1.Op(), LogicOp::Ne);
    jeeves_assert_eq!(ctx, expr2.Op(), LogicOp::Ne);

    // 2. Unary prefix ! and not
    let notTok1 = repos.ParseExpr("!x").unwrap();
    let notTok2 = repos.ParseExpr("not x").unwrap();
    let n1 = repos.At::<LogicExpr>(notTok1);
    let n2 = repos.At::<LogicExpr>(notTok2);
    jeeves_assert_eq!(ctx, n1.Op(), LogicOp::Not);
    jeeves_assert_eq!(ctx, n2.Op(), LogicOp::Not);
    jeeves_assert!(ctx, n1.IsUnary());
    jeeves_assert!(ctx, n2.IsUnary());

    // 3. Compound negation: !(a > b)
    let compNotTok = repos.ParseExpr("!(a > b)").unwrap();
    let compNot = repos.At::<LogicExpr>(compNotTok);
    jeeves_assert_eq!(ctx, compNot.Op(), LogicOp::Not);
    let innerCmp = repos.At::<LogicExpr>(compNot.Left());
    jeeves_assert_eq!(ctx, innerCmp.Op(), LogicOp::Gt);

    // 4. Identifier special characters excluding !
    let specialTok = repos.ParseExpr("var.sub + ifn@1 + flag# - cost$").unwrap();
    jeeves_assert!(ctx, specialTok != u32::MAX);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, TransactionalRollbackOnSyntaxErrors, |ctx| {
    let mut repos = ExprRepos::New();
    let _ = repos.ParseExpr("x + y").unwrap();
    let validExprs = repos.Size();
    let validVars = repos.SzVar();

    // Syntax errors must leave zero residue in repos
    jeeves_assert!(ctx, repos.ParseExpr("x + ").is_err());
    jeeves_assert_eq!(ctx, repos.Size(), validExprs);
    jeeves_assert_eq!(ctx, repos.SzVar(), validVars);

    jeeves_assert!(ctx, repos.ParseExpr("x y").is_err());
    jeeves_assert_eq!(ctx, repos.Size(), validExprs);
    jeeves_assert_eq!(ctx, repos.SzVar(), validVars);

    jeeves_assert!(ctx, repos.ParseExpr("if (a) then").is_err());
    jeeves_assert_eq!(ctx, repos.Size(), validExprs);
    jeeves_assert_eq!(ctx, repos.SzVar(), validVars);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, VarAttribSentinelAndLiteralNegation, |ctx| {
    let mut repos = ExprRepos::New();

    // Literal negation folding (1 node only, zero garbage nodes)
    let negTok = repos.ParseExpr("-42.5").unwrap();
    jeeves_assert_eq!(ctx, repos.Size(), 1);
    let real = repos.At::<RealExpr>(negTok);
    jeeves_assert_eq!(ctx, real.Value(), -42.5);

    // VarAttrib sentinel verification
    let attr = VarAttrib::New("testVar".to_string());
    jeeves_assert!(ctx, attr.IsIndependent());
    jeeves_assert!(ctx, !attr.IsDependent());
    jeeves_assert!(ctx, !attr.IsAggregate());
    jeeves_assert_eq!(ctx, attr.DepExpr(), u32::MAX);
    jeeves_assert_eq!(ctx, attr.AggrIndex(), u32::MAX);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, VarKindFlagsAndNonZeroConfiguration, |ctx| {
    let mut repos = ExprRepos::New();
    let x = repos.VarCreate("x".to_string(), true);
    let vIdx = repos.At::<VarExpr>(x).VarIndex();

    jeeves_assert!(ctx, !repos.VarAttrAt(vIdx).HasBits(VarKind::NonZero));
    jeeves_assert_eq!(ctx, repos.SetVarKind(vIdx, VarKind::NonZero, true), Ok(()));
    jeeves_assert!(ctx, repos.VarAttrAt(vIdx).HasBits(VarKind::NonZero));

    // Setting Prime bit preserves NonZero
    jeeves_assert_eq!(ctx, repos.SetVarKind(vIdx, VarKind::Prime, true), Ok(()));
    jeeves_assert!(ctx, repos.VarAttrAt(vIdx).HasBits(VarKind::NonZero));
    jeeves_assert!(ctx, repos.VarAttrAt(vIdx).HasBits(VarKind::Prime));

    // Toggle NonZero off preserves Prime
    jeeves_assert_eq!(ctx, repos.SetVarKind(vIdx, VarKind::NonZero, false), Ok(()));
    jeeves_assert!(ctx, !repos.VarAttrAt(vIdx).HasBits(VarKind::NonZero));
    jeeves_assert!(ctx, repos.VarAttrAt(vIdx).HasBits(VarKind::Prime));

    // Out of bounds variable ID returns Err
    jeeves_assert!(ctx,
                   repos.SetVarKind(999u32, VarKind::NonZero, true).is_err());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, StagingRollbackAndComposedGrammarMatch, |ctx| {
    let mut repos = ExprRepos::New();
    let _ = repos.VarCreate("a".to_string(), true);
    let initialExprCount = repos.Size();
    let initialVarCount = repos.SzVar();

    // Test composed grammar matching with ExprGrammar and ExprParseCtx
    let mut stream = FixedStream::from("123.45 trailing");
    let mut parser = Parser::New(&mut stream);
    let mut parseCtx = ExprParseCtx::New(&mut repos);
    jeeves_assert!(ctx, parser.ParseGrammar(&ExprGrammar, &mut parseCtx, 0).is_some());
    jeeves_assert!(ctx, parseCtx.RootToken() != u32::MAX);
    jeeves_assert_eq!(ctx, repos.Size(), initialExprCount + 1);
    jeeves_assert_eq!(ctx, repos.SzVar(), initialVarCount);

    // Failed match discards temporary staging and leaves repos 100% untouched
    let mut badStream = FixedStream::from(") bad");
    let mut badParser = Parser::New(&mut badStream);
    let prevExprCount = repos.Size();
    let prevVarCount = repos.SzVar();
    let mut badCtx = ExprParseCtx::New(&mut repos);
    jeeves_assert!(ctx, badParser.ParseGrammar(&ExprGrammar, &mut badCtx, 0).is_none());
    jeeves_assert_eq!(ctx, badCtx.RootToken(), u32::MAX);
    jeeves_assert_eq!(ctx, badCtx.StagedRoot(), u32::MAX);
    jeeves_assert_eq!(ctx, repos.Size(), prevExprCount);
    jeeves_assert_eq!(ctx, repos.SzVar(), prevVarCount);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, RecursionDepthLimitsAndCheckedConstructors, |ctx| {
    let mut repos = ExprRepos::New();

    // Pathological nested parentheses (300 deep) exceeds K_MAX_EXPR_DEPTH = 256
    let mut deep = String::new();
    USeg::FromLen(300).Traverse(|_| deep.push('('));
    deep.push_str("1.0");
    USeg::FromLen(300).Traverse(|_| deep.push(')'));
    jeeves_assert!(ctx, repos.ParseExpr(&deep).is_err());

    // Checked constructors validation
    let r1 = repos.RealCreate(10.0);
    let r2 = repos.RealCreate(20.0);

    // CheckedPowerCreate
    jeeves_assert!(ctx, repos.CheckedPowerCreate(r1, r2).is_ok());
    jeeves_assert!(ctx, repos.CheckedPowerCreate(r1, 9999u32).is_err());

    // CheckedLogicCreate: Not is invalid as binary
    jeeves_assert!(ctx, repos.CheckedLogicCreate(LogicOp::Not, r1, r2).is_err());
    jeeves_assert!(ctx, repos.CheckedLogicCreate(LogicOp::Eq, r1, r2).is_ok());
    jeeves_assert!(ctx,
                   repos.CheckedLogicCreate(LogicOp::Eq, r1, 9999u32).is_err());

    // CheckedLogicUnaryCreate: only Not is valid as unary
    jeeves_assert!(ctx, repos.CheckedLogicUnaryCreate(LogicOp::Eq, r1).is_err());
    jeeves_assert!(ctx, repos.CheckedLogicUnaryCreate(LogicOp::Not, r1).is_ok());
    jeeves_assert!(ctx,
                   repos.CheckedLogicUnaryCreate(LogicOp::Not, 9999u32)
                        .is_err());

    // CheckedCondCreate
    jeeves_assert!(ctx, repos.CheckedCondCreate(r1, r2, r1).is_ok());
    jeeves_assert!(ctx, repos.CheckedCondCreate(r1, 9999u32, r1).is_err());

    // CheckedFuncCreate
    jeeves_assert!(ctx, repos.CheckedFuncCreate("sin".to_string(), r1).is_ok());
    jeeves_assert!(ctx, repos.CheckedFuncCreate("".to_string(), r1).is_err());
    jeeves_assert!(ctx,
                   repos.CheckedFuncCreate("sin".to_string(), 9999u32).is_err());

    // TryAt safe access
    jeeves_assert!(ctx, repos.TryAt::<RealExpr>(r1).is_some());
    jeeves_assert!(ctx, repos.TryAt::<VarExpr>(r1).is_none());
    jeeves_assert!(ctx, repos.TryAt::<RealExpr>(9999u32).is_none());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, ReferenceEvaluatorBasicAndConditionals, |ctx| {
    let mut repos = ExprRepos::New();

    // 1. Basic arithmetic: 2 * x + 3
    let expr1 = repos.ParseExpr("2 * x + 3").unwrap();
    let named1 = [("x", 4.5)];
    let res1 = repos.EvaluateNamed(expr1, Arr::New(named1.as_ptr(), named1.len() as u32))
                    .unwrap();
    jeeves_assert_eq!(ctx, res1, 12.0);

    // 2. Multi-term sum & division: (a + b) / (c * d)
    let expr2 = repos.ParseExpr("(a + b) / (c * d)").unwrap();
    let named2 = [("a", 10.0), ("b", 20.0), ("c", 2.0), ("d", 3.0)];
    let res2 = repos.EvaluateNamed(expr2, Arr::New(named2.as_ptr(), named2.len() as u32))
                    .unwrap();
    jeeves_assert_eq!(ctx, res2, 5.0);

    // 3. Division by zero in denominator
    let namedZero = [("a", 10.0), ("b", 20.0), ("c", 0.0), ("d", 3.0)];
    let resZero = repos.EvaluateNamed(expr2, Arr::New(namedZero.as_ptr(), namedZero.len() as u32));
    jeeves_assert_eq!(ctx, resZero, Err(EvalError::DivisionByZero));

    // 4. Strict laziness in conditionals: if (x == 0) 42 else (1 / x)
    let exprLazy = repos.ParseExpr("if (x == 0) 42 else (1 / x)").unwrap();
    let namedLazyZero = [("x", 0.0)];
    let resLazy = repos.EvaluateNamed(exprLazy,
                                      Arr::New(namedLazyZero.as_ptr(), namedLazyZero.len() as u32))
                       .unwrap();
    jeeves_assert_eq!(ctx, resLazy, 42.0);

    let namedLazyNonZero = [("x", 2.0)];
    let resLazy2 = repos.EvaluateNamed(exprLazy,
                                       Arr::New(namedLazyNonZero.as_ptr(),
                                                namedLazyNonZero.len() as u32))
                        .unwrap();
    jeeves_assert_eq!(ctx, resLazy2, 0.5);

    // 5. Short-circuit logic: (x != 0) and (1 / x > 0)
    let exprShortCircuit = repos.ParseExpr("(x != 0) and (1 / x > 0)").unwrap();
    let resShort = repos.EvaluateNamed(exprShortCircuit,
                                       Arr::New(namedLazyZero.as_ptr(),
                                                namedLazyZero.len() as u32))
                        .unwrap();
    jeeves_assert_eq!(ctx, resShort, 0.0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, ReferenceEvaluatorFunctionsAndDomainAssumptions, |ctx| {
    let mut repos = ExprRepos::New();

    // 1. Mathematical functions: sqrt, exp, log
    let exprFunc = repos.ParseExpr("sqrt(a) + exp(b)").unwrap();
    let namedFunc = [("a", 16.0), ("b", 0.0)];
    let resFunc = repos.EvaluateNamed(exprFunc,
                                      Arr::New(namedFunc.as_ptr(), namedFunc.len() as u32))
                       .unwrap();
    jeeves_assert_eq!(ctx, resFunc, 5.0);

    // Domain error on sqrt(-1)
    let namedNegSqrt = [("a", -4.0), ("b", 0.0)];
    let resNegSqrt = repos.EvaluateNamed(exprFunc,
                                         Arr::New(namedNegSqrt.as_ptr(),
                                                  namedNegSqrt.len() as u32));
    jeeves_assert!(ctx, matches!(resNegSqrt, Err(EvalError::DomainError(_))));

    // 2. NonZero variable assumption validation
    let exprAssump = repos.ParseExpr("100 / y").unwrap();
    let yId = repos.FindVar("y");
    jeeves_assert!(ctx, yId != u32::MAX);
    repos.SetVarKind(yId, VarKind::NonZero, true).unwrap();

    // Bound to 0.0 violates NonZero assumption
    let namedYZero = [("y", 0.0)];
    let resYZero = repos.EvaluateNamed(exprAssump,
                                       Arr::New(namedYZero.as_ptr(), namedYZero.len() as u32));
    jeeves_assert_eq!(ctx,
                      resYZero,
                      Err(EvalError::AssumptionViolated { var_id: yId,
                                                          reason: "NonZero variable cannot be bound to zero", }));

    // Bound to 5.0 succeeds
    let namedYValid = [("y", 5.0)];
    let resYValid = repos.EvaluateNamed(exprAssump,
                                        Arr::New(namedYValid.as_ptr(), namedYValid.len() as u32))
                         .unwrap();
    jeeves_assert_eq!(ctx, resYValid, 20.0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, ExprReposCheckpointRollbackTransaction, |ctx| {
    let mut repos = ExprRepos::New();
    let baseExpr = repos.ParseExpr("a + b").unwrap();
    jeeves_assert_eq!(ctx, repos.Size(), 3);
    jeeves_assert_eq!(ctx, repos.SzVar(), 2);

    let cp = repos.Checkpoint();

    // Perform speculative additions
    let _specExpr = repos.ParseExpr("c * d + e").unwrap();
    jeeves_assert!(ctx, repos.Size() > 3);
    jeeves_assert_eq!(ctx, repos.SzVar(), 5);

    // Rollback to checkpoint
    repos.Rollback(cp);
    jeeves_assert_eq!(ctx, repos.Size(), 3);
    jeeves_assert_eq!(ctx, repos.SzVar(), 2);
    jeeves_assert_eq!(ctx, repos.FindVar("c"), u32::MAX);
    jeeves_assert_eq!(ctx, repos.FindVar("d"), u32::MAX);
    jeeves_assert_eq!(ctx, repos.FindVar("e"), u32::MAX);

    // Original base expression remains completely valid and evaluatable
    let named = [("a", 10.0), ("b", 20.0)];
    let res = repos.EvaluateNamed(baseExpr, Arr::New(named.as_ptr(), named.len() as u32))
                   .unwrap();
    jeeves_assert_eq!(ctx, res, 30.0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco,
             OwningExpressionImportPreservesTargetOnFailure,
             |ctx| {
                 use crate::flux::IFluxImportSource;
                 use crate::fresco::ExprEntry;
                 use crate::shard::IGrammar;

                 let mut target = ExprEntry::Expr(Box::new(RealExpr::New(7.0)));
                 let original = format!("{}", &target as &dyn IFluxExportSource);
                 {
                     let mut descriptor = FieldImp::Null;
                     target.FetchFieldImp(&mut descriptor);
                     let FieldImp::Obj(mut importer) = descriptor else {
                         panic!("object expected");
                     };
                     importer.Field("Type")
                             .unwrap()
                             .TryPostStr("RealExpr")
                             .unwrap();
                     importer.Field("_Value").unwrap().TryPostF64(99.0).unwrap();
                 }
                 jeeves_assert_eq!(ctx,
                                   format!("{}", &target as &dyn IFluxExportSource),
                                   original);

                 let malformed = [r#"{"Type":"RealExpr","_Value":99} trailing"#,
                                  r#"{"Type":"RealExpr","_Value":99,"unknown":0}"#,
                                  r#"{"Type":"RealExpr","_Value":"wrong"}"#,
                                  r#"{"Type":"Unknown","_Value":99}"#,
                                  r#"{"Type":"SumExpr","_Poly":{"_CoSize":2,"_Children":[0]}}"#,
                                  r#"{"Type":"SumExpr","_Poly":{"_CoSize":0,"_Children":[0,1]}} trailing"#,
                                  r#"{"Type":"RealExpr","_Value":99"#];
                 USeg::FromLen(malformed.len() as u32).Traverse(|index| {
                     {
                         let mut json = JsonParseCtx::New(FieldImp::FluxSource(&mut target));
                         jeeves_assert!(ctx,
                                        JsonGrammar.ParseStr(malformed[index as usize], &mut json)
                                                   .is_err());
                     }
                     jeeves_assert_eq!(ctx,
                                       format!("{}", &target as &dyn IFluxExportSource),
                                       original);
                 });
                 {
                     let mut json = JsonParseCtx::New(FieldImp::FluxSource(&mut target));
                     jeeves_assert!(ctx,
                                    JsonGrammar.ParseStr(r#"{"Type":"RealExpr","_Value":99}"#,
                                                         &mut json)
                                               .is_ok());
                 }
                 let ExprEntry::Expr(expression) = &target else {
                     panic!("expression expected");
                 };
                 jeeves_assert_eq!(ctx,
                                   expression.Any().downcast_ref::<RealExpr>().unwrap().Value(),
                                   99.0);
             });

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco,
             RepositoryImportStagesNestedChangesAndSymbolIndex,
             |ctx| {
                 use crate::shard::IGrammar;

                 let mut target = ExprRepos::New();
                 target.ParseExpr("x + 1").unwrap();
                 let original = format!("{}", &target as &dyn IFluxExportSource);
                 let malformed = [r#"{"_Exprs":[{"Type":"RealExpr","_Value":99}],"unknown":0}"#,
                                  r#"{"_VarAttribs":[{"_Name":"renamed"}],"_Exprs":[{"Type":"Unknown"}]}"#];
                 USeg::FromLen(malformed.len() as u32).Traverse(|index| {
                     {
                         let mut json = JsonParseCtx::New(FieldImp::FluxSource(&mut target));
                         jeeves_assert!(ctx,
                                        JsonGrammar.ParseStr(malformed[index as usize], &mut json)
                                                   .is_err());
                     }
                     jeeves_assert_eq!(ctx,
                                       format!("{}", &target as &dyn IFluxExportSource),
                                       original);
                 });
                 let mut replacement = ExprRepos::New();
                 replacement.ParseExpr("z + 2").unwrap();
                 let serialized = format!("{}", &replacement as &dyn IFluxExportSource);
                 let trailing = format!("{serialized} trailing");
                 {
                     let mut stream = FixedStream::from(trailing.as_str());
                     let mut json = JsonParseCtx::New(FieldImp::FluxSource(&mut target));
                     jeeves_assert!(ctx,
                                    matches!(JsonGrammar.ParseStream(&mut stream, &mut json),
                                             Err(crate::shard::ParseError::TrailingGarbage { .. })));
                 }
                 jeeves_assert_eq!(ctx,
                                   format!("{}", &target as &dyn IFluxExportSource),
                                   original);
                 {
                     let mut json = JsonParseCtx::New(FieldImp::FluxSource(&mut target));
                     jeeves_assert!(ctx, JsonGrammar.ParseStr(&serialized, &mut json).is_ok());
                 }
                 jeeves_assert!(ctx, target.FindVar("z") != u32::MAX);
                 jeeves_assert_eq!(ctx, target.FindVar("x"), u32::MAX);
                 jeeves_assert_eq!(ctx,
                                   format!("{}", &target as &dyn IFluxExportSource),
                                   serialized);
             });

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fresco, ExprReposFluxRoundTripHeterogeneousAst, |ctx| {
    let mut repos = ExprRepos::New();
    let e1 = repos.ParseExpr("a + b * c").unwrap();
    let e2 = repos.ParseExpr("sin(x) ^ 2").unwrap();
    let e3 = repos.ParseExpr("if (a > b) (x + 1) else (y - 2)").unwrap();
    let e4 = repos.ParseExpr("not (a == b) and (c < d)").unwrap();

    let json_str = format!("{}", &repos as &dyn IFluxExportSource);
    jeeves_assert!(ctx, json_str.contains("\"Type\": \"RealExpr\""));
    jeeves_assert!(ctx, json_str.contains("\"Type\": \"VarExpr\""));
    jeeves_assert!(ctx, json_str.contains("\"Type\": \"SumExpr\""));
    jeeves_assert!(ctx, json_str.contains("\"Type\": \"ProdExpr\""));
    jeeves_assert!(ctx, json_str.contains("\"Type\": \"PowExpr\""));
    jeeves_assert!(ctx, json_str.contains("\"Type\": \"LogicExpr\""));
    jeeves_assert!(ctx, json_str.contains("\"Type\": \"CondExpr\""));
    jeeves_assert!(ctx, json_str.contains("\"Type\": \"FuncExpr\""));

    // Deserialize into fresh repos
    let mut restored = ExprRepos::New();
    {
        let mut stream = FixedStream::from(json_str.as_str());
        let mut parser = Parser::New(&mut stream);
        let mut json = JsonParseCtx::New(FieldImp::FluxSource(&mut restored));
        let matched = parser.ParseGrammar(&JsonGrammar, &mut json, 0);
        jeeves_assert!(ctx, matched.is_some());
    }

    // Verify restored state
    jeeves_assert_eq!(ctx, restored.Size(), repos.Size());
    jeeves_assert_eq!(ctx, restored.SzVar(), repos.SzVar());

    // Verify evaluation on restored expressions
    let bindings = [("a", 5.0),
                    ("b", 3.0),
                    ("c", 2.0),
                    ("d", 10.0),
                    ("x", 0.5),
                    ("y", 8.0)];
    let bind_arr = Arr::New(bindings.as_ptr(), bindings.len() as u32);

    let res_orig1 = repos.EvaluateNamed(e1, bind_arr).unwrap();
    let res_rest1 = restored.EvaluateNamed(e1, bind_arr).unwrap();
    jeeves_assert_eq!(ctx, res_rest1, res_orig1);

    let res_orig2 = repos.EvaluateNamed(e2, bind_arr).unwrap();
    let res_rest2 = restored.EvaluateNamed(e2, bind_arr).unwrap();
    jeeves_assert_eq!(ctx, res_rest2, res_orig2);

    let res_orig3 = repos.EvaluateNamed(e3, bind_arr).unwrap();
    let res_rest3 = restored.EvaluateNamed(e3, bind_arr).unwrap();
    jeeves_assert_eq!(ctx, res_rest3, res_orig3);

    let res_orig4 = repos.EvaluateNamed(e4, bind_arr).unwrap();
    let res_rest4 = restored.EvaluateNamed(e4, bind_arr).unwrap();
    jeeves_assert_eq!(ctx, res_rest4, res_orig4);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, SymbolicDifferentiationAndPolynomialNormalization, |ctx| {
    use	crate::fresco::{ CanonicalPolynomial, Differentiate, ExprRepos };
    use	crate::silo::Arr;

    let  	mut repos = ExprRepos::New();
    let  	x = repos.VarCreate( "x".to_string(), true);
    let  	xVarId = repos.FindVar( "x");

    // 1. Differentiate f(x) = 3*x^2 + 5*x - 7
    // f'(x) = 6*x + 5
    let  	c3 = repos.RealCreate( 3.0);
    let  	c2 = repos.RealCreate( 2.0);
    let  	c5 = repos.RealCreate( 5.0);
    let  	c7 = repos.RealCreate( 7.0);

    let  	xSquared = repos.PowerCreate( x, c2);
    let  	term1 = repos.MultCreate( c3, xSquared);
    let  	term2 = repos.MultCreate( c5, x);
    let  	sum1 = repos.AddCreate( term1, term2);
    let  	fx = repos.DiffCreate( sum1, c7);

    let  	dfx = Differentiate( &mut repos, fx, xVarId).unwrap();

    let  	bindVal = [( "x", 4.0)];
    let  	bindArr = Arr::New( bindVal.as_ptr(), 1);

    // f'(4) = 6*4 + 5 = 29
    let  	dfVal = repos.EvaluateNamed( dfx, bindArr).unwrap();
    jeeves_assert!( ctx, ( dfVal - 29.0).abs() < 1e-6);

    // 2. Trigonometric differentiation: g(x) = sin(x) -> g'(x) = cos(x)
    let  	sinX = repos.FuncCreate( "sin".to_string(), x);
    let  	dSinX = Differentiate( &mut repos, sinX, xVarId).unwrap();
    let  	bindZero = [( "x", 0.0)];
    let  	cosZeroVal = repos.EvaluateNamed( dSinX, Arr::New( bindZero.as_ptr(), 1)).unwrap();
    jeeves_assert!( ctx, ( cosZeroVal - 1.0).abs() < 1e-6);

    // 3. Polynomial Normalization: (x + 2) * (x - 3) = x^2 - x - 6
    let  	cTwo = repos.RealCreate( 2.0);
    let  	cThree = repos.RealCreate( 3.0);
    let  	factor1 = repos.AddCreate( x, cTwo);
    let  	factor2 = repos.DiffCreate( x, cThree);
    let  	polyExpr = repos.MultCreate( factor1, factor2);

    let  	canon = CanonicalPolynomial::FromExpr( &repos, polyExpr).unwrap();
    jeeves_assert_eq!( ctx, canon.Degree(), 2);
    jeeves_assert_eq!( ctx, canon.TermCount(), 3);

    // Reconstruct canonical polynomial in ExprRepos
    let  	canonExpr = canon.ToExpr( &mut repos);

    // Evaluate both at x = 5: (5+2)*(5-3) = 14, 5^2 - 5 - 6 = 14
    let  	bindFive = [( "x", 5.0)];
    let  	bindFiveArr = Arr::New( bindFive.as_ptr(), 1);
    let  	origRes = repos.EvaluateNamed( polyExpr, bindFiveArr).unwrap();
    let  	canonRes = repos.EvaluateNamed( canonExpr, bindFiveArr).unwrap();
    jeeves_assert_eq!( ctx, origRes, 14.0);
    jeeves_assert_eq!( ctx, canonRes, 14.0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, TensorShapeOperationsAndBroadcasting, |ctx| {
    let  	vShape = TensorShape::Vector( 4).unwrap();
    jeeves_assert_eq!( ctx, vShape.Rank(), 1);
    jeeves_assert_eq!( ctx, vShape.TotalElements(), 4);
    jeeves_assert_eq!( ctx, vShape.SubShape().unwrap(), TensorShape::Scalar());

    let  	mShape = TensorShape::Matrix( 3, 4).unwrap();
    jeeves_assert_eq!( ctx, mShape.Rank(), 2);
    jeeves_assert_eq!( ctx, mShape.TotalElements(), 12);
    jeeves_assert_eq!( ctx, mShape.SubShape().unwrap(), TensorShape::Vector( 4).unwrap());
    jeeves_assert_eq!( ctx, mShape.SliceShape( 1).unwrap(), TensorShape::Vector( 4).unwrap());
    jeeves_assert_eq!( ctx, mShape.SliceShape( 2).unwrap(), TensorShape::Scalar());
    jeeves_assert_eq!( ctx, mShape.LeadingOffset( Arr::from( &[1])).unwrap(), 4);
    jeeves_assert_eq!( ctx, mShape.LeadingOffset( Arr::from( &[2, 3])).unwrap(), 11);

    let  	t3 = TensorShape::FromSlice( &[2, 3, 4]).unwrap();
    jeeves_assert_eq!( ctx, t3.Rank(), 3);
    jeeves_assert_eq!( ctx, t3.TotalElements(), 24);
    jeeves_assert_eq!( ctx, t3.SubShape().unwrap(), TensorShape::Matrix( 3, 4).unwrap());
    jeeves_assert_eq!( ctx, t3.SliceShape( 2).unwrap(), TensorShape::Vector( 4).unwrap());

    let  	bA = TensorShape::Matrix( 3, 1).unwrap();
    let  	bB = TensorShape::Matrix( 1, 4).unwrap();
    let  	bOut = bA.BroadcastWith( &bB).unwrap();
    jeeves_assert_eq!( ctx, bOut, TensorShape::Matrix( 3, 4).unwrap());

    let  	bVec = TensorShape::Vector( 4).unwrap();
    let  	b3Out = t3.BroadcastWith( &bVec).unwrap();
    jeeves_assert_eq!( ctx, b3Out, t3);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, TensorVariablesAndSubVarHierarchy, |ctx| {
    let  	mut repos = ExprRepos::New();

    // 1. Vector Variable
    let  	vId = repos.VectorVarCreate( "v", 3).unwrap();
    jeeves_assert!( ctx, repos.VarAttrAt( vId).Shape().IsVector());
    let  	v0 = repos.SubVar( vId, Arr::from( &[0])).unwrap();
    let  	v1 = repos.SubVar( vId, Arr::from( &[1])).unwrap();
    let  	v2 = repos.SubVar( vId, Arr::from( &[2])).unwrap();
    jeeves_assert_eq!( ctx, repos.VarNameAt( v1), "v[1]");
    jeeves_assert_eq!( ctx, repos.FindVar( "v[1]"), v1);
    jeeves_assert!( ctx, repos.VarAttrAt( v0).IsLeafSubVar());
    jeeves_assert!( ctx, repos.VarAttrAt( v2).IsLeafSubVar());

    // 2. Matrix Variable: SubVar of lower shape (row of matrix)
    let  	mId = repos.MatrixVarCreate( "M", 2, 3).unwrap();
    jeeves_assert!( ctx, repos.VarAttrAt( mId).Shape().IsMatrix());

    let  	row0 = repos.SubVar( mId, Arr::from( &[0])).unwrap();
    let  	row1 = repos.SubVar( mId, Arr::from( &[1])).unwrap();
    jeeves_assert_eq!( ctx, *repos.VarAttrAt( row0).Shape(), TensorShape::Vector( 3).unwrap());
    jeeves_assert_eq!( ctx, *repos.VarAttrAt( row1).Shape(), TensorShape::Vector( 3).unwrap());
    jeeves_assert!( ctx, repos.VarAttrAt( row0).IsSubTensor());
    jeeves_assert_eq!( ctx, repos.VarNameAt( row0), "M[0]");
    jeeves_assert_eq!( ctx, repos.FindVar( "M[0]"), row0);
    jeeves_assert_eq!( ctx, repos.FindVar( "M[1]"), row1);

    // Leaf element via matrix index and via row index
    let  	elem02FromMatrix = repos.SubVar( mId, Arr::from( &[0, 2])).unwrap();
    let  	elem02FromRow = repos.SubVar( row0, Arr::from( &[2])).unwrap();
    jeeves_assert_eq!( ctx, elem02FromMatrix, elem02FromRow);
    jeeves_assert_eq!( ctx, repos.FindVar( "M[0,2]"), elem02FromMatrix);
    jeeves_assert_eq!( ctx, repos.FindVar( "M[0][2]"), elem02FromMatrix);

    // 3. 3D Tensor Variable: SubVar of lower shape (matrix slice)
    let  	tShape = TensorShape::FromSlice( &[2, 3, 4]).unwrap();
    let  	tId = repos.TensorVarCreate( "T", tShape).unwrap();
    let  	slice0 = repos.SubVar( tId, Arr::from( &[0])).unwrap();
    jeeves_assert_eq!( ctx, *repos.VarAttrAt( slice0).Shape(), TensorShape::Matrix( 3, 4).unwrap());
    let  	elem123 = repos.SubVar( tId, Arr::from( &[1, 2, 3])).unwrap();
    jeeves_assert!( ctx, repos.VarAttrAt( elem123).IsLeafSubVar());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, TensorExprAlgebraAndContractions, |ctx| {
    let  	mut repos = ExprRepos::New();

    // Matrix M (2x3) and Vector v (3)
    let  	mVals = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let  	m = TensorExpr::FromReals( &mut repos, TensorShape::Matrix( 2, 3).unwrap(), Arr::from( &mVals)).unwrap();
    let  	vVals = [10.0, 20.0, 30.0];
    let  	v = TensorExpr::FromReals( &mut repos, TensorShape::Vector( 3).unwrap(), Arr::from( &vVals)).unwrap();

    // Row extraction yields lower shape (vector)
    let  	r0 = m.Row( 0).unwrap();
    jeeves_assert_eq!( ctx, *r0.Shape(), TensorShape::Vector( 3).unwrap());
    let  	r1 = m.SubTensor( 1).unwrap();
    jeeves_assert_eq!( ctx, *r1.Shape(), TensorShape::Vector( 3).unwrap());

    // Column extraction yields vector
    let  	c1 = m.Col( 1).unwrap();
    jeeves_assert_eq!( ctx, *c1.Shape(), TensorShape::Vector( 2).unwrap());

    // Dot product: r0 . v = 1*10 + 2*20 + 3*30 = 140
    let  	dotExpr = TensorExpr::VectorDot( &mut repos, &r0, &v).unwrap();
    let  	dotVal = repos.Evaluate( dotExpr, Arr::Empty()).unwrap();
    jeeves_assert_eq!( ctx, dotVal, 140.0);

    // Matrix-vector multiply: M v = [140, 320]
    let  	mv = TensorExpr::MatVecMul( &mut repos, &m, &v).unwrap();
    let  	mvVals = repos.EvaluateTensor( &mv, Arr::Empty()).unwrap();
    jeeves_assert_eq!( ctx, mvVals[0], 140.0);
    jeeves_assert_eq!( ctx, mvVals[1], 320.0);

    // Matrix-matrix multiply: A (2x2) * B (2x2)
    let  	a = TensorExpr::FromReals( &mut repos, TensorShape::Matrix( 2, 2).unwrap(), Arr::from( &[1.0, 2.0, 3.0, 4.0])).unwrap();
    let  	b = TensorExpr::FromReals( &mut repos, TensorShape::Matrix( 2, 2).unwrap(), Arr::from( &[5.0, 6.0, 7.0, 8.0])).unwrap();
    let  	ab = TensorExpr::MatMul( &mut repos, &a, &b).unwrap();
    let  	abVals = repos.EvaluateTensor( &ab, Arr::Empty()).unwrap();
    jeeves_assert_eq!( ctx, abVals[0], 19.0);
    jeeves_assert_eq!( ctx, abVals[1], 22.0);
    jeeves_assert_eq!( ctx, abVals[2], 43.0);
    jeeves_assert_eq!( ctx, abVals[3], 50.0);

    // Transpose
    let  	aT = a.Transpose().unwrap();
    let  	aTVals = repos.EvaluateTensor( &aT, Arr::Empty()).unwrap();
    jeeves_assert_eq!( ctx, aTVals[0], 1.0);
    jeeves_assert_eq!( ctx, aTVals[1], 3.0);
    jeeves_assert_eq!( ctx, aTVals[2], 2.0);
    jeeves_assert_eq!( ctx, aTVals[3], 4.0);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, TensorVectorCalculusAndJacobian, |ctx| {
    let  	mut repos = ExprRepos::New();

    // 1. Vector Variable Gradient: f(x) = x0^2 + 2*x1^2 + 3*x2^2
    let  	xVecId = repos.VectorVarCreate( "x", 3).unwrap();
    let  	x0 = repos.SubVarLinearExpr( xVecId, 0).unwrap();
    let  	x1 = repos.SubVarLinearExpr( xVecId, 1).unwrap();
    let  	x2 = repos.SubVarLinearExpr( xVecId, 2).unwrap();

    let  	c2 = repos.RealCreate( 2.0);
    let  	c3 = repos.RealCreate( 3.0);
    let  	x0Sq = repos.PowerCreate( x0, c2);
    let  	x1Sq = repos.PowerCreate( x1, c2);
    let  	x2Sq = repos.PowerCreate( x2, c2);
    let  	t1 = repos.MultCreate( c2, x1Sq);
    let  	t2 = repos.MultCreate( c3, x2Sq);
    let  	sum1 = repos.AddCreate( x0Sq, t1);
    let  	f = repos.AddCreate( sum1, t2);

    let  	grad = DifferentiateVector( &mut repos, f, xVecId).unwrap();
    jeeves_assert_eq!( ctx, *grad.Shape(), TensorShape::Vector( 3).unwrap());

    let  	binds = [( "x[0]", 1.0), ( "x[1]", 2.0), ( "x[2]", 3.0)];
    let  	gradVals = repos.EvaluateTensorNamed( &grad, Arr::New( binds.as_ptr(), 3)).unwrap();
    jeeves_assert!( ctx, ( gradVals[0] - 2.0).abs() < 1e-6);
    jeeves_assert!( ctx, ( gradVals[1] - 8.0).abs() < 1e-6);
    jeeves_assert!( ctx, ( gradVals[2] - 18.0).abs() < 1e-6);

    // 2. Differentiating with respect to a Row SubVar of a Matrix!
    let  	mId = repos.MatrixVarCreate( "M", 2, 2).unwrap();
    let  	row0Id = repos.SubVar( mId, Arr::from( &[0])).unwrap();
    let  	m00 = repos.SubVarExpr( mId, Arr::from( &[0, 0])).unwrap();
    let  	m01 = repos.SubVarExpr( mId, Arr::from( &[0, 1])).unwrap();
    let  	m00Sq = repos.PowerCreate( m00, c2);
    let  	m01Term = repos.MultCreate( c3, m01);
    let  	g = repos.AddCreate( m00Sq, m01Term);
    let  	rowGrad = DifferentiateVector( &mut repos, g, row0Id).unwrap();
    jeeves_assert_eq!( ctx, *rowGrad.Shape(), TensorShape::Vector( 2).unwrap());

    // 3. Jacobian Matrix: r = [s0*s1 - 2, s0^2 + s1^2 - 5]
    let  	sId = repos.VectorVarCreate( "s", 2).unwrap();
    let  	s0 = repos.SubVarLinearExpr( sId, 0).unwrap();
    let  	s1 = repos.SubVarLinearExpr( sId, 1).unwrap();
    let  	sProd = repos.MultCreate( s0, s1);
    let  	r0 = repos.DiffCreate( sProd, c2);
    let  	c5 = repos.RealCreate( 5.0);
    let  	s0Sq = repos.PowerCreate( s0, c2);
    let  	s1Sq = repos.PowerCreate( s1, c2);
    let  	sSum = repos.AddCreate( s0Sq, s1Sq);
    let  	r1 = repos.DiffCreate( sSum, c5);

    let  	rElems = crate::silo::Buff::FromArr( Arr::from( &[r0, r1]));
    let  	resVec = TensorExpr::New( TensorShape::Vector( 2).unwrap(), rElems).unwrap();
    let  	jac = Jacobian( &mut repos, &resVec, sId).unwrap();
    jeeves_assert_eq!( ctx, *jac.Shape(), TensorShape::Matrix( 2, 2).unwrap());

    let  	sBinds = [( "s[0]", 1.0), ( "s[1]", 2.0)];
    let  	jacVals = repos.EvaluateTensorNamed( &jac, Arr::New( sBinds.as_ptr(), 2)).unwrap();
    jeeves_assert!( ctx, ( jacVals[0] - 2.0).abs() < 1e-6);
    jeeves_assert!( ctx, ( jacVals[1] - 1.0).abs() < 1e-6);
    jeeves_assert!( ctx, ( jacVals[2] - 2.0).abs() < 1e-6);
    jeeves_assert!( ctx, ( jacVals[3] - 4.0).abs() < 1e-6);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, TensorMultiLevelSubVarIndexing, |ctx| {
    let  	mut repos = ExprRepos::New();

    // Shape (2, 3, 4)
    let  	tShape = TensorShape::FromSlice( &[2, 3, 4]).unwrap();
    let  	tId = repos.TensorVarCreate( "T", tShape).unwrap();

    // SubVar at index &[1] should be a Matrix(3, 4)
    let  	slice1 = repos.SubVar( tId, Arr::from( &[1])).unwrap();
    jeeves_assert_eq!( ctx, *repos.VarAttrAt( slice1).Shape(), TensorShape::Matrix( 3, 4).unwrap());
    jeeves_assert_eq!( ctx, repos.VarNameAt( slice1), "T[1]");

    // SubVar at index &[1, 2] should be a Vector(4) - P1 verification!
    let  	row12 = repos.SubVar( tId, Arr::from( &[1, 2])).unwrap();
    jeeves_assert_eq!( ctx, *repos.VarAttrAt( row12).Shape(), TensorShape::Vector( 4).unwrap());
    jeeves_assert_eq!( ctx, repos.VarNameAt( row12), "T[1,2]");
    jeeves_assert!( ctx, repos.VarAttrAt( row12).IsSubTensor());

    // SubVar at index &[1, 2, 3] should be a Leaf scalar
    let  	leaf123 = repos.SubVar( tId, Arr::from( &[1, 2, 3])).unwrap();
    jeeves_assert!( ctx, repos.VarAttrAt( leaf123).Shape().IsScalar());
    jeeves_assert!( ctx, repos.VarAttrAt( leaf123).IsLeafSubVar());
    jeeves_assert_eq!( ctx, repos.VarNameAt( leaf123), "T[1,2,3]");

    // Traversal through intermediate node should yield the same leaf
    let  	leafViaRow = repos.SubVar( row12, Arr::from( &[3])).unwrap();
    jeeves_assert_eq!( ctx, leafViaRow, leaf123);

    // Linear indexing from sub-tensor: row12 has 4 elements, offset corresponds to 1*12 + 2*4 = 20
    let  	leaf0ViaRow = repos.SubVarLinear( row12, 0).unwrap();
    let  	leaf3ViaRow = repos.SubVarLinear( row12, 3).unwrap();
    jeeves_assert_eq!( ctx, leaf3ViaRow, leaf123);
    jeeves_assert_eq!( ctx, repos.VarAttrAt( leaf0ViaRow).SubVarOffset(), 20);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, TensorShapeValidationAndOverflow, |ctx| {
    // 1. Overflow element count
    let  	overRes = TensorShape::FromSlice( &[2, 65536, 65536]);
    jeeves_assert!( ctx, matches!( overRes, Err( ShapeError::Overflow)));

    // 2. Zero dimensions in Vector and Matrix
    let  	vZero = TensorShape::Vector( 0);
    jeeves_assert!( ctx, matches!( vZero, Err( ShapeError::ZeroDimension( 0))));

    let  	mZero = TensorShape::Matrix( 0, 5);
    jeeves_assert!( ctx, matches!( mZero, Err( ShapeError::ZeroDimension( 0))));

    let  	mZeroCol = TensorShape::Matrix( 5, 0);
    jeeves_assert!( ctx, matches!( mZeroCol, Err( ShapeError::ZeroDimension( 1))));

    // 3. Zero dimensions in FromSlice
    let  	sZero = TensorShape::FromSlice( &[0]);
    jeeves_assert!( ctx, matches!( sZero, Err( ShapeError::ZeroDimension( 0))));

    let  	sZeroMulti = TensorShape::FromSlice( &[2, 0, 4]);
    jeeves_assert!( ctx, matches!( sZeroMulti, Err( ShapeError::ZeroDimension( 1))));

    // 4. Valid constructors succeed
    let  	vOk = TensorShape::Vector( 5).unwrap();
    jeeves_assert_eq!( ctx, vOk.TotalElements(), 5);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, TensorSymbolCollisionAndExactNameLookup, |ctx| {
    let  	mut repos = ExprRepos::New();

    // 1. Create vector "x"
    let  	xId = repos.VectorVarCreate( "x", 3).unwrap();
    jeeves_assert_eq!( ctx, repos.FindVar( "x"), xId);

    // Repeated creation of "x" must fail
    let  	dupX = repos.VectorVarCreate( "x", 3);
    jeeves_assert!( ctx, matches!( dupX, Err( ShapeError::DuplicateSymbol( _))));

    // Creation of scalar "x" must fail
    let  	dupScalarX = repos.ScalarVarCreate( "x");
    jeeves_assert!( ctx, matches!( dupScalarX, Err( ShapeError::DuplicateSymbol( _))));

    // Creation of another aggregate whose child collides must fail
    let  	childCollision = repos.ScalarVarCreate( "x[1]");
    jeeves_assert!( ctx, matches!( childCollision, Err( ShapeError::DuplicateSymbol( _))));

    // 2. Exact name lookup with brackets: registered scalar named "A[0][1]"
    let  	exactId = repos.ScalarVarCreate( "A[0][1]").unwrap();
    jeeves_assert_eq!( ctx, repos.FindVar( "A[0][1]"), exactId);
    jeeves_assert_eq!( ctx, repos.FindVarExact( "A[0][1]"), exactId);

    // Normalization doesn't break finding exact match first
    let  	otherId = repos.ScalarVarCreate( "B[0,1]").unwrap();
    jeeves_assert_eq!( ctx, repos.FindVar( "B[0][1]"), otherId);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, JacobianPrevalidationAndRollback, |ctx| {
    let  	mut repos = ExprRepos::New();

    let  	xId = repos.VectorVarCreate( "x", 2).unwrap();
    let  	x0 = repos.SubVarLinearExpr( xId, 0).unwrap();
    let  	c1 = repos.RealCreate( 1.0);
    let  	validRoot = repos.AddCreate( x0, c1);
    let  	invalidRoot = 999999u32; // non-existent node

    let  	initialSize = repos.Size();

    // Residual vector containing a valid root followed by an invalid ID
    let  	rElems = crate::silo::Buff::FromArr( Arr::from( &[validRoot, invalidRoot]));
    let  	resVec = TensorExpr::New( TensorShape::Vector( 2).unwrap(), rElems).unwrap();

    // Calling Jacobian must fail
    let  	jacRes = Jacobian( &mut repos, &resVec, xId);
    jeeves_assert!( ctx, jacRes.is_err());

    // Repository size must be rolled back and exactly match initialSize
    jeeves_assert_eq!( ctx, repos.Size(), initialSize);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!( Fresco, TensorNumericalBroadcasting, |ctx| {
    let  	mut repos = ExprRepos::New();

    // Broadcast matrix (3, 1) and matrix (1, 4) -> (3, 4)
    let  	colVals = [1.0, 2.0, 3.0];
    let  	rowVals = [10.0, 20.0, 30.0, 40.0];

    let  	colShape = TensorShape::Matrix( 3, 1).unwrap();
    let  	rowShape = TensorShape::Matrix( 1, 4).unwrap();
    let  	colExpr = TensorExpr::FromReals( &mut repos, colShape, Arr::from( &colVals)).unwrap();
    let  	rowExpr = TensorExpr::FromReals( &mut repos, rowShape, Arr::from( &rowVals)).unwrap();

    let  	sumExpr = TensorExpr::TensorAdd( &mut repos, &colExpr, &rowExpr).unwrap();
    jeeves_assert_eq!( ctx, *sumExpr.Shape(), TensorShape::Matrix( 3, 4).unwrap());

    let  	evalVals = repos.EvaluateTensor( &sumExpr, Arr::Empty()).unwrap();
    jeeves_assert_eq!( ctx, evalVals.Len(), 12);

    // Row 0: 1+10, 1+20, 1+30, 1+40 -> 11, 21, 31, 41
    jeeves_assert_eq!( ctx, evalVals[0], 11.0);
    jeeves_assert_eq!( ctx, evalVals[1], 21.0);
    jeeves_assert_eq!( ctx, evalVals[2], 31.0);
    jeeves_assert_eq!( ctx, evalVals[3], 41.0);

    // Row 1: 2+10, 2+20, 2+30, 2+40 -> 12, 22, 32, 42
    jeeves_assert_eq!( ctx, evalVals[4], 12.0);
    jeeves_assert_eq!( ctx, evalVals[5], 22.0);
    jeeves_assert_eq!( ctx, evalVals[6], 32.0);
    jeeves_assert_eq!( ctx, evalVals[7], 42.0);

    // Row 2: 3+10, 3+20, 3+30, 3+40 -> 13, 23, 33, 43
    jeeves_assert_eq!( ctx, evalVals[8], 13.0);
    jeeves_assert_eq!( ctx, evalVals[9], 23.0);
    jeeves_assert_eq!( ctx, evalVals[10], 33.0);
    jeeves_assert_eq!( ctx, evalVals[11], 43.0);
});

//-------------------------------------------------------------------------------------------------
// XplrExample protocol demonstration for Fresco symbolic expressions and tensors

#[derive(Clone)]
enum FrescoExampleChild
{
    Leaf(FrescoExampleLeaf),
    Branch(FrescoExampleBranch),
}

#[derive(Clone)]
struct FrescoExampleLeaf
{
    name:      String,
    path:      String,
    content:   String,
    extension: String,
}

impl FrescoExampleLeaf
{
    fn new(name: impl Into<String>, path: impl Into<String>, content: impl Into<String>, extension: impl Into<String>) -> Self
    {
        Self {
            name:      name.into(),
            path:      path.into(),
            content:   content.into(),
            extension: extension.into(),
        }
    }
}

impl crate::fenst::Xplr for FrescoExampleLeaf
{
    fn Name(&self) -> &str { &self.name }
    fn Path(&self) -> &str { &self.path }
    fn IsLeaf(&self) -> bool { true }
    fn Leaf(&self) -> Option<&dyn crate::fenst::LeafXplr> { Some(self) }
}

impl crate::fenst::LeafXplr for FrescoExampleLeaf
{
    fn Size(&self) -> u64 { self.content.len() as u64 }
    fn Extension(&self) -> &str { &self.extension }
    fn ReadChunk(&self, offset: u64, length: u32) -> Result<crate::fenst::StreamChunk, String>
    {
        let start = usize::try_from(offset).unwrap_or(usize::MAX).min(self.content.len());
        let end = start.saturating_add(length as usize).min(self.content.len());
        Ok(crate::fenst::StreamChunk::New(
            self.path.clone(),
            offset,
            self.Size(),
            self.content[start..end].to_string(),
        ))
    }
}

#[derive(Clone)]
struct FrescoExampleBranch
{
    name:     String,
    path:     String,
    children: Vec<FrescoExampleChild>,
}

impl FrescoExampleBranch
{
    fn new(name: impl Into<String>, path: impl Into<String>) -> Self
    {
        Self {
            name:     name.into(),
            path:     path.into(),
            children: Vec::new(),
        }
    }

    fn with_child(mut self, child: FrescoExampleChild) -> Self
    {
        self.children.push(child);
        self
    }
}

impl crate::fenst::Xplr for FrescoExampleBranch
{
    fn Name(&self) -> &str { &self.name }
    fn Path(&self) -> &str { &self.path }
    fn IsLeaf(&self) -> bool { false }
    fn Branch(&self) -> Option<&dyn crate::fenst::BranchXplr> { Some(self) }
}

impl crate::fenst::BranchXplr for FrescoExampleBranch
{
    fn Children(&self) -> Result<crate::silo::Buff<Box<dyn crate::fenst::Xplr>>, String>
    {
        let mut stash = crate::silo::Stash::New();
        for child in &self.children {
            match child {
                FrescoExampleChild::Leaf(l) => {
                    stash.Push(Box::new(l.clone()) as Box<dyn crate::fenst::Xplr>);
                }
                FrescoExampleChild::Branch(b) => {
                    stash.Push(Box::new(b.clone()) as Box<dyn crate::fenst::Xplr>);
                }
            }
        }
        Ok(stash.IntoBuff())
    }

    fn ChildCount(&self) -> Result<u32, String>
    {
        Ok(self.children.len() as u32)
    }
}

// 1. Inspectable example conforming to XplrExample protocol in Fresco
jeeves_test!(Fresco, ExpressionExplorerExample, XplrExample, |ctx| {
    use crate::fenst::Xplr;
    use crate::fresco::Differentiate;

    let mut repos = ExprRepos::New();

    // A. Scalar Function: f(x) = x^2 - 4
    let x_expr = repos.VarCreate("x".to_string(), true);
    let x_var = repos.At::<VarExpr>(x_expr).VarIndex();

    let x_squared = repos.MultCreate(x_expr, x_expr);
    let four = repos.RealCreate(4.0);
    let fx = repos.DiffCreate(x_squared, four);

    // Symbolic derivative: f'(x) = 2*x
    let dfx = Differentiate(&mut repos, fx, x_var).expect("Symbolic derivative of x^2 - 4");

    // Evaluations at x = 3.0: f(3.0) = 5.0, f'(3.0) = 6.0
    let bindings = [("x", 3.0)];
    let val_f = repos.EvaluateNamed(fx, Arr::from(&bindings)).expect("Evaluation of f(3.0)");
    jeeves_assert_eq!(ctx, val_f, 5.0);

    let val_df = repos.EvaluateNamed(dfx, Arr::from(&bindings)).expect("Evaluation of f'(3.0)");
    jeeves_assert_eq!(ctx, val_df, 6.0);

    // B. Matrix: M = [[x, 1.0], [0.0, 2.0]]
    let one = repos.RealCreate(1.0);
    let zero = repos.RealCreate(0.0);
    let two = repos.RealCreate(2.0);
    let m_shape = TensorShape::Matrix(2, 2).expect("2x2 shape");
    let mut elem_stash = crate::silo::Stash::New();
    elem_stash.Push(x_expr);
    elem_stash.Push(one);
    elem_stash.Push(zero);
    elem_stash.Push(two);
    let matrix_expr = TensorExpr::FromElements(m_shape, elem_stash.IntoBuff()).expect("matrix expr");

    let eval_m = repos.EvaluateTensorNamed(&matrix_expr, Arr::from(&bindings)).expect("Matrix evaluation");
    jeeves_assert_eq!(ctx, eval_m[0], 3.0);
    jeeves_assert_eq!(ctx, eval_m[1], 1.0);
    jeeves_assert_eq!(ctx, eval_m[2], 0.0);
    jeeves_assert_eq!(ctx, eval_m[3], 2.0);

    // C. Assemble inspectable Xplr tree conforming to XplrExample protocol
    let root_path = "example://Fresco/ExpressionExplorerExample";

    let scalar_branch = FrescoExampleBranch::new("ScalarCalculus", format!("{}/ScalarCalculus", root_path))
        .with_child(FrescoExampleChild::Leaf(FrescoExampleLeaf::new(
            "Function_f",
            format!("{}/ScalarCalculus/Function_f", root_path),
            "f(x) = x^2 - 4",
            "math",
        )))
        .with_child(FrescoExampleChild::Leaf(FrescoExampleLeaf::new(
            "Derivative_df_dx",
            format!("{}/ScalarCalculus/Derivative_df_dx", root_path),
            "f'(x) = 2*x",
            "math",
        )));

    let matrix_branch = FrescoExampleBranch::new("MatrixAlgebra", format!("{}/MatrixAlgebra", root_path))
        .with_child(FrescoExampleChild::Leaf(FrescoExampleLeaf::new(
            "Matrix_M",
            format!("{}/MatrixAlgebra/Matrix_M", root_path),
            "M [2x2] = [[x, 1.0], [0.0, 2.0]]",
            "matrix",
        )));

    let eval_leaf = FrescoExampleLeaf::new(
        "Evaluation",
        format!("{}/Evaluation", root_path),
        format!("Bindings: x = 3.0\nf(3.0) = {val_f:.1}\nf'(3.0) = {val_df:.1}\nM(3.0) = [[3.0, 1.0], [0.0, 2.0]]"),
        "txt",
    );

    let root = FrescoExampleBranch::new("ExpressionExplorerExample", root_path)
        .with_child(FrescoExampleChild::Branch(scalar_branch))
        .with_child(FrescoExampleChild::Branch(matrix_branch))
        .with_child(FrescoExampleChild::Leaf(eval_leaf));

    jeeves_assert_eq!(ctx, root.Name(), "ExpressionExplorerExample");
    jeeves_assert_eq!(ctx, root.Path(), root_path);

    return Ok(Box::new(root) as Box<dyn Xplr + Send>);
});

// 2. Comprehensive verification of the Fresco XplrExample protocol implementation
jeeves_test!(Fresco, XplrExampleProtocolDemonstration, |ctx| {
    use crate::fenst::cask2d::{Cask2dLayout, Cask2dOptions};
    use crate::fenst::example::XplrExample;
    use crate::fenst::provider::XplrRegistry;

    // A. Verify dynamic discovery in inventory under 'Fresco' suite
    let all = XplrExample::Registered();
    let mut found = None;
    all.Traverse(|ex| {
        if ex.suite == "Fresco" && ex.name == "ExpressionExplorerExample" {
            found = Some(*ex);
        }
    });
    jeeves_assert!(ctx, found.is_some(), "ExpressionExplorerExample must be registered under Fresco suite");
    let example = found.unwrap();

    // B. Direct execution: Run() executes Produce(ctx) and returns Box<dyn Xplr + Send>
    let executed = example.Run().expect("Example Run should succeed");
    jeeves_assert_eq!(ctx, executed.Name(), "ExpressionExplorerExample");
    jeeves_assert_eq!(ctx, executed.Path(), "example://Fresco/ExpressionExplorerExample");
    jeeves_assert!(ctx, !executed.IsLeaf());

    // C. URI Resolution via XplrRegistry under example://Fresco/...
    let registry = XplrRegistry::New();
    let (scheme, root_branch) = registry
        .OpenRoot("example://Fresco/ExpressionExplorerExample")
        .expect("OpenRoot on example://Fresco/ExpressionExplorerExample should succeed");
    jeeves_assert_eq!(ctx, scheme, "example");
    jeeves_assert_eq!(ctx, root_branch.Name(), "ExpressionExplorerExample");

    // D. Hierarchical traversal into sub-branches
    let children = root_branch.Children().expect("Root branch should produce children");
    jeeves_assert_eq!(ctx, children.Len(), 3);
    jeeves_assert_eq!(ctx, children[0].Name(), "ScalarCalculus");
    jeeves_assert_eq!(ctx, children[1].Name(), "MatrixAlgebra");
    jeeves_assert_eq!(ctx, children[2].Name(), "Evaluation");

    // E. Deep navigation to sub-branches via URI path
    let (_, scalar_branch) = registry
        .OpenRoot("example://Fresco/ExpressionExplorerExample/ScalarCalculus")
        .expect("OpenRoot on sub-branch should succeed");
    jeeves_assert_eq!(ctx, scalar_branch.Name(), "ScalarCalculus");
    let scalar_children = scalar_branch.Children().expect("Scalar children");
    jeeves_assert_eq!(ctx, scalar_children.Len(), 2);
    jeeves_assert_eq!(ctx, scalar_children[0].Name(), "Function_f");
    jeeves_assert_eq!(ctx, scalar_children[1].Name(), "Derivative_df_dx");

    // F. Leaf inspection and stream chunk reading
    let fn_leaf = scalar_children[0].Leaf().expect("Function_f should be a leaf");
    jeeves_assert_eq!(ctx, fn_leaf.Extension(), "math");
    let chunk = fn_leaf.ReadChunk(0, 64).expect("ReadChunk should succeed");
    jeeves_assert!(ctx, chunk.Content().contains("f(x) = x^2 - 4"));
    jeeves_assert!(ctx, chunk.IsEof());

    // G. Cask2D layout computation from URI
    let opts = Cask2dOptions::default();
    let layout = Cask2dLayout::FromUri("example://Fresco/ExpressionExplorerExample", &opts)
        .expect("Cask2dLayout::FromUri should succeed for Fresco example");
    jeeves_assert!(ctx, layout.Bounds().width > 0.0);
    jeeves_assert!(ctx, layout.Bounds().height > 0.0);
    jeeves_assert!(ctx, !layout.Nodes().is_empty());
});

//-------------------------------------------------------------------------------------------------
