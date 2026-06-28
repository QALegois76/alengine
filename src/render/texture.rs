// Support texture : pipeline texturé + upload d'ImageBitmap vers une texture GPU.
// Utilisé par le fond de carte (tuiles satellite WMTS).

use wasm_bindgen::prelude::*;
use web_sys::{
    GpuAddressMode, GpuBindGroupLayout, GpuBindGroupLayoutDescriptor, GpuBindGroupLayoutEntry,
    GpuCopyExternalImageDestInfo, GpuCopyExternalImageSourceInfo, GpuDevice, GpuExtent3dDict,
    GpuFilterMode, GpuSampler, GpuSamplerBindingLayout, GpuSamplerDescriptor, GpuTexture,
    GpuTextureBindingLayout, GpuTextureDescriptor, GpuTextureFormat, ImageBitmap,
};

use super::constants::{
    GPU_SHADER_STAGE_FRAGMENT, GPU_TEXTURE_USAGE_COPY_DST, GPU_TEXTURE_USAGE_RENDER_ATTACHMENT,
    GPU_TEXTURE_USAGE_TEXTURE_BINDING,
};

// Shader texturé : échantillonne une texture sur un quad.
// group(2) = texture + sampler. L'UV est passé dans location(1).xy (stride 24,
// comme tous les meshes), .z est ignoré.
pub const TEXTURED_SHADER: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos:  vec4<f32>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> model_matrix: mat4x4<f32>;
@group(2) @binding(0) var tex: texture_2d<f32>;
@group(2) @binding(1) var samp: sampler;

struct VIn  { @location(0) position: vec3<f32>, @location(1) uv: vec3<f32> }
struct VOut { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32> }

@vertex
fn vs_main(v: VIn) -> VOut {
    var o: VOut;
    o.clip = camera.view_proj * model_matrix * vec4<f32>(v.position, 1.0);
    o.uv   = v.uv.xy;
    return o;
}

@fragment
fn fs_main(i: VOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, i.uv);
}
"#;

// Bind group layout group(2) : texture filtrable + sampler, visibles en fragment.
pub fn create_texture_bind_group_layout(
    device: &GpuDevice,
) -> Result<GpuBindGroupLayout, JsValue> {
    let tex_entry = GpuBindGroupLayoutEntry::new(0, GPU_SHADER_STAGE_FRAGMENT);
    tex_entry.set_texture(&GpuTextureBindingLayout::new());

    let samp_entry = GpuBindGroupLayoutEntry::new(1, GPU_SHADER_STAGE_FRAGMENT);
    samp_entry.set_sampler(&GpuSamplerBindingLayout::new());

    let desc = GpuBindGroupLayoutDescriptor::new(&[tex_entry, samp_entry]);
    device.create_bind_group_layout(&desc)
}

// Sampler linéaire, clamp aux bords (évite le bleeding entre tuiles voisines).
pub fn create_sampler(device: &GpuDevice) -> GpuSampler {
    let desc = GpuSamplerDescriptor::new();
    desc.set_mag_filter(GpuFilterMode::Linear);
    desc.set_min_filter(GpuFilterMode::Linear);
    desc.set_address_mode_u(GpuAddressMode::ClampToEdge);
    desc.set_address_mode_v(GpuAddressMode::ClampToEdge);
    device.create_sampler_with_descriptor(&desc)
}

// Crée une texture RGBA sRGB et y copie l'ImageBitmap (décodage par le navigateur).
pub fn upload_bitmap(device: &GpuDevice, bitmap: &ImageBitmap) -> Result<GpuTexture, JsValue> {
    let width = bitmap.width();
    let height = bitmap.height();

    let size = [
        js_sys::Number::from(width as f64),
        js_sys::Number::from(height as f64),
        js_sys::Number::from(1.0_f64),
    ];
    // copyExternalImageToTexture exige COPY_DST + RENDER_ATTACHMENT ; TEXTURE_BINDING
    // pour l'échantillonnage.
    let desc = GpuTextureDescriptor::new(
        GpuTextureFormat::Rgba8unormSrgb,
        &size,
        GPU_TEXTURE_USAGE_COPY_DST
            | GPU_TEXTURE_USAGE_TEXTURE_BINDING
            | GPU_TEXTURE_USAGE_RENDER_ATTACHMENT,
    );
    let texture = device.create_texture(&desc)?;

    let source = GpuCopyExternalImageSourceInfo::new(bitmap);
    let dest = GpuCopyExternalImageDestInfo::new(&texture);
    let copy_size = GpuExtent3dDict::new(width);
    copy_size.set_height(height);

    device
        .queue()
        .copy_external_image_to_texture_with_gpu_extent_3d_dict(&source, &dest, &copy_size)?;

    Ok(texture)
}
