//-- _tests.rs ----------------------------------------------------------------------------------------------------------------------
use	crate::flux::{ BuffStream, FixedStream, IFluxExportSource, IFluxImportSource, IStream, JsonOutStream, OutStream, fluxexport::FieldExp, fluximport::FieldImp };
use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_test };
use	std::fs::{ create_dir_all, remove_file, write };
use	std::io::{ Cursor, Write };

//---------------------------------------------------------------------------------------------------------------------------------

struct Point
{
    _X: f64,
    _Y: f64,
}
crate::ImplFluxSource!( Point, _X, _Y);

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Flux, JsonOutStreamBasic, |_ctx| {
    let  	prices = crate::Buff![12.34_f32, 56.78, 90.12, 34.56, 78.90];
    let  	arr = prices.Arr();
    let  	pt = Point { _X: 10.0, _Y: 30.3 };
    let  	mut output = String::new();
    {
        let  	mut jsonStream = JsonOutStream::New( &mut output, true);
        jsonStream.KeyField( "point", FieldExp::FluxSource( &pt));
        jsonStream.KeyField( "prices", FieldExp::FluxSource( &arr));
    }
    let  	_ = create_dir_all( "out/gen");
    write( "out/gen/a.json", output).unwrap();
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Flux, JsonOutStreamEscapesAndNull, |ctx| {
    let  	mut output = String::new();
    {
        let  	mut stream = JsonOutStream::New( &mut output, false);
        stream.KeyField( "a\"b", FieldExp::Str( "line\n\\tab\u{0001}"));
        stream.KeyField( "empty", FieldExp::Null);
        stream.KeyField( "non_finite", FieldExp::F64( f64::NAN));
    }
    jeeves_assert!( ctx, output.contains( "\"a\\\"b\""));
    jeeves_assert!( ctx, output.contains( "\"line\\n\\\\tab\\u0001\""));
    jeeves_assert!( ctx, output.contains( "\"empty\": null"));
    jeeves_assert!( ctx, output.contains( "\"non_finite\": null"));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Flux, OutStreamStreamingWrite, |_ctx| {
    let  	mut stream = OutStream::from( Cursor::new( Vec::< u8>::new()));
    stream.write_all( b"streamed output").unwrap();
    stream.flush().unwrap();
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Flux, InStream, |ctx| {
    let  	data = "abc";
    let  	mut stream = FixedStream::from( data);
    // Test random-access At()
    jeeves_assert_eq!( ctx, stream.At( 0), b'a');
    jeeves_assert_eq!( ctx, stream.At( 1), b'b');
    jeeves_assert_eq!( ctx, stream.At( 2), b'c');
    jeeves_assert_eq!( ctx, stream.At( 5), 0);
    // Test stateless BytesAt()
    jeeves_assert_eq!( 
        ctx,
        std::str::from_utf8( unsafe {
            let  	a = stream.BytesAt( 1, 2);
            std::slice::from_raw_parts( a.Data(), a.Size() as usize)
        })
        .unwrap(),
        "bc"
    );
    jeeves_assert_eq!( 
        ctx,
        std::str::from_utf8( unsafe {
            let  	a = stream.BytesAt( 1, 10);
            std::slice::from_raw_parts( a.Data(), a.Size() as usize)
        })
        .unwrap(),
        "bc"
    );
    jeeves_assert_eq!( 
        ctx,
        std::str::from_utf8( unsafe {
            let  	a = stream.BytesAt( 5, 1);
            std::slice::from_raw_parts( a.Data(), a.Size() as usize)
        })
        .unwrap(),
        ""
    );
    jeeves_assert_eq!( 
        ctx,
        std::str::from_utf8( unsafe {
            let  	a = stream.BytesAt( 5, 10);
            std::slice::from_raw_parts( a.Data(), a.Size() as usize)
        })
        .unwrap(),
        ""
    );
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Flux, InStreamFromFile, |ctx| {
    let  	path = "test_inbuffstream.txt";
    write( path, b"hello").unwrap();
    let  	mut stream = BuffStream::FromFile( path).unwrap();
    jeeves_assert_eq!( ctx, stream.At( 0), b'h');
    jeeves_assert_eq!( ctx, stream.At( 1), b'e');
    jeeves_assert_eq!( 
        ctx,
        std::str::from_utf8( unsafe {
            let  	a = stream.BytesAt( 1, 4);
            std::slice::from_raw_parts( a.Data(), a.Size() as usize)
        })
        .unwrap(),
        "ello"
    );
    remove_file( path).unwrap();
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Flux, FluxSourceDisplayDebug, |ctx| {
    let  	pt1 = Point { _X: 10.0, _Y: 30.3 };
    let  	expSource: &dyn IFluxExportSource = &pt1;
    let  	disp = format!( "{}", expSource);
    let  	debug = format!( "{:?}", expSource);
    jeeves_assert!( ctx, disp.contains( "\"_X\": 10"));
    jeeves_assert!( ctx, disp.contains( "\"_Y\": 30.3"));
    jeeves_assert!( ctx, debug.contains( "\n"));
    let  	mut pt2 = Point { _X: 0., _Y: 0. };
    {
        let  	mut field = FieldImp::Null;
        pt2.FetchFieldImp( &mut field);
        if let  	FieldImp::Obj( ref mut cb) = field {
            let  	mut xField = FieldImp::Null;
            jeeves_assert!( ctx, cb( "_X", &mut xField));
            xField.PostF64( 10.0);
            let  	mut yField = FieldImp::Null;
            jeeves_assert!( ctx, cb( "_Y", &mut yField));
            yField.PostF64( 30.3);
            jeeves_assert!( ctx, !cb( "_Z", &mut FieldImp::Null));
        } else {
            panic!( "Expected FieldImp::Obj");
        }
    }
    jeeves_assert_eq!( ctx, pt2._X, 10.0);
    jeeves_assert_eq!( ctx, pt2._Y, 30.3);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Flux, OutStreamChunkedCache, |ctx| {
    let  	mut dest = Vec::new();
    {
        // Use a tiny cache size of 16 bytes to force multiple flushes
        let  	mut stream = OutStream::WithCacheSize( &mut dest, 16);
        let  	data = b"The quick brown fox jumps over the lazy dog! 1234567890";
        stream.write_all( data).unwrap();
        stream.flush().unwrap();
    }
    jeeves_assert_eq!( 
        ctx,
        dest.as_slice(),
        b"The quick brown fox jumps over the lazy dog! 1234567890"
    );
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!( Flux, FluxImportNarrowOverflow, |ctx| {
    use	crate::flux::FluxError;
    let  	mut small_val: u8 = 0;
    let  	mut field = FieldImp::Null;
    small_val.FetchFieldImp( &mut field);
    // In-range value should succeed
    let  	res = field.TryPostU64( 250);
    jeeves_assert_eq!( ctx, res, Ok( ()));
    jeeves_assert_eq!( ctx, small_val, 250);
    // Out-of-range value should fail with Overflow
    let  	mut field_overflow = FieldImp::Null;
    small_val.FetchFieldImp( &mut field_overflow);
    let  	res_overflow = field_overflow.TryPostU64( 256);
    jeeves_assert_eq!( ctx, res_overflow, Err( FluxError::Overflow));
    // Value remains unchanged
    jeeves_assert_eq!( ctx, small_val, 250);
});

//---------------------------------------------------------------------------------------------------------------------------------
