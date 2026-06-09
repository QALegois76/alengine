use crate::models::{Assets, RenderItem, Scene};
use crate::render::compute_matrix;
use wasm_bindgen::prelude::*;
use web_sys::{
    GpuCanvasContext, GpuDevice, GpuIndexFormat, GpuLoadOp, GpuRenderPassColorAttachment,
    GpuRenderPassDescriptor, GpuStoreOp, GpuTextureView,
};

// Draw an entire scene composed of objects. Each object provides its own
// pipeline and buffers; we bind them in turn and issue indexed draws.
pub fn draw_scene(
    device: &GpuDevice,
    context: &GpuCanvasContext,
    scene: &Scene,
    assets: &Assets,
) -> Result<(), JsValue> {
    let texture = context.get_current_texture()?;
    let view = texture.create_view()?;

    let command_encoder = device.create_command_encoder();
    let pass_descriptor = create_pass(&view);
    let pass = command_encoder.begin_render_pass(&pass_descriptor)?;

    let items = build_render_items(scene);

    for item in items {
        let mesh = assets
            .meshes
            .get(item.mesh.index as usize)
            .ok_or_else(|| missing_asset("mesh", item.mesh.index))?;
        let material = assets
            .materials
            .get(item.material.index as usize)
            .ok_or_else(|| missing_asset("material", item.material.index))?;

        pass.set_pipeline(&material.pipeline);
        pass.set_vertex_buffer(0, Some(&mesh.vertex_buffer));
        pass.set_index_buffer(&mesh.index_buffer, GpuIndexFormat::Uint16);

        pass.draw_indexed(mesh.index_count);
    }

    pass.end();

    device.queue().submit(&[command_encoder.finish()]);
    Ok(())
}

fn create_pass(view: &GpuTextureView) -> GpuRenderPassDescriptor {
    let color_attachment = GpuRenderPassColorAttachment::new_with_gpu_texture_view(
        GpuLoadOp::Clear,
        GpuStoreOp::Store,
        view,
    );
    color_attachment.set_clear_value(&[0.02.into(), 0.025.into(), 0.035.into(), 1.0.into()]);
    GpuRenderPassDescriptor::new(&[js_sys::JsOption::wrap(color_attachment)])
}

fn missing_asset(kind: &str, index: u32) -> JsValue {
    js_sys::Error::new(&format!("missing {kind} asset at index {index}")).into()
}

pub fn build_render_items(scene: &Scene) -> Vec<RenderItem> {
    let mut items = Vec::new();

    for (index, renderer) in scene.mesh_renderers.iter().enumerate() {
        if let (Some(renderer), Some(transform)) = (renderer, scene.transforms.get(index)) {
            items.push(RenderItem {
                mesh: renderer.mesh,
                material: renderer.material,
                transform: compute_matrix(*transform),
            });
        }
    }

    items
}
