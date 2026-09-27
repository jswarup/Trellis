// labels.wgsl ------------------------------------------------------------------------------------
struct View {
    matrix: mat4x4<f32>,
    inverseMatrix: mat4x4<f32>,
    settings: vec4<f32>,
    appearance: vec4<f32>,
};
@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var atlas: texture_2d<f32>;
@group(1) @binding(1) var atlasSampler: sampler;
struct Label {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
@vertex fn vs_label(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>) -> Label {
    return Label(view.matrix * vec4<f32>(position, 1.0), uv);
}
@fragment fn fs_label(in: Label) -> @location(0) vec4<f32> {
    let coverage = textureSample(atlas, atlasSampler, in.uv).r;
    if coverage < 0.08 { discard; }
    return vec4<f32>(vec3<f32>(0.025), coverage);
}
//-------------------------------------------------------------------------------------------------
