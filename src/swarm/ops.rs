// ops.rs ----------------------------------------------------------------------------------------------------------
use crate::swarm::traits::{BackendKind, CpuKernelFn, KernelSource};
use crate::symph::compshade::{Collatz, HashToFloat, WangHash};
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------

// Standard compute operations supported out-of-the-box across all backends.
// Modeled directly from Trellis swarm/ops.h.
#[derive( Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardOp
{
    Double,
    VectorAdd,
    Collatz,
    PointCloud,
    CameraTransform,
}
pub fn  StandardOpLabel( op: StandardOp) -> &'static str {
    match op
    {
        StandardOp::Double => "double_kernel",
        StandardOp::VectorAdd => "vecadd_kernel",
        StandardOp::Collatz => "collatz_kernel",
        StandardOp::PointCloud => "pointcloud_kernel",
        StandardOp::CameraTransform => "camera_transform_kernel",
    }
}
pub fn  StandardOpEntryPoint( op: StandardOp, backend: BackendKind) -> &'static str {
    match backend
    {
        BackendKind::RustGpu => match op
        {
            StandardOp::Double => "double_cs",
            StandardOp::VectorAdd => "vecadd_cs",
            StandardOp::Collatz => "collatz_cs",
            StandardOp::PointCloud => "pts_pointcloud_cs",
            StandardOp::CameraTransform => "camera_transform_cs",
        },
        BackendKind::CudaOxide => match op
        {
            StandardOp::Double => "double_kernel",
            StandardOp::VectorAdd => "vecadd_kernel",
            StandardOp::Collatz => "collatz_kernel",
            StandardOp::PointCloud => "pointcloud_kernel",
            StandardOp::CameraTransform => "camera_transform_kernel",
        },
        BackendKind::Cpu => "main",
    }
}
pub fn  StandardOpWgsl( op: StandardOp) -> &'static str {
    match op
    {
        StandardOp::Double => {
            r#"@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@compute @workgroup_size( 64) fn  double_cs( @builtin( global_invocation_id) gid: vec3< u32>)
{
    let  idx = gid.x;
    if idx < arrayLength( &data)
    { data[idx] = data[idx] * 2.0; }
}"#
        }
        StandardOp::VectorAdd => {
            r#"@group(0) @binding(0) var<storage, read> a: array<f32>;
@group( 0) @binding( 1) var< storage, read> b: array< f32>;
@group( 0) @binding( 2) var< storage, read_write> result: array< f32>;
@compute @workgroup_size( 64) fn  vecadd_cs( @builtin( global_invocation_id) gid: vec3< u32>)
{
    let  idx = gid.x;
    if idx < arrayLength( &result)
    { result[idx] = a[idx] + b[idx]; }
}"#
        }
        StandardOp::Collatz => {
            r#"@group(0) @binding(0) var<storage, read> input: array<u32>;
@group( 0) @binding( 1) var< storage, read_write> output: array< u32>;
fn  collatz_steps( n_in: u32) -> u32
{
    var n: u32 = n_in;
    var steps: u32 = 0u;
    while n != 1u
    {
        if ( n % 2u) == 0u
        { n = n / 2u; } else
        { n = 3u * n + 1u; }
        steps = steps + 1u;
    }
    return steps;
}
@compute @workgroup_size( 64) fn  collatz_cs( @builtin( global_invocation_id) gid: vec3< u32>)
{
    let  idx = gid.x;
    if idx < arrayLength( &output)
    { output[idx] = collatz_steps( input[idx]); }
}"#
        }
        StandardOp::PointCloud => {
            r#"@group(0) @binding(0) var<storage, read_write> points: array<vec4<f32>>;
fn  wang_hash( seed_in: u32) -> u32
{
    var seed: u32 = seed_in;
    seed = ( seed ^ 61u) ^ ( seed >> 16u);
    seed = seed * 9u;
    seed = seed ^ ( seed >> 4u);
    seed = seed * 0x27d4eb2du;
    seed = seed ^ ( seed >> 15u);
    return seed;
}
fn  hash_to_float( h: u32) -> f32
{
    return f32( h & 0x00FFFFFFu) / 16777216.0;
}
@compute @workgroup_size( 64) fn  pts_pointcloud_cs( @builtin( global_invocation_id) gid: vec3< u32>)
{
    let  idx = gid.x;
    if idx < arrayLength( &points)
    {
        let  hx = wang_hash( idx * 3u);
        let  hy = wang_hash( idx * 3u + 1u);
        let  hz = wang_hash( idx * 3u + 2u);
        let  x = hash_to_float( hx) * 40.0 - 20.0;
        let  y = hash_to_float( hy) * 40.0 - 20.0;
        let  z = hash_to_float( hz) * 40.0 - 20.0;
        points[idx] = vec4< f32>( x, y, z, 1.0);
    }
}"#
        }
        StandardOp::CameraTransform => {
            r#"@group(0) @binding(0) var<storage, read> in_points: array<f32>;
@group( 0) @binding( 1) var< storage, read> cam_params: array< f32>;
@group( 0) @binding( 2) var< storage, read_write> out_projected: array< f32>;
@compute @workgroup_size( 64) fn  camera_transform_cs( @builtin( global_invocation_id) gid: vec3< u32>)
{
    let  idx = gid.x;
    let  in_base = idx * 3u;
    let  out_base = idx * 6u;
    if in_base + 2u < arrayLength( &in_points) && out_base + 5u < arrayLength( &out_projected) && arrayLength( &cam_params) >= 13u
    {
        let  x = in_points[in_base];
        let  y = in_points[in_base + 1u];
        let  z = in_points[in_base + 2u];
        let  rot_x = cam_params[0];
        let  rot_y = cam_params[1];
        let  zoom = cam_params[2];
        let  pan_x = cam_params[3];
        let  pan_y = cam_params[4];
        let  fov = cam_params[5];
        let  distance = cam_params[6];
        let  width = cam_params[7];
        let  height = cam_params[8];
        let  cx = cam_params[9];
        let  cy = cam_params[10];
        let  cz = cam_params[11];
        let  scale_norm = cam_params[12];
        let  nx = ( x - cx) * scale_norm;
        let  ny = ( y - cy) * scale_norm;
        let  nz = ( z - cz) * scale_norm;
        let  cos_y = cos( rot_y);
        let  sin_y = sin( rot_y);
        let  x1 = nx * cos_y + nz * sin_y;
        let  z1 = -nx * sin_y + nz * cos_y;
        let  cos_x = cos( rot_x);
        let  sin_x = sin( rot_x);
        let  y2 = ny * cos_x - z1 * sin_x;
        let  z2 = ny * sin_x + z1 * cos_x;
        let  scale = ( fov * zoom) / ( distance + z2);
        let  proj_x = width / 2.0 + pan_x + x1 * scale;
        let  proj_y = height / 2.0 + pan_y - y2 * scale;
        let  depth_factor = max( 0.3, min( 1.0, ( 300.0 - z2) / 400.0));
        let  radius = 3.0 + depth_factor * 4.0;
        let  core_radius = 1.0 + depth_factor * 1.5;
        let  alpha = 0.5 + depth_factor * 0.5;
        out_projected[out_base] = proj_x;
        out_projected[out_base + 1u] = proj_y;
        out_projected[out_base + 2u] = radius;
        out_projected[out_base + 3u] = core_radius;
        out_projected[out_base + 4u] = alpha;
        out_projected[out_base + 5u] = depth_factor;
    }
}"#
        }
    }
}
pub fn  StandardOpPtx( op: StandardOp) -> &'static str {
    match op
    {
        StandardOp::Double => ".version 7.0\n.target sm_70\n.entry double_kernel",
        StandardOp::VectorAdd => ".version 7.0\n.target sm_70\n.entry vecadd_kernel",
        StandardOp::Collatz => ".version 7.0\n.target sm_70\n.entry collatz_kernel",
        StandardOp::PointCloud => ".version 7.0\n.target sm_70\n.entry pointcloud_kernel",
        StandardOp::CameraTransform => {
            ".version 7.0\n.target sm_70\n.entry camera_transform_kernel"
        }
    }
}
pub fn  StandardOpCpuKernelFn( op: StandardOp) -> CpuKernelFn
{
    match op
    {
        StandardOp::Double => Arc::new( |_inputs, outputs, gid_x, _gid_y, _gid_z| {
            if outputs.is_empty()
            {
                return;
            }
            let  out_slice = &mut outputs[0];
            let  float_count = out_slice.len() / std::mem::size_of::<f32>();
            if ( gid_x as usize) < float_count
            {
                let  f_ptr = out_slice.as_mut_ptr() as *mut f32;
                unsafe {
                    let  val = f_ptr.add( gid_x as usize);
                    *val *= 2.0;
                }
            }
        }),
        StandardOp::VectorAdd => Arc::new( |inputs, outputs, gid_x, _gid_y, _gid_z| {
            if inputs.len() < 2 || outputs.is_empty()
            {
                return;
            }
            let  in_a = inputs[0];
            let  in_b = inputs[1];
            let  out = &mut outputs[0];
            let  count = ( out.len() / 4).min( in_a.len() / 4).min( in_b.len() / 4);
            if ( gid_x as usize) < count
            {
                let  a_ptr = in_a.as_ptr() as *const f32;
                let  b_ptr = in_b.as_ptr() as *const f32;
                let  out_ptr = out.as_mut_ptr() as *mut f32;
                unsafe {
                    *out_ptr.add( gid_x as usize) =
                        *a_ptr.add( gid_x as usize) + *b_ptr.add( gid_x as usize);
                }
            }
        }),
        StandardOp::Collatz => Arc::new( |inputs, outputs, gid_x, _gid_y, _gid_z| {
            if inputs.is_empty() || outputs.is_empty()
            {
                return;
            }
            let  in_slice = inputs[0];
            let  out_slice = &mut outputs[0];
            let  count = ( out_slice.len() / 4).min( in_slice.len() / 4);
            if ( gid_x as usize) < count
            {
                let  in_ptr = in_slice.as_ptr() as *const u32;
                let  out_ptr = out_slice.as_mut_ptr() as *mut u32;
                unsafe {
                    let  val = *in_ptr.add( gid_x as usize);
                    *out_ptr.add( gid_x as usize) = Collatz( val);
                }
            }
        }),
        StandardOp::PointCloud => Arc::new( |_inputs, outputs, gid_x, _gid_y, _gid_z| {
            if outputs.is_empty()
            {
                return;
            }
            let  out = &mut outputs[0];
            let  base = ( gid_x as usize) * 4;
            let  total_floats = out.len() / 4;
            if base + 3 < total_floats
            {
                let  out_ptr = out.as_mut_ptr() as *mut f32;
                let  hx = WangHash( gid_x * 3);
                let  hy = WangHash( gid_x * 3 + 1);
                let  hz = WangHash( gid_x * 3 + 2);
                let  x = HashToFloat( hx) * 40.0 - 20.0;
                let  y = HashToFloat( hy) * 40.0 - 20.0;
                let  z = HashToFloat( hz) * 40.0 - 20.0;
                unsafe {
                    *out_ptr.add( base) = x;
                    *out_ptr.add( base + 1) = y;
                    *out_ptr.add( base + 2) = z;
                    *out_ptr.add( base + 3) = 1.0;
                }
            }
        }),
        StandardOp::CameraTransform => Arc::new( |inputs, outputs, gid_x, _gid_y, _gid_z| {
            if inputs.len() < 2 || outputs.is_empty()
            {
                return;
            }
            let  in_points = inputs[0];
            let  cam_params = inputs[1];
            let  out = &mut outputs[0];
            let  in_base = ( gid_x as usize) * 3;
            let  out_base = ( gid_x as usize) * 6;
            let  in_floats = in_points.len() / 4;
            let  cam_floats = cam_params.len() / 4;
            let  out_floats = out.len() / 4;
            if in_base + 2 < in_floats && out_base + 5 < out_floats && cam_floats >= 13
            {
                let  in_ptr = in_points.as_ptr() as *const f32;
                let  cam_ptr = cam_params.as_ptr() as *const f32;
                let  out_ptr = out.as_mut_ptr() as *mut f32;
                unsafe {
                    let  x = *in_ptr.add( in_base);
                    let  y = *in_ptr.add( in_base + 1);
                    let  z = *in_ptr.add( in_base + 2);
                    let  rot_x = *cam_ptr.add( 0);
                    let  rot_y = *cam_ptr.add( 1);
                    let  zoom = *cam_ptr.add( 2);
                    let  pan_x = *cam_ptr.add( 3);
                    let  pan_y = *cam_ptr.add( 4);
                    let  fov = *cam_ptr.add( 5);
                    let  distance = *cam_ptr.add( 6);
                    let  width = *cam_ptr.add( 7);
                    let  height = *cam_ptr.add( 8);
                    let  cx = *cam_ptr.add( 9);
                    let  cy = *cam_ptr.add( 10);
                    let  cz = *cam_ptr.add( 11);
                    let  scale_norm = *cam_ptr.add( 12);
                    let  nx = ( x - cx) * scale_norm;
                    let  ny = ( y - cy) * scale_norm;
                    let  nz = ( z - cz) * scale_norm;
                    let  cos_y = rot_y.cos();
                    let  sin_y = rot_y.sin();
                    let  x1 = nx * cos_y + nz * sin_y;
                    let  z1 = -nx * sin_y + nz * cos_y;
                    let  cos_x = rot_x.cos();
                    let  sin_x = rot_x.sin();
                    let  y2 = ny * cos_x - z1 * sin_x;
                    let  z2 = ny * sin_x + z1 * cos_x;
                    let  scale = ( fov * zoom) / ( distance + z2);
                    let  proj_x = width / 2.0 + pan_x + x1 * scale;
                    let  proj_y = height / 2.0 + pan_y - y2 * scale;
                    let  depth_factor = ( ( 300.0 - z2) / 400.0).clamp( 0.3, 1.0);
                    let  radius = 3.0 + depth_factor * 4.0;
                    let  core_radius = 1.0 + depth_factor * 1.5;
                    let  alpha = 0.5 + depth_factor * 0.5;
                    *out_ptr.add( out_base) = proj_x;
                    *out_ptr.add( out_base + 1) = proj_y;
                    *out_ptr.add( out_base + 2) = radius;
                    *out_ptr.add( out_base + 3) = core_radius;
                    *out_ptr.add( out_base + 4) = alpha;
                    *out_ptr.add( out_base + 5) = depth_factor;
                }
            }
        }),
    }
}
pub fn  StandardOpKernelSource( op: StandardOp, backend: BackendKind) -> KernelSource
{
    match backend
    {
        BackendKind::Cpu => KernelSource::Cpu( StandardOpCpuKernelFn( op)),
        BackendKind::RustGpu => KernelSource::Wgsl( StandardOpWgsl( op)),
        BackendKind::CudaOxide => KernelSource::Ptx( StandardOpPtx( op)),
    }
}
