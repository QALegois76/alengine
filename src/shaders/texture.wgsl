// Procedural spherical "texture" (checker-like) based on normal.
const PI: f32 = 3.14159265359;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position * 0.75, 1.0);
    output.normal = normalize(input.normal);
    return output;
}

fn spherical_uv(n: vec3<f32>) -> vec2<f32> {
    let u = 0.5 + atan2(n.z, n.x) / (2.0 * PI);
    let v = 0.5 - asin(n.y) / PI;
    return vec2<f32>(u, v);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(input.normal);
    let uv = spherical_uv(n);
    let scale = 12.0;
    let s = sin(uv.x * scale * PI) * sin(uv.y * scale * PI);
    let checker = select(vec3<f32>(0.15, 0.2, 0.7), vec3<f32>(0.9, 0.9, 0.25), s);
    // lighting
    let light = normalize(vec3<f32>(0.35, 0.55, 1.0));
    let diff = max(dot(n, light), 0.0);
    let color = checker * (0.25 + diff * 0.75);
    return vec4<f32>(color, 1.0);
}