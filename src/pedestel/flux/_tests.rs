//-- _tests.rs ----------------------------------------------------------------------------------------------------------------------
use	crate::flux::{ BuffStream, FixedStream, IFluxExportSource, IFluxImportSource, IStream, JsonOutStream, OutStream, fluxexport::FieldExp, fluximport::FieldImp };
use	crate::{ jeeves_assert, jeeves_assert_eq, jeeves_test };
use	std::fs::{ create_dir_all, remove_file, write };
use	std::io::{ Cursor, Write };

//-------------------------------------------------------------------------------------------------

jeeves_test!(Flux, NativeFileGoldenAndTypedParity, |ctx| {
    use crate::flux::{FluxFile, FluxFileFormat, FluxValue as V};
    use crate::silo::Arr;
    let binary: &[u8] = include_bytes!("../../../wiki/formats/examples/minimal.flx");
    let text: &[u8] = include_bytes!("../../../wiki/formats/examples/minimal.tflx");
    let file = FluxFile::Decode(Arr::from(binary)).unwrap();
    let textFile = FluxFile::Decode(Arr::from(text)).unwrap();
    jeeves_assert_eq!(ctx, file.Payload(), textFile.Payload());
    let encoded = file.Encode(FluxFileFormat::Binary).unwrap();
    let encodedBytes: &[u8] = encoded.Arr().into();
    jeeves_assert_eq!(ctx, encodedBytes, binary);
    let payload = V::Array(crate::Buff![V::U64(u64::MAX),
                                        V::F64(-0.0),
                                        V::F64(f64::from_bits(1)),
                                        V::F64(f64::INFINITY),
                                        V::F64(f64::from_bits(0xfff8000000000001)),
                                        V::String("quote\" slash\\ newline\n é 水 🐦\0".into()),
                                        V::Null,
                                        V::Array(crate::silo::Buff::New()),
                                        V::Record(crate::silo::Buff::New())]);
    let typed = FluxFile::FromParts(file.Metadata().clone(), payload).unwrap();
    let first = typed.Encode(FluxFileFormat::Binary).unwrap();
    let text = typed.Encode(FluxFileFormat::Text).unwrap();
    let restored = FluxFile::Decode(text.Arr()).unwrap();
    let second = restored.Encode(FluxFileFormat::Binary).unwrap();
    let a: &[u8] = first.Arr().into();
    let b: &[u8] = second.Arr().into();
    jeeves_assert_eq!(ctx, a, b);
    let value = V::FromSource(typed.Payload()).unwrap();
    let source = FluxFile::FromParts(typed.Metadata().clone(), value).unwrap();
    let third = source.Encode(FluxFileFormat::Binary).unwrap();
    let c: &[u8] = third.Arr().into();
    jeeves_assert_eq!(ctx, a, c);
});

jeeves_test!(Flux, NativeFileCorruptionAndBounds, |ctx| {
    use crate::flux::{FluxFile, FluxFileFormat, FluxValue as V};
    use crate::silo::{Arr, Buff, USeg};
    let binary: &[u8] = include_bytes!("../../../wiki/formats/examples/minimal.flx");
    USeg::FromLen(binary.len() as u32).Traverse(|cut| {
        jeeves_assert!(ctx,
                       FluxFile::Decode(Arr::from(&binary[..cut as usize])).is_err());
    });
    USeg::FromLen(binary.len() as u32).Traverse(|index| {
                                          let mut mutated =
                                              Buff::FromDispenser(binary.len() as u32, |i| {
                                                  binary[i as usize]
                                              });
                                          mutated[index] ^= 1;
                                          jeeves_assert!(ctx,
                                                         FluxFile::Decode(mutated.Arr()).is_err());
                                      });
    let text = include_str!("../../../wiki/formats/examples/minimal.tflx");
    let whitespace = text.replace("Data {", "Data # comment\n{")
                         .replace('\n', "\r\n");
    jeeves_assert!(ctx,
                   FluxFile::Decode(Arr::from(whitespace.as_bytes())).is_ok());
    let unchecked = format!("{}Integrity none;\nEnd;\n",
                            text.split("Integrity").next().unwrap());
    jeeves_assert!(ctx,
                   FluxFile::Decode(Arr::from(unchecked.as_bytes())).is_ok());
    let malformed = unchecked.replace("u64(42)", "u64(18446744073709551616)");
    jeeves_assert!(ctx,
                   FluxFile::Decode(Arr::from(malformed.as_bytes())).is_err());
    let malformed = unchecked.replace("\"hello\"", "\"\\ud800\"");
    jeeves_assert!(ctx,
                   FluxFile::Decode(Arr::from(malformed.as_bytes())).is_err());
    let malformed = unchecked.replace("\"X\":", "\"Count\":");
    jeeves_assert!(ctx,
                   FluxFile::Decode(Arr::from(malformed.as_bytes())).is_err());
    let malformed = unchecked.replace("End;", "");
    jeeves_assert!(ctx,
                   FluxFile::Decode(Arr::from(malformed.as_bytes())).is_err());
    let mut nested = V::Null;
    USeg::FromLen(66).Traverse(|_| nested = V::Array(crate::Buff![nested.clone()]));
    let file = FluxFile::Decode(Arr::from(binary)).unwrap();
    let deep = FluxFile::FromParts(file.Metadata().clone(), nested).unwrap();
    jeeves_assert!(ctx, deep.Encode(FluxFileFormat::Binary).is_err());
    jeeves_assert_eq!(ctx,
                      crate::flux::file::Crc32c(Arr::from(&b"123456789"[..])),
                      0xe3069283);
});

