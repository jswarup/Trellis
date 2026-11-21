//-- mod.rs ------------------------------------------------------------------------------------------------------------------------
pub mod geometry;
pub mod point;
pub mod sketch_flux;
pub use geometry::{FleckGeometryAssetExt, GeometryAsset, GeometryLabels, GeometrySamples, GeometryVertex, LabelVertex};
pub mod	ptio;
pub mod	sketch_extrude;
pub mod	sketch_revolve;
pub mod	sketch_sweep;
pub mod	sketch_loft;
pub mod	sketch_csg;
pub mod	sketch_geometry;
pub mod	sketch_io;
pub mod	sketch_mesh;
pub mod	sketch_query;
pub mod	sketch_tessellate;
pub mod	vex;
pub mod	waveobjio;
pub use	point::{ BBox3f, Dir3f, Pt3f, WPt2f, WPt3f };
pub use	ptio::{ ParsePts, ParsePtsBytes, ParsePtsStream, PtsCloud, PtsGrammar, PtsParseCtx, PtsPoint, PtsShard };
pub use	sketch_extrude::SketchExtrude;
pub use	sketch_flux::SketchFlux;
pub use	sketch_revolve::{RevolveAxis, SketchRevolve, SketchRevolveOptions};
pub use	sketch_sweep::SketchSweep;
pub use	sketch_loft::{LoftSection, SketchLoft};
pub use	sketch_csg::{CsgOp, SketchCsg};
pub use	sketch_geometry::SketchGeometry;
pub use	sketch_io::SketchIo;
pub use	sketch_mesh::{Sketch3dProjection, Sketch3dProjectionOptions};
pub use	sketch_query::{SketchQuery, SnapCandidate, SnapKind};
pub use	sketch_tessellate::SketchTessellate;
pub use	vex::*;
pub use	waveobjio::{ Face, FaceVertex, ParseWaveObj, ParseWaveObjBytes, ParseWaveObjStream, WaveObjGrammar, WaveObjModel, WaveObjParserCtx, WaveObjShard };
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
