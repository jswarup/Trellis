// ops.rs ----------------------------------------------------------------------------------------------------------
use crate::silo::Arr;
use crate::swarm::traits::{BackendKind, ComputeBuffer, KernelSource, SwarmError, WorkgroupDim};
pub use crate::flock::StandardOpCpuKernelFn;
pub use crate::symph::{StandardOp, StandardOpLabel};

//-------------------------------------------------------------------------------------------------
/// Authoritative backend descriptor defining entry point names and dispatch contracts
/// for standard operations across all compute backends (CPU, RustGpu, CudaOxide).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StandardOpDescriptor
{
    pub op: StandardOp,
    pub label: &'static str,
    pub wgsl_entry_point: &'static str,
    pub ptx_entry_point: &'static str,
    pub spirv_entry_point: &'static str,
    pub cpu_entry_point: &'static str,
    pub required_buffer_count: u32,
    pub requires_linear_dim: bool,
}

impl StandardOpDescriptor
{
    pub const fn ForOp(op: StandardOp) -> Self
    {
        match op {
            StandardOp::Double => Self {
                op,
                label: StandardOpLabel(op),
                wgsl_entry_point: "double_cs",
                ptx_entry_point: StandardOpLabel(op),
                spirv_entry_point: "compute::double_cs",
                cpu_entry_point: "main",
                required_buffer_count: 1,
                requires_linear_dim: true,
            },
            StandardOp::VectorAdd => Self {
                op,
                label: StandardOpLabel(op),
                wgsl_entry_point: "vecadd_cs",
                ptx_entry_point: StandardOpLabel(op),
                spirv_entry_point: "compute::vecadd_cs",
                cpu_entry_point: "main",
                required_buffer_count: 3,
                requires_linear_dim: true,
            },
            StandardOp::Collatz => Self {
                op,
                label: StandardOpLabel(op),
                wgsl_entry_point: "collatz_cs",
                ptx_entry_point: StandardOpLabel(op),
                spirv_entry_point: "compute::collatz_cs",
                cpu_entry_point: "main",
                required_buffer_count: 2,
                requires_linear_dim: true,
            },
            StandardOp::PointCloud => Self {
                op,
                label: StandardOpLabel(op),
                wgsl_entry_point: "pts_pointcloud_cs",
                ptx_entry_point: StandardOpLabel(op),
                spirv_entry_point: "compute::pts_pointcloud_cs",
                cpu_entry_point: "main",
                required_buffer_count: 1,
                requires_linear_dim: true,
            },
            StandardOp::CameraTransform => Self {
                op,
                label: StandardOpLabel(op),
                wgsl_entry_point: "camera_transform_cs",
                ptx_entry_point: StandardOpLabel(op),
                spirv_entry_point: "compute::camera_transform_cs",
                cpu_entry_point: "main",
                required_buffer_count: 3,
                requires_linear_dim: true,
            },
        }
    }

    pub fn EntryPoint(&self, backend: BackendKind) -> &'static str
    {
        match backend {
            BackendKind::Cpu => self.cpu_entry_point,
            BackendKind::RustGpu => self.spirv_entry_point,
            BackendKind::CudaOxide => self.ptx_entry_point,
        }
    }

    pub fn ValidateDispatch(
        &self,
        buffers: Arr<'_, &ComputeBuffer>,
        dim: WorkgroupDim,
    ) -> Result<(), SwarmError>
    {
        if buffers.Len() != self.required_buffer_count {
            return Err(SwarmError::ExecutionError(
                "Buffer count does not match operation requirement",
            ));
        }
        if self.requires_linear_dim && (dim.Y() != 1 || dim.Z() != 1) {
            return Err(SwarmError::ExecutionError(
                "Operation requires linear dispatch dimensions",
            ));
        }
        Ok(())
    }
}

