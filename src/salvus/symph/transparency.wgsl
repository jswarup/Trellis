// transparency.wgsl ------------------------------------------------------------------------------
// Resolve weighted, order-independent surface colors over the opaque viewport.
@group(0) @binding(0) var accumulation: texture_2d<f32>;
@group(0) @binding(1) var revealage: texture_2d<f32>;
@vertex fn vs_resolve(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4<f32>(p[index], 0.0, 1.0);
}
@fragment fn fs_resolve(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(p.xy);
    let sum = textureLoad(accumulation, pixel, 0);
    let alpha = 1.0 - textureLoad(revealage, pixel, 0).r;
    return vec4<f32>(sum.rgb / max(sum.a, 0.00001), alpha);
}
//-------------------------------------------------------------------------------------------------
