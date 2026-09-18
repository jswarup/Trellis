//-- _tests.rs ----------------------------------------------------------------------------------------------------------------------
use crate::flux::{
    BuffStream, FixedStream, IFluxExportSource, IFluxImportSource, IStream, JsonOutStream,
    OutStream, fluxexport::FieldExp, fluximport::FieldImp,
};
use std::fs::{create_dir_all, remove_file, write};
use std::io::{Cursor, Write};

//---------------------------------------------------------------------------------------------------------------------------------

struct Point {
    _X: f64,
    _Y: f64,
}

crate::ImplFluxSource!(Point, _X, _Y);

#[test]
fn TestJsonOutStream() {
    let prices = crate::Buff![12.34_f32, 56.78, 90.12, 34.56, 78.90];
    let arr = prices.Arr();

    let pt = Point { _X: 10.0, _Y: 30.3 };

    let mut output = String::new();
    {
        let mut jsonStream = JsonOutStream::New(&mut output, true);

        jsonStream.KeyField("point", FieldExp::FluxSource(&pt));
        jsonStream.KeyField("prices", FieldExp::FluxSource(&arr));
    }

    let _ = create_dir_all("out/gen");
    write("out/gen/a.json", output).unwrap();
}

//-------------------------------------------------------------------------------------------------

#[test]
fn TestJsonOutStreamEscapesAndNull() {
    let mut output = String::new();
    {
        let mut stream = JsonOutStream::New(&mut output, false);
        stream.KeyField("a\"b", FieldExp::Str("line\n\\tab\u{0001}"));
        stream.KeyField("empty", FieldExp::Null);
        stream.KeyField("non_finite", FieldExp::F64(f64::NAN));
    }

    assert!(output.contains("\"a\\\"b\""));
    assert!(output.contains("\"line\\n\\\\tab\\u0001\""));
    assert!(output.contains("\"empty\": null"));
    assert!(output.contains("\"non_finite\": null"));
}

//-------------------------------------------------------------------------------------------------

#[test]
fn TestOutStreamStreamingWrite() {
    let mut stream = OutStream::from(Cursor::new(Vec::<u8>::new()));
    stream.write_all(b"streamed output").unwrap();
    stream.flush().unwrap();
}

//---------------------------------------------------------------------------------------------------------------------------------

#[test]
fn TestInStream() {
    let data = "abc";
    let mut stream = FixedStream::from(data);

    // Test random-access At()
    assert_eq!(stream.At(0), b'a');
    assert_eq!(stream.At(1), b'b');
    assert_eq!(stream.At(2), b'c');
    assert_eq!(stream.At(5), 0);

    // Test stateless BytesAt()
    assert_eq!(
        std::str::from_utf8(unsafe {
            let a = stream.BytesAt(1, 2);
            std::slice::from_raw_parts(a.Data(), a.Size() as usize)
        })
        .unwrap(),
        "bc"
    );
    assert_eq!(
        std::str::from_utf8(unsafe {
            let a = stream.BytesAt(1, 10);
            std::slice::from_raw_parts(a.Data(), a.Size() as usize)
        })
        .unwrap(),
        "bc"
    );
    assert_eq!(
        std::str::from_utf8(unsafe {
            let a = stream.BytesAt(5, 1);
            std::slice::from_raw_parts(a.Data(), a.Size() as usize)
        })
        .unwrap(),
        ""
    );
    assert_eq!(
        std::str::from_utf8(unsafe {
            let a = stream.BytesAt(5, 10);
            std::slice::from_raw_parts(a.Data(), a.Size() as usize)
        })
        .unwrap(),
        ""
    );
}

//---------------------------------------------------------------------------------------------------------------------------------

#[test]
fn TestInStreamFromFile() {
    let path = "test_inbuffstream.txt";
    write(path, b"hello").unwrap();
    let mut stream = BuffStream::FromFile(path).unwrap();
    assert_eq!(stream.At(0), b'h');
    assert_eq!(stream.At(1), b'e');
    assert_eq!(
        std::str::from_utf8(unsafe {
            let a = stream.BytesAt(1, 4);
            std::slice::from_raw_parts(a.Data(), a.Size() as usize)
        })
        .unwrap(),
        "ello"
    );
    remove_file(path).unwrap();
}

//---------------------------------------------------------------------------------------------------------------------------------

#[test]
fn TestFluxSourceDisplayDebug() {
    let pt1 = Point { _X: 10.0, _Y: 30.3 };
    let expSource: &dyn IFluxExportSource = &pt1;

    let disp = format!("{}", expSource);
    let debug = format!("{:?}", expSource);

    assert!(disp.contains("\"_X\": 10"));
    assert!(disp.contains("\"_Y\": 30.3"));
    assert!(debug.contains("\n"));

    let mut pt2 = Point { _X: 0., _Y: 0. };
    {
        let mut field = FieldImp::Null;
        pt2.FetchFieldImp(&mut field);
        if let FieldImp::Obj(ref mut cb) = field {
            let mut xField = FieldImp::Null;
            assert!(cb("_X", &mut xField));
            xField.PostF64(10.0);

            let mut yField = FieldImp::Null;
            assert!(cb("_Y", &mut yField));
            yField.PostF64(30.3);

            assert!(!cb("_Z", &mut FieldImp::Null));
        } else {
            panic!("Expected FieldImp::Obj");
        }
    }
    assert_eq!(pt2._X, 10.0);
    assert_eq!(pt2._Y, 30.3);
}

//---------------------------------------------------------------------------------------------------------------------------------
