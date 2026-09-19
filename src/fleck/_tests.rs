//-- _tests.rs ----------------------------------------------------------------------------------------------------------------------
use crate::{
    Buff,
    fleck::{
        BBox3f, Dir3f, ParseWaveObj, ParseWaveObjBytes, ParseWaveObjStream, Pt3f, WPt2f, WPt3f,
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
    let cloudFromBytes = ParsePtsBytes(bytes).unwrap();
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
    let model = ParseWaveObjBytes(objData).unwrap();
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
    assert_eq!(v3.AsArray(), &[10.0, 20.0, 30.0]);
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
    let sumRefRef = &a + &b;
    assert_eq!(sumRefRef._Data, [5.0, 7.0, 9.0]);
    // Ref + Val
    let sumRefVal = &a + b;
    assert_eq!(sumRefVal._Data, [5.0, 7.0, 9.0]);
    // Val + Ref
    let sumValRef = a + &b;
    assert_eq!(sumValRef._Data, [5.0, 7.0, 9.0]);
    // Subtraction
    let diffRefRef = &b - &a;
    assert_eq!(diffRefRef._Data, [3.0, 3.0, 3.0]);
    let diffValVal = b - a;
    assert_eq!(diffValVal._Data, [3.0, 3.0, 3.0]);
    // Negation
    let negVal = -a;
    assert_eq!(negVal._Data, [-1.0, -2.0, -3.0]);
    let negRef = -&a;
    assert_eq!(negRef._Data, [-1.0, -2.0, -3.0]);
    // Hadamard Multiplication
    let hadamard = &a * &b;
    assert_eq!(hadamard._Data, [4.0, 10.0, 18.0]);
    // Component-wise Division
    let quotient = &b / &a;
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
    assert_eq!((&vf * sf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((&vf * &sf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((vf * &sf)._Data, [5.0, 7.5, 10.0]);
    // Scalar * Vector
    assert_eq!((sf * vf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((&sf * &vf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((&sf * vf)._Data, [5.0, 7.5, 10.0]);
    assert_eq!((sf * &vf)._Data, [5.0, 7.5, 10.0]);
    // Vector / Scalar
    assert_eq!((vf / 2.0f32)._Data, [1.0, 1.5, 2.0]);
    assert_eq!((&vf / 2.0f32)._Data, [1.0, 1.5, 2.0]);
    assert_eq!((&vf / &2.0f32)._Data, [1.0, 1.5, 2.0]);
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
    assert_eq!(dot, 10 * 1 + 20 * 2 + 30 * 3);
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
    let points = [
        [-10.0f32, 5.0, 20.0],
        [30.0, -15.0, 40.0],
        [0.0, 25.0, -5.0],
    ];
    let bbox = BBox3f::FromPoints(&points);
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

//---------------------------------------------------------------------------------------------------------------------------------
