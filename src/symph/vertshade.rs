// vertshade.rs ----------------------------------------------------------------------------------------------------
use crate::silo::Arr;
//-------------------------------------------------------------------------------------------------
// Vector primitives and camera uniform blocks for shader/vertex math.
// Modeled directly from Trellis symph/vertshade.h.
#[derive( Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2
{
    pub x: f32,
    pub y: f32,
}
#[derive( Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3
{
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
#[derive( Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec4
{
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

//-------------------------------------------------------------------------------------------------
// CameraUniforms — parameters serialized explicitly for the camera compute kernel.
#[derive( Debug, Clone, Copy)]
pub struct CameraUniforms
{
    _RotX:      f32,
    _RotY:      f32,
    _Zoom:      f32,
    _PanX:      f32,
    _PanY:      f32,
    _Fov:       f32,
    _Distance:  f32,
    _Width:     f32,
    _Height:    f32,
    _CenterX:   f32,
    _CenterY:   f32,
    _CenterZ:   f32,
    _ScaleNorm: f32,
}
impl Default for CameraUniforms {
    fn	default() -> Self
    {
        Self {
            _RotX: 0.0,
            _RotY: 0.0,
            _Zoom: 1.0,
            _PanX: 0.0,
            _PanY: 0.0,
            _Fov: 60.0,
            _Distance: 500.0,
            _Width: 1920.0,
            _Height: 1080.0,
            _CenterX: 0.0,
            _CenterY: 0.0,
            _CenterZ: 0.0,
            _ScaleNorm: 1.0,
        }
    }
}
#[derive( Debug, Clone, Copy, PartialEq)]
pub struct VertexTransformResult
{
    pub clipPos: Vec4,
    pub ptSize: f32,
    pub depthFactor: f32,
}

//-------------------------------------------------------------------------------------------------
/// Camera parameters use the shared 13-float compute buffer order.
impl CameraUniforms
{
    pub const VALUE_COUNT: u32 = 13;

    pub fn  FromValues( values: Arr< '_, f32>) -> Option< Self>
    {
        if values.Size() < Self::VALUE_COUNT
        {
            return None;
        }
        return Some( Self {
            _RotX:      values[0],
            _RotY:      values[1],
            _Zoom:      values[2],
            _PanX:      values[3],
            _PanY:      values[4],
            _Fov:       values[5],
            _Distance:  values[6],
            _Width:     values[7],
            _Height:    values[8],
            _CenterX:   values[9],
            _CenterY:   values[10],
            _CenterZ:   values[11],
            _ScaleNorm: values[12],
        });
    }

    pub fn  Values( &self) -> [f32; 13]
    {
        return [
            self._RotX, self._RotY, self._Zoom, self._PanX, self._PanY,
            self._Fov, self._Distance, self._Width, self._Height,
            self._CenterX, self._CenterY, self._CenterZ, self._ScaleNorm,
        ];
    }
}

//-------------------------------------------------------------------------------------------------

/// Prepares camera rotation once for a batch of points. The camera snapshot is immutable.
pub struct CameraProjection
{
    _Camera:    CameraUniforms,
    _SinX:      f32,
    _CosX:      f32,
    _SinY:      f32,
    _CosY:      f32,
}

impl CameraProjection
{
    pub fn  New( camera: &CameraUniforms) -> Self
    {
        let ( sinX, cosX)    = camera._RotX.sin_cos();
        let ( sinY, cosY)    = camera._RotY.sin_cos();
        return Self {
            _Camera:    *camera,
            _SinX:      sinX,
            _CosX:      cosX,
            _SinY:      sinY,
            _CosY:      cosY,
        };
    }

    fn  ProjectPosition( &self, pos: &Vec3) -> ( Vec2, f32, f32)
    {
        let cam     = &self._Camera;
        let nx      = ( pos.x - cam._CenterX) * cam._ScaleNorm;
        let ny      = ( pos.y - cam._CenterY) * cam._ScaleNorm;
        let nz      = ( pos.z - cam._CenterZ) * cam._ScaleNorm;
        let x1      = nx * self._CosY + nz * self._SinY;
        let z1      = -nx * self._SinY + nz * self._CosY;
        let y2      = ny * self._CosX - z1 * self._SinX;
        let z2      = ny * self._SinX + z1 * self._CosX;
        let scale   = ( cam._Fov * cam._Zoom) / ( cam._Distance + z2).max( 1e-4);
        let screen  = Vec2 {
            x: cam._Width / 2.0 + cam._PanX + x1 * scale,
            y: cam._Height / 2.0 + cam._PanY - y2 * scale,
        };
        let depth   = ( ( 300.0 - z2) / 400.0).clamp( 0.3, 1.0);
        return ( screen, z2, depth);
    }

    /// Screen X/Y, radius, core radius, alpha, and depth factor, matching the compute ABI.
    pub fn  Project( &self, pos: &Vec3) -> [f32; 6]
    {
        let ( screen, _, depth) = self.ProjectPosition( pos);
        return [screen.x, screen.y, 3.0 + depth * 4.0,
                1.0 + depth * 1.5, 0.5 + depth * 0.5, depth];
    }

    pub fn  Transform( &self, pos: &Vec3) -> VertexTransformResult
    {
        let ( screen, z, depth) = self.ProjectPosition( pos);
        return VertexTransformResult {
            clipPos: Vec4 {
                x: screen.x / self._Camera._Width * 2.0 - 1.0,
                y: screen.y / self._Camera._Height * 2.0 - 1.0,
                z: z / 400.0,
                w: 1.0,
            },
            ptSize: 6.0 + depth * 8.0,
            depthFactor: depth,
        };
    }
}

/// Transforms one world-space point to NDC; reuse CameraProjection for multiple points.
pub fn  VertexTransformPos( pos: &Vec3, cam: &CameraUniforms) -> VertexTransformResult
{
    return CameraProjection::New( cam).Transform( pos);
}

//-------------------------------------------------------------------------------------------------
