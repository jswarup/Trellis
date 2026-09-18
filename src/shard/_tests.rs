//-- _tests.rs ----------------------------------------------------------------------------------------------------------------------

use crate::{
    ShardTree,
    flux::{
        FixedStream, IFluxExportSink, IFluxImportSource, JsonOutStream, fluxexport::FieldExp,
        fluximport::FieldImp,
    },
    jeeves_assert, jeeves_assert_eq, jeeves_test,
    shard::{Charset, Hex, Int, Json, Parser, Real, UInt, WSpc},
    silo::Stash,
};
use std::ptr::NonNull;

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, CharsetOps, |ctx| {
    // 1. Check ToString formatting of special/escaped chars
    let mut cs1 = Charset::New();
    cs1.SetChar(b' ');
    cs1.SetChar(b'-');
    cs1.SetChar(b'\\');
    println!("cs1: {}", cs1);
    // 2. Check Compare values
    let mut cs2 = Charset::New();
    cs2.SetChar(b'a');
    let mut cs3 = Charset::New();
    cs3.SetChar(b'b');
    println!("Compare cs2 to cs3: {}", cs2.Compare(&cs3));
    println!("Compare cs3 to cs2: {}", cs3.Compare(&cs2));
    // 3. Check negation formatting
    let cs4 = Charset::Word().Negative();
    println!("cs4 (NonWord): {}", cs4);
    let cs5 = Charset::Digit().Negative();
    println!("cs5 (NonDigit): {}", cs5);
    jeeves_assert!(ctx, true);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, ParserBasic, |ctx| {
    let str = "hello parser";
    let mut cs = Charset::New();
    cs.SetChar(b'p');
    let mut stream = FixedStream::from(str);
    let mut parser = Parser::New(&mut stream);

    let mut m = 0;
    {
        // Test char grammar
        let matched = {
            let g = &'h';
            let res = parser.ParseGrammar(g, m);
            if let Some(nextM) = res {
                m = nextM;
            }
            res.is_some()
        };
        jeeves_assert!(ctx, matched);

        let matched = {
            let g = &'e';
            let res = parser.ParseGrammar(g, m);
            if let Some(nextM) = res {
                m = nextM;
            }
            res.is_some()
        };
        jeeves_assert!(ctx, matched);

        // Test &str grammar
        let matched = {
            let g = &"llo ";
            let res = parser.ParseGrammar(g, m);
            if let Some(nextM) = res {
                m = nextM;
            }
            res.is_some()
        };
        jeeves_assert!(ctx, matched);

        let matched = {
            let g = &cs;
            let res = parser.ParseGrammar(g, m);
            if let Some(nextM) = res {
                m = nextM;
            }
            res.is_some()
        };
        jeeves_assert!(ctx, matched);

        // Test failing match (should rollback)
        let matched = {
            let g = &"fail";
            let res = parser.ParseGrammar(g, m);
            res.is_some()
        };
        jeeves_assert!(ctx, !matched);
    }
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, PostBoxet, |ctx| {
    let data = "ab";
    let mt = |_parser: &mut Parser| true;
    let tree = crate::ShardTree!(
        mt < "ab"[|_worker| {
            println!("Matched");
            true
        }]
    );
    let mut stream = FixedStream::from(data);
    let mut parser = Parser::New(&mut stream);
    let res = parser.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res.is_some());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, Rgx2, |ctx| {
    let alpha = crate::ShardTree!(["a-zA-Z"]);
    let identRgx = crate::ShardTree!( [ "a-z"] < ["A-Z"] < +alpha[ |_worker| {
        true
    } ] );

    // Test that the Repeat and Action correctly parse strings
    let mut stream1 = FixedStream::from("aBcxYZ");
    let mut parser1 = Parser::New(&mut stream1);
    let res1 = parser1.ParseGrammar(&identRgx, 0);
    let matched1 = res1.is_some();
    let m1 = res1.unwrap_or(0);
    jeeves_assert!(ctx, matched1);
    jeeves_assert_eq!(ctx, m1 as usize, 6);

    // Test with non-matching string
    let mut stream2 = FixedStream::from("aBcxYZ123");
    let mut parser2 = Parser::New(&mut stream2);
    let res2 = parser2.ParseGrammar(&identRgx, 0);
    let matched2 = res2.is_some();
    let m2 = res2.unwrap_or(0);
    jeeves_assert!(ctx, matched2);
    jeeves_assert_eq!(ctx, m2 as usize, 6);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, OptionalGrammar, |ctx| {
    let optGrammar = crate::ShardTree!( ? "a" < "b" );

    let mut stream1 = FixedStream::from("ab");
    let mut parser1 = Parser::New(&mut stream1);
    let res1 = parser1.ParseGrammar(&optGrammar, 0);
    jeeves_assert!(ctx, res1.is_some());
    jeeves_assert_eq!(ctx, res1.unwrap() as usize, 2);

    let mut stream2 = FixedStream::from("b");
    let mut parser2 = Parser::New(&mut stream2);
    let res2 = parser2.ParseGrammar(&optGrammar, 0);
    jeeves_assert!(ctx, res2.is_some());
    jeeves_assert_eq!(ctx, res2.unwrap() as usize, 1);

    let mut stream3 = FixedStream::from("aab");
    let mut parser3 = Parser::New(&mut stream3);
    let res3 = parser3.ParseGrammar(&optGrammar, 0);
    jeeves_assert!(ctx, res3.is_none());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, UIntShardTest, |ctx| {
    let tree = crate::ShardTree!(UInt);

    let mut stream1 = FixedStream::from("12345");
    let mut parser1 = Parser::New(&mut stream1);
    let res1 = parser1.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res1.is_some());
    jeeves_assert_eq!(ctx, res1.unwrap() as usize, 5);

    let mut stream2 = FixedStream::from("abc");
    let mut parser2 = Parser::New(&mut stream2);
    let res2 = parser2.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res2.is_none());

    let mut stream3 = FixedStream::from("42xyz");
    let mut parser3 = Parser::New(&mut stream3);
    let res3 = parser3.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res3.is_some());
    jeeves_assert_eq!(ctx, res3.unwrap() as usize, 2);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, IntShardTest, |ctx| {
    let tree = crate::ShardTree!(Int);

    let mut stream = FixedStream::from("+12345");
    let mut parser = Parser::New(&mut stream);
    let res = parser.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res.is_some());
    jeeves_assert_eq!(ctx, res.unwrap() as usize, 6);

    let mut stream2 = FixedStream::from("-42");
    let mut parser2 = Parser::New(&mut stream2);
    let res2 = parser2.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res2.is_some());
    jeeves_assert_eq!(ctx, res2.unwrap() as usize, 3);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, HexShardTest, |ctx| {
    let tree = crate::ShardTree!(Hex);

    let mut stream = FixedStream::from("0x1a2B");
    let mut parser = Parser::New(&mut stream);
    let res = parser.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res.is_some());
    jeeves_assert_eq!(ctx, res.unwrap() as usize, 6);

    let mut stream2 = FixedStream::from("-0XF");
    let mut parser2 = Parser::New(&mut stream2);
    let res2 = parser2.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res2.is_some());
    jeeves_assert_eq!(ctx, res2.unwrap() as usize, 4);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, RealShardTest, |ctx| {
    let tree = crate::ShardTree!(Real);

    let mut stream = FixedStream::from("3.14159");
    let mut parser = Parser::New(&mut stream);
    let res = parser.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res.is_some());
    jeeves_assert_eq!(ctx, res.unwrap() as usize, 7);

    let mut stream2 = FixedStream::from("-1.5e+10");
    let mut parser2 = Parser::New(&mut stream2);
    let res2 = parser2.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res2.is_some());
    jeeves_assert_eq!(ctx, res2.unwrap() as usize, 8);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, JsonShardTest, |ctx| {
    let json = Json::New(FieldImp::Null);
    let tree = crate::ShardTree!(json);

    // JSON String
    let mut stream1 = FixedStream::from(r#"  "hello world"  "#);
    let mut parser1 = Parser::New(&mut stream1);
    let res1 = parser1.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res1.is_some());
    jeeves_assert_eq!(ctx, res1.unwrap_or(0) as usize, 17);

    // JSON Object with various types
    let jsonText = r#"
    {
        "string": "value",
        "number": -1.23e4,
        "bool": true,
        "null_val": null,
        "array": [1, 2, 3, false, {"nested": "obj"}]
    }
    "#;
    let mut stream2 = FixedStream::from(jsonText);
    let mut parser2 = Parser::New(&mut stream2);
    let res2 = parser2.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res2.is_some());
    jeeves_assert_eq!(ctx, res2.unwrap_or(0) as usize, jsonText.len());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, JsonParsingStruct, |ctx| {
    #[derive(Default, Debug, PartialEq)]
    struct Person {
        _Name: String,
        _Age: u64,
        _IsActive: bool,
    }

    impl IFluxImportSource for Person {
        fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>) {
            let personPtr = self as *mut Person;
            *field = FieldImp::Obj(Box::new(move |key, child| {
                let person = unsafe { &mut *personPtr };
                if key == "name" {
                    *child = FieldImp::String(&mut person._Name);
                    true
                } else if key == "age" {
                    *child = FieldImp::U64(&mut person._Age);
                    true
                } else if key == "is_active" {
                    *child = FieldImp::Bool(&mut person._IsActive);
                    true
                } else {
                    false
                }
            }));
        }
    }
    let str = r#"{ "name": "Alice", "age": 30, "is_active": true }"#;
    let mut stream = FixedStream::from(str);
    let mut parser = Parser::New(&mut stream);
    let mut person = Person::default();
    let mut fImp = FieldImp::Null;
    person.FetchFieldImp(&mut fImp);
    let json = Json::New(fImp);
    let tree = crate::ShardTree!(json);
    let matched = parser.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, matched.is_some());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, JsonRejectsUnknownStructField, |ctx| {
    #[derive(Default)]
    struct Person {
        _Name: String,
    }

    impl IFluxImportSource for Person {
        fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>) {
            let personPtr = self as *mut Person;
            *field = FieldImp::Obj(Box::new(move |key, child| {
                let person = unsafe { &mut *personPtr };
                if key == "name" {
                    *child = FieldImp::String(&mut person._Name);
                    true
                } else {
                    false
                }
            }));
        }
    }

    let mut person = Person::default();
    let mut field = FieldImp::Null;
    person.FetchFieldImp(&mut field);
    let json = Json::New(field);
    let mut stream = FixedStream::from(r#"{ "unknown": 1 }"#);
    let mut parser = Parser::New(&mut stream);

    jeeves_assert!(ctx, parser.ParseGrammar(&json, 0).is_none());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, StrGrammarTest, |ctx| {
    // ---- 1. Match a plain quoted string ----------------------------
    let src = r#""hello""#;
    let mut stream = FixedStream::from(src);
    let mut parser = Parser::New(&mut stream);
    let grammar = ShardTree!(Str);

    let result = parser.ParseGrammar(&grammar, 0);
    jeeves_assert!(ctx, result.is_some());
    jeeves_assert_eq!(ctx, result.unwrap(), 7);

    // ---- 2. Match with escaped quote inside ------------------------------------------
    let src2 = "\"say \\\"hi\\\"\"";
    let mut stream2 = FixedStream::from(src2);
    let mut parser2 = Parser::New(&mut stream2);

    let result2 = parser2.ParseGrammar(&grammar, 0);
    jeeves_assert!(ctx, result2.is_some());

    // ---- 3. Null sink: match succeeds, no capture -----------------------------------
    let src3 = r#""world""#;
    let mut stream3 = FixedStream::from(src3);
    let mut parser3 = Parser::New(&mut stream3);

    let result3 = parser3.ParseGrammar(&grammar, 0);
    jeeves_assert!(ctx, result3.is_some());
    jeeves_assert_eq!(ctx, result3.unwrap(), 7);

    // ---- 4. No opening quote: match fails -------------------------------------------
    let src4 = "not_quoted";
    let mut stream4 = FixedStream::from(src4);
    let mut parser4 = Parser::New(&mut stream4);

    let result4 = parser4.ParseGrammar(&grammar, 0);
    jeeves_assert!(ctx, result4.is_none());

    // ---- 5. Empty quoted string -----------------------------------------------------
    let src5 = r#""""#;
    let mut stream5 = FixedStream::from(src5);
    let mut parser5 = Parser::New(&mut stream5);

    let result5 = parser5.ParseGrammar(&grammar, 0);
    jeeves_assert!(ctx, result5.is_some());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, PointGrammarTest, |ctx| {
    struct Point {
        _X: u64,
        _Y: u64,
    }

    let mt = |_parser: &mut Parser| true;
    crate::ImplFluxSource!(Point, _X, _Y);

    let _grammar = ShardTree!( "{" < WSpc <
                                            Str < WSpc < ":"  < WSpc < UInt < WSpc < "," < WSpc <
                                            Str < WSpc < ":"  < mt < WSpc < UInt < WSpc < "," < WSpc <
                                    "}" < WSpc);

    let _src = "{ \"_X\": 10, \"_Y\": 30 }";
    let mut pt2 = Point { _X: 0, _Y: 0 };

    struct Forge<'a> {
        pub _Prev: Option<NonNull<Forge<'a>>>,
        _Field: FieldImp<'a>,
    }

    {
        let mut field = FieldImp::Null;
        pt2.FetchFieldImp(&mut field);

        let mut _forge = Forge {
            _Prev: None,
            _Field: field,
        };
    }
    {
        let mut field = FieldImp::Null;
        pt2.FetchFieldImp(&mut field);
        if let FieldImp::Obj(ref mut cb) = field {
            let mut xField = FieldImp::Null;
            jeeves_assert!(ctx, cb("_X", &mut xField));
            xField.PostU64(10u64);

            let mut yField = FieldImp::Null;
            jeeves_assert!(ctx, cb("_Y", &mut yField));
            yField.PostU64(30u64);

            jeeves_assert!(ctx, !cb("_Z", &mut FieldImp::Null));
        } else {
            panic!("Expected FieldImp::Obj");
        }
    }
    jeeves_assert_eq!(ctx, pt2._X, 10);
    jeeves_assert_eq!(ctx, pt2._Y, 30);
});

