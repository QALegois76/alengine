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

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let light = normalize(vec3<f32>(0.35, 0.55, 1.0));
    let n = normalize(input.normal);
    // approximate view direction as coming from +Z
    let view_dir = normalize(vec3<f32>(0.0, 0.0, 1.0));

    // diffuse and specular
    let diff = max(dot(n, light), 0.0);
    let reflect_dir = normalize(2.0 * dot(n, light) * n - light);
    let spec = pow(max(dot(reflect_dir, view_dir), 0.0), 64.0);

    // fresnel-like rim effect
    let fresnel = pow(1.0 - max(dot(n, view_dir), 0.0), 3.0);

    let base = vec3<f32>(0.6, 0.8, 0.95);
    // combine translucent base with rim and specular highlights
    let color = base * (0.2 + diff * 0.6) + vec3<f32>(0.9, 0.95, 1.0) * fresnel * 0.6 + vec3<f32>(1.0) * spec * 0.6;
    // slightly desaturated to mimic glass
    let final = mix(color * 0.9, vec3<f32>(1.0), 0.06);
    return vec4<f32>(final, 1.0);
}