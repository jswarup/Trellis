// geometry.rs ------------------------------------------------------------------------------------
//! Validated, normalized geometry shared by GPU viewports. Original bounds remain in source units.
use crate::fleck::{PtsCloud, WaveObjModel};
use crate::silo::{Arr, Buff, IArr, IArrMut, USeg};

//-------------------------------------------------------------------------------------------------

#[repr( C)]
#[derive( Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeometryVertex
{
    _Position:      [f32; 3],
    _Intensity:     f32,
    _Color:         [f32; 4],
}
impl GeometryVertex
{
    pub fn New( position: [f32; 3], color: [f32; 4]) -> Self
    {
        Self { _Position:  position,
               _Intensity: 0.5,
               _Color:     color, }
    }
    pub fn	Position( &self) -> [f32; 3]
    {
        self._Position
    }
    pub fn	Color( &self) -> [f32; 4]
    {
        self._Color
    }
    pub fn	Intensity( &self) -> f32
    {
        self._Intensity
    }
}
#[repr( C)]
#[derive( Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LabelVertex
{
    _Position: [f32; 3],
    _Uv:       [f32; 2],
}
impl LabelVertex
{
    pub fn New( position: [f32; 3], uv: [f32; 2]) -> Self
    {
        Self { _Position: position,
               _Uv:       uv, }
    }
    pub fn Position( &self) -> [f32; 3] { self._Position }
}
pub struct GeometryLabels
{
    _Vertices: Buff<LabelVertex>,
    _Pixels:   Buff<u8>,
    _Size:     [u32; 2],
    _Levels:   Buff<u32>,
}
impl GeometryLabels
{
    pub fn New( vertices: Buff<LabelVertex>, pixels: Buff<u8>, size: [u32; 2], levels: Buff<u32>)
               -> Result<Self, String>
    {
        let mut valid = !size.contains( &0)
                        && u64::from( size[0]) * u64::from( size[1]) == u64::from( pixels.Size());
        let mut previous = 0;
        levels.Arr().Traverse( |count| {
                        valid &= *count >= previous && *count <= vertices.Size() && *count % 6 == 0;
                        previous = *count;
                    });
        valid &= previous == vertices.Size();
        vertices.Arr().Traverse( |v| {
                          valid &= v._Position
                                    .iter()
                                    .chain( v._Uv.iter())
                                    .all( |n| n.is_finite());
                      });
        if !valid {
            return Err( "Invalid label atlas.".into());
        }
        Ok( Self { _Vertices: vertices,
                  _Pixels:   pixels,
                  _Size:     size,
                  _Levels:   levels, })
    }
    pub fn Vertices( &self) -> Arr<'_, LabelVertex> { self._Vertices.Arr() }
    pub fn Pixels( &self) -> Arr<'_, u8> { self._Pixels.Arr() }
    pub fn Size( &self) -> [u32; 2] { self._Size }
    pub fn DrawCount( &self, depth: u32) -> u32
    {
        if self._Levels.IsEmpty() {
            return 0;
        }
        self._Levels[depth.clamp( 1, self._Levels.Size()) - 1]
    }
}
pub struct GeometrySamples
{
    _Vertices: Buff<GeometryVertex>,
    _Levels:   Buff<u32>,
}

impl GeometrySamples
{
    pub fn  Vertices( &self) -> Arr<'_, GeometryVertex>
    {
        self._Vertices.Arr()
    }

    pub fn  DrawCount( &self, depth: u32) -> u32
    {
        self._Levels[depth.clamp( 1, self._Levels.Size()) - 1]
    }
}

pub struct GeometryAsset
{
    _Labels: Option<GeometryLabels>,
    _Samples: Option<GeometrySamples>,
    _Vertices:      Buff< GeometryVertex>,
    _Triangles:     Buff< [u32; 3]>,
    _Edges:         Buff< [u32; 2]>,
    _Bounds:        ( [f32; 3], [f32; 3]),
    _Faces:         u32,
    _PointCloud:    bool,
    _Levels:        Buff<[u32; 3]>,
}
impl std::fmt::Debug for GeometryAsset {
    fn	fmt( &self, f: &mut std::fmt::Formatter< '_>) -> std::fmt::Result
    {
        f.debug_struct( "GeometryAsset")
            .field( "vertices", &self.VertexCount())
            .field( "faces", &self._Faces)
            .finish()
    }
}
impl GeometryAsset
{
    /// Attaches an independent point stream in source units, with matching depth prefixes.
    pub fn  WithSamples( mut self, mut vertices: Buff<GeometryVertex>, levels: Buff<u32>)
                       -> Result<Self, String>
    {
        let mut valid = !vertices.IsEmpty() && !levels.IsEmpty()
                        && levels.Size() == self._Levels.Size();
        let mut previous = 0;
        levels.Arr().Traverse( |count| {
            valid &= *count >= previous && *count <= vertices.Size();
            previous = *count;
        });
        valid &= previous == vertices.Size();
        vertices.Arr().Traverse( |vertex| valid &= Self::ValidVertex( vertex, self._Bounds));
        if !valid
        {
            return Err( "Invalid point samples or depth ranges.".into());
        }
        let ( center, scale) = Self::Normalization( self._Bounds)?;
        vertices.MutArr().TraverseMut( |vertex| {
            vertex._Position = Self::Local( vertex._Position, center, scale);
        });
        self._Samples = Some( GeometrySamples { _Vertices: vertices, _Levels: levels });
        Ok( self)
    }

