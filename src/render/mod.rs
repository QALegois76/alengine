mod browser;
mod buffer;
pub mod camera;
mod constants;
mod frame;
mod gpu;
mod mesh_buffers;
mod pipeline;

use crate::models::{Assets, Handle, Material, Mesh, Scene, Transform};
use cgmath::{InnerSpace, Matrix4, Quaternion, Vector3};
use constants::{
    GPU_BUFFER_USAGE_COPY_DST, GPU_BUFFER_USAGE_UNIFORM,
};
use wasm_bindgen::prelude::*;
use web_sys::{GpuBindGroup, GpuBuffer, GpuCanvasContext, GpuDevice, GpuTexture};

#[wasm_bindgen]
pub struct Render {
    device: GpuDevice,
    context: GpuCanvasContext,
    format: web_sys::GpuTextureFormat,
    scene: Scene,
    assets: Assets,

    // Caméra
    pub(crate) camera: camera::Camera,
    camera_uniform_buffer: GpuBuffer,
    camera_bind_group: Option<GpuBindGroup>, // créé après le premier pipeline (lazy)

    // Depth buffer (GpuTexture directement car DepthStencilAttachment::new prend &GpuTexture)
    depth_texture: GpuTexture,

    // État d'entrée — mis à jour depuis JS via on_* methods, consommé dans tick().
    mouse_dx: f32,
    mouse_dy: f32,
    scroll_delta: f32,
    left_down: bool,
    middle_down: bool,
    right_down: bool,
    key_w: bool,
    key_s: bool,
    key_a: bool,
    key_d: bool,
    key_q: bool,
    key_e: bool,
}

#[wasm_bindgen]
impl Render {
    pub async fn create() -> Result<Render, JsValue> {
        let canvas = browser::canvas_from_document()?;
        let width  = canvas.width().max(1);
        let height = canvas.height().max(1);

        let gpu_state = gpu::initialize(&canvas).await?;
        let depth_texture = gpu::create_depth_texture(&gpu_state.device, width, height)?;

        let mut cam = camera::Camera::default();
        cam.aspect = width as f32 / height as f32;

        // Le camera uniform buffer est créé ici ; son bind group est créé de façon
        // différée dans add_sphere() dès qu'un pipeline (et son layout) est disponible.
        let camera_data = cam.uniform_data();
        let camera_uniform_buffer = buffer::create_buffer_with_data(
            &gpu_state.device,
            &camera_data,
            GPU_BUFFER_USAGE_UNIFORM | GPU_BUFFER_USAGE_COPY_DST,
        )?;

        Ok(Self {
            device: gpu_state.device,
            context: gpu_state.context,
            format: gpu_state.format,
            scene: Scene::new(),
            assets: Assets { meshes: Vec::new(), materials: Vec::new() },
            camera: cam,
            camera_uniform_buffer,
            camera_bind_group: None,
            depth_texture,
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            scroll_delta: 0.0,
            left_down: false,
            middle_down: false,
            right_down: false,
            key_w: false,
            key_s: false,
            key_a: false,
            key_d: false,
            key_q: false,
            key_e: false,
        })
    }

    // Ajoute une sphère ico dans la scène.
    // Le premier appel initialise aussi le camera bind group (lazy).
    pub fn add_sphere(
        &mut self,
        transform: Transform,
        shader_source: Option<String>,
    ) -> Result<(), JsValue> {
        let mesh_buffers = mesh_buffers::create_ico_sphere_buffers(&self.device)?;
        let mesh = Mesh {
            vertex_buffer: mesh_buffers.vertex_buffer,
            index_buffer:  mesh_buffers.index_buffer,
            index_count:   mesh_buffers.index_count,
        };
        let mesh_index = self.assets.meshes.len() as u32;
        self.assets.meshes.push(mesh);

        let shader = shader_source
            .as_deref()
            .unwrap_or(crate::models_3d::ico_sphere::SHADER);
        let pipeline =
            pipeline::create_pipeline_from_shader(&self.device, self.format, shader)?;

        // --- Camera bind group (lazy, une seule fois) ---
        // Utilise le group(0) layout du premier pipeline créé.
        // Tous les shaders doivent suivre la même convention : group(0) = CameraUniform.
        if self.camera_bind_group.is_none() {
            let layout = pipeline.get_bind_group_layout(0);
            let entry  = web_sys::GpuBindGroupEntry::new_with_gpu_buffer(0, &self.camera_uniform_buffer);
            let bg = self.device.create_bind_group(
                &web_sys::GpuBindGroupDescriptor::new(&[entry], &layout),
            );
            self.camera_bind_group = Some(bg);
        }

        // --- Model uniform buffer (group 1) ---
        let matrix = compute_matrix(transform);
        let matrix_flat: Vec<f32> = matrix.iter().flatten().copied().collect();
        let model_buffer = buffer::create_buffer_with_data(
            &self.device,
            &matrix_flat,
            GPU_BUFFER_USAGE_UNIFORM | GPU_BUFFER_USAGE_COPY_DST,
        )?;

        let model_layout = pipeline.get_bind_group_layout(1);
        let model_entry  = web_sys::GpuBindGroupEntry::new_with_gpu_buffer(0, &model_buffer);
        let model_bg = self.device.create_bind_group(
            &web_sys::GpuBindGroupDescriptor::new(&[model_entry], &model_layout),
        );

        let material_index = self.assets.materials.len() as u32;
        self.assets.materials.push(Material {
            pipeline,
            bind_group: Some(model_bg),
        });
        let _ = model_buffer; // garde la variable alive jusqu'ici pour le bind group JS

        self.scene.add_mesh_renderer(
            transform,
            Handle::new(mesh_index),
            Handle::new(material_index),
        );

        Ok(())
    }

