//-- waveobjio.rs ------------------------------------------------------------------------------------------------------------------
use	std::fmt;
use	std::collections::HashSet;
use	crate::{
    fleck::
    { BBox3f, Dir3f, Pt3f, PtsPointsDto, WaveObjMeshDto, WPt2f, WPt3f },
    flux::instream::
    { FixedStream, IStream },
    shard::
    { Charset, IGrammar, Int, Parser, Real },
    silo::
    { Arr, Buff, Stash, traits::IArr },
    ShardTree,
};

//---------------------------------------------------------------------------------------------------------------------------------
/// Represents an index reference into vertex, texture coordinate, and normal buffers for a polygon face.
#[derive( Clone, Copy, Debug, PartialEq, Eq)]
#[derive(Default)]
pub struct FaceVertex
{
    pub _VertexIdx:   u32,
    pub _TexCoordIdx: Option< u32>,
    pub _NormalIdx:   Option< u32>,
}

//---------------------------------------------------------------------------------------------------------------------------------

impl FaceVertex
{
    pub fn	New( vertexIdx: u32) -> Self
    {
        Self {
            _VertexIdx:   vertexIdx,
            _TexCoordIdx: None,
            _NormalIdx:   None,
        }
    }
    pub fn	WithTexCoord( vertexIdx: u32, texCoordIdx: u32) -> Self
    {
        Self {
            _VertexIdx:   vertexIdx,
            _TexCoordIdx: Some( texCoordIdx),
            _NormalIdx:   None,
        }
    }
    pub fn	WithNormal( vertexIdx: u32, normalIdx: u32) -> Self
    {
        Self {
            _VertexIdx:   vertexIdx,
            _TexCoordIdx: None,
            _NormalIdx:   Some( normalIdx),
        }
    }
    pub fn	Full( vertexIdx: u32, texCoordIdx: u32, normalIdx: u32) -> Self
    {
        Self {
            _VertexIdx:   vertexIdx,
            _TexCoordIdx: Some( texCoordIdx),
            _NormalIdx:   Some( normalIdx),
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
/// Represents a polygonal face containing a list of face vertices.
#[derive( Clone, Debug, PartialEq)]
pub struct Face
{
    pub _Vertices: Buff< FaceVertex>,
}
impl Default for Face {
    fn	default() -> Self
    {
        Self::New()
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl Face
{
    pub fn	New() -> Self
    {
        Self {
            _Vertices: Buff::New(),
        }
    }
    pub fn	Push( &mut self, vert: FaceVertex)
    {
        let  	mut stash = Stash::WithCapacity( self._Vertices.Size() + 1);
        self._Vertices.Arr().Traverse( |&v| {
            stash.Push( v);
        });
        stash.Push( vert);
        self._Vertices = stash.IntoBuff();
    }
    pub fn	Len( &self) -> usize
    {
        self._Vertices.Size() as usize
    }
    pub fn	IsEmpty( &self) -> bool
    {
        self._Vertices.IsEmpty()
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
/// Represents a parsed Wavefront .obj 3D model.
#[derive( Clone)]
pub struct WaveObjModel
{
    pub _Vertices:    Buff< WPt3f>,
    pub _TexCoords:   Buff< WPt2f>,
    pub _Normals:     Buff< Dir3f>,
    pub _Faces:       Buff< Face>,
    pub _Objects:     Buff< String>,
    pub _Groups:      Buff< String>,
    pub _MtlLibs:     Buff< String>,
    pub _UseMtls:     Buff< String>,
}

//---------------------------------------------------------------------------------------------------------------------------------

impl Default for WaveObjModel {
    fn	default() -> Self
    {
        Self::New()
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl WaveObjModel
{
    pub fn	New() -> Self
    {
        Self {
            _Vertices:    Buff::New(),
            _TexCoords:   Buff::New(),
            _Normals:     Buff::New(),
            _Faces:       Buff::New(),
            _Objects:     Buff::New(),
            _Groups:      Buff::New(),
            _MtlLibs:     Buff::New(),
            _UseMtls:     Buff::New(),
        }
    }
    pub( crate) fn	Create( ctx: WaveObjParserCtx) -> Self
    {
        Self {
            _Vertices:    ctx._VStash.IntoBuff(),
            _TexCoords:   ctx._VtStash.IntoBuff(),
            _Normals:     ctx._VnStash.IntoBuff(),
            _Faces:       ctx._FStash.IntoBuff(),
            _Objects:     ctx._ObjStash.IntoBuff(),
            _Groups:      ctx._GrpStash.IntoBuff(),
            _MtlLibs:     ctx._MtlStash.IntoBuff(),
            _UseMtls:     ctx._UseMtlStash.IntoBuff(),
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn	VertexCount( &self) -> u32
    {
        self._Vertices.Size()
    }
    pub fn	FaceCount( &self) -> u32
    {
        self._Faces.Size()
    }
    pub fn	NormalCount( &self) -> u32
    {
        self._Normals.Size()
    }
    pub fn	TexCoordCount( &self) -> u32
    {
        self._TexCoords.Size()
    }

    //-----------------------------------------------------------------------------------------------------------------------------

    pub fn	BoundingBox( &self) -> ( [f32; 3], [f32; 3])
    {
        if self._Vertices.IsEmpty() {
            return ( [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        }
        let  	arr = self._Vertices.Arr();
        let  	mut bbox = BBox3f::Empty();
        arr.Traverse( |v| {
            bbox.Extend( Pt3f::New( v._X, v._Y, v._Z));
        });
        ( bbox.Min(), bbox.Max())
    }
    pub fn	BBox( &self) -> BBox3f
    {
        let  	( bboxMin, bboxMax) = self.BoundingBox();
        BBox3f::New( Pt3f::from( bboxMin), Pt3f::from( bboxMax))
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// Converts vertices to PtsPointsDto for seamless display with Fenst / Swarm.
    pub fn	ToDto( &self) -> PtsPointsDto
    {
        let  	( bboxMin, bboxMax) = self.BoundingBox();
        let  	totalPoints = self._Vertices.Size();
        let  	arr = self._Vertices.Arr();
        let  	pointsBuff = Buff::FromDispenser( totalPoints, |i| {
            let  	v = arr[i];
            [v._X, v._Y, v._Z]
        });
        PtsPointsDto {
            _Points:   pointsBuff,
            _Count:    totalPoints as usize,
            _BboxMin:  bboxMin,
            _BboxMax:  bboxMax,
        }
    }

    //-----------------------------------------------------------------------------------------------------------------------------
    /// Converts WaveObjModel to WaveObjMeshDto with vertices, triangulated indices, unique wireframe edges, and face normals.
    pub fn	ToMeshDto( &self) -> WaveObjMeshDto
    {
        let  	( bboxMin, bboxMax) = self.BoundingBox();
        let  	vertCount = self._Vertices.Size();
        let  	arr = self._Vertices.Arr();
        let  	pointsBuff = Buff::FromDispenser( vertCount, |i| {
            let  	v = arr[i];
            [v._X, v._Y, v._Z]
        });
        let  	numFaces = self._Faces.Size();
        let  	mut trianglesStash = Stash::< [u32; 3]>::WithCapacity( ( numFaces * 2).max( 16));
        let  	mut edgeSet = HashSet::new();
        let  	facesArr = self._Faces.Arr();
        for fIdx in 0..numFaces {
            let  	face = &facesArr[fIdx];
            let  	faceVertCount = face.Len();
            if faceVertCount >= 3 {
                let  	vertsArr = face._Vertices.Arr();
                let  	v0Raw = vertsArr[0]._VertexIdx;
                let  	v0 = if v0Raw > 0 { v0Raw - 1 } else { 0 };
                for i in 1..( faceVertCount - 1) {
                    let  	v1Raw = vertsArr[i as u32]._VertexIdx;
                    let  	v2Raw = vertsArr[( i + 1) as u32]._VertexIdx;
                    let  	v1 = if v1Raw > 0 { v1Raw - 1 } else { 0 };
                    let  	v2 = if v2Raw > 0 { v2Raw - 1 } else { 0 };
                    trianglesStash.Push( [v0, v1, v2]);
                }
                // Collect polygon boundary edges for wireframe
                for i in 0..faceVertCount {
                    let  	nextI = ( i + 1) % faceVertCount;
                    let  	eaRaw = vertsArr[i as u32]._VertexIdx;
                    let  	ebRaw = vertsArr[nextI as u32]._VertexIdx;
                    let  	ea = if eaRaw > 0 { eaRaw - 1 } else { 0 };
                    let  	eb = if ebRaw > 0 { ebRaw - 1 } else { 0 };
                    let  	edge = if ea < eb { ( ea, eb) } else { ( eb, ea) };
                    edgeSet.insert( edge);
                }
            }
        }
        let  	trianglesBuff = trianglesStash.IntoBuff();
        let  	mut edgesStash = Stash::< [u32; 2]>::WithCapacity( ( edgeSet.len() as u32).max( 16));
        for ( e0, e1) in edgeSet {
            edgesStash.Push( [e0, e1]);
        }
        let  	edgesBuff = edgesStash.IntoBuff();
        // Compute per-triangle face normals for lighting
        let  	triArr = trianglesBuff.Arr();
        let  	numTriangles = trianglesBuff.Size();
        let  	normalsBuff = Buff::FromDispenser( numTriangles, |i| {
            let  	tri = triArr[i];
            let  	p0Idx = tri[0];
            let  	p1Idx = tri[1];
            let  	p2Idx = tri[2];
            if p0Idx < vertCount && p1Idx < vertCount && p2Idx < vertCount {
                let  	p0 = arr[p0Idx];
                let  	p1 = arr[p1Idx];
                let  	p2 = arr[p2Idx];
                let  	ux = p1._X - p0._X;
                let  	uy = p1._Y - p0._Y;
                let  	uz = p1._Z - p0._Z;
                let  	vx = p2._X - p0._X;
                let  	vy = p2._Y - p0._Y;
                let  	vz = p2._Z - p0._Z;
                let  	nx = uy * vz - uz * vy;
                let  	ny = uz * vx - ux * vz;
                let  	nz = ux * vy - uy * vx;
                let  	len = ( nx * nx + ny * ny + nz * nz).sqrt();
                if len > 1e-6 {
                    [nx / len, ny / len, nz / len]
                } else {
                    [0.0, 1.0, 0.0]
                }
            } else {
                [0.0, 1.0, 0.0]
            }
        });
        WaveObjMeshDto {
            _Points:        pointsBuff,
            _Triangles:     trianglesBuff,
            _Edges:         edgesBuff,
            _Normals:       normalsBuff,
            _VertexCount:   vertCount as usize,
            _FaceCount:     numFaces as usize,
            _BboxMin:       bboxMin,
            _BboxMax:       bboxMax,
        }
    }
    pub fn	Triangulate( &self) -> Buff< [FaceVertex; 3]>
    {
        let  	numFaces = self._Faces.Size();
        let  	mut trianglesStash = Stash::< [FaceVertex; 3]>::WithCapacity( ( numFaces * 2).max( 16));
        let  	facesArr = self._Faces.Arr();
        for fIdx in 0..numFaces {
            let  	face = &facesArr[fIdx];
            let  	vertCount = face.Len();
            if vertCount >= 3 {
                let  	vertsArr = face._Vertices.Arr();
                let  	v0 = vertsArr[0];
                for i in 1..( vertCount - 1) {
                    let  	v1 = vertsArr[i as u32];
                    let  	v2 = vertsArr[( i + 1) as u32];
                    trianglesStash.Push( [v0, v1, v2]);
                }
            }
        }
        trianglesStash.IntoBuff()
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
//---------------------------------------------------------------------------------------------------------------------------------
/// Transient mutable state for parsing a Wavefront .obj stream.
/// Owns all `Stash` buffers that accumulate vertices, normals, faces, etc.
/// Constructed once per parse via `New(streamSz)` and consumed into `Buff` results.
pub( crate) struct WaveObjParserCtx
{
    _VStash:      Stash< WPt3f>,
    _VtStash:     Stash< WPt2f>,
    _VnStash:     Stash< Dir3f>,
    _FStash:      Stash< Face>,
    _ObjStash:    Stash< String>,
    _GrpStash:    Stash< String>,
    _MtlStash:    Stash< String>,
    _UseMtlStash: Stash< String>,
    _Vals:        Stash< f32>,
    _FaceVerts:   Stash< FaceVertex>,
    _CurFaceVert: FaceVertex,
}

//---------------------------------------------------------------------------------------------------------------------------------

impl WaveObjParserCtx
{
    fn	New( streamSz: u32) -> Self
    {
        let  	estVerts = ( streamSz / 40).max( 128);
        let  	estFaces = ( streamSz / 50).max( 128);
        Self {
            _VStash:      Stash::< WPt3f>::WithCapacity( estVerts),
            _VtStash:     Stash::< WPt2f>::WithCapacity( ( estVerts / 2).max( 64)),
            _VnStash:     Stash::< Dir3f>::WithCapacity( ( estVerts / 2).max( 64)),
            _FStash:      Stash::< Face>::WithCapacity( estFaces),
            _ObjStash:    Stash::< String>::New(),
            _GrpStash:    Stash::< String>::New(),
            _MtlStash:    Stash::< String>::New(),
            _UseMtlStash: Stash::< String>::New(),
            _Vals:        Stash::< f32>::WithCapacity( 4),
            _FaceVerts:   Stash::< FaceVertex>::WithCapacity( 4),
            _CurFaceVert: FaceVertex::default(),
        }
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive( Clone, Copy)]
struct WaveObjParserCtxMM( *mut WaveObjParserCtx);

//---------------------------------------------------------------------------------------------------------------------------------

impl WaveObjParserCtxMM
{
    #[inline( always)]
    #[allow( clippy::mut_from_ref)]
    fn	Get( &self) -> &mut WaveObjParserCtx
    {
        unsafe { &mut *self.0 }
    }
    #[inline( always)]
    fn	PushMtlLib( &self, arr: Arr< '_, u8>) -> bool
    {
        self.Get()._MtlStash.Push( arr.Str().to_string());
        true
    }
    #[inline( always)]
    fn	PushUseMtl( &self, arr: Arr< '_, u8>) -> bool
    {
        self.Get()._UseMtlStash.Push( arr.Str().to_string());
        true
    }
    #[inline( always)]
    fn	PushVal( &self, arr: Arr< '_, u8>) -> bool
    {
        self.Get()._Vals.Push( arr.Str().parse::< f32>().unwrap());
        true
    }
    #[inline( always)]
    fn	EndVt( &self) -> bool
    {
        let  	ctx = self.Get();
        let  	cnt = ctx._Vals.Size();
        if cnt >= 2 {
            let  	vals = ctx._Vals.Arr();
            let  	w = if cnt >= 3 { vals[2] } else { 0.0 };
            ctx._VtStash.Push( WPt2f::WithW( vals[0], vals[1], w));
        } else if cnt == 1 {
            ctx._VtStash.Push( WPt2f::New( ctx._Vals.Arr()[0], 0.0));
        }
        ctx._Vals.Clear();
        true
    }
    #[inline( always)]
    fn	EndVn( &self) -> bool
    {
        let  	ctx = self.Get();
        if ctx._Vals.Size() >= 3 {
            let  	vals = ctx._Vals.Arr();
            ctx._VnStash.Push( Dir3f::New( vals[0], vals[1], vals[2]));
        }
        ctx._Vals.Clear();
        true
    }
    #[inline( always)]
    fn	EndV( &self) -> bool
    {
        let  	ctx = self.Get();
        let  	cnt = ctx._Vals.Size();
        if cnt >= 3 {
            let  	vals = ctx._Vals.Arr();
            let  	w = if cnt >= 4 { vals[3] } else { 1.0 };
            ctx._VStash.Push( WPt3f::WithW( vals[0], vals[1], vals[2], w));
        }
        ctx._Vals.Clear();
        true
    }
    #[inline( always)]
    fn	ParseFaceV( &self, arr: Arr< '_, u8>) -> bool
    {
        let  	v = arr.Str().parse::< i32>().unwrap();
        let  	ctx = self.Get();
        let  	numV = ctx._VStash.Size() as i32;
        let  	idx = if v < 0 { numV + v + 1 } else { v };
        ctx._CurFaceVert._VertexIdx = idx.max( 0) as u32;
        true
    }
    #[inline( always)]
    fn	ParseFaceVt( &self, arr: Arr< '_, u8>) -> bool
    {
        if arr.Size() == 0 {
            return true;
        }
        let  	v = arr.Str().parse::< i32>().unwrap();
        let  	ctx = self.Get();
        let  	numT = ctx._VtStash.Size() as i32;
        let  	idx = if v < 0 { numT + v + 1 } else { v };
        ctx._CurFaceVert._TexCoordIdx = Some( idx.max( 0) as u32);
        true
    }
    #[inline( always)]
    fn	ParseFaceVn( &self, arr: Arr< '_, u8>) -> bool
    {
        let  	v = arr.Str().parse::< i32>().unwrap();
        let  	ctx = self.Get();
        let  	numN = ctx._VnStash.Size() as i32;
        let  	idx = if v < 0 { numN + v + 1 } else { v };
        ctx._CurFaceVert._NormalIdx = Some( idx.max( 0) as u32);
        true
    }
    #[inline( always)]
    fn	EndFaceVertex( &self) -> bool
    {
        let  	ctx = self.Get();
        ctx._FaceVerts.Push( ctx._CurFaceVert);
        ctx._CurFaceVert = FaceVertex::default();
        true
    }
    #[inline( always)]
    fn	EndFace( &self) -> bool
    {
        let  	ctx = self.Get();
        if ctx._FaceVerts.Size() > 0 {
            ctx._FStash.Push( Face { _Vertices: ctx._FaceVerts.ToBuff() });
            ctx._FaceVerts.Clear();
        }
        true
    }
    #[inline( always)]
    fn	PushObj( &self, arr: Arr< '_, u8>) -> bool
    {
        self.Get()._ObjStash.Push( arr.Str().to_string());
        true
    }
    #[inline( always)]
    fn	PushGrp( &self, arr: Arr< '_, u8>) -> bool
    {
        self.Get()._GrpStash.Push( arr.Str().to_string());
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

pub struct WaveObjShard< 'a>
{
    pub _Model: &'a mut WaveObjModel,
}

//---------------------------------------------------------------------------------------------------------------------------------

impl< 'a> IGrammar for WaveObjShard< 'a>
{
    fn	Match( &self, parser: &mut Parser) -> bool
    {
        let  	modelPtr: *mut WaveObjModel = self._Model as *const WaveObjModel as *mut WaveObjModel;
        let  	model = unsafe { &mut *modelPtr };
        let  	streamSz = parser.InStream().Size();
        let  	mut ctx = WaveObjParserCtx::New( streamSz);
        let  	ctxMM = WaveObjParserCtxMM( &mut ctx as *mut _);
        let  	nonWs = *Charset::NonSpace();
        let  	nonEnd = Charset::EndLine().Negative();
        let  	objGrammar = ShardTree!( 
            *( 
                *[ " \t" ]
                < ( 
                    ( "mtllib" < +[ " \t" ] < ( +nonWs)[ |arr| ctxMM.PushMtlLib( arr) ] )
                    | ( "usemtl" < +[ " \t" ] < ( +nonWs)[ |arr| ctxMM.PushUseMtl( arr) ] )
                    | ( ( "vt" < +[ " \t" ] < Real[ |arr| ctxMM.PushVal( arr) ] < ?( +[ " \t" ] < Real[ |arr| ctxMM.PushVal( arr) ] < ?( +[ " \t" ] < Real[ |arr| ctxMM.PushVal( arr) ] ) ) )[ |_arr| ctxMM.EndVt() ] )
                    | ( ( "vn" < +[ " \t" ] < Real[ |arr| ctxMM.PushVal( arr) ] < +[ " \t" ] < Real[ |arr| ctxMM.PushVal( arr) ] < +[ " \t" ] < Real[ |arr| ctxMM.PushVal( arr) ] )[ |_arr| ctxMM.EndVn() ] )
                    | ( ( "v" < +[ " \t" ] < Real[ |arr| ctxMM.PushVal( arr) ] < +[ " \t" ] < Real[ |arr| ctxMM.PushVal( arr) ] < +[ " \t" ] < Real[ |arr| ctxMM.PushVal( arr) ] < ?( +[ " \t" ] < Real[ |arr| ctxMM.PushVal( arr) ] ) )[ |_arr| ctxMM.EndV() ] )
                    | ( ( "f" < +( +[ " \t" ] < ( 
                        Int[ |arr| ctxMM.ParseFaceV( arr) ]
                        < ?( "/"
                             < ?Int[ |arr| ctxMM.ParseFaceVt( arr) ]
                             < ?( "/"
                                  < Int[ |arr| ctxMM.ParseFaceVn( arr) ]
                             )
                        )
                    )[ |_arr| ctxMM.EndFaceVertex() ] ) )[ |_arr| ctxMM.EndFace() ] )
                    | ( "o" < +[ " \t" ] < ( +nonWs)[ |arr| ctxMM.PushObj( arr) ] )
                    | ( "g" < +[ " \t" ] < ( +nonWs)[ |arr| ctxMM.PushGrp( arr) ] )
                    | *nonEnd
                )
                < *[ " \t" ]
                < ?( ( ?'\r' < '\n' ) | '\r' )
            )
        );
        if parser.ParseGrammar( &objGrammar, parser.CurrMark() ).is_none() {
            return false;
        }
        *model = WaveObjModel::Create( ctx);
        true
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
/// Parses a Wavefront .obj file from a string slice.
pub fn	ParseWaveObj( input: &str) -> Result< WaveObjModel, String>
{
    let  	mut stream = FixedStream::from( input);
    ParseWaveObjStream( &mut stream)
}

//---------------------------------------------------------------------------------------------------------------------------------
/// Parses a Wavefront .obj file from a raw byte slice.
pub fn	ParseWaveObjBytes( bytes: Arr< '_, u8>) -> Result< WaveObjModel, String>
{
    let  	raw = bytes.into();
    let  	s = std::str::from_utf8( raw).map_err( |e| e.to_string())?;
    ParseWaveObj( s)
}

//---------------------------------------------------------------------------------------------------------------------------------
/// Parses a Wavefront .obj file from an input stream.
pub fn	ParseWaveObjStream( stream: &mut dyn IStream) -> Result< WaveObjModel, String>
{
    let  	mut model = WaveObjModel::New();
    let  	mut parser = Parser::New( stream);
    let  	shard = WaveObjShard { _Model: &mut model };
    let  	res = parser.ParseGrammar( &shard, 0);
    if res.is_some() {
        Ok( model)
    } else {
        Err( "Failed to parse Wavefront .obj stream".to_string())
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl fmt::Debug for WaveObjModel {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        f.debug_struct( "WaveObjModel")
            .field( "vertices", &( self.VertexCount() as usize))
            .field( "tex_coords", &( self.TexCoordCount() as usize))
            .field( "normals", &( self.NormalCount() as usize))
            .field( "faces", &( self.FaceCount() as usize))
            .field( "objects", &( self._Objects.Size() as usize))
            .field( "groups", &( self._Groups.Size() as usize))
            .finish()
    }
}

//---------------------------------------------------------------------------------------------------------------------------------

impl fmt::Display for WaveObjModel {
    fn	fmt( &self, f: &mut fmt::Formatter< '_>) -> fmt::Result
    {
        write!( f, "WaveObjModel(v: {}, f: {}, n: {})", self.VertexCount(), self.FaceCount(), self.NormalCount())
    }
}

//---------------------------------------------------------------------------------------------------------------------------------
