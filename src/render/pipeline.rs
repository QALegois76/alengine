use wasm_bindgen::prelude::*;
use web_sys::{
    GpuBindGroupLayout, GpuBindGroupLayoutDescriptor, GpuBindGroupLayoutEntry, GpuBlendComponent,
    GpuBlendFactor, GpuBlendOperation, GpuBlendState, GpuBufferBindingLayout, GpuBufferBindingType,
    GpuColorTargetState, GpuCompareFunction, GpuDepthStencilState, GpuDevice, GpuFragmentState,
    GpuPipelineLayout, GpuPipelineLayoutDescriptor, GpuPrimitiveState, GpuPrimitiveTopology,
    GpuRenderPipeline, GpuRenderPipelineDescriptor, GpuShaderModuleDescriptor, GpuTextureFormat,
    GpuVertexAttribute, GpuVertexBufferLayout, GpuVertexFormat, GpuVertexState,
};

use super::constants::{
    NORMAL_ATTRIBUTE_OFFSET, POSITION_ATTRIBUTE_OFFSET, VERTEX_STRIDE,
};

// Crée un bind group layout contenant un seul uniform buffer au binding 0.
// `visibility` est un masque GPU_SHADER_STAGE_*.
pub fn create_uniform_bind_group_layout(
    device: &GpuDevice,
    visibility: u32,
) -> Result<GpuBindGroupLayout, JsValue> {
    let buffer = GpuBufferBindingLayout::new();
    buffer.set_type(GpuBufferBindingType::Uniform);

    let entry = GpuBindGroupLayoutEntry::new(0, visibility);
    entry.set_buffer(&buffer);

    let desc = GpuBindGroupLayoutDescriptor::new(&[entry]);
    device.create_bind_group_layout(&desc)
}

// Pipeline layout explicite partagé par tous les pipelines :
//   group(0) = caméra, group(1) = model matrix.
// Avec un layout explicite, un même bind group est compatible avec tous les
// pipelines (contrairement à GpuAutoLayoutMode::Auto qui en génère un par pipeline).
pub fn create_scene_pipeline_layout(
    device: &GpuDevice,
    camera_layout: &GpuBindGroupLayout,
    model_layout: &GpuBindGroupLayout,
) -> GpuPipelineLayout {
    let layouts = [
        js_sys::JsOption::wrap(camera_layout.clone()),
        js_sys::JsOption::wrap(model_layout.clone()),
    ];
    let desc = GpuPipelineLayoutDescriptor::new(&layouts);
    device.create_pipeline_layout(&desc)
}

pub fn create_pipeline_from_shader(
    device: &GpuDevice,
    format: GpuTextureFormat,
    layout: &GpuPipelineLayout,
    shader_source: &str,
    topology: GpuPrimitiveTopology,
) -> Result<GpuRenderPipeline, JsValue> {
    let shader = device.create_shader_module(&GpuShaderModuleDescriptor::new(shader_source));

    let vertex = GpuVertexState::new(&shader);
    vertex.set_entry_point("vs_main");
    vertex.set_buffers(&[js_sys::JsOption::wrap(vertex_layout())]);

    let target   = js_sys::JsOption::wrap(GpuColorTargetState::new(format));
    let fragment = GpuFragmentState::new(&shader, &[target]);
    fragment.set_entry_point("fs_main");

    // Depth/stencil : test Less, écriture activée.
    let depth_stencil = GpuDepthStencilState::new(GpuTextureFormat::Depth24plus);
    depth_stencil.set_depth_write_enabled(true);
    depth_stencil.set_depth_compare(GpuCompareFunction::Less);

    // Topologie : triangle-list pour les meshes, line-list pour la grille/axes.
    let primitive = GpuPrimitiveState::new();
    primitive.set_topology(topology);

    let descriptor = GpuRenderPipelineDescriptor::new(layout, &vertex);
    descriptor.set_fragment(&fragment);
    descriptor.set_depth_stencil(&depth_stencil);
    descriptor.set_primitive(&primitive);

    device.create_render_pipeline(&descriptor)
}

// Pipeline pour les grilles infinies : plein écran (aucun vertex buffer, les
// sommets sont générés par vertex_index), alpha blending pour l'antialiasing
// et le fondu, depth test activé mais SANS écriture (le shader écrit frag_depth
// pour être correctement occulté par la scène sans s'auto-occulter entre plans).
pub fn create_grid_pipeline(
    device: &GpuDevice,
    format: GpuTextureFormat,
    layout: &GpuPipelineLayout,
    shader_source: &str,
) -> Result<GpuRenderPipeline, JsValue> {
    let shader = device.create_shader_module(&GpuShaderModuleDescriptor::new(shader_source));

    let vertex = GpuVertexState::new(&shader);
    vertex.set_entry_point("vs_main");
    // Pas de set_buffers : le vertex shader fabrique un triangle plein écran.

    // Blend alpha standard (src.a, 1 - src.a).
    let color_blend = GpuBlendComponent::new();
    color_blend.set_src_factor(GpuBlendFactor::SrcAlpha);
    color_blend.set_dst_factor(GpuBlendFactor::OneMinusSrcAlpha);
    color_blend.set_operation(GpuBlendOperation::Add);
    let alpha_blend = GpuBlendComponent::new();
    alpha_blend.set_src_factor(GpuBlendFactor::One);
    alpha_blend.set_dst_factor(GpuBlendFactor::OneMinusSrcAlpha);
    alpha_blend.set_operation(GpuBlendOperation::Add);
    let blend = GpuBlendState::new(&alpha_blend, &color_blend);

    let target_state = GpuColorTargetState::new(format);
    target_state.set_blend(&blend);
    let target = js_sys::JsOption::wrap(target_state);
    let fragment = GpuFragmentState::new(&shader, &[target]);
    fragment.set_entry_point("fs_main");

    let depth_stencil = GpuDepthStencilState::new(GpuTextureFormat::Depth24plus);
    depth_stencil.set_depth_write_enabled(false);
    depth_stencil.set_depth_compare(GpuCompareFunction::Less);

    let primitive = GpuPrimitiveState::new();
    primitive.set_topology(GpuPrimitiveTopology::TriangleList);

    let descriptor = GpuRenderPipelineDescriptor::new(layout, &vertex);
    descriptor.set_fragment(&fragment);
    descriptor.set_depth_stencil(&depth_stencil);
    descriptor.set_primitive(&primitive);

    device.create_render_pipeline(&descriptor)
}

// Layout sommet interleavé : position.xyz (loc 0) + un second vec3 (loc 1).
// Le second vec3 est la normale pour les meshes, la couleur pour les lignes.
fn vertex_layout() -> GpuVertexBufferLayout {
    let position =
        GpuVertexAttribute::new(GpuVertexFormat::Float32x3, POSITION_ATTRIBUTE_OFFSET, 0);
    let attr1 = GpuVertexAttribute::new(GpuVertexFormat::Float32x3, NORMAL_ATTRIBUTE_OFFSET, 1);
    GpuVertexBufferLayout::new(VERTEX_STRIDE, &[position, attr1])
}
