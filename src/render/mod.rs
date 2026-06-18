mod browser;
mod buffer;
mod constants;
mod frame;
mod gpu;
mod mesh_buffers;
mod pipeline;

use crate::models::{Assets, Handle, Material, Mesh, Scene, Transform};
use cgmath::{InnerSpace, Matrix4, Quaternion, Vector3};
use wasm_bindgen::prelude::*;
use web_sys::{GpuCanvasContext, GpuDevice};

#[wasm_bindgen]
pub struct Render {
    device: GpuDevice,
    context: GpuCanvasContext,
    format: web_sys::GpuTextureFormat,
    scene: Scene,
    assets: Assets,
}

#[wasm_bindgen]
impl Render {
    pub async fn create() -> Result<Render, JsValue> {
        // Step 1/5: Browser canvas setup.
        let canvas = browser::canvas_from_document()?;

        // Step 2/5: WebGPU device and canvas context setup.
        let gpu_state = gpu::initialize(&canvas).await?;

        let scene = Scene::new();
        let assets = Assets {
            meshes: Vec::new(),
            materials: Vec::new(),
        };

        Ok(Self {
            device: gpu_state.device,
            context: gpu_state.context,
            format: gpu_state.format,
            scene,
            assets,
        })
    }

    pub fn add_sphere(
        &mut self,
        transform: Transform,
        shader_source: Option<String>,
    ) -> Result<(), JsValue> {
        let mesh_buffers = mesh_buffers::create_ico_sphere_buffers(&self.device)?;
        let mesh = Mesh {
            vertex_buffer: mesh_buffers.vertex_buffer,
            index_buffer: mesh_buffers.index_buffer,
            index_count: mesh_buffers.index_count,
        };
        let mesh_index = self.assets.meshes.len() as u32;
        self.assets.meshes.push(mesh);

        let shader = shader_source.as_deref().unwrap_or(crate::models_3d::ico_sphere::SHADER);
        let pipeline = pipeline::create_pipeline_from_shader(&self.device, self.format, shader)?;

        // Create uniform buffer for transform
        let matrix = compute_matrix(transform);
        let matrix_flattened: Vec<f32> = matrix.iter().flatten().copied().collect();
        let matrix_u8 = unsafe {
            std::slice::from_raw_parts(
                matrix_flattened.as_ptr() as *const u8,
                matrix_flattened.len() * 4,
            )
        };

        let usage = 0x0040 | 0x0008; // UNIFORM | COPY_DST
        let descriptor = web_sys::GpuBufferDescriptor::new((matrix_flattened.len() * 4) as u32, usage);
        descriptor.set_mapped_at_creation(true);
        let buffer = self.device.create_buffer(&descriptor)?;
        let array_buffer = buffer.get_mapped_range()?;
        js_sys::Uint8Array::new(&array_buffer).copy_from(matrix_u8);
        buffer.unmap();

        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let entry = web_sys::GpuBindGroupEntry::new_with_gpu_buffer(0, &buffer);
        let bind_group = self.device.create_bind_group(&web_sys::GpuBindGroupDescriptor::new(
            &[entry],
            &bind_group_layout,
        ));

        let material = Material {
            pipeline,
            bind_group: Some(bind_group),
        };
        let material_index = self.assets.materials.len() as u32;
        self.assets.materials.push(material);

        self.scene.add_mesh_renderer(
            transform,
            Handle::new(mesh_index),
            Handle::new(material_index),
        );

        Ok(())
    }

    pub fn draw_frame(&self) -> Result<(), JsValue> {
        // Step 5/5: Frame encoding and submission for the whole scene.
        frame::draw_scene(&self.device, &self.context, &self.scene, &self.assets)
    }
}

fn gpu_error(message: &str) -> JsValue {
    js_sys::Error::new(message).into()
}

fn compute_matrix(transform: Transform) -> [[f32; 4]; 4] {
    let position = Vector3::new(transform.x, transform.y, transform.z);
    let rotation = if transform.rx == 0.0 && transform.ry == 0.0 && transform.rz == 0.0 && transform.rw == 0.0
    {
        Quaternion::new(1.0, 0.0, 0.0, 0.0)
    } else {
        Quaternion::new(transform.rw, transform.rx, transform.ry, transform.rz).normalize()
    };
    let scale = Vector3::new(transform.sx, transform.sy, transform.sz);

    let translation = Matrix4::from_translation(position);
    let rotation = Matrix4::from(rotation);
    let scaling = Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);

    (translation * rotation * scaling).into()
}
