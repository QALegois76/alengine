use wasm_bindgen::prelude::*;
use web_sys::{
    GpuBuffer, GpuCanvasContext, GpuDevice, GpuIndexFormat, GpuLoadOp,
    GpuRenderPassColorAttachment, GpuRenderPassDescriptor, GpuRenderPipeline, GpuStoreOp,
};

pub fn draw_ico_sphere(
    device: &GpuDevice,
    context: &GpuCanvasContext,
    pipeline: &GpuRenderPipeline,
    vertex_buffer: &GpuBuffer,
    index_buffer: &GpuBuffer,
    index_count: u32,
) -> Result<(), JsValue> {
    // Step: Current frame texture.
    let texture = context.get_current_texture()?;
    let view = texture.create_view()?;

    // Step: Color attachment description.
    let color_attachment =
        GpuRenderPassColorAttachment::new_with_gpu_texture_view(GpuLoadOp::Clear, GpuStoreOp::Store, &view);
    color_attachment.set_clear_value(&[0.02.into(), 0.025.into(), 0.035.into(), 1.0.into()]);

    let color_attachment = js_sys::JsOption::wrap(color_attachment);
    let render_pass_descriptor = GpuRenderPassDescriptor::new(&[color_attachment]);
    let command_encoder = device.create_command_encoder();

    // Step: Render pass commands.
    // What this does: binds the pipeline, binds mesh buffers, and records one
    // indexed draw call.
    let render_pass = command_encoder.begin_render_pass(&render_pass_descriptor)?;
    render_pass.set_pipeline(pipeline);
    render_pass.set_vertex_buffer(0, Some(vertex_buffer));
    render_pass.set_index_buffer(index_buffer, GpuIndexFormat::Uint16);
    render_pass.draw_indexed(index_count);
    render_pass.end();

    // Step: Queue submission.
    // What this does: finalizes the command encoder into a command buffer and
    // submits it to the GPU queue for execution.
    let command_buffer = command_encoder.finish();
    device.queue().submit(&[command_buffer]);

    Ok(())
}