    pub fn  Samples( &self) -> Option<&GeometrySamples>
    {
        self._Samples.as_ref()
    }

    pub fn  PointCount( &self) -> u32
    {
        self._Samples.as_ref().map_or( self.VertexCount(), |samples| samples._Vertices.Size())
    }

    pub fn  PointDrawCount( &self, depth: u32) -> u32
    {
        self._Samples.as_ref().map_or_else( || self.DrawCounts( depth)[0],
            |samples| samples.DrawCount( depth))
    }

    fn  ValidVertex( vertex: &GeometryVertex, bounds: ( [f32; 3], [f32; 3])) -> bool
    {
        let mut valid = vertex._Color.iter().all( |value| value.is_finite());
        USeg::FromLen( 3).Traverse( |axis| {
            let axis = axis as usize;
            let value = vertex._Position[axis];
            valid &= value.is_finite() && value >= bounds.0[axis] && value <= bounds.1[axis];
        });
        valid
    }

    pub fn WithLabels( mut self, mut labels: GeometryLabels) -> Result<Self, String>
    {
        if labels._Levels.Size() != self._Levels.Size() {
            return Err( "Label depth ranges differ from geometry.".into());
        }
        let ( center, scale) = Self::Normalization( self._Bounds)?;
        labels._Vertices
              .MutArr()
              .TraverseMut( |v| v._Position = Self::Local( v._Position, center, scale));
        self._Labels = Some( labels);
        Ok( self)
    }
    pub fn Labels( &self) -> Option<&GeometryLabels> { self._Labels.as_ref() }

    /// Constructs normalized generated geometry with cumulative per-level draw counts.
    pub fn FromMesh( mut vertices: Buff<GeometryVertex>, triangles: Buff<[u32; 3]>,
                    edges: Buff<[u32; 2]>, levels: Buff<[u32; 3]>, bounds: ( [f32; 3], [f32; 3]))
                    -> Result<Self, String>
    {
        let ( center, scale) = Self::Normalization( bounds)?;
        let count = vertices.Size();
        let mut valid = count > 0;
        vertices.Arr().Traverse( |vertex| valid &= Self::ValidVertex( vertex, bounds));
        if levels.IsEmpty()
        {
            triangles.Arr().Traverse( |face| valid &= face.iter().all( |index| *index < count));
            edges.Arr().Traverse( |edge| valid &= edge.iter().all( |index| *index < count));
        }
        let mut previous = [0; 3];
        levels.Arr().Traverse( |level| {
                        let ordered = level[0] >= previous[0]
                                 && level[0] <= count
                                 && level[1] >= previous[1]
                                 && level[1] <= triangles.Size()
                                 && level[2] >= previous[2]
                                 && level[2] <= edges.Size();
                        valid &= ordered;
                        if ordered
                        {
                            USeg::WithLen( previous[1], level[1] - previous[1])
                                .Traverse( |index| {
                                    valid &= triangles[index].iter().all( |vertex| *vertex < level[0]);
                                });
                            USeg::WithLen( previous[2], level[2] - previous[2])
                                .Traverse( |index| {
                                    valid &= edges[index].iter().all( |vertex| *vertex < level[0]);
                                });
                        }
                        previous = *level;
                    });
        valid &= levels.IsEmpty() || previous == [count, triangles.Size(), edges.Size()];
        if !valid {
            return Err( "Invalid generated mesh or depth ranges.".into());
        }
        vertices.MutArr().TraverseMut( |vertex| {
                             vertex._Position = Self::Local( vertex._Position, center, scale);
                         });
        Ok( Self { _Labels:     None,
                  _Samples:    None,
                  _Faces:      triangles.Size(),
                  _Vertices:   vertices,
                  _Triangles:  triangles,
                  _Edges:      edges,
                  _Bounds:     bounds,
                  _PointCloud: false,
                  _Levels:     levels, })
    }

