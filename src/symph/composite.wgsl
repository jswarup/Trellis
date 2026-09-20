// Place the rendered viewport inside the Iced window. Scissor is set by the host.
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> region: vec4<f32>;
struct Quad {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@vertex fn vs_quad(@builtin(vertex_index) i: u32) -> Quad {
    let corners = array<vec2<f32>, 3>(vec2(0.0,0.0), vec2(2.0,0.0), vec2(0.0,2.0));
    let uv = corners[i];
    let p = region.xy + uv * region.zw;
    return Quad(vec4<f32>(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0), uv);
}
@fragment fn fs_quad(in: Quad) -> @location(0) vec4<f32> {
    return textureSample(scene, scene_sampler, in.uv);
}
@fragment fn fs_encoded(in: Quad) -> @location(0) vec4<f32> {
    let linear = textureSample(scene, scene_sampler, in.uv);
    return vec4<f32>(pow(linear.rgb, vec3<f32>(1.0 / 2.2)), linear.a);
}
