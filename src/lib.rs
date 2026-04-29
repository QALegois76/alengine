mod utils;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    GpuAutoLayoutMode, GpuCanvasConfiguration, GpuCanvasContext, GpuColorTargetState,
    GpuFragmentState, GpuLoadOp, GpuRenderPassColorAttachment, GpuRenderPassDescriptor,
    GpuRenderPipelineDescriptor, GpuShaderModuleDescriptor, GpuStoreOp, GpuVertexState,
    HtmlCanvasElement,
};

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const SHADER: &str = include_str!("shaders/triangle.wgsl");

#[wasm_bindgen]
pub async fn run() -> Result<(), JsValue> {
    utils::set_panic_hook();

    let window = web_sys::window().ok_or_else(|| js_error("window is not available"))?;
    let document = window
        .document()
        .ok_or_else(|| js_error("document is not available"))?;
    let canvas = document
        .get_element_by_id("canvas")
        .ok_or_else(|| js_error("missing canvas element with id `canvas`"))?
        .dyn_into::<HtmlCanvasElement>()?;

    let width = canvas.client_width().max(1) as u32;
    let height = canvas.client_height().max(1) as u32;
    canvas.set_width(width);
    canvas.set_height(height);

    let gpu = window.navigator().gpu();
    let adapter = JsFuture::from(gpu.request_adapter())
        .await?
        .dyn_into::<web_sys::GpuAdapter>()
        .map_err(|_| js_error("WebGPU adapter request returned nothing"))?;
    let device = JsFuture::from(adapter.request_device())
        .await?
        .dyn_into::<web_sys::GpuDevice>()?;

    let context = canvas
        .get_context("webgpu")?
        .ok_or_else(|| js_error("could not get a WebGPU canvas context"))?
        .dyn_into::<GpuCanvasContext>()?;
    let format = gpu.get_preferred_canvas_format();
    let configuration = GpuCanvasConfiguration::new(&device, format);
    context.configure(&configuration)?;

    let shader = device.create_shader_module(&GpuShaderModuleDescriptor::new(SHADER));
    let vertex = GpuVertexState::new(&shader);
    vertex.set_entry_point("vs_main");

    let target = js_sys::JsOption::wrap(GpuColorTargetState::new(format));
    let fragment = GpuFragmentState::new(&shader, &[target]);
    fragment.set_entry_point("fs_main");

    let pipeline_descriptor =
        GpuRenderPipelineDescriptor::new_with_gpu_auto_layout_mode(GpuAutoLayoutMode::Auto, &vertex);
    pipeline_descriptor.set_fragment(&fragment);
    let pipeline = device.create_render_pipeline(&pipeline_descriptor)?;

    let texture = context.get_current_texture()?;
    let view = texture.create_view()?;
    let color_attachment =
        GpuRenderPassColorAttachment::new_with_gpu_texture_view(GpuLoadOp::Clear, GpuStoreOp::Store, &view);
    color_attachment.set_clear_value(&[0.02.into(), 0.025.into(), 0.035.into(), 1.0.into()]);

    let color_attachment = js_sys::JsOption::wrap(color_attachment);
    let render_pass_descriptor = GpuRenderPassDescriptor::new(&[color_attachment]);
    let command_encoder = device.create_command_encoder();
    let render_pass = command_encoder.begin_render_pass(&render_pass_descriptor)?;
    render_pass.set_pipeline(&pipeline);
    render_pass.draw(3);
    render_pass.end();

    let command_buffer = command_encoder.finish();
    device.queue().submit(&[command_buffer]);

    Ok(())
}

fn js_error(message: &str) -> JsValue {
    js_sys::Error::new(message).into()
}
