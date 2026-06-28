use crate::models::{Assets, RenderItem, Scene};
use crate::render::compute_matrix;
use wasm_bindgen::prelude::*;
use web_sys::{
    GpuBindGroup, GpuCanvasContext, GpuDevice, GpuIndexFormat, GpuLoadOp,
    GpuRenderPassColorAttachment, GpuRenderPassDepthStencilAttachment, GpuRenderPassDescriptor,
    GpuStoreOp, GpuTexture, GpuTextureView,
};

// Encode et soumet une frame complète.
//
// Bind groups :
//   group(0) = camera_bind_group (partagé, posé une fois avant la boucle)
//   group(1) = model  bind group (par objet, dans material.bind_group)
pub fn draw_scene(
    device: &GpuDevice,
    context: &GpuCanvasContext,
    scene: &Scene,
    assets: &Assets,
    camera_bind_group: Option<&GpuBindGroup>,
    depth_texture: &GpuTexture,
) -> Result<(), JsValue> {
    let texture = context.get_current_texture()?;
    let color_view = texture.create_view()?;

    let encoder    = device.create_command_encoder();
    let pass_desc  = create_pass(&color_view, depth_texture);
    let pass       = encoder.begin_render_pass(&pass_desc)?;

    // Pose le camera bind group une seule fois pour toute la frame.
    if let Some(cbg) = camera_bind_group {
        pass.set_bind_group(0, Some(cbg));
    }

    let items = build_render_items(scene);
    for item in &items {
        let mesh = assets
            .meshes
            .get(item.mesh.index as usize)
            .ok_or_else(|| missing_asset("mesh", item.mesh.index))?;
        let material = assets
            .materials
            .get(item.material.index as usize)
            .ok_or_else(|| missing_asset("material", item.material.index))?;

        pass.set_pipeline(&material.pipeline);

        // Model matrix au group(1).
        if let Some(model_bg) = &material.bind_group {
            pass.set_bind_group(1, Some(model_bg));
        }

        pass.set_vertex_buffer(0, Some(&mesh.vertex_buffer));
        pass.set_index_buffer(&mesh.index_buffer, GpuIndexFormat::Uint16);
        pass.draw_indexed(mesh.index_count);
    }

    pass.end();
    device.queue().submit(&[encoder.finish()]);
    Ok(())
}

fn create_pass(
    color_view: &GpuTextureView,
    depth_texture: &GpuTexture,
) -> GpuRenderPassDescriptor {
    let color_att = GpuRenderPassColorAttachment::new_with_gpu_texture_view(
        GpuLoadOp::Clear,
        GpuStoreOp::Store,
        color_view,
    );
    color_att.set_clear_value(&[0.02_f64.into(), 0.025_f64.into(), 0.035_f64.into(), 1.0_f64.into()]);

    let depth_att = GpuRenderPassDepthStencilAttachment::new(depth_texture);
    depth_att.set_depth_load_op(GpuLoadOp::Clear);
    depth_att.set_depth_store_op(GpuStoreOp::Store);
    depth_att.set_depth_clear_value(1.0_f32);

    let desc = GpuRenderPassDescriptor::new(&[js_sys::JsOption::wrap(color_att)]);
    desc.set_depth_stencil_attachment(&depth_att);
    desc
}

fn missing_asset(kind: &str, index: u32) -> JsValue {
    js_sys::Error::new(&format!("missing {kind} asset at index {index}")).into()
}

pub fn build_render_items(scene: &Scene) -> Vec<RenderItem> {
    let mut items = Vec::new();
    for (index, renderer) in scene.mesh_renderers.iter().enumerate() {
        if let (Some(renderer), Some(transform)) = (renderer, scene.transforms.get(index)) {
            items.push(RenderItem {
                mesh:      renderer.mesh,
                material:  renderer.material,
                transform: compute_matrix(*transform),
            });
        }
    }
    items
}
