use wasm_bindgen::prelude::*;
use web_sys::{GpuBuffer, GpuDevice};

use crate::models_3d;

use super::buffer::create_buffer_with_data;
use super::constants::{
    GPU_BUFFER_USAGE_COPY_DST, GPU_BUFFER_USAGE_INDEX, GPU_BUFFER_USAGE_VERTEX,
};

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

    create_buffers_from_mesh(device, &mesh.vertices, &mesh.indices)
}

pub fn create_buffers_from_mesh(
    device: &GpuDevice,
    vertices: &[f32],
    indices: &[u16],
) -> Result<MeshBuffers, JsValue> {
    // Step: Vertex buffer upload.
    let vertex_buffer = create_buffer_with_data(
        device,
        vertices,
        GPU_BUFFER_USAGE_VERTEX | GPU_BUFFER_USAGE_COPY_DST,
    )?;

    // Step: Index buffer upload.
    let index_buffer = create_buffer_with_data(
        device,
        indices,
        GPU_BUFFER_USAGE_INDEX | GPU_BUFFER_USAGE_COPY_DST,
    )?;

    Ok(MeshBuffers {
        vertex_buffer,
        index_buffer,
        index_count: indices.len() as u32,
    })
}