jeeves_test!(Flux, NativeFileSafeOverwriteAndFailure, |ctx| {
    use crate::flux::{FluxFile, FluxValue as V};
    use crate::silo::Arr;
    let path = std::env::temp_dir().join(format!("trellis-flux-safe-{}.flx", std::process::id()));
    let file = FluxFile::New("trellis.example", 1, V::U64(1)).unwrap();
    file.Save(&path).unwrap();
    let updated = FluxFile::FromParts(file.Metadata().clone(), V::U64(2)).unwrap();
    updated.Save(&path).unwrap();
    jeeves_assert_eq!(ctx,
                      FluxFile::Read(&path).unwrap().Payload().UInt().unwrap(),
                      2);
    let before = std::fs::read(&path).unwrap();
    let duplicate = V::Record(crate::Buff![("X", V::Null), ("X", V::Null)]);
    let invalid = FluxFile::FromParts(file.Metadata().clone(), duplicate).unwrap();
    jeeves_assert!(ctx, invalid.Save(&path).is_err());
    jeeves_assert_eq!(ctx, std::fs::read(&path).unwrap(), before);
    let directory =
        path.with_file_name(format!("trellis-flux-target-directory-{}.flx", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    jeeves_assert!(ctx, file.Save(&directory).is_err());
    jeeves_assert!(ctx, directory.is_dir());
    jeeves_assert!(ctx, FluxFile::Decode(Arr::from(&before[..])).is_ok());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
});

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
        std::str::from_utf8( stream.BytesAt( 1, 2).into())
        .unwrap(),
        "bc"
    );
    jeeves_assert_eq!( 
        ctx,
        std::str::from_utf8( stream.BytesAt( 1, 10).into())
        .unwrap(),
        "bc"
    );
    jeeves_assert_eq!( 
        ctx,
        std::str::from_utf8( stream.BytesAt( 5, 1).into())
        .unwrap(),
        ""
    );
    jeeves_assert_eq!( 
        ctx,
        std::str::from_utf8( stream.BytesAt( 5, 10).into())
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
        std::str::from_utf8( stream.BytesAt( 1, 4).into())
        .unwrap(),
        "ello"
    );
    remove_file( path).unwrap();
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Flux, ScopedArrayImportAndBounds, |ctx| {
    use crate::shard::{IGrammar, JsonGrammar, JsonParseCtx};
    use crate::silo::{Buff, MutArr, Stash, USeg};

    let mut values = [0u64; 3];
    {
        let mut view = MutArr::from(&mut values);
        let mut json = JsonParseCtx::New(FieldImp::FluxSource(&mut view));
        jeeves_assert!(ctx, JsonGrammar.ParseStr("[10,20,30]", &mut json).is_ok());
    }
    jeeves_assert_eq!(ctx, values, [10, 20, 30]);

    let mut fixed = Buff::FromDispenser(2, |_| 0u64);
    {
        let mut json = JsonParseCtx::New(FieldImp::FluxSource(&mut fixed));
        jeeves_assert!(ctx, JsonGrammar.ParseStr("[1,2,3]", &mut json).is_err());
    }

    let mut growing = Stash::<u64>::New();
    let mut input = String::from("[");
    USeg::FromLen(4096).Traverse(|index| {
                           if index != 0 {
                               input.push(',');
                           }
                           input.push_str(&index.to_string());
                       });
    input.push(']');
    {
        let mut json = JsonParseCtx::New(FieldImp::FluxSource(&mut growing));
        jeeves_assert!(ctx, JsonGrammar.ParseStr(&input, &mut json).is_ok());
    }
    jeeves_assert_eq!(ctx, growing.Size(), 4096);
    USeg::FromLen(growing.Size()).Traverse(|index| {
                                     jeeves_assert_eq!(ctx, growing[index], u64::from(index));
                                 });
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Flux, OwningSegmentImportDiscardAndCommit, |ctx| {
    use crate::silo::USeg;

    let original = USeg::New(2, 8);
    let mut target = original;
    {
        let mut descriptor = FieldImp::Null;
        target.FetchFieldImp(&mut descriptor);
        let FieldImp::Obj(mut importer) = descriptor else {
            panic!("object expected");
        };
        jeeves_assert!(ctx,
                       importer.Field("_First").unwrap().TryPostU64(10).is_ok());
        jeeves_assert!(ctx, importer.Field("_Last").unwrap().TryPostU64(20).is_ok());
    }
    jeeves_assert_eq!(ctx, target, original);
    {
        let mut descriptor = FieldImp::Null;
        target.FetchFieldImp(&mut descriptor);
        let FieldImp::Obj(mut importer) = descriptor else {
            panic!("object expected");
        };
        importer.Field("_Last").unwrap().TryPostU64(20).unwrap();
        importer.Field("_First").unwrap().TryPostU64(10).unwrap();
        jeeves_assert!(ctx, importer.Commit().is_ok());
    }
    jeeves_assert_eq!(ctx, target, USeg::New(10, 20));

    let mut borrowed = "original";
    {
        let mut descriptor = FieldImp::Null;
        borrowed.FetchFieldImp(&mut descriptor);
        jeeves_assert!(ctx, descriptor.TryPostStr("replacement").is_err());
    }
    jeeves_assert_eq!(ctx, borrowed, "original");
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
            let xField = cb.Field( "_X").unwrap();
            xField.PostF64( 10.0);
            let yField = cb.Field( "_Y").unwrap();
            yField.PostF64( 30.3);
            jeeves_assert!( ctx, cb.Field( "_Z").is_err());
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

jeeves_test!( Flux, StreamCompressionAndChunkedStreaming, |ctx| {
    use	crate::flux::compress::{
        ChunkedStreamReader, ChunkedStreamWriter, CompressBlock, DecompressBlock, FlxCompressKind,
    };
    use	crate::silo::USeg;
    use	std::io::Cursor;

    // 1. Block Compression & Decompression (>60% reduction on repetitive data)
    let  	mut sampleData = Vec::with_capacity( 4096);
    let  	mut b = 0u32;
    while b < 1024
    {
        sampleData.extend_from_slice( &[0x00, 0x00, 0x00, 0x00]);
        b += 1;
    }
    jeeves_assert_eq!( ctx, sampleData.len(), 4096);

    let  	compressed = CompressBlock( &sampleData);
    let  	compressedSize = compressed.Size() as usize;

    // 4096 bytes of zeros should compress to under 100 bytes (>> 60% reduction)
    jeeves_assert!( ctx, compressedSize < 100);

    let  	mut compSlice = vec![0u8; compressedSize];
    USeg::FromLen( compressed.Size()).Traverse( |i| {
        compSlice[i as usize] = compressed[i];
    });

    let  	decompressed = DecompressBlock( &compSlice, 4096).unwrap();
    jeeves_assert_eq!( ctx, decompressed.Size() as usize, 4096);
    USeg::FromLen( 4096).Traverse( |i| {
        jeeves_assert_eq!( ctx, decompressed[i], sampleData[i as usize]);
    });

    // 2. Incremental Chunked Streaming across small chunk boundaries (256 bytes)
    let  	mut backingStore = Vec::new();
    {
        let  	mut writer = ChunkedStreamWriter::WithChunkSize(
            &mut backingStore,
            FlxCompressKind::ChunkedLz,
            256,
        );
        let  	written = writer.WriteBytes( &sampleData).unwrap();
        jeeves_assert_eq!( ctx, written, 4096);
        writer.Finish().unwrap();
    }

    let  	cursor = Cursor::new( backingStore);
    let  	mut reader = ChunkedStreamReader::New( cursor, FlxCompressKind::ChunkedLz);

    let  	mut restoredData = vec![0u8; 4096];
    let  	mut totalRead = 0usize;
    while totalRead < 4096
    {
        let  	readCount = reader.ReadBytes( &mut restoredData[totalRead..totalRead + 128]).unwrap();
        if readCount == 0
        {
            break;
        }
        totalRead += readCount;
    }

    jeeves_assert_eq!( ctx, totalRead, 4096);
    jeeves_assert_eq!( ctx, restoredData, sampleData);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Flux, NativeFileInspectVerifyAndConvert, |ctx| {
    use crate::flux::{
        FluxFile, FluxFileError, FluxFileFormat, FluxIntegrityStatus, FluxReadLimits,
    };
    use crate::silo::Arr;

    let binary: &[u8] = include_bytes!("../../../wiki/formats/examples/minimal.flx");
    let text: &[u8] = include_bytes!("../../../wiki/formats/examples/minimal.tflx");

    // 1. Format Detection
    jeeves_assert_eq!(
        ctx,
        FluxFileFormat::Detect(Arr::from(binary)).unwrap(),
        FluxFileFormat::Binary
    );
    jeeves_assert_eq!(
        ctx,
        FluxFileFormat::Detect(Arr::from(text)).unwrap(),
        FluxFileFormat::Text
    );
    jeeves_assert!(ctx, FluxFileFormat::Detect(Arr::from(&b"unknown format"[..])).is_err());

    // 2. Binary Inspection without full payload AST construction
    let info = FluxFile::InspectBytes(Arr::from(binary), FluxReadLimits::default()).unwrap();
    jeeves_assert_eq!(ctx, info.Format(), FluxFileFormat::Binary);
    jeeves_assert_eq!(ctx, info.FileBytes(), 293);
    jeeves_assert_eq!(ctx, info.MetadataBytes(), 180);
    jeeves_assert_eq!(ctx, info.PayloadBytes(), 49);
    jeeves_assert_eq!(ctx, info.Integrity(), FluxIntegrityStatus::Verified);
    jeeves_assert_eq!(ctx, info.Metadata().Schema(), "trellis.example");
    jeeves_assert_eq!(ctx, info.Metadata().SchemaVersion(), 1);
    jeeves_assert_eq!(ctx, info.Metadata().ProducerName(), "Trellis");

    // 3. Text Inspection
    let textInfo = FluxFile::InspectBytes(Arr::from(text), FluxReadLimits::default()).unwrap();
    jeeves_assert_eq!(ctx, textInfo.Format(), FluxFileFormat::Text);
    jeeves_assert_eq!(ctx, textInfo.Integrity(), FluxIntegrityStatus::Verified);
    jeeves_assert_eq!(ctx, textInfo.Metadata().Schema(), "trellis.example");

    // 4. File Convert (.flx -> .tflx -> .flx)
    let tempDir = std::env::temp_dir();
    let binPath = tempDir.join(format!("trellis-convert-src-{}.flx", std::process::id()));
    let textPath = tempDir.join(format!("trellis-convert-dst-{}.tflx", std::process::id()));
    let binRoundtripPath = tempDir.join(format!("trellis-convert-rt-{}.flx", std::process::id()));

    let originalFile = FluxFile::Decode(Arr::from(binary)).unwrap();
    originalFile.Save(&binPath).unwrap();

    // Convert binary to text
    FluxFile::Convert(&binPath, &textPath, FluxFileFormat::Text).unwrap();
    let textRestored = FluxFile::Read(&textPath).unwrap();
    jeeves_assert_eq!(ctx, textRestored.Payload(), originalFile.Payload());

    // Convert text back to binary
    FluxFile::Convert(&textPath, &binRoundtripPath, FluxFileFormat::Binary).unwrap();
    let binRestored = FluxFile::Read(&binRoundtripPath).unwrap();
    jeeves_assert_eq!(ctx, binRestored.Payload(), originalFile.Payload());

    let _ = std::fs::remove_file(&binPath);
    let _ = std::fs::remove_file(&textPath);
    let _ = std::fs::remove_file(&binRoundtripPath);

    // 5. Structured Error Inspection
    let truncated = Arr::from(&binary[..40]);
    match FluxFile::Decode(truncated) {
        Err(FluxFileError::UnexpectedEof { offset }) => {
            jeeves_assert_eq!(ctx, offset, 40);
        }
        _ => jeeves_assert!(ctx, false),
    }

    let mut corruptHeader = binary[..64].to_vec();
    corruptHeader[25] ^= 1; // Corrupt file length
    match FluxFile::Decode(Arr::from(&corruptHeader[..])) {
        Err(FluxFileError::ChecksumMismatch { region, .. }) => {
            jeeves_assert_eq!(ctx, region, "header");
        }
        _ => jeeves_assert!(ctx, false),
    }
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Flux, NativeFileStreamingExportAndImport, |ctx| {
    use crate::flux::{
        FieldExp, FieldImp, FluxFile, FluxFileError, FluxReadLimits, IFluxExportSource,
        IFluxImportSource, IObjectImport,
    };

    #[derive(Default, Debug, PartialEq)]
    struct MockGeometryPoint {
        x:    f64,
        y:    f64,
        tag:  u64,
        name: String,
    }

    impl IFluxExportSource for MockGeometryPoint {
        fn FetchFieldExp<'a>(&'a self, field: &mut FieldExp<'a>) {
            let mut index = 0;
            *field = FieldExp::Obj(Box::new(move |key, item| {
                match index {
                    0 => {
                        *key = "x".to_string();
                        *item = FieldExp::F64(self.x);
                        index += 1;
                        true
                    }
                    1 => {
                        *key = "y".to_string();
                        *item = FieldExp::F64(self.y);
                        index += 1;
                        true
                    }
                    2 => {
                        *key = "tag".to_string();
                        *item = FieldExp::U64(self.tag);
                        index += 1;
                        true
                    }
                    3 => {
                        *key = "name".to_string();
                        *item = FieldExp::Str(&self.name);
                        index += 1;
                        true
                    }
                    _ => false,
                }
            }));
        }
    }

    impl IFluxImportSource for MockGeometryPoint {
        fn FetchFieldImp<'a>(&'a mut self, field: &mut FieldImp<'a>) {
            struct Importer<'b> {
                target: &'b mut MockGeometryPoint,
            }
            impl<'b> IObjectImport for Importer<'b> {
                fn Field(&mut self, key: &str) -> Result<FieldImp<'_>, crate::flux::FluxError> {
                    match key {
                        "x" => Ok(FieldImp::F64(&mut self.target.x)),
                        "y" => Ok(FieldImp::F64(&mut self.target.y)),
                        "tag" => Ok(FieldImp::U64(&mut self.target.tag)),
                        "name" => Ok(FieldImp::String(&mut self.target.name)),
                        _ => Ok(FieldImp::Null),
                    }
                }
            }
            *field = FieldImp::Obj(Box::new(Importer { target: self }));
        }
    }

    let original = MockGeometryPoint {
        x:    12.5,
        y:    -45.25,
        tag:  999,
        name: "anchor_vertex".to_string(),
    };

    // 1. Direct streaming encode to binary buffer without FluxValue AST intermediate
    let encoded = FluxFile::EncodeStreamBinary("trellis.geometry", 1, &original).unwrap();

    // 2. Direct streaming decode from binary buffer into destination struct
    let mut restored = MockGeometryPoint::default();
    let info = FluxFile::DecodeStream(encoded.Arr(), &mut restored).unwrap();
    jeeves_assert_eq!(ctx, info.Metadata().Schema(), "trellis.geometry");
    jeeves_assert_eq!(ctx, restored, original);

    // 3. ImportPayload directly from a decoded FluxFile
    let decodedFile = FluxFile::Decode(encoded.Arr()).unwrap();
    let mut second = MockGeometryPoint::default();
    decodedFile.ImportPayload(&mut second).unwrap();
    jeeves_assert_eq!(ctx, second, original);

    // 4. Bounded limits rejection on streaming
    let strictLimits = FluxReadLimits::New().WithMaxTotalValues(2);
    let mut failed = MockGeometryPoint::default();
    let res = FluxFile::DecodeStreamWithLimits(encoded.Arr(), &mut failed, strictLimits);
    match res {
        Err(FluxFileError::LimitExceeded { limit, .. }) => {
            jeeves_assert_eq!(ctx, limit, "total values");
        }
        _ => jeeves_assert!(ctx, false),
    }
});

//-------------------------------------------------------------------------------------------------
