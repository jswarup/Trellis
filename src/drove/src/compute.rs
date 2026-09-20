use spirv_std::{ glam::UVec3, spirv };

//-------------------------------------------------------------------------------------------------
// First Drove shader: used to validate the pinned Rust-to-SPIR-V toolchain.
#[spirv( compute( threads( 64)))]
pub fn	double_cs(
    #[spirv( global_invocation_id)] globalId: UVec3,
    #[spirv( storage_buffer, descriptor_set = 0, binding = 0)] data: &mut [f32],
)
{
    let   index = globalId.x as usize;
    if index < data.len() {
        data[index] *= 2.0;
    }
}
