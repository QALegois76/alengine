// Bind groups :
//   group(0) binding(0) → CameraUniform  (view_proj + view_pos, partagé par frame)
//   group(1) binding(0) → model matrix   (par objet)

struct CameraUniform {
    view_proj:    mat4x4<f32>,
    view_pos:     vec4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> model_matrix: mat4x4<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos:    vec4<f32>,
    @location(0)       world_pos:   vec3<f32>,
    @location(1)       world_normal: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos4   = model_matrix * vec4<f32>(input.position, 1.0);
    out.clip_pos     = camera.view_proj * world_pos4;
    out.world_pos    = world_pos4.xyz;
    out.world_normal = normalize((model_matrix * vec4<f32>(input.normal, 0.0)).xyz);
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir  = normalize(vec3<f32>(0.35, 0.55, 1.0));
    let view_dir   = normalize(camera.view_pos.xyz - input.world_pos);
    let half_vec   = normalize(light_dir + view_dir);

    let base_color = input.world_normal * 0.5 + vec3<f32>(0.5);
    let diffuse    = max(dot(input.world_normal, light_dir), 0.0);
    let specular   = pow(max(dot(input.world_normal, half_vec), 0.0), 48.0) * 0.25;
    let ambient    = 0.06;

    let color = base_color * (ambient + diffuse * 0.75) + specular;
    return vec4<f32>(color, 1.0);
}
