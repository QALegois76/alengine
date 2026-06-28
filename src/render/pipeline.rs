use wasm_bindgen::prelude::*;
use web_sys::{
    GpuAutoLayoutMode, GpuColorTargetState, GpuCompareFunction, GpuDepthStencilState, GpuDevice,
    GpuFragmentState, GpuRenderPipeline, GpuRenderPipelineDescriptor, GpuShaderModuleDescriptor,
    GpuTextureFormat, GpuVertexAttribute, GpuVertexBufferLayout, GpuVertexFormat, GpuVertexState,
};

use super::constants::{NORMAL_ATTRIBUTE_OFFSET, POSITION_ATTRIBUTE_OFFSET, VERTEX_STRIDE};

pub fn create_ico_sphere_pipeline(
    device: &GpuDevice,
    format: GpuTextureFormat,
) -> Result<GpuRenderPipeline, JsValue> {
    create_pipeline_from_shader(device, format, crate::models_3d::ico_sphere::SHADER)
}

pub fn create_pipeline_from_shader(
    device: &GpuDevice,
    format: GpuTextureFormat,
    shader_source: &str,
) -> Result<GpuRenderPipeline, JsValue> {
    let shader = device.create_shader_module(&GpuShaderModuleDescriptor::new(shader_source));

    let vertex = GpuVertexState::new(&shader);
    vertex.set_entry_point("vs_main");
    vertex.set_buffers(&[js_sys::JsOption::wrap(ico_sphere_vertex_layout())]);

    let target   = js_sys::JsOption::wrap(GpuColorTargetState::new(format));
    let fragment = GpuFragmentState::new(&shader, &[target]);
    fragment.set_entry_point("fs_main");

    // Depth/stencil : test Less, écriture activée.
    let depth_stencil = GpuDepthStencilState::new(GpuTextureFormat::Depth24plus);
    depth_stencil.set_depth_write_enabled(true);
    depth_stencil.set_depth_compare(GpuCompareFunction::Less);

    let descriptor = GpuRenderPipelineDescriptor::new_with_gpu_auto_layout_mode(
        GpuAutoLayoutMode::Auto,
        &vertex,
    );
    descriptor.set_fragment(&fragment);
    descriptor.set_depth_stencil(&depth_stencil);

    device.create_render_pipeline(&descriptor)
}

fn ico_sphere_vertex_layout() -> GpuVertexBufferLayout {
    let position =
        GpuVertexAttribute::new(GpuVertexFormat::Float32x3, POSITION_ATTRIBUTE_OFFSET, 0);
    let normal = GpuVertexAttribute::new(GpuVertexFormat::Float32x3, NORMAL_ATTRIBUTE_OFFSET, 1);
    GpuVertexBufferLayout::new(VERTEX_STRIDE, &[position, normal])
}
