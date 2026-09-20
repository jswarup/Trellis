// GPU geometry: indexed meshes and instanced circular point sprites.
struct View {
    matrix: mat4x4<f32>,
    settings: vec4<f32>, // physical width, height, point diameter, color mode
};
@group(0) @binding(0) var<uniform> view: View;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) intensity: f32,
    @location(2) color: vec4<f32>,
};
struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
fn color(v: Vertex) -> vec3<f32> {
    var rgb = v.color.rgb;
    if view.settings.w > 0.5 && view.settings.w < 1.5 {
        rgb = mix(vec3<f32>(0.12, 0.24, 0.5), vec3<f32>(1.0, 0.8, 0.2), v.intensity);
    }
    if view.settings.w > 1.5 {
        rgb = mix(vec3<f32>(0.18, 0.4, 0.9), vec3<f32>(1.0, 0.6, 0.2), clamp(v.position.y * 0.5 + 0.5, 0.0, 1.0));
    }
    return pow(rgb, vec3<f32>(2.2));
}
fn transform(v: Vertex) -> Fragment {
    var out: Fragment;
    out.position = view.matrix * vec4<f32>(v.position, 1.0);
    out.world = v.position;
    out.color = color(v);
    out.uv = vec2<f32>(0.0);
    return out;
}
@vertex fn vs_mesh(v: Vertex) -> Fragment { return transform(v); }
@vertex fn vs_point(v: Vertex, @builtin(vertex_index) index: u32) -> Fragment {
    let corners = array<vec2<f32>, 6>(vec2(-1.0,-1.0), vec2(1.0,-1.0), vec2(-1.0,1.0),
                                    vec2(-1.0,1.0), vec2(1.0,-1.0), vec2(1.0,1.0));
    var out = transform(v);
    out.uv = corners[index];
    out.position = vec4<f32>(out.position.xy + out.uv * view.settings.z / view.settings.xy * out.position.w,
                            out.position.zw);
    return out;
}
@fragment fn fs_mesh(in: Fragment) -> @location(0) vec4<f32> {
    let crossNormal = cross(dpdx(in.world), dpdy(in.world));
    let normal = crossNormal / max(length(crossNormal), 0.000001);
    let light = 0.28 + 0.72 * abs(dot(normal, normalize(vec3<f32>(0.4,0.7,1.0))));
    return vec4<f32>(in.color * light, 1.0);
}
@fragment fn fs_wire(in: Fragment) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color * 0.3, 1.0);
}
@fragment fn fs_point(in: Fragment) -> @location(0) vec4<f32> {
    let radius = length(in.uv);
    if radius > 1.0 { discard; }
    let alpha = 1.0 - smoothstep(1.0 - max(fwidth(radius), 0.01), 1.0, radius);
    return vec4<f32>(in.color, alpha);
}
