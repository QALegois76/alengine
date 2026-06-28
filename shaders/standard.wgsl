// standard.wgsl — Shader diffus standard avec support caméra (view + projection).
//
// Bind groups :
//   group(0) binding(0) : CameraUniform  — partagé pour toute la frame
//   group(1) binding(0) : ModelUniform   — par objet (model matrix)
//
// Ce shader remplacera ico_sphere.wgsl une fois CameraSystem implémenté.
// Actuel ico_sphere.wgsl n'utilise que group(0) pour la model matrix (sans caméra).

struct CameraUniform {
    view_proj:     mat4x4<f32>,
    view_position: vec4<f32>,
}

struct ModelUniform {
    model: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> model_data: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       world_normal:  vec3<f32>,
    @location(1)       world_pos:     vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let world_pos4   = model_data.model * vec4<f32>(in.position, 1.0);
    let world_normal = normalize((model_data.model * vec4<f32>(in.normal, 0.0)).xyz);

    out.clip_position = camera.view_proj * world_pos4;
    out.world_normal  = world_normal;
    out.world_pos     = world_pos4.xyz;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir  = normalize(vec3<f32>(0.35, 0.55, 1.0));
    let view_dir   = normalize(camera.view_position.xyz - in.world_pos);
    let half_vec   = normalize(light_dir + view_dir);

    // Albedo depuis la normale (visualisation de débogage).
    let albedo     = in.world_normal * 0.5 + 0.5;

    // Éclairage Blinn-Phong simplifié.
    let diffuse    = max(dot(in.world_normal, light_dir), 0.0);
    let specular   = pow(max(dot(in.world_normal, half_vec), 0.0), 32.0) * 0.3;
    let ambient    = 0.08;

    let color = albedo * (ambient + diffuse * 0.75) + specular;
    return vec4<f32>(color, 1.0);
}
