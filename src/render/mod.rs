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

pub struct Render {
    device: GpuDevice,
    context: GpuCanvasContext,
    pub scene: Scene,
    assets: Assets,
}

impl Render {
    pub async fn new() -> Result<Self, JsValue> {
        // Step 1/5: Browser canvas setup.
        let canvas = browser::canvas_from_document()?;

        // Step 2/5: WebGPU device and canvas context setup.
        let gpu_state = gpu::initialize(&canvas).await?;

        // Step 3/5: Pipeline setup for the default object.
        let pipeline = pipeline::create_ico_sphere_pipeline(&gpu_state.device, gpu_state.format)?;

        // Step 4/5: Mesh data and GPU buffers for the default object.
        let mesh_buffers = mesh_buffers::create_ico_sphere_buffers(&gpu_state.device)?;

        // Build scene and add the default ico sphere object.
        let mesh_handle = Handle::new(0);
        let material_handle = Handle::new(0);
        let mut scene = Scene::new();
        scene.add_mesh_renderer(Transform::identity(), mesh_handle, material_handle);

        let assets = Assets {
            meshes: vec![Mesh {
                vertex_buffer: mesh_buffers.vertex_buffer,
                index_buffer: mesh_buffers.index_buffer,
                index_count: mesh_buffers.index_count,
            }],
            materials: vec![Material {
                pipeline,
                bind_group: None,
            }],
        };

        Ok(Self {
            device: gpu_state.device,
            context: gpu_state.context,
            scene,
            assets,
        })
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
    let position = Vector3::new(
        transform.position[0],
        transform.position[1],
        transform.position[2],
    );
    let rotation = if transform.rotation == [0.0, 0.0, 0.0, 0.0] {
        Quaternion::new(1.0, 0.0, 0.0, 0.0)
    } else {
        Quaternion::new(
            transform.rotation[3],
            transform.rotation[0],
            transform.rotation[1],
            transform.rotation[2],
        )
        .normalize()
    };
    let scale = Vector3::new(transform.scale[0], transform.scale[1], transform.scale[2]);

    let translation = Matrix4::from_translation(position);
    let rotation = Matrix4::from(rotation);
    let scaling = Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);

    (translation * rotation * scaling).into()
}