    // Appelé depuis requestAnimationFrame. dt est en secondes.
    pub fn tick(&mut self, dt: f32) -> Result<(), JsValue> {
        self.process_input(dt);
        self.upload_camera()?;
        self.draw_frame()
    }

    // Rendu direct sans mise à jour de la caméra (compatibilité).
    pub fn draw_frame(&self) -> Result<(), JsValue> {
        frame::draw_scene(
            &self.device,
            &self.context,
            &self.scene,
            &self.assets,
            self.camera_bind_group.as_ref(),
            &self.depth_texture,
        )
    }

    // --- API d'entrée appelée depuis JS ---

    // Appelé sur mousemove. buttons = e.buttons (bitmask: 1=gauche, 2=droit, 4=milieu).
    pub fn on_mouse_move(&mut self, dx: f32, dy: f32, buttons: u32) {
        self.left_down   = (buttons & 1) != 0;
        self.right_down  = (buttons & 2) != 0;
        self.middle_down = (buttons & 4) != 0;
        self.mouse_dx += dx;
        self.mouse_dy += dy;
    }

    // Appelé sur mousedown / mouseup. button = e.button (0=gauche, 1=milieu, 2=droit).
    pub fn on_mouse_button(&mut self, button: u32, down: bool) {
        match button {
            0 => self.left_down   = down,
            1 => self.middle_down = down,
            2 => self.right_down  = down,
            _ => {}
        }
    }

    // Appelé sur wheel. delta > 0 = scroll vers le bas = zoom out.
    pub fn on_scroll(&mut self, delta: f32) {
        self.scroll_delta += delta;
    }

    // Appelé sur keydown / keyup.
    pub fn on_key(&mut self, code: &str, down: bool) {
        match code {
            "KeyW" | "ArrowUp"    => self.key_w = down,
            "KeyS" | "ArrowDown"  => self.key_s = down,
            "KeyA" | "ArrowLeft"  => self.key_a = down,
            "KeyD" | "ArrowRight" => self.key_d = down,
            "Space"               => self.key_q = down,
            "ShiftLeft" | "ShiftRight" => self.key_e = down,
            _ => {}
        }
    }

    pub fn toggle_camera_mode(&mut self) {
        self.camera.toggle_mode();
    }

    pub fn camera_mode(&self) -> String {
        self.camera.mode_name().to_string()
    }

    // Expose l'aspect ratio pour la mettre à jour depuis JS (resize).
    pub fn set_aspect(&mut self, aspect: f32) {
        self.camera.aspect = aspect;
    }
}

// --- Fonctions privées ---

impl Render {
    fn process_input(&mut self, dt: f32) {
        let dx     = self.mouse_dx;
        let dy     = self.mouse_dy;
        let scroll = self.scroll_delta;
        self.mouse_dx    = 0.0;
        self.mouse_dy    = 0.0;
        self.scroll_delta = 0.0;

        match self.camera.mode {
            camera::CameraMode::Orbit => {
                if self.left_down   { self.camera.orbit(dx, dy); }
                if self.right_down || self.middle_down { self.camera.pan(dx, dy); }
                if scroll.abs() > 0.0 { self.camera.zoom(scroll); }
            }
            camera::CameraMode::Fps => {
                if self.left_down { self.camera.fps_look(dx, dy); }
                let fwd = if self.key_w { 1.0 } else if self.key_s { -1.0 } else { 0.0 };
                let rgt = if self.key_d { 1.0 } else if self.key_a { -1.0 } else { 0.0 };
                let up  = if self.key_q { 1.0 } else if self.key_e { -1.0 } else { 0.0 };
                self.camera.fps_move(fwd, rgt, up, dt);
            }
        }
    }

    fn upload_camera(&self) -> Result<(), JsValue> {
        let data = self.camera.uniform_data();
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4)
        };
        self.device
            .queue()
            .write_buffer_with_u32_and_u8_slice(&self.camera_uniform_buffer, 0, bytes)
    }
}

fn gpu_error(message: &str) -> JsValue {
    js_sys::Error::new(message).into()
}

// Calcule la model matrix TRS à partir d'un Transform.
pub(crate) fn compute_matrix(transform: Transform) -> [[f32; 4]; 4] {
    let position = Vector3::new(transform.x, transform.y, transform.z);
    let qlen_sq  = transform.rx * transform.rx
        + transform.ry * transform.ry
        + transform.rz * transform.rz
        + transform.rw * transform.rw;
    let rotation = if qlen_sq < 1e-6 {
        Quaternion::new(1.0, 0.0, 0.0, 0.0)
    } else {
        Quaternion::new(transform.rw, transform.rx, transform.ry, transform.rz).normalize()
    };
    let t = Matrix4::from_translation(position);
    let r = Matrix4::from(rotation);
    let s = Matrix4::from_nonuniform_scale(transform.sx, transform.sy, transform.sz);
    (t * r * s).into()
}
