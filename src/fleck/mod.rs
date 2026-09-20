//-- mod.rs ------------------------------------------------------------------------------------------------------------------------
pub mod	point;
pub mod geometry;
pub mod	ptio;
pub mod	vex;
pub mod	waveobjio;
pub use	point::{ BBox3f, Dir3f, Pt3f, WPt2f, WPt3f };
pub use	ptio::{ ParsePts, ParsePtsBytes, ParsePtsStream, PtsCloud, PtsPoint, PtsShard };
pub use	vex::*;
pub use	waveobjio::{ Face, FaceVertex, ParseWaveObj, ParseWaveObjBytes, ParseWaveObjStream, WaveObjModel, WaveObjShard };
#[cfg( feature = "tests")]
pub mod	_tests;

//---------------------------------------------------------------------------------------------------------------------------------

#[derive( Clone, Debug, Default, PartialEq)]
pub struct PtsPointsDto
{
    pub _Points:     crate::silo::Buff< [f32; 3]>,
    pub _Count:      usize,
    pub _BboxMin:    [f32; 3],
    pub _BboxMax:    [f32; 3],
}

//---------------------------------------------------------------------------------------------------------------------------------

#[derive( Clone, Debug, Default, PartialEq)]
pub struct WaveObjMeshDto
{
    pub _Points:        crate::silo::Buff< [f32; 3]>,
    pub _Triangles:     crate::silo::Buff< [u32; 3]>,
    pub _Edges:         crate::silo::Buff< [u32; 2]>,
    pub _Normals:       crate::silo::Buff< [f32; 3]>,
    pub _VertexCount:   usize,
    pub _FaceCount:     usize,
    pub _BboxMin:       [f32; 3],
    pub _BboxMax:       [f32; 3],
}

//---------------------------------------------------------------------------------------------------------------------------------