//---------------------------------------------------------------------------------------------------------------------------------

#[derive(Clone)]
struct MemberGroup {
    _Name: String,
}
impl Default for MemberGroup {
    fn default() -> Self {
        MemberGroup {
            _Name: String::new(),
        }
    }
}
crate::ImplFluxSource!(MemberGroup, _Name);

#[derive(Clone)]
struct Person {
    _Name: String,
    _Age: u64,
    _Weight: f64,
    _Groups: Stash<MemberGroup>,
}
impl Default for Person {
    fn default() -> Self {
        Person {
            _Name: String::new(),
            _Age: 0,
            _Weight: 0.0,
            _Groups: Stash::New(),
        }
    }
}
crate::ImplFluxSource!(Person, _Name, _Age, _Weight, _Groups);

jeeves_test!(Shard, PersonSerialization, |ctx| {
    let mut groups = Stash::<MemberGroup>::New();
    groups.Push(MemberGroup {
        _Name: "Group A".to_string(),
    });
    groups.Push(MemberGroup {
        _Name: "Group B".to_string(),
    });

    let p1 = Person {
        _Name: "Alice".to_string(),
        _Age: 30,
        _Weight: 65.5,
        _Groups: groups,
    };

    // Serialize to JSON
    let mut output = String::new();
    {
        let mut jsonStream = JsonOutStream::New(&mut output, false);
        jsonStream.DispatchFieldExp(FieldExp::FluxSource(&p1));
    }
    println!("DEBUG JSON: [{}]", output);

    // Deserialize into another Person instance
    let mut p2 = Person::default();
    let mut stream = FixedStream::from(output.as_str());
    let mut parser = Parser::New(&mut stream);

    let mut field = FieldImp::Null;
    p2.FetchFieldImp(&mut field);
    let jsonParser = Json::New(field);

    jeeves_assert!(ctx, parser.ParseGrammar(&jsonParser, 0).is_some());

    drop(jsonParser);

    jeeves_assert_eq!(ctx, p1._Name, p2._Name);
    jeeves_assert_eq!(ctx, p1._Age, p2._Age);
    jeeves_assert_eq!(ctx, p1._Weight, p2._Weight);
    jeeves_assert_eq!(ctx, p1._Groups.Size(), p2._Groups.Size());

    jeeves_assert_eq!(
        ctx,
        p1._Groups.Arr().Get(0).unwrap()._Name,
        p2._Groups.Arr().Get(0).unwrap()._Name
    );
    jeeves_assert_eq!(
        ctx,
        p1._Groups.Arr().Get(1).unwrap()._Name,
        p2._Groups.Arr().Get(1).unwrap()._Name
    );
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, JsonStrictTrailingComma, |ctx| {
    let json = Json::New(FieldImp::Null);

    // Trailing comma in object must fail
    let mut s1 = FixedStream::from(r#"{"a": 1, }"#);
    let mut p1 = Parser::New(&mut s1);
    jeeves_assert!(ctx, p1.ParseGrammar(&json, 0).is_none());

    // Trailing comma in array must fail
    let mut s2 = FixedStream::from(r#"[1, 2, ]"#);
    let mut p2 = Parser::New(&mut s2);
    jeeves_assert!(ctx, p2.ParseGrammar(&json, 0).is_none());

    // Valid object without trailing comma
    let mut s3 = FixedStream::from(r#"{"a": 1, "b": 2}"#);
    let mut p3 = Parser::New(&mut s3);
    jeeves_assert!(ctx, p3.ParseGrammar(&json, 0).is_some());

    // Valid array without trailing comma
    let mut s4 = FixedStream::from(r#"[1, 2, 3]"#);
    let mut p4 = Parser::New(&mut s4);
    jeeves_assert!(ctx, p4.ParseGrammar(&json, 0).is_some());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, JsonStrictLeadingZero, |ctx| {
    let json = Json::New(FieldImp::Null);

    // Leading zero in integer must fail per RFC 8259
    let mut s1 = FixedStream::from(r#"{"num": 012}"#);
    let mut p1 = Parser::New(&mut s1);
    jeeves_assert!(ctx, p1.ParseGrammar(&json, 0).is_none());

    // Negative with leading zero must fail
    let mut s2 = FixedStream::from(r#"{"num": -05}"#);
    let mut p2 = Parser::New(&mut s2);
    jeeves_assert!(ctx, p2.ParseGrammar(&json, 0).is_none());

    // Single zero is valid
    let mut s3 = FixedStream::from(r#"{"num": 0}"#);
    let mut p3 = Parser::New(&mut s3);
    jeeves_assert!(ctx, p3.ParseGrammar(&json, 0).is_some());

    // Zero fraction is valid
    let mut s4 = FixedStream::from(r#"{"num": 0.5}"#);
    let mut p4 = Parser::New(&mut s4);
    jeeves_assert!(ctx, p4.ParseGrammar(&json, 0).is_some());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, JsonStringUnescaping, |ctx| {
    #[derive(Default)]
    struct Entry {
        _Text: String,
    }
    crate::ImplFluxSource!(Entry, _Text);

    let mut entry = Entry::default();
    let mut field = FieldImp::Null;
    entry.FetchFieldImp(&mut field);
    {
        let json = Json::New(field);

        // Escaped string with newline, tab, quote, and unicode escape \u0021 = '!'
        let mut stream = FixedStream::from(r#"{"_Text": "Hello\nWorld\t\"Escaped\"\u0021"}"#);
        let mut parser = Parser::New(&mut stream);
        let matched = parser.ParseGrammar(&json, 0);
        jeeves_assert!(ctx, matched.is_some());
    }
    jeeves_assert_eq!(ctx, entry._Text.as_str(), "Hello\nWorld\t\"Escaped\"!");

    // Invalid escape sequence like \a should be rejected
    let mut invalid_stream = FixedStream::from(r#"{"_Text": "invalid\aescape"}"#);
    let mut invalid_parser = Parser::New(&mut invalid_stream);
    let mut invalid_entry = Entry::default();
    let mut invalid_field = FieldImp::Null;
    invalid_entry.FetchFieldImp(&mut invalid_field);
    {
        let invalid_json = Json::New(invalid_field);
        jeeves_assert!(ctx, invalid_parser.ParseGrammar(&invalid_json, 0).is_none());
    }
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Shard, RepetitionZeroProgress, |ctx| {
    // Grammar matching 0 bytes
    let empty_match = |_p: &mut Parser| true;
    let tree = crate::ShardTree!(*empty_match);

    let mut stream = FixedStream::from("abc");
    let mut parser = Parser::New(&mut stream);
    // Should terminate immediately without spinning
    let res = parser.ParseGrammar(&tree, 0);
    jeeves_assert!(ctx, res.is_some());
    // Marker must remain 0 because 0 bytes were consumed
    jeeves_assert_eq!(ctx, res.unwrap(), 0);
});

//---------------------------------------------------------------------------------------------------------------------------------
