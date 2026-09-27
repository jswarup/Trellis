// geometry.rs ------------------------------------------------------------------------------------
//! Validated, normalized geometry shared by GPU viewports. Original bounds remain in source units.
use	crate::fenst::cask::{ Cask, CaskKind, LayoutCask, TraverseDepthRoots };
use	crate::fleck::{ PtsCloud, WaveObjModel };
use	crate::silo::{ Arr, Buff, IArr };

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
pub struct GeometryAsset
{
    _Vertices:      Buff< GeometryVertex>,
    _Triangles:     Buff< [u32; 3]>,
    _Edges:         Buff< [u32; 2]>,
    _Bounds:        ( [f32; 3], [f32; 3]),
    _Faces:         u32,
    _PointCloud:    bool,
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
    pub fn	FromCask( root: &Cask, maxDepth: usize) -> Result< Self, String>
    {
        LayoutCask( root, 0.0, 0.0);
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut triangles: Vec<[u32; 3]> = Vec::new();
        let mut edges: Vec<[u32; 2]> = Vec::new();
        let mut level = 0usize;
        TraverseDepthRoots( &[root], |ancestors, enter| {
            if !enter {
                level = level.saturating_sub( 1);
                return true;
            }
            let node = *ancestors.last().unwrap();
            if matches!( node._Kind, CaskKind::Label( _)) {
                return false;
            }
            level += 1;
            if level > maxDepth {
                return false;
            }
            let b = node._Bounds.get();
            let childCount = node.Children().iter().filter( |child| {
                matches!( child._Kind, CaskKind::Window)
            }).count();
            let ySize = b.height.max( 1.0);
            let zSize = if childCount == 0 {
                ySize
            }
            else {
                ySize.max( ( childCount as f32).sqrt() * ySize * 0.5)
            };
            let siblingIndex = if ancestors.len() > 1 {
                ancestors[ancestors.len() - 2]
                    .Children()
                    .iter()
                    .position( |child| std::ptr::eq( child, node))
                    .unwrap_or( 0)
            }
            else {
                0
            };
            let z0 = -( level as f32) * zSize * 0.35
                + ( siblingIndex / 2) as f32 * zSize * 1.1;
            let z1 = z0 + zSize;
            let base = positions.len() as u32;
            positions.extend( [
                [b.x, b.y, z0], [b.x + b.width, b.y, z0],
                [b.x + b.width, b.y + b.height, z0], [b.x, b.y + b.height, z0],
                [b.x, b.y, z1], [b.x + b.width, b.y, z1],
                [b.x + b.width, b.y + b.height, z1], [b.x, b.y + b.height, z1],
            ]);
            triangles.extend( [
                [base, base + 1, base + 2], [base, base + 2, base + 3],
                [base + 4, base + 6, base + 5], [base + 4, base + 7, base + 6],
                [base, base + 4, base + 5], [base, base + 5, base + 1],
                [base + 1, base + 5, base + 6], [base + 1, base + 6, base + 2],
                [base + 2, base + 6, base + 7], [base + 2, base + 7, base + 3],
                [base + 3, base + 7, base + 4], [base + 3, base + 4, base],
            ]);
            edges.extend( [
                [base, base + 1], [base + 1, base + 2], [base + 2, base + 3], [base + 3, base],
                [base + 4, base + 5], [base + 5, base + 6], [base + 6, base + 7], [base + 7, base + 4],
                [base, base + 4], [base + 1, base + 5], [base + 2, base + 6], [base + 3, base + 7],
            ]);
            true
        });
        if positions.is_empty() {
            return Err( "The cask contains no renderable windows.".into());
        }
        let bounds = positions.iter().fold(
            ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]),
            |( mut min, mut max), position| {
                min = std::array::from_fn( |i| min[i].min( position[i]));
                max = std::array::from_fn( |i| max[i].max( position[i]));
                (min, max)
            },
        );
        let (center, scale) = Self::Normalization( bounds)?;
        let pastels = [
            [0.96, 0.70, 0.74, 1.0], [0.71, 0.82, 0.98, 1.0],
            [0.68, 0.90, 0.76, 1.0], [0.98, 0.84, 0.62, 1.0],
            [0.83, 0.74, 0.96, 1.0], [0.67, 0.89, 0.89, 1.0],
        ];
        let vertices = Buff::FromDispenser( positions.len() as u32, |i| {
            let p = positions[i as usize];
            GeometryVertex { _Position: Self::Local( p, center, scale), _Intensity: 0.5,
                              _Color: pastels[( i as usize / 8) % pastels.len()] }
        });
        Ok( Self { _Vertices: vertices,
                   _Triangles: Buff::FromDispenser( triangles.len() as u32, |i| triangles[i as usize]),
                   _Edges: Buff::FromDispenser( edges.len() as u32, |i| edges[i as usize]),
                   _Bounds: bounds, _Faces: ( triangles.len() / 2) as u32, _PointCloud: false })
    }
    pub fn	FromPts( cloud: PtsCloud) -> Result< Self, String>
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
            _Vertices:      vertices,
            _Triangles:     Buff::New(),
            _Edges:         Buff::New(),
            _Bounds:        bounds,
            _Faces:         0,
            _PointCloud:    true,
        })
    }
    pub fn	FromObj( model: WaveObjModel) -> Result< Self, String>
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
            _Vertices:      vertices,
            _Triangles:     mesh._Triangles,
            _Edges:         mesh._Edges,
            _Bounds:        bounds,
            _Faces:         model.FaceCount(),
            _PointCloud:    false,
        })
    }
    fn	Normalization( bounds: ( [f32; 3], [f32; 3])) -> Result< ( [f64; 3], f64), String>
    {
        if !bounds
            .0
            .iter()
            .chain( bounds.1.iter())
            .all( |n| n.is_finite())
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
}

//-------------------------------------------------------------------------------------------------
