//-- _tests.rs ----------------------------------------------------------------------------------------------------------------------
use crate::{
    Buff,
    fenst::geometry::GeometryAsset,
    fleck::{
        BBox3f, Dir3f, ParseWaveObj, ParseWaveObjBytes, ParseWaveObjStream, Pt3f, RevolveAxis,
        Sketch3dProjection, Sketch3dProjectionOptions, SketchExtrude, SketchFlux, SketchGeometry,
        SketchIo, SketchQuery, SketchRevolve, SketchRevolveOptions, SketchTessellate,
        WPt2f, WPt3f,
        ptio::{ParsePts, ParsePtsBytes, ParsePtsStream, PtsCloud, PtsShard},
        vex::{
            Cross, Dot, ICrossProduct, IInnerProductSpace, IScalar, IVectorSpace, Lerp, Vex, Vex2f,
            Vex3d, Vex3f, Vex3i, Vex4f,
        },
    },
    flux::instream::FixedStream,
    jeeves_test,
    shard::Parser,
    silo::Buff,
};
jeeves_test!(Fleck, GeometryGpuPreparation, |_ctx| {
    use crate::fleck::geometry::{FleckGeometryAssetExt, GeometryAsset};
    let  	cloud = GeometryAsset::FromPts( ParsePts( "2\n0.001 0 0 255 0 0\n0.002 0 0 0 255 0\n").unwrap()).unwrap();
    assert_eq!(cloud.VertexCount(), 2);
    assert!(cloud.Vertices()[0].Position()[0] < -0.99);
    assert!(cloud.Vertices()[1].Position()[0] > 0.99);
    assert_eq!(cloud.Vertices()[0].Color(), [1.0, 0.0, 0.0, 1.0]);
    assert!(GeometryAsset::FromPts(ParsePts("").unwrap()).is_err());
    let invalid = ParseWaveObj("v 0 0 0\nf 1 2 3\n").unwrap();
    assert!(GeometryAsset::FromObj(invalid).is_err());
    let  	mesh = GeometryAsset::FromObj( ParseWaveObj( "v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n").unwrap()).unwrap();
    assert_eq!(mesh.Triangles().Size(), 1);
    assert_eq!(mesh.Triangles()[0], [0, 1, 2]);
    let point = GeometryAsset::FromPts(ParsePts("12345 12345 12345\n").unwrap()).unwrap();
    assert_eq!(point.Vertices()[0].Position(), [0.0; 3]);
    let extreme =
        GeometryAsset::FromPts(ParsePts("-3e38 0 0 -3e38\n3e38 0 0 3e38\n").unwrap()).unwrap();
    assert_eq!(extreme.Vertices()[0].Intensity(), 0.0);
    assert_eq!(extreme.Vertices()[1].Intensity(), 1.0);
    assert_eq!(extreme.Vertices()[0].Position(), [-1.0, 0.0, 0.0]);
    assert_eq!(extreme.Vertices()[1].Position(), [1.0, 0.0, 0.0]);
    assert!(GeometryAsset::FromPts(ParsePts("NaN 0 0\n").unwrap()).is_err());
    assert!(GeometryAsset::FromObj(ParseWaveObj("v inf 0 0\n").unwrap()).is_err());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, GeneratedGeometryValidation, |_ctx| {
    use crate::fleck::geometry::{GeometryAsset, GeometryVertex};
    let vertices = || Buff![GeometryVertex::New([0.0; 3], [1.0; 4])];
    assert!(GeometryAsset::FromMesh(vertices(),
                                    Buff![[0, 1, 2]],
                                    Buff::New(),
                                    Buff::New(),
                                    ([0.0; 3], [1.0; 3])).is_err());
    assert!(GeometryAsset::FromMesh(vertices(),
                                    Buff::New(),
                                    Buff::New(),
                                    Buff![[2, 0, 0]],
                                    ([0.0; 3], [1.0; 3])).is_err());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, GeneratedGeometryDepthIsolation, |_ctx| {
    use crate::fleck::geometry::{GeometryAsset, GeometryVertex};
    let vertices = || {
        Buff![GeometryVertex::New([0.0, 0.0, 0.0], [1.0; 4]),
              GeometryVertex::New([1.0, 0.0, 0.0], [1.0; 4]),
              GeometryVertex::New([0.0, 1.0, 0.0], [1.0; 4]),
              GeometryVertex::New([1.0, 1.0, 0.0], [1.0; 4]),]
    };
    let bounds = ([0.0; 3], [1.0; 3]);
    assert!(GeometryAsset::FromMesh(vertices(),
                                    Buff![[0, 1, 3]],
                                    Buff::New(),
                                    Buff![[3, 1, 0], [4, 1, 0]],
                                    bounds).is_err());
    assert!(GeometryAsset::FromMesh(vertices(),
                                    Buff::New(),
                                    Buff![[0, 3]],
                                    Buff![[3, 0, 1], [4, 0, 1]],
                                    bounds).is_err());
    let mesh = GeometryAsset::FromMesh(vertices(),
                                       Buff![[0, 1, 2], [1, 2, 3]],
                                       Buff![[0, 2], [1, 3]],
                                       Buff![[3, 1, 1], [4, 2, 2]],
                                       bounds).unwrap();
    assert_eq!(mesh.DrawCounts(1), [3, 1, 1]);
    assert_eq!(mesh.DrawCounts(2), [4, 2, 2]);
    assert!(GeometryAsset::FromMesh(vertices(),
                                    Buff::New(),
                                    Buff::New(),
                                    Buff![[4, 0, 0], [3, 0, 0]],
                                    bounds).is_err());
    assert!(GeometryAsset::FromMesh(vertices(),
                                    Buff::New(),
                                    Buff::New(),
                                    Buff![[3, 0, 0]],
                                    bounds).is_err());
    assert!(GeometryAsset::FromMesh(vertices(),
                                    Buff![[0, 1, 2]],
                                    Buff::New(),
                                    Buff::New(),
                                    bounds).is_ok());
});

jeeves_test!(Fleck, GeneratedGeometryBoundsAndColors, |_ctx| {
    use crate::fleck::geometry::{GeometryAsset, GeometryVertex};
    let build = |position, color, bounds| {
        GeometryAsset::FromMesh(Buff![GeometryVertex::New(position, color)],
                                Buff::New(),
                                Buff::New(),
                                Buff::New(),
                                bounds)
    };
    let bounds = ([0.0; 3], [1.0; 3]);
    assert!(build([1.1, 0.0, 0.0], [1.0; 4], bounds).is_err());
    assert!(build([-0.1, 0.0, 0.0], [1.0; 4], bounds).is_err());
    assert!(build([f32::NAN, 0.0, 0.0], [1.0; 4], bounds).is_err());
    assert!(build([0.0; 3], [f32::INFINITY, 1.0, 1.0, 1.0], bounds).is_err());
    assert!(build([0.0; 3], [1.0, 1.0, 1.0, f32::NAN], bounds).is_err());
    assert!(build([1.0; 3], [1.0; 4], bounds).is_ok());
    assert!(build([0.0; 3], [1.0; 4], ([0.0; 3], [0.0; 3])).is_ok());
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, GeometrySamplesValidation, |_ctx| {
    use crate::fleck::geometry::{GeometryAsset, GeometryVertex};
    use crate::silo::USeg;
    let mesh = || {
        GeometryAsset::FromMesh(Buff![GeometryVertex::New([0.0; 3], [1.0; 4]),
                                      GeometryVertex::New([1.0; 3], [1.0; 4])],
                                Buff::New(),
                                Buff::New(),
                                Buff![[1, 0, 0], [2, 0, 0]],
                                ([0.0; 3], [1.0; 3])).unwrap()
    };
    let samples = || {
        Buff![GeometryVertex::New([0.0; 3], [1.0; 4]),
              GeometryVertex::New([1.0; 3], [1.0; 4])]
    };
    let plain = mesh();
    assert!(plain.Samples().is_none());
    assert_eq!(plain.PointCount(), plain.VertexCount());
    assert_eq!(plain.PointDrawCount(1), plain.DrawCounts(1)[0]);
    let sampled = mesh().WithSamples(samples(), Buff![1, 2]).unwrap();
    assert_eq!(sampled.PointCount(), 2);
    assert_eq!(sampled.PointDrawCount(0), 1);
    assert_eq!(sampled.PointDrawCount(u32::MAX), 2);
    assert_eq!(sampled.Samples().unwrap().Vertices()[0].Position(),
               sampled.Vertices()[0].Position());
    assert_eq!(sampled.Samples().unwrap().Vertices()[1].Position(),
               sampled.Vertices()[1].Position());
    assert!(mesh().WithSamples(samples(), Buff::New()).is_err());
    assert!(mesh().WithSamples(Buff::New(), Buff![0, 0]).is_err());
    assert!(mesh().WithSamples(samples(), Buff![2]).is_err());
    assert!(mesh().WithSamples(samples(), Buff![2, 1]).is_err());
    assert!(mesh().WithSamples(samples(), Buff![1, 3]).is_err());
    assert!(mesh().WithSamples(samples(), Buff![0, 1]).is_err());
    USeg::FromLen(3).Traverse(|case| {
                        let position = if case == 0 {
                            [1.1; 3]
                        } else if case == 1 {
                            [f32::NAN; 3]
                        } else {
                            [0.0; 3]
                        };
                        let color = if case == 2 {
                            [f32::INFINITY; 4]
                        } else {
                            [1.0; 4]
                        };
                        assert!(mesh().WithSamples(Buff![GeometryVertex::New(position, color)],
                                                   Buff![1, 1])
                                      .is_err());
                    });
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, PtsBasic3D, |_ctx| {
    let ptsData = "10.0 20.0 30.0\n40.5 -50.25 60.125\n0.0 0.0 0.0\n";
    let res = ParsePts(ptsData);
    assert!(res.is_ok());
    let cloud = res.unwrap();
    assert_eq!(cloud.Count() as usize, 3);
    assert!(!cloud.IsEmpty());
    let arr = cloud.Points().Arr();
    let p0 = arr[0];
    assert_eq!(p0._Pos._X, 10.0);
    assert_eq!(p0._Pos._Y, 20.0);
    assert_eq!(p0._Pos._Z, 30.0);
    assert_eq!(p0._Intensity, None);
    assert_eq!(p0._Color, None);
    let p1 = arr[1];
    assert_eq!(p1._Pos._X, 40.5);
    assert_eq!(p1._Pos._Y, -50.25);
    assert_eq!(p1._Pos._Z, 60.125);
    let (minBbox, maxBbox) = cloud.BoundingBox();
    assert_eq!(minBbox, [0.0, -50.25, 0.0]);
    assert_eq!(maxBbox, [40.5, 20.0, 60.125]);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, PtsWithHeaderAndIntensity, |_ctx| {
    let ptsData =
        "# Point cloud with header and intensity\n2\n1.0 2.0 3.0 0.75\n-4.0 -5.0 -6.0 0.25\n";
    let res = ParsePts(ptsData);
    assert!(res.is_ok());
    let cloud = res.unwrap();
    assert_eq!(cloud._HeaderCount, Some(2));
    assert_eq!(cloud.Count() as usize, 2);
    let arr = cloud.Points().Arr();
    let p0 = arr[0];
    assert_eq!(p0._Pos._X, 1.0);
    assert_eq!(p0._Pos._Y, 2.0);
    assert_eq!(p0._Pos._Z, 3.0);
    assert_eq!(p0._Intensity, Some(0.75));
    let p1 = arr[1];
    assert_eq!(p1._Pos._X, -4.0);
    assert_eq!(p1._Pos._Y, -5.0);
    assert_eq!(p1._Pos._Z, -6.0);
    assert_eq!(p1._Intensity, Some(0.25));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, PtsWithColorRGB, |_ctx| {
    let ptsData = "10.0 20.0 30.0 255 128 64\n0.0 0.0 0.0 0 255 0\n";
    let res = ParsePts(ptsData);
    assert!(res.is_ok());
    let cloud = res.unwrap();
    assert_eq!(cloud.Count() as usize, 2);
    let arr = cloud.Points().Arr();
    let p0 = arr[0];
    assert_eq!(p0._Pos._X, 10.0);
    assert_eq!(p0._Pos._Y, 20.0);
    assert_eq!(p0._Pos._Z, 30.0);
    let color0 = p0._Color.unwrap();
    assert_eq!(color0._R, 255);
    assert_eq!(color0._G, 128);
    assert_eq!(color0._B, 64);
    let p1 = arr[1];
    let color1 = p1._Color.unwrap();
    assert_eq!(color1._R, 0);
    assert_eq!(color1._G, 255);
    assert_eq!(color1._B, 0);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, PtsWithIntensityAndColor, |_ctx| {
    let ptsData = "1.0 2.0 3.0 0.9 255 200 150\n";
    let res = ParsePts(ptsData);
    assert!(res.is_ok());
    let cloud = res.unwrap();
    assert_eq!(cloud.Count() as usize, 1);
    let arr = cloud.Points().Arr();
    let p0 = arr[0];
    assert_eq!(p0._Pos._X, 1.0);
    assert_eq!(p0._Pos._Y, 2.0);
    assert_eq!(p0._Pos._Z, 3.0);
    assert_eq!(p0._Intensity, Some(0.9));
    let color0 = p0._Color.unwrap();
    assert_eq!(color0._R, 255);
    assert_eq!(color0._G, 200);
    assert_eq!(color0._B, 150);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, PtsScientificNotationAndComments, |_ctx| {
    let ptsData = "// Header comment\n# Another comment\n\n1.25e-2 -3.5E+1 4.0e0\n  \t  10.0   20.0   30.0  # inline comment\n";
    let res = ParsePts(ptsData);
    assert!(res.is_ok());
    let cloud = res.unwrap();
    assert_eq!(cloud.Count() as usize, 2);
    let arr = cloud.Points().Arr();
    let p0 = arr[0];
    assert_eq!(p0._Pos._X, 0.0125);
    assert_eq!(p0._Pos._Y, -35.0);
    assert_eq!(p0._Pos._Z, 4.0);
    let p1 = arr[1];
    assert_eq!(p1._Pos._X, 10.0);
    assert_eq!(p1._Pos._Y, 20.0);
    assert_eq!(p1._Pos._Z, 30.0);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, PtsToDtoConversion, |_ctx| {
    let ptsData = "0.0 0.0 0.0\n100.0 200.0 300.0\n";
    let cloud = ParsePts(ptsData).unwrap();
    let dto = cloud.ToDto();
    assert_eq!(dto._Count, 2);
    assert_eq!(dto._Points.Len(), 2);
    assert_eq!(dto._Points[0], [0.0, 0.0, 0.0]);
    assert_eq!(dto._Points[1], [100.0, 200.0, 300.0]);
    assert_eq!(dto._BboxMin, [0.0, 0.0, 0.0]);
    assert_eq!(dto._BboxMax, [100.0, 200.0, 300.0]);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, PtsParseBytesAndStream, |_ctx| {
    let bytes = b"10.0 20.0 30.0\n";
    let cloudFromBytes = ParsePtsBytes(bytes.into()).unwrap();
    assert_eq!(cloudFromBytes.Count() as usize, 1);
    let mut stream = FixedStream::from("10.0 20.0 30.0\n");
    let cloudFromStream = ParsePtsStream(&mut stream).unwrap();
    assert_eq!(cloudFromStream.Count() as usize, 1);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, PtsShardGrammarDirect, |_ctx| {
    let ptsData = "5.0 15.0 25.0\n";
    let mut stream = FixedStream::from(ptsData);
    let mut parser = Parser::New(&mut stream);
    let mut cloud = PtsCloud::New();
    let shard = PtsShard { _Cloud: &mut cloud };
    let res = parser.ParseGrammar(&shard, 0);
    assert!(res.is_some());
    assert_eq!(cloud.Count() as usize, 1);
    let arr = cloud.Points().Arr();
    let p0 = arr[0];
    assert_eq!(p0._Pos._X, 5.0);
    assert_eq!(p0._Pos._Y, 15.0);
    assert_eq!(p0._Pos._Z, 25.0);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, WaveObjBasicCube, |_ctx| {
    let objData = r#"
# Wavefront OBJ Cube
o Cube
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 1.0 1.0 0.0
v 0.0 1.0 0.0
v 0.0 0.0 1.0
v 1.0 0.0 1.0
v 1.0 1.0 1.0
v 0.0 1.0 1.0
# 6 quad faces
f 1 2 3 4
f 5 6 7 8
f 1 2 6 5
f 2 3 7 6
f 3 4 8 7
f 4 1 5 8
"#;
    let res = ParseWaveObj(objData);
    assert!(res.is_ok());
    let model = res.unwrap();
    assert_eq!(model.VertexCount() as usize, 8);
    assert_eq!(model.FaceCount() as usize, 6);
    assert_eq!(model._Objects.Size() as usize, 1);
    assert_eq!(model._Objects.Arr()[0].as_str(), "Cube");
    let (bMin, bMax) = model.BoundingBox();
    assert_eq!(bMin, [0.0, 0.0, 0.0]);
    assert_eq!(bMax, [1.0, 1.0, 1.0]);
    let triangles = model.Triangulate();
    assert_eq!(triangles.Size() as usize, 12); // 6 quads = 12 triangles
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, WaveObjWithNormalsAndTexCoords, |_ctx| {
    let objData = r#"
# Vertices, TexCoords, Normals, and Faces with v/vt/vn
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vt 0.0 0.0
vt 1.0 0.0
vt 0.0 1.0
vn 0.0 0.0 1.0
f 1/1/1 2/2/1 3/3/1
"#;
    let res = ParseWaveObj(objData);
    assert!(res.is_ok());
    let model = res.unwrap();
    assert_eq!(model.VertexCount() as usize, 3);
    assert_eq!(model.TexCoordCount() as usize, 3);
    assert_eq!(model.NormalCount() as usize, 1);
    assert_eq!(model.FaceCount() as usize, 1);
    let facesArr = model._Faces.Arr();
    let face0 = &facesArr[0];
    assert_eq!(face0.Len(), 3);
    let v0 = face0._Vertices.Arr()[0];
    assert_eq!(v0._VertexIdx, 1);
    assert_eq!(v0._TexCoordIdx, Some(1));
    assert_eq!(v0._NormalIdx, Some(1));
    let v1 = face0._Vertices.Arr()[1];
    assert_eq!(v1._VertexIdx, 2);
    assert_eq!(v1._TexCoordIdx, Some(2));
    assert_eq!(v1._NormalIdx, Some(1));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, WaveObjFaceFormats, |_ctx| {
    let objData = "v 0.0 0.0 0.0\nv 1.0 0.0 0.0\nv 0.0 1.0 0.0\nv 1.0 1.0 0.0\nvt 0.0 0.0\nvt 1.0 1.0\nvn 0.0 0.0 1.0\n# v only\nf 1 2 3\n# v/vt\nf 1/1 2/2 3/1\n# v//vn\nf 1//1 2//1 3//1\n# v/vt/vn\nf 1/1/1 2/2/1 4/2/1 3/1/1\n";
    let model = ParseWaveObj(objData).unwrap();
    assert_eq!(model.FaceCount() as usize, 4);
    let faces = model._Faces.Arr();
    // Face 0: v only
    let f0 = &faces[0];
    assert_eq!(f0._Vertices.Arr()[0]._VertexIdx, 1);
    assert_eq!(f0._Vertices.Arr()[0]._TexCoordIdx, None);
    assert_eq!(f0._Vertices.Arr()[0]._NormalIdx, None);
    // Face 1: v/vt
    let f1 = &faces[1];
    assert_eq!(f1._Vertices.Arr()[0]._VertexIdx, 1);
    assert_eq!(f1._Vertices.Arr()[0]._TexCoordIdx, Some(1));
    assert_eq!(f1._Vertices.Arr()[0]._NormalIdx, None);
    // Face 2: v//vn
    let f2 = &faces[2];
    assert_eq!(f2._Vertices.Arr()[0]._VertexIdx, 1);
    assert_eq!(f2._Vertices.Arr()[0]._TexCoordIdx, None);
    assert_eq!(f2._Vertices.Arr()[0]._NormalIdx, Some(1));
    // Face 3: quad v/vt/vn
    let f3 = &faces[3];
    assert_eq!(f3.Len(), 4);
    assert_eq!(f3._Vertices.Arr()[3]._VertexIdx, 3);
    assert_eq!(f3._Vertices.Arr()[3]._TexCoordIdx, Some(1));
    assert_eq!(f3._Vertices.Arr()[3]._NormalIdx, Some(1));
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, WaveObjNegativeIndexing, |_ctx| {
    let objData = r#"
v 10.0 20.0 30.0
v 40.0 50.0 60.0
v 70.0 80.0 90.0
f -3 -2 -1
"#;
    let model = ParseWaveObj(objData).unwrap();
    assert_eq!(model.VertexCount() as usize, 3);
    assert_eq!(model.FaceCount() as usize, 1);
    let f0 = &model._Faces.Arr()[0];
    assert_eq!(f0._Vertices.Arr()[0]._VertexIdx, 1);
    assert_eq!(f0._Vertices.Arr()[1]._VertexIdx, 2);
    assert_eq!(f0._Vertices.Arr()[2]._VertexIdx, 3);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, WaveObjToDtoAndTriangulate, |_ctx| {
    let objData = b"v 0.0 0.0 0.0\nv 10.0 0.0 0.0\nv 10.0 10.0 0.0\nv 0.0 10.0 0.0\nf 1 2 3 4\n";
    let model = ParseWaveObjBytes(objData.into()).unwrap();
    let dto = model.ToDto();
    assert_eq!(dto._Count, 4);
    assert_eq!(dto._BboxMin, [0.0, 0.0, 0.0]);
    assert_eq!(dto._BboxMax, [10.0, 10.0, 0.0]);
    let triangles = model.Triangulate();
    assert_eq!(triangles.Size() as usize, 2);
    let mut stream = FixedStream::from("v 5.0 5.0 5.0\n");
    let streamModel = ParseWaveObjStream(&mut stream).unwrap();
    assert_eq!(streamModel.VertexCount() as usize, 1);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, WaveObjMetadataAndMaterials, |_ctx| {
    let objData = r#"
mtllib materials.mtl
o MainModel
g GroupA
usemtl Metal_Shiny
v 1.0 2.0 3.0
v 4.0 5.0 6.0
v 7.0 8.0 9.0
f 1 2 3
"#;
    let model = ParseWaveObj(objData).unwrap();
    assert_eq!(model._MtlLibs.Size() as usize, 1);
    assert_eq!(model._MtlLibs.Arr()[0].as_str(), "materials.mtl");
    assert_eq!(model._Objects.Arr()[0].as_str(), "MainModel");
    assert_eq!(model._Groups.Arr()[0].as_str(), "GroupA");
    assert_eq!(model._UseMtls.Arr()[0].as_str(), "Metal_Shiny");
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, Pt3fBasicOps, |_ctx| {
    let pt = Pt3f::New(1.5, 2.5, 3.5);
    assert_eq!(pt._X, 1.5);
    assert_eq!(pt._Y, 2.5);
    assert_eq!(pt._Z, 3.5);
    assert_eq!(pt.Pos(), [1.5, 2.5, 3.5]);
    let defaultPt = Pt3f::default();
    assert_eq!(defaultPt.Pos(), [0.0, 0.0, 0.0]);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, WPt3fBasicOps, |_ctx| {
    let pt = WPt3f::New(1.0, 2.0, 3.0);
    assert_eq!(pt._X, 1.0);
    assert_eq!(pt._Y, 2.0);
    assert_eq!(pt._Z, 3.0);
    assert_eq!(pt._W, 1.0);
    assert_eq!(pt.Pos(), [1.0, 2.0, 3.0]);
    let ptW = WPt3f::WithW(4.0, 5.0, 6.0, 2.0);
    assert_eq!(ptW._W, 2.0);
    let defaultPt = WPt3f::default();
    assert_eq!(defaultPt.Pos(), [0.0, 0.0, 0.0]);
    assert_eq!(defaultPt._W, 0.0);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, WPt2fBasicOps, |_ctx| {
    let pt = WPt2f::New(0.25, 0.75);
    assert_eq!(pt._U, 0.25);
    assert_eq!(pt._V, 0.75);
    assert_eq!(pt._W, 0.0);
    assert_eq!(pt.Pos(), [0.25, 0.75]);
    let ptW = WPt2f::WithW(0.1, 0.2, 0.3);
    assert_eq!(ptW._W, 0.3);
    let defaultPt = WPt2f::default();
    assert_eq!(defaultPt.Pos(), [0.0, 0.0]);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, Dir3fBasicOps, |_ctx| {
    let dir = Dir3f::New(0.0, 0.0, 1.0);
    assert_eq!(dir._X, 0.0);
    assert_eq!(dir._Y, 0.0);
    assert_eq!(dir._Z, 1.0);
    assert_eq!(dir.Vec(), [0.0, 0.0, 1.0]);
    let defaultDir = Dir3f::default();
    assert_eq!(defaultDir.Vec(), [0.0, 0.0, 0.0]);
});

//---------------------------------------------------------------------------------------------------------------------------------
//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, VexBasicConstructorsAndAccessors, |_ctx| {
    let v2 = Vex2f::New2(3.0, 4.0);
    assert_eq!(v2.X(), 3.0);
    assert_eq!(v2.Y(), 4.0);
    assert_eq!(v2._Data, [3.0, 4.0]);
    let mut v3 = Vex3f::New3(1.0, 2.0, 3.0);
    assert_eq!(v3.X(), 1.0);
    assert_eq!(v3.Y(), 2.0);
    assert_eq!(v3.Z(), 3.0);
    v3.SetX(10.0);
    v3.SetY(20.0);
    v3.SetZ(30.0);
    assert_eq!(v3.Array(), &[10.0, 20.0, 30.0]);
    let v4 = Vex4f::New4(1.0, 2.0, 3.0, 4.0);
    assert_eq!(v4.W(), 4.0);
    let splat = Vex3f::Splat(5.0);
    assert_eq!(splat._Data, [5.0, 5.0, 5.0]);
    let mapped = v2.Map(|c| c * 2.0);
    assert_eq!(mapped._Data, [6.0, 8.0]);
    let zipped = v2.ZipMap(&mapped, |a, b| a + b);
    assert_eq!(zipped._Data, [9.0, 12.0]);
    let vInt = Vex3i::New3(10, -20, 30);
    assert_eq!(vInt.X(), 10);
    assert_eq!(vInt.Y(), -20);
    assert_eq!(vInt.Z(), 30);
    let vDouble = Vex3d::New3(1.5, 2.5, 3.5);
    assert_eq!(vDouble.X(), 1.5);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, VexByValueAndByRefOperators, |_ctx| {
    let a = Vex3f::New3(1.0, 2.0, 3.0);
    let b = Vex3f::New3(4.0, 5.0, 6.0);
    // Val + Val
    let sumValVal = a + b;
    assert_eq!(sumValVal._Data, [5.0, 7.0, 9.0]);
    // Ref + Ref
    let sumRefRef = a + b;
    assert_eq!(sumRefRef._Data, [5.0, 7.0, 9.0]);
    // Ref + Val
    let sumRefVal = a + b;
    assert_eq!(sumRefVal._Data, [5.0, 7.0, 9.0]);
    // Val + Ref
    let sumValRef = a + b;
    assert_eq!(sumValRef._Data, [5.0, 7.0, 9.0]);
    // Subtraction
    let diffRefRef = b - a;
    assert_eq!(diffRefRef._Data, [3.0, 3.0, 3.0]);
    let diffValVal = b - a;
    assert_eq!(diffValVal._Data, [3.0, 3.0, 3.0]);
    // Negation
    let negVal = -a;
    assert_eq!(negVal._Data, [-1.0, -2.0, -3.0]);
    let negRef = -&a;
    assert_eq!(negRef._Data, [-1.0, -2.0, -3.0]);
    // Hadamard Multiplication
    let hadamard = a * b;
    assert_eq!(hadamard._Data, [4.0, 10.0, 18.0]);
    // Component-wise Division
    let quotient = b / a;
    assert_eq!(quotient._Data, [4.0, 2.5, 2.0]);
    // Compound Assignments
    let mut acc = a;
    acc += &b;
    assert_eq!(acc._Data, [5.0, 7.0, 9.0]);
    acc -= &b;
    assert_eq!(acc._Data, [1.0, 2.0, 3.0]);
    acc *= &b;
    assert_eq!(acc._Data, [4.0, 10.0, 18.0]);
    acc /= &b;
    assert_eq!(acc._Data, [1.0, 2.0, 3.0]);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, VexScalarArithmetic, |_ctx| {
    let vf = Vex3f::New3(2.0, 3.0, 4.0);
    let sf = 2.5f32;
    // Vector * Scalar
    assert_eq!((vf * sf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((vf * sf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((vf * sf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((vf * sf)._Data, [5.0, 7.5, 10.0]);
    // Scalar * Vector
    assert_eq!((sf * vf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((sf * vf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((sf * vf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((sf * vf)._Data, [5.0, 7.5, 10.0]);
    // Vector / Scalar
    assert_eq!((vf / 2.0f32)._Data, [1.0, 1.5, 2.0]);
    assert_eq!((&vf / 2.0f32)._Data, [1.0, 1.5, 2.0]);
    assert_eq!((vf / 2.0f32)._Data, [1.0, 1.5, 2.0]);
    // Compound Scalar Assignments
    let mut mutV = vf;
    mutV *= 2.0f32;
    assert_eq!(mutV._Data, [4.0, 6.0, 8.0]);
    mutV /= 2.0f32;
    assert_eq!(mutV._Data, [2.0, 3.0, 4.0]);
    // Integer Scalar Multiplication
    let vi = Vex3i::New3(1, -2, 3);
    assert_eq!((vi * 3i32)._Data, [3, -6, 9]);
    assert_eq!((3i32 * vi)._Data, [3, -6, 9]);
    // Double Scalar Multiplication
    let vd = Vex3d::New3(1.0, 2.0, 3.0);
    assert_eq!((vd * 0.5f64)._Data, [0.5, 1.0, 1.5]);
    assert_eq!((0.5f64 * vd)._Data, [0.5, 1.0, 1.5]);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, VexVectorSpaceAndInnerProduct, |_ctx| {
    let zero = Vex3f::Zero();
    assert!(zero.IsZero());
    let u = Vex3f::New3(1.0, 0.0, 0.0);
    let v = Vex3f::New3(0.0, 1.0, 0.0);
    assert!(!u.IsZero());
    // Dot product
    assert_eq!(u.Dot(&v), 0.0);
    assert_eq!(Dot(&u, &v), 0.0);
    let w = Vex3f::New3(3.0, 4.0, 0.0);
    assert_eq!(w.MagnitudeSquared(), 25.0);
    assert_eq!(w.Magnitude(), 5.0);
    // Normalization
    let normW = w.Normalized().unwrap();
    assert_eq!(normW._Data, [0.6, 0.8, 0.0]);
    assert_eq!(normW.Magnitude(), 1.0);
    // Distance
    let p1 = Vex3f::New3(1.0, 2.0, 3.0);
    let p2 = Vex3f::New3(4.0, 6.0, 3.0);
    assert_eq!(p1.DistanceSquared(&p2), 25.0);
    assert_eq!(p1.Distance(&p2), 5.0);
    // Angle
    let angleRad = u.Angle(&v);
    let piOver2 = std::f32::consts::FRAC_PI_2;
    assert!((angleRad - piOver2).abs() < 1e-5);
    // Projection and Rejection
    let vecA = Vex3f::New3(3.0, 3.0, 0.0);
    let vecB = Vex3f::New3(5.0, 0.0, 0.0);
    let proj = vecA.Project(&vecB).unwrap();
    assert_eq!(proj._Data, [3.0, 0.0, 0.0]);
    let reject = vecA.Reject(&vecB).unwrap();
    assert_eq!(reject._Data, [0.0, 3.0, 0.0]);
    // Reflection (ray hitting a horizontal surface facing up)
    let inRay = Vex3f::New3(1.0, -1.0, 0.0);
    let normal = Vex3f::New3(0.0, 1.0, 0.0);
    let reflected = inRay.Reflect(&normal);
    assert_eq!(reflected._Data, [1.0, 1.0, 0.0]);
    // Linear Interpolation (Lerp)
    let start = Vex3f::New3(0.0, 0.0, 0.0);
    let end = Vex3f::New3(10.0, 20.0, 30.0);
    let mid = start.Lerp(&end, 0.5);
    assert_eq!(mid._Data, [5.0, 10.0, 15.0]);
    assert_eq!(Lerp(&start, &end, 0.5)._Data, [5.0, 10.0, 15.0]);
    // Cross Product
    let cross = u.Cross(&v);
    assert_eq!(cross._Data, [0.0, 0.0, 1.0]);
    assert_eq!(Cross(&u, &v)._Data, [0.0, 0.0, 1.0]);
    // Orthogonality of cross product
    let aRnd = Vex3f::New3(2.5, -3.1, 4.2);
    let bRnd = Vex3f::New3(1.1, 7.8, -0.9);
    let cRnd = aRnd.Cross(&bRnd);
    assert!(cRnd.Dot(&aRnd).abs() < 1e-4);
    assert!(cRnd.Dot(&bRnd).abs() < 1e-4);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, VexIndexingAndConversionInterop, |_ctx| {
    let mut v = Vex3f::New3(10.0, 20.0, 30.0);
    // Indexing
    assert_eq!(v[0], 10.0f32);
    assert_eq!(v[1], 20.0f32);
    assert_eq!(v[2], 30.0f32);
    v[1] = 99.0f32;
    assert_eq!(v[1], 99.0f32);
    v[2u64] = 123.0f32;
    assert_eq!(v[2], 123.0f32);
    // From / Into tuples
    let v2: Vex2f = (1.0, 2.0).into();
    assert_eq!(v2._Data, [1.0, 2.0]);
    let v3: Vex3f = (1.0, 2.0, 3.0).into();
    assert_eq!(v3._Data, [1.0, 2.0, 3.0]);
    let v4: Vex4f = (1.0, 2.0, 3.0, 4.0).into();
    assert_eq!(v4._Data, [1.0, 2.0, 3.0, 4.0]);
    // Iterators
    let collected: Vec<f32> = v.into_iter().collect();
    assert_eq!(collected, vec![10.0, 99.0, 123.0]);
    // Pt3f Interop
    let pt = Pt3f::New(1.0, 2.0, 3.0);
    let vPt: Vex3f = pt.into();
    assert_eq!(vPt._Data, [1.0, 2.0, 3.0]);
    let backPt: Pt3f = vPt.into();
    assert_eq!(backPt, pt);
    // Dir3f Interop
    let dir = Dir3f::New(0.0, 1.0, 0.0);
    let vDir: Vex3f = dir.into();
    assert_eq!(vDir._Data, [0.0, 1.0, 0.0]);
    let backDir: Dir3f = vDir.into();
    assert_eq!(backDir, dir);
    // WPt3f Interop
    let wpt3 = WPt3f::WithW(1.0, 2.0, 3.0, 0.5);
    let vWpt3: Vex4f = wpt3.into();
    assert_eq!(vWpt3._Data, [1.0, 2.0, 3.0, 0.5]);
    let backWpt3: WPt3f = vWpt3.into();
    assert_eq!(backWpt3, wpt3);
    // WPt2f Interop
    let wpt2 = WPt2f::New(4.0, 5.0);
    let vWpt2: Vex2f = wpt2.into();
    assert_eq!(vWpt2._Data, [4.0, 5.0]);
    let backWpt2: WPt2f = vWpt2.into();
    assert_eq!(backWpt2, wpt2);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, VexWithIntScalarTypes, |_ctx| {
    // Test IScalar methods on Int types
    assert_eq!(i32::ZERO, 0);
    assert_eq!(i32::ONE, 1);
    assert_eq!(16i32.Sqrt(), 4);
    assert_eq!((-42i32).Abs(), 42);
    assert_eq!(i32::FromF32(12.75), 12);
    assert_eq!(100i32.ToF64(), 100.0);
    // Test Vex< i32, 3> construction and algebra
    let v1 = Vex::<i32, 3>::New([10, 20, 30]);
    let v2 = Vex::<i32, 3>::New([1, 2, 3]);
    let sum = v1 + v2;
    assert_eq!(sum._Data, [11, 22, 33]);
    let diff = v1 - v2;
    assert_eq!(diff._Data, [9, 18, 27]);
    let scaled = v1 * 2;
    assert_eq!(scaled._Data, [20, 40, 60]);
    let dot = Dot(&v1, &v2);
    assert_eq!(dot, 10 + 20 * 2 + 30 * 3);
    // Test Vex< i8, 4>
    let vI8_1 = Vex::<i8, 4>::New([10, 20, 30, 40]);
    let vI8_2 = Vex::<i8, 4>::New([5, 10, 15, 20]);
    let sumI8 = vI8_1 + vI8_2;
    assert_eq!(sumI8._Data, [15, 30, 45, 60]);
    // Test Vex< i16, 2>
    let vI16_1 = Vex::<i16, 2>::New([300, 400]);
    let vI16_2 = Vex::<i16, 2>::New([100, 200]);
    let diffI16 = vI16_1 - vI16_2;
    assert_eq!(diffI16._Data, [200, 200]);
    // Test Vex< i64, 2>
    let vI64_1 = Vex::<i64, 2>::New([100, 200]);
    let vI64_2 = Vex::<i64, 2>::New([50, 25]);
    let dotI64 = Dot(&vI64_1, &vI64_2);
    assert_eq!(dotI64, 100 * 50 + 200 * 25);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, BuffVectorSpaceAndInnerProduct, |_ctx| {
    // 1. Float Vector Space (4D Euclidean space)
    let b1 = Buff![1.0f32, 2.0, 3.0, 4.0];
    let b2 = Buff![5.0f32, 6.0, 7.0, 8.0];
    assert_eq!(b1.Dim(), 4);
    assert!(!b1.IsZero());
    assert!(Buff::<f32>::Zero().IsZero());
    // Addition & Subtraction
    let sum = &b1 + &b2;
    assert_eq!(sum, Buff![6.0f32, 8.0, 10.0, 12.0]);
    let diff = &b2 - &b1;
    assert_eq!(diff, Buff![4.0f32, 4.0, 4.0, 4.0]);
    // Negation
    let negB1 = -&b1;
    assert_eq!(negB1, Buff![-1.0f32, -2.0, -3.0, -4.0]);
    // Scalar multiplication & division
    let scaled = &b1 * 2.0f32;
    assert_eq!(scaled, Buff![2.0f32, 4.0, 6.0, 8.0]);
    let divScaled = &scaled / 2.0f32;
    assert_eq!(divScaled, b1);
    // In-place operators
    let mut bMut = b1.clone();
    bMut += &b2;
    assert_eq!(bMut, Buff![6.0f32, 8.0, 10.0, 12.0]);
    bMut *= 0.5f32;
    assert_eq!(bMut, Buff![3.0f32, 4.0, 5.0, 6.0]);
    // Inner product (Dot product)
    // 1*5 + 2*6 + 3*7 + 4*8 = 5 + 12 + 21 + 32 = 70
    let dot = b1.Dot(&b2);
    assert_eq!(dot, 70.0f32);
    // Magnitude & Normalization (3D: 3, 4, 0 -> mag = 5)
    let b3d = Buff![3.0f32, 4.0, 0.0];
    assert_eq!(b3d.MagnitudeSquared(), 25.0f32);
    assert_eq!(b3d.Magnitude(), 5.0f32);
    let norm = b3d.Normalized().unwrap();
    assert_eq!(norm, Buff![0.6f32, 0.8, 0.0]);
    assert!((norm.Magnitude() - 1.0f32).abs() < 1e-6);
    // Distance
    let ptA = Buff![0.0f32, 0.0, 0.0];
    let ptB = Buff![0.0f32, 3.0, 4.0];
    assert_eq!(ptA.Distance(&ptB), 5.0f32);
    assert_eq!(ptA.DistanceSquared(&ptB), 25.0f32);
    // Lerp
    let lerpRes = b1.Lerp(&b2, 0.5f32);
    assert_eq!(lerpRes, Buff![3.0f32, 4.0, 5.0, 6.0]);
    // Projection & Rejection
    let u = Buff![1.0f32, 0.0, 0.0];
    let v = Buff![3.0f32, 4.0, 0.0];
    let proj = v.Project(&u).unwrap();
    assert_eq!(proj, Buff![3.0f32, 0.0, 0.0]);
    let rej = v.Reject(&u).unwrap();
    assert_eq!(rej, Buff![0.0f32, 4.0, 0.0]);
    // Reflection: (3, 4) reflected across normal (0, 1) -> (3, -4)
    let normY = Buff![0.0f32, 1.0, 0.0];
    let refl = v.Reflect(&normY);
    assert_eq!(refl, Buff![3.0f32, -4.0, 0.0]);
    // 2. Custom Int Vector Space (5D space with i32)
    let vInt1 = Buff![10i32, 20, 30, 40, 50];
    let vInt2 = Buff![1i32, 2, 3, 4, 5];
    let sumInt = &vInt1 + &vInt2;
    assert_eq!(sumInt, Buff![11i32, 22, 33, 44, 55]);
    let scaledInt = &vInt1 * 3i32;
    assert_eq!(scaledInt, Buff![30i32, 60, 90, 120, 150]);
    // Dot product: 10*1 + 20*2 + 30*3 + 40*4 + 50*5 = 10 + 40 + 90 + 160 + 250 = 550
    let dotInt = vInt1.Dot(&vInt2);
    assert_eq!(dotInt, 550i32);
    // ZeroVec & Splat
    let zeroBuff = Buff::<i8>::ZeroVec(8);
    assert_eq!(zeroBuff.Len(), 8);
    assert!(zeroBuff.IsZero());
    let splatBuff = Buff::<i8>::Splat(127i8, 4);
    assert_eq!(splatBuff, Buff![127i8, 127i8, 127i8, 127i8]);
});

//---------------------------------------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, BBox3fBasicOps, |_ctx| {
    let empty = BBox3f::Empty();
    assert!(empty.IsEmpty());
    let points = [[-10.0f32, 5.0, 20.0],
                  [30.0, -15.0, 40.0],
                  [0.0, 25.0, -5.0]];
    let bbox = BBox3f::FromPoints((&points).into());
    assert!(!bbox.IsEmpty());
    assert_eq!(bbox.Min(), [-10.0, -15.0, -5.0]);
    assert_eq!(bbox.Max(), [30.0, 25.0, 40.0]);
    let center = bbox.Center();
    assert_eq!(center, Pt3f::New(10.0, 5.0, 17.5));
    let extent = bbox.Extent();
    assert_eq!(extent, Pt3f::New(40.0, 40.0, 45.0));
    assert_eq!(bbox.MaxDim(), 45.0);
    let scaleNorm = bbox.ScaleNorm(240.0);
    assert!((scaleNorm - (240.0 / 45.0)).abs() < 1e-5);
    let corners = bbox.Corners();
    assert_eq!(corners.len(), 8);
    assert_eq!(corners[0], Pt3f::New(-10.0, -15.0, -5.0));
    assert_eq!(corners[6], Pt3f::New(30.0, 25.0, 40.0));
    let edges = BBox3f::BoxEdges();
    assert_eq!(edges.len(), 12);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, NativeSketchMetadataLayersGroupsAndArcRange, |ctx| {
    use crate::fenst::sketch::*;
    use crate::fleck::sketch_flux::SketchFlux;
    use crate::flux::{FluxFile, FluxFileFormat};
    use crate::{jeeves_assert, jeeves_assert_eq};
    let mut doc = SketchDocument::New();
    let lineId = doc.AddLine(SketchPoint::New(-0.0, 0.12345678912345678),
                             SketchPoint::New(55.25, 33.125))
                    .unwrap();
    let ellipseId = doc.AddEllipse(SketchPoint::New(80.0, 50.0), 30.0, 10.0, 0.35)
                       .unwrap();
    if let SketchPrimitive::Ellipse(ellipse) = doc.FindEntityMut(ellipseId).unwrap().PrimitiveMut()
    {
        *ellipse = ellipse.WithArc(0.25, -1.5);
    }
    let mut layer = SketchLayer::New(7, "quotes \" and 水");
    layer.SetLocked(true);
    layer.SetConstruction(true);
    layer.SetStyle(SketchStyle { _Color:       [0.1, 0.2, 0.3, 0.4],
                                 _LineWidth:   2.25,
                                 _LinePattern: 1, });
    let mut entity = doc.FindEntity(lineId)
                        .unwrap()
                        .clone()
                        .WithLayer(7)
                        .WithGroup(4)
                        .WithConstruction(true);
    *doc.FindEntityMut(lineId).unwrap() = entity.clone();
    entity = doc.FindEntity(ellipseId).unwrap().clone().WithGroup(4);
    *doc.FindEntityMut(ellipseId).unwrap() = entity;
    let layers = crate::Buff![SketchLayer::New(0, "Default"), layer];
    let groups = crate::Buff![SketchGroup::New(4, "Pair", crate::Buff![lineId, ellipseId])];
    doc.RestoreSettings("in", SketchPlane::Xy(), layers.Arr(), groups.Arr(), 42)
       .unwrap();
    let file = SketchFlux::File(&doc, None).unwrap();
    let encoded = file.Encode(FluxFileFormat::Text).unwrap();
    let restoredFile = FluxFile::Decode(encoded.Arr()).unwrap();
    let restored = SketchFlux::Document(&restoredFile).unwrap();
    jeeves_assert_eq!(ctx,
                      SketchFlux::Payload(&doc),
                      SketchFlux::Payload(&restored));
    jeeves_assert_eq!(ctx, restored.Revision(), 42);
    let next = SketchFlux::File(&restored, Some(restoredFile.Metadata())).unwrap();
    jeeves_assert_eq!(ctx,
                      next.Metadata().Get("DocumentId").unwrap(),
                      file.Metadata().Get("DocumentId").unwrap());
    if let SketchPrimitive::Ellipse(ellipse) = restored.FindEntity(ellipseId).unwrap().Primitive() {
        let points = SketchTessellate::TessellateEllipse(ellipse, 0.5, 1.0);
        jeeves_assert_eq!(ctx, points[0], ellipse.Evaluate(ellipse._StartAngle));
        jeeves_assert_eq!(ctx,
                          points[points.Size() - 1],
                          ellipse.Evaluate(ellipse._StartAngle + ellipse._SweepAngle));
        jeeves_assert!(ctx,
                       SketchQuery::PointToEllipseDistance(ellipse.Evaluate(0.25), ellipse) < 1e-6);
        jeeves_assert!(ctx,
                       SketchQuery::PointToEllipseDistance(ellipse.Evaluate(2.5), ellipse) > 1.0);
    }
    let mut bad = doc.FindEntity(lineId).unwrap().clone().WithLayer(99);
    *doc.FindEntityMut(lineId).unwrap() = bad.clone();
    let invalid = SketchFlux::File(&doc, Some(file.Metadata())).unwrap();
    jeeves_assert!(ctx, SketchFlux::Document(&invalid).is_err());
    bad = bad.WithLayer(7);
    *doc.FindEntityMut(lineId).unwrap() = bad;
});
// Planar Sketch Analytic Geometry, Transformations, and Curvature Tests
jeeves_test!(Fleck, SketchGeometryTransformsAndCurvature, |_ctx| {
    use crate::fenst::sketch::{Circle, CircularArc, Ellipse, LineSegment, SketchPoint};

    // 1. Line Segment
    let line = LineSegment::New(SketchPoint::New(0.0, 0.0), SketchPoint::New(10.0, 0.0));
    assert_eq!(line.Length(), 10.0);
    assert_eq!(line.Midpoint(), SketchPoint::New(5.0, 0.0));

    let rot_line = SketchGeometry::RotateLine(line,
                                              SketchPoint::New(0.0, 0.0),
                                              std::f64::consts::FRAC_PI_2);
    assert!((rot_line._End._X - 0.0).abs() < 1e-9);
    assert!((rot_line._End._Y - 10.0).abs() < 1e-9);

    // 2. Circle
    let circle = Circle::New(SketchPoint::New(10.0, 20.0), 5.0);
    assert!((circle.Circumference() - 2.0 * std::f64::consts::PI * 5.0).abs() < 1e-9);
    assert!((SketchGeometry::CircleCurvature(&circle) - 0.2).abs() < 1e-9);

    let scaled_c = SketchGeometry::ScaleCircle(circle, SketchPoint::New(0.0, 0.0), 2.0);
    assert_eq!(scaled_c._Radius, 10.0);
    assert_eq!(scaled_c._Center, SketchPoint::New(20.0, 40.0));

    // 3. Circular Arc
    let arc = CircularArc::New(SketchPoint::New(0.0, 0.0), 10.0, 0.0, std::f64::consts::PI);
    assert_eq!(arc.StartPoint(), SketchPoint::New(10.0, 0.0));
    assert!((arc.EndPoint()._X - (-10.0)).abs() < 1e-9);
    assert!((arc.Midpoint()._Y - 10.0).abs() < 1e-9);
    assert!((SketchGeometry::ArcSagitta(&arc) - 10.0).abs() < 1e-9);

    // 4. Ellipse
    let ellipse = Ellipse::New(SketchPoint::New(0.0, 0.0), 20.0, 10.0, 0.0);
    let k_max = SketchGeometry::EllipseMaxCurvature(&ellipse); // a / b^2 = 20 / 100 = 0.2
    let k_min = SketchGeometry::EllipseMinCurvature(&ellipse); // b / a^2 = 10 / 400 = 0.025
    assert!((k_max - 0.2).abs() < 1e-9);
    assert!((k_min - 0.025).abs() < 1e-9);
});

//-------------------------------------------------------------------------------------------------
// Planar Sketch Distance Queries and Snap Candidate Extraction
jeeves_test!(Fleck, SketchQueryDistanceAndSnapping, |_ctx| {
    use crate::fenst::sketch::{Circle, CircularArc, LineSegment, SketchPoint, SketchPrimitive};
    use crate::silo::Stash;

    // 1. Distance to Line
    let line = LineSegment::New(SketchPoint::New(0.0, 0.0), SketchPoint::New(100.0, 0.0));
    assert_eq!(SketchQuery::PointToLineDistance(SketchPoint::New(50.0, 10.0), &line),
               10.0);
    assert_eq!(SketchQuery::PointToLineDistance(SketchPoint::New(-10.0, 0.0), &line),
               10.0);

    // 2. Distance to Circle
    let circle = Circle::New(SketchPoint::New(0.0, 0.0), 20.0);
    assert_eq!(SketchQuery::PointToCircleDistance(SketchPoint::New(25.0, 0.0), &circle),
               5.0);
    assert_eq!(SketchQuery::PointToCircleDistance(SketchPoint::New(15.0, 0.0), &circle),
               5.0);

    // 3. Distance to Arc
    let arc = CircularArc::New(SketchPoint::New(0.0, 0.0),
                               20.0,
                               0.0,
                               std::f64::consts::FRAC_PI_2);
    // Point within sweep cone (e.g. angle pi/4)
    let p_in = SketchPoint::New(20.0, 20.0); // distance to center is sqrt(800) ~ 28.28, radius is 20 -> dist ~ 8.28
    let d_in = SketchQuery::PointToArcDistance(p_in, &arc);
    assert!((d_in - (p_in.DistanceTo(SketchPoint::ORIGIN) - 20.0)).abs() < 1e-9);

    // Point outside sweep cone (e.g. at (-10, 0)) -> closest to arc start (20, 0)
    let p_out = SketchPoint::New(30.0, -10.0);
    let d_out = SketchQuery::PointToArcDistance(p_out, &arc);
    assert_eq!(d_out, p_out.DistanceTo(arc.StartPoint()));

    // 4. Snap Candidate Extraction
    let mut candidates = Stash::New();
    SketchQuery::CollectSnapCandidates(&SketchPrimitive::Line(line),
                                       1,
                                       crate::silo::Arr::Empty(),
                                       &mut candidates);
    assert_eq!(candidates.Size(), 3); // start, end, midpoint
});

//-------------------------------------------------------------------------------------------------
// Adaptive Curve Tessellation with Error Bounds
jeeves_test!(Fleck, SketchTessellateAdaptiveCurveApproximation, |_ctx| {
    use crate::fenst::sketch::{Circle, CircularArc, Ellipse, SketchPoint};

    // 1. Circle Tessellation
    let circle = Circle::New(SketchPoint::New(0.0, 0.0), 100.0);
    let poly = SketchTessellate::TessellateCircle(&circle, 0.5, 1.0);
    assert!(poly.Size() >= 16);
    // First and last point must coincide to close polygon
    assert_eq!(poly[0], poly[poly.Size() - 1]);

    // 2. Arc Tessellation
    let arc = CircularArc::New(SketchPoint::New(0.0, 0.0), 50.0, 0.0, std::f64::consts::PI);
    let arc_poly = SketchTessellate::TessellateArc(&arc, 0.5, 1.0);
    assert!(arc_poly.Size() >= 4);
    assert!((arc_poly[0]._X - 50.0).abs() < 1e-9);
    assert!((arc_poly[arc_poly.Size() - 1]._X - (-50.0)).abs() < 1e-9);

    // 3. Ellipse Tessellation
    let ellipse = Ellipse::New(SketchPoint::New(0.0, 0.0), 80.0, 20.0, 0.0);
    let ellipse_poly = SketchTessellate::TessellateEllipse(&ellipse, 0.5, 1.0);
    assert!(ellipse_poly.Size() >= 16);
    assert_eq!(ellipse_poly[0], ellipse_poly[ellipse_poly.Size() - 1]);
});

//-------------------------------------------------------------------------------------------------
// Planar Sketch IO Roundtrip Serialization and SVG Export
jeeves_test!(Fleck, SketchIoSerializationAndSvgExport, |_ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPoint};

    let mut doc = SketchDocument::New();
    doc.SetUnits("mm");
    doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(100.0, 50.0))
       .unwrap();
    doc.AddCircle(SketchPoint::New(200.0, 100.0), 30.0).unwrap();
    doc.AddArc(SketchPoint::New(300.0, 100.0),
               40.0,
               0.0,
               std::f64::consts::FRAC_PI_2)
       .unwrap();
    doc.AddEllipse(SketchPoint::New(400.0, 100.0), 50.0, 25.0, 0.5)
       .unwrap();

    // 1. Serialization
    let serialized = SketchIo::SerializeSketch(&doc);
    assert!(serialized.starts_with("TSKETCH 1.0"));
    assert!(serialized.contains("UNITS mm"));
    assert!(serialized.contains("Line"));
    assert!(serialized.contains("Circle"));
    assert!(serialized.contains("Arc"));
    assert!(serialized.contains("Ellipse"));
    assert!(serialized.ends_with("END\n"));

    // 2. Deserialization / Roundtrip
    let parsed = SketchIo::ParseSketch(&serialized).expect("parse serialized sketch");
    assert_eq!(parsed.Units(), "mm");
    assert_eq!(parsed.EntityCount(), 4);

    // 3. SVG Export
    let svg = SketchIo::ExportSvg(&doc);
    assert!(svg.starts_with("<svg xmlns="));
    assert!(svg.contains("<line "));
    assert!(svg.contains("<circle "));
    assert!(svg.contains("<path "));
    assert!(svg.contains("<ellipse "));
    assert!(svg.ends_with("</svg>\n"));
});

//-------------------------------------------------------------------------------------------------
// 3D Workplane Spatial Transforms, Presets, and Ray Casting
jeeves_test!(Fleck, SketchPlaneSpatialTransformsAndProjections, |_ctx| {
    use crate::fenst::sketch::{SketchPlane, SketchPoint};

    // 1. Presets and Orthonormality
    let xy = SketchPlane::Xy();
    assert_eq!(xy.PresetName(), "XY (Front)");
    assert_eq!(xy.Normal(), [0.0, 0.0, 1.0]);

    let xz = SketchPlane::Xz();
    assert_eq!(xz.PresetName(), "XZ (Top)");
    assert_eq!(xz.Normal(), [0.0, 1.0, 0.0]);

    let yz = SketchPlane::Yz();
    assert_eq!(yz.PresetName(), "YZ (Right)");
    assert_eq!(yz.Normal(), [1.0, 0.0, 0.0]);

    // 2. Bidirectional Coordinate Mapping (u, v) <-> (x, y, z)
    let p2d = SketchPoint::New(15.0, -25.0);
    let p3d = xz.ProjectTo3d(p2d);
    let recovered = xz.ProjectFrom3d(p3d);
    assert!((recovered._X - p2d._X).abs() < 1e-9);
    assert!((recovered._Y - p2d._Y).abs() < 1e-9);

    // 3. Plane Offsetting
    let offset_xy = xy.WithOffset(50.0);
    assert_eq!(offset_xy.Origin(), [0.0, 0.0, 50.0]);
    let p_off = offset_xy.ProjectTo3d(p2d);
    assert_eq!(p_off[2], 50.0);

    // 4. Ray Intersection / Unprojection
    let ray_origin = [15.0, -25.0, 100.0];
    let ray_dir = [0.0, 0.0, -1.0];
    let hit = xy.RayIntersection(ray_origin, ray_dir).expect("hit plane");
    assert!((hit._X - 15.0).abs() < 1e-9);
    assert!((hit._Y - (-25.0)).abs() < 1e-9);

    // Parallel ray returns None
    let parallel_dir = [1.0, 0.0, 0.0];
    assert!(xy.RayIntersection(ray_origin, parallel_dir).is_none());

    // 5. Default options contract
    let _ = Sketch3dProjectionOptions::default();
});

//-------------------------------------------------------------------------------------------------
// 3D Wireframe Projection and Datum Triad Generation
jeeves_test!(Fleck, Sketch3dProjectionWireframeAsset, |_ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPlane, SketchPoint};

    let mut doc = SketchDocument::New();
    doc.SetPlane(SketchPlane::Xz());
    doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(50.0, 20.0)).unwrap();
    doc.AddCircle(SketchPoint::New(100.0, 100.0), 25.0).unwrap();

    let options = Sketch3dProjectionOptions::default();
    let asset = Sketch3dProjection::ProjectDocument(&doc, options).expect("project wireframe");

    assert!(asset.VertexCount() > 0);
    assert!(asset.Edges().Size() > 0);
    let (min_bound, max_bound) = asset.Bounds();
    assert!(max_bound[0] >= min_bound[0]);
    assert!(max_bound[1] >= min_bound[1]);
    assert!(max_bound[2] >= min_bound[2]);
});

//-------------------------------------------------------------------------------------------------
// 3D Solid Extrude Engine with Ear-Clipping Triangulation
jeeves_test!(Fleck, Sketch3dSolidExtrudeWatertightAsset, |_ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPoint};

    let mut doc = SketchDocument::New();
    // Add a closed rectangle: (0,0) -> (40,0) -> (40,30) -> (0,30) -> (0,0)
    doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(40.0, 0.0)).unwrap();
    doc.AddLine(SketchPoint::New(40.0, 0.0), SketchPoint::New(40.0, 30.0)).unwrap();
    doc.AddLine(SketchPoint::New(40.0, 30.0), SketchPoint::New(0.0, 30.0)).unwrap();
    doc.AddLine(SketchPoint::New(0.0, 30.0), SketchPoint::New(0.0, 0.0)).unwrap();

    let extrude_depth = 15.0;
    let asset = SketchExtrude::ExtrudeDocument(&doc, extrude_depth).expect("extrude rectangle");

    // Must have vertices, triangles (faces), and edges
    assert!(asset.VertexCount() > 0);
    assert!(asset.Triangles().Size() > 0);
    assert!(asset.FaceCount() > 0);
    assert!(asset.Edges().Size() > 0);

    let (min_bound, max_bound) = asset.Bounds();
    assert!(max_bound[0] >= min_bound[0]);
    assert!(max_bound[1] >= min_bound[1]);
    assert!(max_bound[2] >= min_bound[2]);
});

//-------------------------------------------------------------------------------------------------
// 3D Solid Revolve / Lathe Engine with Rotational Meshing and End-Caps
jeeves_test!(Fleck, Sketch3dSolidRevolveWatertightAsset, |_ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPoint};

    let mut doc = SketchDocument::New();
    // Add a closed profile offset from spindle Y-axis: (10, 0) -> (25, 0) -> (25, 40) -> (10, 40) -> (10, 0)
    doc.AddLine(SketchPoint::New(10.0, 0.0), SketchPoint::New(25.0, 0.0)).unwrap();
    doc.AddLine(SketchPoint::New(25.0, 0.0), SketchPoint::New(25.0, 40.0)).unwrap();
    doc.AddLine(SketchPoint::New(25.0, 40.0), SketchPoint::New(10.0, 40.0)).unwrap();
    doc.AddLine(SketchPoint::New(10.0, 40.0), SketchPoint::New(10.0, 0.0)).unwrap();

    // 1. Full 360 degree lathe revolution
    let options_full = SketchRevolveOptions {
        axis: RevolveAxis::Y,
        angle_deg: 360.0,
        segments: 16,
    };
    let asset_full = SketchRevolve::RevolveDocument(&doc, options_full).expect("revolve full");
    assert!(asset_full.VertexCount() > 0);
    assert!(asset_full.Triangles().Size() > 0);
    assert!(asset_full.Edges().Size() > 0);

    let (min_bound, max_bound) = asset_full.Bounds();
    assert!(max_bound[0] >= min_bound[0]);
    assert!(max_bound[1] >= min_bound[1]);
    assert!(max_bound[2] >= min_bound[2]);

    // 2. Partial 180 degree revolution with planar end caps
    let options_partial = SketchRevolveOptions {
        axis: RevolveAxis::Y,
        angle_deg: 180.0,
        segments: 16,
    };
    let asset_partial = SketchRevolve::RevolveDocument(&doc, options_partial).expect("revolve partial");
    assert!(asset_partial.VertexCount() > 0);
    assert!(asset_partial.Triangles().Size() > 0);
    assert!(asset_partial.FaceCount() > 0);

    // 3. Spindle-touching profile with non-degenerate faces (F08)
    let mut doc_touching = SketchDocument::New();
    doc_touching.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(20.0, 0.0)).unwrap();
    doc_touching.AddLine(SketchPoint::New(20.0, 0.0), SketchPoint::New(20.0, 30.0)).unwrap();
    doc_touching.AddLine(SketchPoint::New(20.0, 30.0), SketchPoint::New(0.0, 30.0)).unwrap();
    doc_touching.AddLine(SketchPoint::New(0.0, 30.0), SketchPoint::New(0.0, 0.0)).unwrap();
    let asset_touching = SketchRevolve::RevolveDocument(&doc_touching, options_full).expect("revolve touching");
    assert!(asset_touching.VertexCount() > 0);
    assert!(asset_touching.Triangles().Size() > 0);

    // 4. Axis-crossing profile rejection (F09)
    let mut doc_crossing = SketchDocument::New();
    doc_crossing.AddLine(SketchPoint::New(-10.0, 0.0), SketchPoint::New(10.0, 0.0)).unwrap();
    doc_crossing.AddLine(SketchPoint::New(10.0, 0.0), SketchPoint::New(10.0, 20.0)).unwrap();
    doc_crossing.AddLine(SketchPoint::New(10.0, 20.0), SketchPoint::New(-10.0, 20.0)).unwrap();
    doc_crossing.AddLine(SketchPoint::New(-10.0, 20.0), SketchPoint::New(-10.0, 0.0)).unwrap();
    assert!(SketchRevolve::RevolveDocument(&doc_crossing, options_full).is_err());
});

//-------------------------------------------------------------------------------------------------
// Engineering Dimension Serialization Round-Trip in Flux (.flx)
jeeves_test!(Fleck, SketchDimensionFluxRoundTrip, |_ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPoint};
    use crate::fenst::sketch_dimension::{DimensionKind, SketchDimension};

    let mut doc = SketchDocument::New();
    let l_id = doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(100.0, 0.0)).unwrap();

    let dim1 = SketchDimension::New(
        1,
        DimensionKind::LinearAligned,
        SketchPoint::New(0.0, 0.0),
        SketchPoint::New(100.0, 0.0),
        15.0,
        l_id,
    );
    let dim2 = SketchDimension::New(
        2,
        DimensionKind::Radial,
        SketchPoint::New(50.0, 50.0),
        SketchPoint::New(75.0, 50.0),
        0.0,
        u32::MAX,
    );
    doc.AddDimension(dim1);
    doc.AddDimension(dim2);
    assert_eq!(doc.DimensionCount(), 2);

    let file = SketchFlux::File(&doc, None).expect("create flux file with dimensions");
    let decoded_doc = SketchFlux::Document(&file).expect("decode flux file with dimensions");
    assert_eq!(decoded_doc.DimensionCount(), 2);

    let decoded_dim1 = decoded_doc.FindDimension(1).expect("find dim 1");
    assert_eq!(decoded_dim1._Kind, DimensionKind::LinearAligned);
    assert!((decoded_dim1.Value() - 100.0).abs() < 1e-6);
    assert_eq!(decoded_dim1._TargetEntityId, l_id);

    let decoded_dim2 = decoded_doc.FindDimension(2).expect("find dim 2");
    assert_eq!(decoded_dim2._Kind, DimensionKind::Radial);
    assert!((decoded_dim2.Value() - 25.0).abs() < 1e-6);
});

//-------------------------------------------------------------------------------------------------
// SVG Dimension Export with XML Escaping and Annotation Bounds
jeeves_test!(Fleck, SketchSvgExportDimensionEscapingAndBounds, |_ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPoint};
    use crate::fenst::sketch_dimension::{DimensionKind, SketchDimension};

    let mut doc = SketchDocument::New();
    doc.SetUnits("mm & inches <std>");
    doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(100.0, 0.0)).unwrap();

    let dim = SketchDimension::New(
        1,
        DimensionKind::LinearAligned,
        SketchPoint::New(0.0, 0.0),
        SketchPoint::New(100.0, 0.0),
        25.0,
        1,
    );
    doc.AddDimension(dim);

    let svg = SketchIo::ExportSvg(&doc);
    assert!(svg.contains("&amp;"));
    assert!(svg.contains("&lt;"));
    assert!(svg.contains("&gt;"));
    assert!(!svg.contains("mm & inches <std>"));
    let bounds = doc.ComputeBounds();
    assert!(bounds.Height() >= 25.0);
});

//-------------------------------------------------------------------------------------------------
// Strict Dimension Deserialization Validation (R05)
jeeves_test!(Fleck, SketchDimensionStrictFluxValidation, |_ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPoint};
    use crate::fenst::sketch_dimension::{DimensionKind, SketchDimension};
    use crate::flux::{FluxFile, FluxValue};
    use crate::silo::{Buff, IArr, Stash};

    let mut doc = SketchDocument::New();
    let l_id = doc.AddLine(SketchPoint::New(0.0, 0.0), SketchPoint::New(100.0, 0.0)).unwrap();
    let dim = SketchDimension::New(1, DimensionKind::LinearAligned, SketchPoint::New(0.0, 0.0), SketchPoint::New(100.0, 0.0), 15.0, l_id);
    doc.AddDimension(dim);

    let file = SketchFlux::File(&doc, None).unwrap();
    let meta = file.Metadata().clone();

    // Helper to mutate root fields
    let mutate_root = |key: &str, new_val: FluxValue| -> FluxFile {
        let FluxValue::Object(items) = file.Payload() else { panic!() };
        let mut mutated = Stash::New();
        items.Arr().Traverse(|(k, v)| {
            if k == key {
                mutated.Push((k.clone(), new_val.clone()));
            } else {
                mutated.Push((k.clone(), v.clone()));
            }
        });
        FluxFile::FromParts(meta.clone(), FluxValue::Object(mutated.IntoBuff())).unwrap()
    };

    // 1. Non-array Dimensions payload is rejected
    let bad1 = mutate_root("Dimensions", FluxValue::String("not an array".into()));
    assert!(SketchFlux::Document(&bad1).is_err());

    // 2. Unknown dimension kind is rejected
    let make_dim_obj = |kind: &str, p1: [f64; 2], target: u64, id: u64| -> FluxValue {
        FluxValue::Object(Buff::FromDispenser(6, |i| match i {
            0 => ("Id".into(), FluxValue::U64(id)),
            1 => ("Kind".into(), FluxValue::String(kind.into())),
            2 => ("P1".into(), FluxValue::Array(Buff::FromDispenser(2, |j| FluxValue::F64(p1[j as usize])))),
            3 => ("P2".into(), FluxValue::Array(Buff::FromDispenser(2, |j| FluxValue::F64([100.0, 0.0][j as usize])))),
            4 => ("Offset".into(), FluxValue::F64(15.0)),
            _ => ("Target".into(), FluxValue::U64(target)),
        }))
    };

    let bad2 = mutate_root("Dimensions", FluxValue::Array(Buff::FromDispenser(1, |_| make_dim_obj("SplineDim", [0.0, 0.0], l_id as u64, 1))));
    assert!(SketchFlux::Document(&bad2).is_err());

    // 3. Nonfinite coordinates rejected
    let bad3 = mutate_root("Dimensions", FluxValue::Array(Buff::FromDispenser(1, |_| make_dim_obj("LinearAligned", [f64::NAN, 0.0], l_id as u64, 1))));
    assert!(SketchFlux::Document(&bad3).is_err());

    // 4. Dangling target entity ID rejected
    let bad4 = mutate_root("Dimensions", FluxValue::Array(Buff::FromDispenser(1, |_| make_dim_obj("LinearAligned", [0.0, 0.0], 9999, 1))));
    assert!(SketchFlux::Document(&bad4).is_err());

    // 5. Duplicate dimension ID rejected
    let bad5 = mutate_root("Dimensions", FluxValue::Array(Buff::FromDispenser(2, |_| make_dim_obj("LinearAligned", [0.0, 0.0], l_id as u64, 1))));
    assert!(SketchFlux::Document(&bad5).is_err());
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, GeometryOriginRebasingPreservesSmallFeatures, |_ctx| {
    use crate::fenst::sketch::{SketchPlane, SketchPoint};
    use crate::silo::{Buff, Stash};

    // Sub-millimeter triangle situated 1 billion units from origin
    let origin_offset = 1_000_000_000.0;
    let mut positions = Stash::New();
    positions.Push([origin_offset, origin_offset, 0.0]);
    positions.Push([origin_offset + 0.005, origin_offset, 0.0]);
    positions.Push([origin_offset, origin_offset + 0.005, 0.0]);

    let mut colors = Stash::New();
    colors.Push([1.0, 1.0, 1.0, 1.0]);
    colors.Push([1.0, 1.0, 1.0, 1.0]);
    colors.Push([1.0, 1.0, 1.0, 1.0]);

    let mut triangles = Stash::New();
    triangles.Push([0, 1, 2]);

    let bounds = (
        [origin_offset, origin_offset, 0.0],
        [origin_offset + 0.005, origin_offset + 0.005, 0.0],
    );

    let geom = GeometryAsset::FromMeshF64(
        positions.Arr(),
        colors.Arr(),
        triangles.IntoBuff(),
        Buff::New(),
        Buff::New(),
        bounds,
    ).unwrap();

    assert_eq!(geom.VertexCount(), 3);
    assert_eq!(geom.WorldBounds(), Some(bounds));

    // Normalized positions must retain non-zero relative separation
    let v0 = geom.Vertices()[0].Position();
    let v1 = geom.Vertices()[1].Position();
    let dx = (v1[0] - v0[0]).abs();
    assert!(dx > 0.01, "Sub-meter features must not collapse to 0.0 in local normalized space, got dx={dx}");

    // Also verify SketchExtrude on a high-origin workplane
    let mut plane = SketchPlane::Xy();
    plane._Origin = [1_000_000_000.0, 500_000_000.0, 0.0];
    let mut profile = Stash::New();
    profile.Push(SketchPoint::New(0.0, 0.0));
    profile.Push(SketchPoint::New(10.0, 0.0));
    profile.Push(SketchPoint::New(10.0, 10.0));
    profile.Push(SketchPoint::New(0.0, 10.0));

    let solid = SketchExtrude::ExtrudeProfile(profile.Arr(), &plane, 5.0).unwrap();
    let world_b = solid.WorldBounds().unwrap();
    assert!((world_b.0[0] - 1_000_000_000.0).abs() < 1e-3);
    assert!((world_b.1[0] - 1_000_000_010.0).abs() < 1e-3);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, SketchRevolveNegativeSidePreservesSection, |_ctx| {
    use crate::fenst::sketch::{SketchPlane, SketchPoint};
    use crate::silo::Stash;

    let plane = SketchPlane::Xy();
    // Profile strictly in negative X: X in [-10, -5], Y in [0, 4]
    let mut profile = Stash::New();
    profile.Push(SketchPoint::New(-10.0, 0.0));
    profile.Push(SketchPoint::New(-5.0, 0.0));
    profile.Push(SketchPoint::New(-5.0, 4.0));
    profile.Push(SketchPoint::New(-10.0, 4.0));

    let options = SketchRevolveOptions {
        axis: RevolveAxis::Y,
        angle_deg: 90.0,
        segments: 16,
    };

    let solid = SketchRevolve::RevolveProfile(profile.Arr(), &plane, options).unwrap();
    let wb = solid.WorldBounds().unwrap();

    // At phi = 0, section is at [-10, -5]. Under 90 deg rotation around Y, X sweeps towards 0, Z sweeps into -Z.
    // Ensure min X starts at -10 and does not get mirrored to positive (+5, +10) at phi=0
    assert!(wb.0[0] <= -10.0 + 1e-4);
    assert!(solid.FaceCount() > 0);
    assert!(solid.VertexCount() >= 16);
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, SketchTessellateAdheresToErrorBoundForLargeCurves, |_ctx| {
    use crate::fenst::sketch::{Circle, CircularArc, SketchPoint};
    use crate::silo::IArr;

    // Extremely large circle at high zoom (sagitta ratio delta / R_screen < 1e-6)
    let huge_circle = Circle::New(SketchPoint::New(0.0, 0.0), 10_000_000.0);
    let pts = SketchTessellate::TessellateCircle(&huge_circle, 0.25, 10.0);

    // Old behavior fell back to PI / 6.0 (16 segments).
    // New behavior must compute stable Taylor half_step and clamp up to 2048 segments (+1 closure point = 2049)
    assert_eq!(pts.Size(), 2049);
    pts.Arr().Traverse(|p| {
        assert!(p._X.is_finite() && p._Y.is_finite());
        let dist = (p._X * p._X + p._Y * p._Y).sqrt();
        assert!((dist - 10_000_000.0).abs() < 1e-3);
    });

    // Large circular arc
    let huge_arc = CircularArc::New(SketchPoint::New(0.0, 0.0), 10_000_000.0, 0.0, std::f64::consts::PI * 0.5);
    let arc_pts = SketchTessellate::TessellateArc(&huge_arc, 0.25, 10.0);
    assert!(arc_pts.Size() > 100);
    arc_pts.Arr().Traverse(|p| {
        assert!(p._X.is_finite() && p._Y.is_finite());
    });
});

//-------------------------------------------------------------------------------------------------

jeeves_test!(Fleck, SketchFluxLargeWorkloadScaling, |_ctx| {
    use crate::fenst::sketch::{SketchDocument, SketchPoint};
    use crate::flux::file::{FluxFile, FluxFileFormat};

    let mut doc = SketchDocument::New();
    let count = 10_000u32;
    let mut i = 0u32;
    while i < count {
        let x = (i as f64) * 0.1;
        doc.AddLine(SketchPoint::New(x, 0.0), SketchPoint::New(x, 10.0)).unwrap();
        i += 1;
    }
    assert_eq!(doc.EntityCount(), count);

    // 1. Single-pass export with zero duplicate payload tree construction
    let file = SketchFlux::File(&doc, None).expect("create file for 10k entities");

    // 2. Binary FLX round-trip (verifies MAX_VALUES = 2,000,000 capacity)
    let bin_bytes = file.Encode(FluxFileFormat::Binary).expect("encode binary flx");
    assert!(bin_bytes.Size() > 0);
    let decoded_bin_file = FluxFile::Decode(bin_bytes.Arr()).expect("decode binary flx");
    let decoded_doc = SketchFlux::Document(&decoded_bin_file).expect("restore sketch from binary flx");
    assert_eq!(decoded_doc.EntityCount(), count);

    // 3. Text TFLX round-trip
    let text_bytes = file.Encode(FluxFileFormat::Text).expect("encode text tflx");
    assert!(text_bytes.Size() > 0);
    let decoded_text_file = FluxFile::Decode(text_bytes.Arr()).expect("decode text tflx");
    let decoded_text_doc = SketchFlux::Document(&decoded_text_file).expect("restore sketch from text tflx");
    assert_eq!(decoded_text_doc.EntityCount(), count);
});

//-------------------------------------------------------------------------------------------------
