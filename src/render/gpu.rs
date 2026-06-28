use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    GpuCanvasConfiguration, GpuCanvasContext, GpuDevice, GpuTexture, GpuTextureDescriptor,
    GpuTextureFormat, HtmlCanvasElement,
};

use super::constants::GPU_TEXTURE_USAGE_RENDER_ATTACHMENT;

use super::gpu_error;

pub struct GpuState {
    pub device: GpuDevice,
    pub context: GpuCanvasContext,
    pub format: GpuTextureFormat,
}

pub async fn initialize(canvas: &HtmlCanvasElement) -> Result<GpuState, JsValue> {
    let window = web_sys::window().ok_or_else(|| gpu_error("window is not available"))?;
    let gpu = window.navigator().gpu();
    let adapter = JsFuture::from(gpu.request_adapter())
        .await?
        .dyn_into::<web_sys::GpuAdapter>()
        .map_err(|_| gpu_error("WebGPU adapter request returned nothing"))?;
    let device = JsFuture::from(adapter.request_device())
        .await?
        .dyn_into::<GpuDevice>()?;

    let context = canvas
        .get_context("webgpu")?
        .ok_or_else(|| gpu_error("could not get a WebGPU canvas context"))?
        .dyn_into::<GpuCanvasContext>()?;
    let format = gpu.get_preferred_canvas_format();
    let configuration = GpuCanvasConfiguration::new(&device, format);
    context.configure(&configuration)?;

    Ok(GpuState { device, context, format })
}

// Crée la texture de profondeur et sa vue.
// Appelé une fois à l'init. En cas de resize, il faut recréer (non géré ici pour l'instant).
// Retourne la GpuTexture (pas la view) car GpuRenderPassDepthStencilAttachment::new
// prend &GpuTexture dans web-sys 0.3.97.
pub fn create_depth_texture(
    device: &GpuDevice,
    width: u32,
    height: u32,
) -> Result<GpuTexture, JsValue> {
    // web-sys 0.3.97 : GpuTextureDescriptor::new(format, size: &[Number], usage)
    let size = [
        js_sys::Number::from(width as f64),
        js_sys::Number::from(height as f64),
        js_sys::Number::from(1.0_f64),
    ];

    let desc = GpuTextureDescriptor::new(
        GpuTextureFormat::Depth24plus,
        &size,
        GPU_TEXTURE_USAGE_RENDER_ATTACHMENT,
    );

    device.create_texture(&desc)
}
