mod utils;
#[path = "3d_models/mod.rs"]
mod models_3d;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    GpuAutoLayoutMode, GpuBuffer, GpuBufferDescriptor, GpuCanvasConfiguration, GpuCanvasContext,
    GpuColorTargetState, GpuFragmentState, GpuIndexFormat, GpuLoadOp,
    GpuRenderPassColorAttachment, GpuRenderPassDescriptor, GpuRenderPipelineDescriptor,
    GpuShaderModuleDescriptor, GpuStoreOp, GpuVertexAttribute, GpuVertexBufferLayout,
    GpuVertexFormat, GpuVertexState, HtmlCanvasElement,
};

const GPU_BUFFER_USAGE_COPY_DST: u32 = 0x8;
const GPU_BUFFER_USAGE_INDEX: u32 = 0x10;
const GPU_BUFFER_USAGE_VERTEX: u32 = 0x20;
const POSITION_ATTRIBUTE_OFFSET: u32 = 0;
const NORMAL_ATTRIBUTE_OFFSET: u32 = 12;
const VERTEX_STRIDE: u32 = 24;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

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

    let shader = device.create_shader_module(&GpuShaderModuleDescriptor::new(models_3d::ico_sphere::SHADER));
    let vertex = GpuVertexState::new(&shader);
    vertex.set_entry_point("vs_main");
    vertex.set_buffers(&[js_sys::JsOption::wrap(ico_sphere_vertex_layout())]);

    let target = js_sys::JsOption::wrap(GpuColorTargetState::new(format));
    let fragment = GpuFragmentState::new(&shader, &[target]);
    fragment.set_entry_point("fs_main");

    let pipeline_descriptor =
        GpuRenderPipelineDescriptor::new_with_gpu_auto_layout_mode(GpuAutoLayoutMode::Auto, &vertex);
    pipeline_descriptor.set_fragment(&fragment);
    let pipeline = device.create_render_pipeline(&pipeline_descriptor)?;

    let vertex_data = models_3d::ico_sphere::interleaved_vertices();
    let vertex_buffer = create_buffer_with_data(
        &device,
        &vertex_data,
        GPU_BUFFER_USAGE_VERTEX | GPU_BUFFER_USAGE_COPY_DST,
    )?;
    let index_buffer = create_buffer_with_data(
        &device,
        &models_3d::ico_sphere::INDICES,
        GPU_BUFFER_USAGE_INDEX | GPU_BUFFER_USAGE_COPY_DST,
    )?;

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
    render_pass.set_vertex_buffer(0, Some(&vertex_buffer));
    render_pass.set_index_buffer(&index_buffer, GpuIndexFormat::Uint16);
    render_pass.draw_indexed(models_3d::ico_sphere::INDEX_COUNT);
    render_pass.end();

    let command_buffer = command_encoder.finish();
    device.queue().submit(&[command_buffer]);

    Ok(())
}

fn js_error(message: &str) -> JsValue {
    js_sys::Error::new(message).into()
}

fn ico_sphere_vertex_layout() -> GpuVertexBufferLayout {
    let position = GpuVertexAttribute::new(GpuVertexFormat::Float32x3, POSITION_ATTRIBUTE_OFFSET, 0);
    let normal = GpuVertexAttribute::new(GpuVertexFormat::Float32x3, NORMAL_ATTRIBUTE_OFFSET, 1);
    GpuVertexBufferLayout::new(VERTEX_STRIDE, &[position, normal])
}

fn create_buffer_with_data<T>(
    device: &web_sys::GpuDevice,
    data: &[T],
    usage: u32,
) -> Result<GpuBuffer, JsValue> {
    let bytes = bytes_of(data);
    let buffer = device.create_buffer(&GpuBufferDescriptor::new(bytes.len() as u32, usage))?;
    device
        .queue()
        .write_buffer_with_u32_and_u8_slice(&buffer, 0, bytes)?;
    Ok(buffer)
}

fn bytes_of<T>(data: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            data.as_ptr().cast::<u8>(),
            std::mem::size_of_val(data),
        )
    }
}