    pub fn FromPts( cloud: PtsCloud) -> Result<Self, String>
    {
        if cloud.IsEmpty() {
            return Err( "The file contains no points.".into());
        }
        let  	bounds = cloud.BoundingBox();
        let  	( center, scale) = Self::Normalization( bounds)?;
        let  	mut valid = true;
        let  	mut intensityMin = f32::INFINITY;
        let  	mut intensityMax = f32::NEG_INFINITY;
        cloud.Points().Arr().Traverse( |p| {
            valid &= p._Pos.Pos().iter().all( |n| n.is_finite());
            if let  	Some( value) = p._Intensity.filter( |v| v.is_finite()) {
                intensityMin = intensityMin.min( value);
                intensityMax = intensityMax.max( value);
            }
        });
        if !valid {
            return Err( "Point coordinates must be finite.".into());
        }
        let  	vertices = Buff::FromDispenser( cloud.Count(), |i| {
            let  	p = cloud.Points()[i];
            let  	color = p
                ._Color
                .map( |c| {
                    [
                        f32::from( c._R) / 255.0,
                        f32::from( c._G) / 255.0,
                        f32::from( c._B) / 255.0,
                        1.0,
                    ]
                })
                .unwrap_or( [0.35, 0.76, 0.94, 1.0]);
            let  	intensity = p
                ._Intensity
                .filter( |v| v.is_finite())
                .map( |v| {
                    if intensityMax > intensityMin {
                        ( ( f64::from( v) - f64::from( intensityMin))
                            / ( f64::from( intensityMax) - f64::from( intensityMin))) as f32
                    }
                    else {
                        0.5
                    }
                })
                .unwrap_or( 0.5);
            GeometryVertex {
                _Position:      Self::Local( p._Pos.Pos(), center, scale),
                _Intensity:     intensity,
                _Color:         color,
            }
        });
        Ok( Self {
            _Labels: None,
            _Samples: None,
            _Vertices:      vertices,
            _Triangles:     Buff::New(),
            _Edges:         Buff::New(),
            _Bounds:        bounds,
            _Faces:         0,
            _PointCloud:    true,
            _Levels:        Buff::New(),
        })
    }
    pub fn FromObj( model: WaveObjModel) -> Result<Self, String>
    {
        if model.VertexCount() == 0 {
            return Err( "The file contains no vertices.".into());
        }
        let  	bounds = model.BoundingBox();
        let  	( center, scale) = Self::Normalization( bounds)?;
        let  	mut valid = true;
        model._Vertices.Arr().Traverse( |v| {
            valid &= v._X.is_finite() && v._Y.is_finite() && v._Z.is_finite();
        });
        model._Faces.Arr().Traverse( |face| {
            valid &= face.Len() >= 3;
            face._Vertices.Arr().Traverse( |v| {
                valid &= v._VertexIdx > 0 && v._VertexIdx <= model.VertexCount();
            });
        });
        if !valid {
            return Err( "Invalid OBJ coordinates or face vertex indices.".into());
        }
        let  	mesh = model.ToMeshDto();
        let  	vertices = Buff::FromDispenser( model.VertexCount(), |i| GeometryVertex {
            _Position:      Self::Local( mesh._Points[i], center, scale),
            _Intensity:     0.5,
            _Color:         [0.65, 0.72, 0.85, 1.0],
        });
        Ok( Self {
            _Labels: None,
            _Samples: None,
            _Vertices:      vertices,
            _Triangles:     mesh._Triangles,
            _Edges:         mesh._Edges,
            _Bounds:        bounds,
            _Faces:         model.FaceCount(),
            _PointCloud:    false,
            _Levels:        Buff::New(),
        })
    }
    fn	Normalization( bounds: ( [f32; 3], [f32; 3])) -> Result< ( [f64; 3], f64), String>
    {
        if !bounds
            .0
            .iter()
            .chain( bounds.1.iter())
            .all( |n| n.is_finite())
            || bounds.0.iter().zip( bounds.1.iter()).any( |( min, max)| min > max)
        {
            return Err( "Geometry bounds must be finite.".into());
        }
        let  	center =
            std::array::from_fn( |i| ( f64::from( bounds.0[i]) + f64::from( bounds.1[i])) * 0.5);
        let  	extent: [f64; 3] =
            std::array::from_fn( |i| f64::from( bounds.1[i]) - f64::from( bounds.0[i]));
        let  	radius =
            ( extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt() * 0.5;
        Ok( ( center, if radius > 0.0 { 1.0 / radius } else { 1.0 }))
    }
    fn	Local( point: [f32; 3], center: [f64; 3], scale: f64) -> [f32; 3]
    {
        std::array::from_fn( |i| ( ( f64::from( point[i]) - center[i]) * scale) as f32)
    }
    pub fn	Vertices( &self) -> Arr< '_, GeometryVertex>
    {
        self._Vertices.Arr()
    }
    pub fn	Triangles( &self) -> Arr< '_, [u32; 3]>
    {
        self._Triangles.Arr()
    }
    pub fn	Edges( &self) -> Arr< '_, [u32; 2]>
    {
        self._Edges.Arr()
    }
    pub fn	VertexCount( &self) -> u32
    {
        self._Vertices.Size()
    }
    pub fn	FaceCount( &self) -> u32
    {
        self._Faces
    }
    pub fn	IsPointCloud( &self) -> bool
    {
        self._PointCloud
    }
    pub fn	Bounds( &self) -> ( [f32; 3], [f32; 3])
    {
        self._Bounds
    }
    pub fn MaxDepth( &self) -> u32 { self._Levels.Size() }
    pub fn DrawCounts( &self, depth: u32) -> [u32; 3]
    {
        if self._Levels.IsEmpty() {
            return [self.VertexCount(),
                    self._Triangles.Size(),
                    self._Edges.Size()];
        }
        self._Levels[depth.clamp( 1, self._Levels.Size()) - 1]
    }
}

//-------------------------------------------------------------------------------------------------
