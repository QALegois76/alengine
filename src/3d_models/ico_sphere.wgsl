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
    let diffuse = max(dot(normalize(input.normal), light), 0.0);
    let base_color = input.normal * 0.5 + vec3<f32>(0.5);
    let color = base_color * (0.25 + diffuse * 0.75);
    return vec4<f32>(color, 1.0);
}