//-------------------------------------------------------------------------------------------------
// Standard compute operations supported out-of-the-box across all backends.
// Modeled directly from Trellis swarm/ops.h.
pub fn StandardOpEntryPoint(op: StandardOp, backend: BackendKind) -> &'static str
{
    StandardOpDescriptor::ForOp(op).EntryPoint(backend)
}
pub fn	StandardOpWgsl( op: StandardOp) -> &'static str {
    match op {
        StandardOp::Double => {
            r#"@group(0) @binding(0) var<storage, read_write> data: array<f32>;
@compute @workgroup_size( 64) fn	double_cs( @builtin( global_invocation_id) gid: vec3< u32>)
{
    let  	idx = gid.x;
    if idx < arrayLength( &data)
    { data[idx] = data[idx] * 2.0; }
}"#
        }
        StandardOp::VectorAdd => {
            r#"@group(0) @binding(0) var<storage, read> a: array<f32>;
@group( 0) @binding( 1) var< storage, read> b: array< f32>;
@group( 0) @binding( 2) var< storage, read_write> result: array< f32>;
@compute @workgroup_size( 64) fn	vecadd_cs( @builtin( global_invocation_id) gid: vec3< u32>)
{
    let  	idx = gid.x;
    if idx < arrayLength( &result) && idx < arrayLength( &a) && idx < arrayLength( &b)
    { result[idx] = a[idx] + b[idx]; }
}"#
        }
        StandardOp::Collatz => {
            r#"@group(0) @binding(0) var<storage, read> input: array<u32>;
@group( 0) @binding( 1) var< storage, read_write> output: array< u32>;
fn	collatz_steps( n_in: u32) -> u32
{
    var n: u32 = n_in;
    var steps: u32 = 0u;
    if n == 0u { return 0xFFFFFFFFu; }
    while n != 1u {
        if ( n % 2u) == 0u
        { n = n / 2u; } else
        { if n >= 0x55555555u { return 0xFFFFFFFFu; } n = 3u * n + 1u; }
        steps = steps + 1u;
    }
    return steps;
}
@compute @workgroup_size( 64) fn	collatz_cs( @builtin( global_invocation_id) gid: vec3< u32>)
{
    let  	idx = gid.x;
    if idx < arrayLength( &output) && idx < arrayLength( &input)
    { output[idx] = collatz_steps( input[idx]); }
}"#
        }
        StandardOp::PointCloud => {
            r#"@group(0) @binding(0) var<storage, read_write> points: array<vec4<f32>>;
fn	wang_hash( seed_in: u32) -> u32
{
    var seed: u32 = seed_in;
    seed = ( seed ^ 61u) ^ ( seed >> 16u);
    seed = seed * 9u;
    seed = seed ^ ( seed >> 4u);
    seed = seed * 0x27d4eb2du;
    seed = seed ^ ( seed >> 15u);
    return seed;
}
fn	hash_to_float( h: u32) -> f32
{
    return f32( h & 0x00FFFFFFu) / 16777216.0;
}
@compute @workgroup_size( 64) fn	pts_pointcloud_cs( @builtin( global_invocation_id) gid: vec3< u32>)
{
    let  	idx = gid.x;
    if idx < arrayLength( &points) {
        let  	hx = wang_hash( idx * 3u);
        let  	hy = wang_hash( idx * 3u + 1u);
        let  	hz = wang_hash( idx * 3u + 2u);
        let  	x = hash_to_float( hx) * 40.0 - 20.0;
        let  	y = hash_to_float( hy) * 40.0 - 20.0;
        let  	z = hash_to_float( hz) * 40.0 - 20.0;
        points[idx] = vec4< f32>( x, y, z, 1.0);
    }
}"#
        }
        StandardOp::CameraTransform => {
            r#"@group(0) @binding(0) var<storage, read> in_points: array<f32>;
@group( 0) @binding( 1) var< storage, read> cam_params: array< f32>;
@group( 0) @binding( 2) var< storage, read_write> out_projected: array< f32>;
@compute @workgroup_size( 64) fn	camera_transform_cs( @builtin( global_invocation_id) gid: vec3< u32>)
{
    let  	idx = gid.x;
    let  	in_base = idx * 3u;
    let  	out_base = idx * 6u;
    if in_base + 2u < arrayLength( &in_points) && out_base + 5u < arrayLength( &out_projected) && arrayLength( &cam_params) >= 13u {
        let  	x = in_points[in_base];
        let  	y = in_points[in_base + 1u];
        let  	z = in_points[in_base + 2u];
        let  	rot_x = cam_params[0];
        let  	rot_y = cam_params[1];
        let  	zoom = cam_params[2];
        let  	pan_x = cam_params[3];
        let  	pan_y = cam_params[4];
        let  	fov = cam_params[5];
        let  	distance = cam_params[6];
        let  	width = cam_params[7];
        let  	height = cam_params[8];
        let  	cx = cam_params[9];
        let  	cy = cam_params[10];
        let  	cz = cam_params[11];
        let  	scale_norm = cam_params[12];
        let  	nx = ( x - cx) * scale_norm;
        let  	ny = ( y - cy) * scale_norm;
        let  	nz = ( z - cz) * scale_norm;
        let  	cos_y = cos( rot_y);
        let  	sin_y = sin( rot_y);
        let  	x1 = nx * cos_y + nz * sin_y;
        let  	z1 = -nx * sin_y + nz * cos_y;
        let  	cos_x = cos( rot_x);
        let  	sin_x = sin( rot_x);
        let  	y2 = ny * cos_x - z1 * sin_x;
        let  	z2 = ny * sin_x + z1 * cos_x;
        let  	scale = ( fov * zoom) / ( distance + z2);
        let  	proj_x = width / 2.0 + pan_x + x1 * scale;
        let  	proj_y = height / 2.0 + pan_y - y2 * scale;
        let  	depth_factor = max( 0.3, min( 1.0, ( 300.0 - z2) / 400.0));
        let  	radius = 3.0 + depth_factor * 4.0;
        let  	core_radius = 1.0 + depth_factor * 1.5;
        let  	alpha = 0.5 + depth_factor * 0.5;
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
pub fn	StandardOpPtx( op: StandardOp) -> &'static str {
    match op {
        StandardOp::Double => ".version 7.0\n.target sm_70\n.entry double_kernel",
        StandardOp::VectorAdd => ".version 7.0\n.target sm_70\n.entry vecadd_kernel",
        StandardOp::Collatz => ".version 7.0\n.target sm_70\n.entry collatz_kernel",
        StandardOp::PointCloud => ".version 7.0\n.target sm_70\n.entry pointcloud_kernel",
        StandardOp::CameraTransform => {
            ".version 7.0\n.target sm_70\n.entry camera_transform_kernel"
        }
    }
}
pub fn	StandardOpKernelSource(
    op: StandardOp, backend: BackendKind,
) -> Result<KernelSource, crate::swarm::traits::SwarmError>
{
    match backend {
        BackendKind::Cpu => Ok( KernelSource::CpuStandard( op, crate::flock::StandardOpCpuKernelFn( op))),
        BackendKind::RustGpu => crate::drove::ComputeSpirV( op)
            .map( KernelSource::SpirV)
            .ok_or_else( || crate::swarm::traits::SwarmError::CompilationError(
                format!( "Drove has no SPIR-V entry point for {}", StandardOpLabel( op)),
            )),
        BackendKind::CudaOxide => Ok( KernelSource::Ptx( StandardOpPtx( op))),
    }
}
