use wasm_bindgen::prelude::*;
use web_sys::{
    GpuAutoLayoutMode, GpuColorTargetState, GpuDevice, GpuFragmentState, GpuRenderPipeline,
    GpuRenderPipelineDescriptor, GpuShaderModuleDescriptor, GpuTextureFormat, GpuVertexAttribute,
    GpuVertexBufferLayout, GpuVertexFormat, GpuVertexState,
};

use crate::models_3d;

use super::constants::{NORMAL_ATTRIBUTE_OFFSET, POSITION_ATTRIBUTE_OFFSET, VERTEX_STRIDE};

pub fn create_ico_sphere_pipeline(
    device: &GpuDevice,
    format: GpuTextureFormat,
) -> Result<GpuRenderPipeline, JsValue> {
    // Step: Shader module creation.
    // What this does: compiles WGSL source embedded in the wasm into a shader
    // module. The WGSL defines vertex and fragment entry points.
    let shader = device.create_shader_module(&GpuShaderModuleDescriptor::new(models_3d::ico_sphere::SHADER));

    // Step: Vertex state and attribute layout.
    // What this does: tells WebGPU that each vertex contains position at
    // `@location(0)` and normal at `@location(1)`, both as `vec3<f32>`.
    let vertex = GpuVertexState::new(&shader);
    vertex.set_entry_point("vs_main");
    vertex.set_buffers(&[js_sys::JsOption::wrap(ico_sphere_vertex_layout())]);

    // Step: Fragment target setup.
    // What this does: declares that the fragment shader writes one color target
    // using the same texture format as the configured canvas.
    let target = js_sys::JsOption::wrap(GpuColorTargetState::new(format));
    let fragment = GpuFragmentState::new(&shader, &[target]);
    fragment.set_entry_point("fs_main");

    // Step: Render pipeline creation.
    // What this does: creates the immutable GPU pipeline object used during draw.
    let descriptor =
        GpuRenderPipelineDescriptor::new_with_gpu_auto_layout_mode(GpuAutoLayoutMode::Auto, &vertex);
    descriptor.set_fragment(&fragment);
    device.create_render_pipeline(&descriptor)
}

fn ico_sphere_vertex_layout() -> GpuVertexBufferLayout {
    let position = GpuVertexAttribute::new(GpuVertexFormat::Float32x3, POSITION_ATTRIBUTE_OFFSET, 0);
    let normal = GpuVertexAttribute::new(GpuVertexFormat::Float32x3, NORMAL_ATTRIBUTE_OFFSET, 1);
    GpuVertexBufferLayout::new(VERTEX_STRIDE, &[position, normal])
}
