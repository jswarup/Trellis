// vertshade.rs ----------------------------------------------------------------------------------------------------
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
// CameraUniforms — camera uniform parameter block for dedicated vertex & fragment rendering pipelines.
#[derive( Debug, Clone, Copy)]
pub struct CameraUniforms
{
    pub _RotX: f32,
    pub _RotY: f32,
    pub _Zoom: f32,
    pub _PanX: f32,
    pub _PanY: f32,
    pub _Fov: f32,
    pub _Distance: f32,
    pub _Width: f32,
    pub _Height: f32,
    pub _CenterX: f32,
    pub _CenterY: f32,
    pub _CenterZ: f32,
    pub _ScaleNorm: f32,
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
// VertexTransformPos — transforms a 3D point in world space to NDC coordinates.
pub fn	VertexTransformPos( pos: &Vec3, cam: &CameraUniforms) -> VertexTransformResult
{
    let  	nx = ( pos.x - cam._CenterX) * cam._ScaleNorm;
    let  	ny = ( pos.y - cam._CenterY) * cam._ScaleNorm;
    let  	nz = ( pos.z - cam._CenterZ) * cam._ScaleNorm;
    let  	cos_y = cam._RotY.cos();
    let  	sin_y = cam._RotY.sin();
    let  	x1 = nx * cos_y + nz * sin_y;
    let  	z1 = -nx * sin_y + nz * cos_y;
    let  	cos_x = cam._RotX.cos();
    let  	sin_x = cam._RotX.sin();
    let  	y2 = ny * cos_x - z1 * sin_x;
    let  	z2 = ny * sin_x + z1 * cos_x;
    let  	denom = cam._Distance + z2;
    let  	w = if denom > 1e-4 { denom } else { 1e-4 };
    let  	scale = ( cam._Fov * cam._Zoom) / w;
    let  	proj_x = cam._Width / 2.0 + cam._PanX + x1 * scale;
    let  	proj_y = cam._Height / 2.0 + cam._PanY - y2 * scale;
    let  	ndc_x = ( proj_x / cam._Width) * 2.0 - 1.0;
    let  	ndc_y = ( proj_y / cam._Height) * 2.0 - 1.0;
    let  	ndc_z = z2 / 400.0;
    let  	depth_factor = ( ( 300.0 - z2) / 400.0).clamp( 0.3, 1.0);
    let  	pt_size = 6.0 + depth_factor * 8.0;
    VertexTransformResult {
        clipPos: Vec4 {
            x: ndc_x,
            y: ndc_y,
            z: ndc_z,
            w: 1.0,
        },
        ptSize: pt_size,
        depthFactor: depth_factor,
    }
}
