use wasm_bindgen::prelude::*;
use web_sys::{GpuBuffer, GpuDevice};

use crate::models_3d;

use super::buffer::create_buffer_with_data;
use super::constants::{GPU_BUFFER_USAGE_COPY_DST, GPU_BUFFER_USAGE_INDEX, GPU_BUFFER_USAGE_VERTEX};

pub struct MeshBuffers {
    pub vertex_buffer: GpuBuffer,
    pub index_buffer: GpuBuffer,
    pub index_count: u32,
}

pub fn create_ico_sphere_buffers(device: &GpuDevice) -> Result<MeshBuffers, JsValue> {
    // Step: CPU mesh generation.
    // What this does: creates a subdivided ico sphere in CPU memory. Its vertex
    // vector is interleaved as position.xyz then normal.xyz.
    let mesh = models_3d::ico_sphere::mesh();

    // Step: Vertex buffer upload.
    // What this does: allocates a GPU buffer with VERTEX usage and uploads the
    // interleaved f32 bytes to it.
    let vertex_buffer = create_buffer_with_data(
        device,
        &mesh.vertices,
        GPU_BUFFER_USAGE_VERTEX | GPU_BUFFER_USAGE_COPY_DST,
    )?;

    // Step: Index buffer upload.
    // What this does: allocates a GPU buffer with INDEX usage and uploads u16
    // triangle indices.
    let index_buffer = create_buffer_with_data(
        device,
        &mesh.indices,
        GPU_BUFFER_USAGE_INDEX | GPU_BUFFER_USAGE_COPY_DST,
    )?;

    Ok(MeshBuffers {
        vertex_buffer,
        index_buffer,
        index_count: mesh.index_count(),
    })
}
