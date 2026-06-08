mod browser;
mod buffer;
mod constants;
mod frame;
mod gpu;
mod mesh_buffers;
mod pipeline;

use wasm_bindgen::prelude::*;
use web_sys::{GpuBuffer, GpuCanvasContext, GpuDevice, GpuRenderPipeline};

pub struct Render {
    device: GpuDevice,
    context: GpuCanvasContext,
    pipeline: GpuRenderPipeline,
    vertex_buffer: GpuBuffer,
    index_buffer: GpuBuffer,
    index_count: u32,
}

impl Render {
    pub async fn new() -> Result<Self, JsValue> {
        // Step 1/5: Browser canvas setup.
        let canvas = browser::canvas_from_document()?;

        // Step 2/5: WebGPU device and canvas context setup.
        let gpu_state = gpu::initialize(&canvas).await?;

        // Step 3/5: Pipeline setup.
        let pipeline = pipeline::create_ico_sphere_pipeline(&gpu_state.device, gpu_state.format)?;

        // Step 4/5: Mesh data and GPU buffers.
        let mesh_buffers = mesh_buffers::create_ico_sphere_buffers(&gpu_state.device)?;

        Ok(Self {
            device: gpu_state.device,
            context: gpu_state.context,
            pipeline,
            vertex_buffer: mesh_buffers.vertex_buffer,
            index_buffer: mesh_buffers.index_buffer,
            index_count: mesh_buffers.index_count,
        })
    }

    pub fn draw_frame(&self) -> Result<(), JsValue> {
        // Step 5/5: Frame encoding and submission.
        frame::draw_ico_sphere(
            &self.device,
            &self.context,
            &self.pipeline,
            &self.vertex_buffer,
            &self.index_buffer,
            self.index_count,
        )
    }
}

fn gpu_error(message: &str) -> JsValue {
    js_sys::Error::new(message).into()
}
