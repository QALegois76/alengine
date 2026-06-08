use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{GpuCanvasConfiguration, GpuCanvasContext, GpuDevice, GpuTextureFormat, HtmlCanvasElement};

use super::gpu_error;

pub struct GpuState {
    pub device: GpuDevice,
    pub context: GpuCanvasContext,
    pub format: GpuTextureFormat,
}

pub async fn initialize(canvas: &HtmlCanvasElement) -> Result<GpuState, JsValue> {
    // Step: Adapter/device request.
    // What this does: requests a WebGPU adapter and logical device from the
    // browser. The device is the handle used to create buffers, pipelines, and
    // command encoders.
    let window = web_sys::window().ok_or_else(|| gpu_error("window is not available"))?;
    let gpu = window.navigator().gpu();
    let adapter = JsFuture::from(gpu.request_adapter())
        .await?
        .dyn_into::<web_sys::GpuAdapter>()
        .map_err(|_| gpu_error("WebGPU adapter request returned nothing"))?;
    let device = JsFuture::from(adapter.request_device())
        .await?
        .dyn_into::<GpuDevice>()?;

    // Step: Canvas WebGPU context configuration.
    // What this does: gets the `webgpu` context from the canvas and configures it
    // with the browser-preferred texture format.
    let context = canvas
        .get_context("webgpu")?
        .ok_or_else(|| gpu_error("could not get a WebGPU canvas context"))?
        .dyn_into::<GpuCanvasContext>()?;
    let format = gpu.get_preferred_canvas_format();
    let configuration = GpuCanvasConfiguration::new(&device, format);
    context.configure(&configuration)?;

    Ok(GpuState {
        device,
        context,
        format,
    })
}
