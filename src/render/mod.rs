mod browser;
mod buffer;
pub mod camera;
mod constants;
mod frame;
mod geo;
mod gpu;
mod grid;
mod mesh_buffers;
mod pipeline;
mod texture;

use crate::models::{Assets, Handle, Material, Mesh, Scene, Transform};
use cgmath::{InnerSpace, Matrix4, Quaternion, Vector3};
use constants::{
    GPU_BUFFER_USAGE_COPY_DST, GPU_BUFFER_USAGE_INDEX, GPU_BUFFER_USAGE_UNIFORM,
    GPU_BUFFER_USAGE_VERTEX, GPU_SHADER_STAGE_FRAGMENT, GPU_SHADER_STAGE_VERTEX,
};
use geo::CoordinateTransform;
use std::collections::{HashMap, HashSet};
use wasm_bindgen::prelude::*;
use web_sys::{
    GpuBindGroup, GpuBindGroupLayout, GpuBuffer, GpuCanvasContext, GpuDevice, GpuPipelineLayout,
    GpuPrimitiveTopology, GpuRenderPipeline, GpuSampler, GpuTexture,
};

// Mesh de debug (lignes) en espace monde : le marqueur d'origine.
struct DebugMesh {
    vertex_buffer: GpuBuffer,
    index_buffer: GpuBuffer,
    index_count: u32,
}

// Tuile de fond de carte : un quad texturé + son bind group texture (group 2).
struct BasemapTile {
    mesh: DebugMesh,
    texture: GpuBindGroup,
}

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
    camera_bind_group: GpuBindGroup,

    // Layouts explicites partagés par tous les pipelines.
    model_bind_group_layout: GpuBindGroupLayout,
    pipeline_layout: GpuPipelineLayout,

    // Repères plein écran : grilles infinies (XY/XZ/YZ) et axes infinis (X/Y/Z).
    grid_pipeline: GpuRenderPipeline,
    axis_pipeline: GpuRenderPipeline,
    plane_bind_groups: [GpuBindGroup; 3],
    axis_bind_groups: [GpuBindGroup; 3],
    plane_visible: [bool; 3],
    axes_visible: [bool; 3],

    // Marqueur d'origine (lignes).
    line_pipeline: GpuRenderPipeline,
    identity_model_bind_group: GpuBindGroup,
    origin: DebugMesh,
    origin_visible: bool,

    // Globe : tuiles satellite drapées sur la sphère, streamées dynamiquement.
    basemap_pipeline: GpuRenderPipeline,
    texture_bind_group_layout: GpuBindGroupLayout,
    sampler: GpuSampler,
    globe_enabled: bool,
    globe_radius: f32,
    globe_tiles: HashMap<(u32, u32, u32), BasemapTile>, // cache (z, col, row) → tuile
    globe_requested: HashSet<(u32, u32, u32)>,          // tuiles en cours de fetch
    globe_visible: HashSet<(u32, u32, u32)>,            // fenêtre courante à dessiner

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

        let camera_data = cam.uniform_data();
        let camera_uniform_buffer = buffer::create_buffer_with_data(
            &gpu_state.device,
            &camera_data,
            GPU_BUFFER_USAGE_UNIFORM | GPU_BUFFER_USAGE_COPY_DST,
        )?;

        // --- Layouts explicites, partagés par tous les pipelines ---
        // group(0) = caméra (vertex + fragment), group(1) = model / id de repère.
        let camera_bind_group_layout = pipeline::create_uniform_bind_group_layout(
            &gpu_state.device,
            GPU_SHADER_STAGE_VERTEX | GPU_SHADER_STAGE_FRAGMENT,
        )?;
        let model_bind_group_layout = pipeline::create_uniform_bind_group_layout(
            &gpu_state.device,
            GPU_SHADER_STAGE_VERTEX | GPU_SHADER_STAGE_FRAGMENT,
        )?;
        let pipeline_layout = pipeline::create_scene_pipeline_layout(
            &gpu_state.device,
            &camera_bind_group_layout,
            &model_bind_group_layout,
        );

        // Camera bind group (group 0), créé une fois depuis le layout explicite.
        let camera_entry =
            web_sys::GpuBindGroupEntry::new_with_gpu_buffer(0, &camera_uniform_buffer);
        let camera_bind_group = gpu_state.device.create_bind_group(
            &web_sys::GpuBindGroupDescriptor::new(&[camera_entry], &camera_bind_group_layout),
        );

        // Model bind group identité (group 1) pour le marqueur d'origine.
        let identity: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let identity_buffer = buffer::create_buffer_with_data(
            &gpu_state.device,
            &identity,
            GPU_BUFFER_USAGE_UNIFORM | GPU_BUFFER_USAGE_COPY_DST,
        )?;
        let identity_entry =
            web_sys::GpuBindGroupEntry::new_with_gpu_buffer(0, &identity_buffer);
        let identity_model_bind_group = gpu_state.device.create_bind_group(
            &web_sys::GpuBindGroupDescriptor::new(&[identity_entry], &model_bind_group_layout),
        );

        // Pipelines repères : lignes (origine), grilles infinies, axes infinis.
        let line_pipeline = pipeline::create_pipeline_from_shader(
            &gpu_state.device,
            gpu_state.format,
            &pipeline_layout,
            grid::LINE_SHADER,
            GpuPrimitiveTopology::LineList,
        )?;
        let grid_src = grid::grid_shader();
        let grid_pipeline = pipeline::create_grid_pipeline(
            &gpu_state.device,
            gpu_state.format,
            &pipeline_layout,
            &grid_src,
        )?;
        let axis_src = grid::axis_shader();
        let axis_pipeline = pipeline::create_grid_pipeline(
            &gpu_state.device,
            gpu_state.format,
            &pipeline_layout,
            &axis_src,
        )?;

        // Uniformes d'id (group 1) : plan (0=XY,1=XZ,2=YZ) et axe (0=X,1=Y,2=Z).
        let plane_bind_groups = [
            make_id_bind_group(&gpu_state.device, &model_bind_group_layout, 0)?,
            make_id_bind_group(&gpu_state.device, &model_bind_group_layout, 1)?,
            make_id_bind_group(&gpu_state.device, &model_bind_group_layout, 2)?,
        ];
        let axis_bind_groups = [
            make_id_bind_group(&gpu_state.device, &model_bind_group_layout, 0)?,
            make_id_bind_group(&gpu_state.device, &model_bind_group_layout, 1)?,
            make_id_bind_group(&gpu_state.device, &model_bind_group_layout, 2)?,
        ];

        // Marqueur d'origine.
        let (origin_v, origin_i) = grid::origin_mesh(0.12);
        let origin = make_debug_mesh(&gpu_state.device, &origin_v, &origin_i)?;

        // Fond de carte : layout texture (group 2), sampler et pipeline texturé.
        let texture_bind_group_layout =
            texture::create_texture_bind_group_layout(&gpu_state.device)?;
        let sampler = texture::create_sampler(&gpu_state.device);
        let textured_pipeline_layout = pipeline::create_textured_pipeline_layout(
            &gpu_state.device,
            &camera_bind_group_layout,
            &model_bind_group_layout,
            &texture_bind_group_layout,
        );
        let basemap_pipeline = pipeline::create_pipeline_from_shader(
            &gpu_state.device,
            gpu_state.format,
            &textured_pipeline_layout,
            texture::TEXTURED_SHADER,
            GpuPrimitiveTopology::TriangleList,
        )?;

        let mut render = Self {
            device: gpu_state.device,
            context: gpu_state.context,
            format: gpu_state.format,
            scene: Scene::new(),
            assets: Assets { meshes: Vec::new(), materials: Vec::new() },
            camera: cam,
            camera_uniform_buffer,
            camera_bind_group,
            model_bind_group_layout,
            pipeline_layout,
            grid_pipeline,
            axis_pipeline,
            plane_bind_groups,
            axis_bind_groups,
            plane_visible: [false, false, false], // off par défaut (le globe est la scène)
            axes_visible: [true, true, true],
            line_pipeline,
            identity_model_bind_group,
            origin,
            origin_visible: true,
            basemap_pipeline,
            texture_bind_group_layout,
            sampler,
            globe_enabled: true,
            globe_radius: geo::GLOBE_RADIUS,
            globe_tiles: HashMap::new(),
            globe_requested: HashSet::new(),
            globe_visible: HashSet::new(),
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
        };

        // Vue par défaut : la France (priorité de travail), pays entier visible.
        render.focus_on(2.5, 46.6, 0.22);

        Ok(render)
    }

    // Ajoute une sphère ico dans la scène.
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
        let pipeline = pipeline::create_pipeline_from_shader(
            &self.device,
            self.format,
            &self.pipeline_layout,
            shader,
            GpuPrimitiveTopology::TriangleList,
        )?;

        // --- Model uniform buffer (group 1) ---
        let matrix = compute_matrix(transform);
        let matrix_flat: Vec<f32> = matrix.iter().flatten().copied().collect();
        let model_buffer = buffer::create_buffer_with_data(
            &self.device,
            &matrix_flat,
            GPU_BUFFER_USAGE_UNIFORM | GPU_BUFFER_USAGE_COPY_DST,
        )?;

        let model_entry  = web_sys::GpuBindGroupEntry::new_with_gpu_buffer(0, &model_buffer);
        let model_bg = self.device.create_bind_group(
            &web_sys::GpuBindGroupDescriptor::new(&[model_entry], &self.model_bind_group_layout),
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
        self.update_clip_planes();
        self.upload_camera()?;
        self.draw_frame()
    }

    // Rendu direct sans mise à jour de la caméra (compatibilité).
    pub fn draw_frame(&self) -> Result<(), JsValue> {
        let mut planes: Vec<&GpuBindGroup> = Vec::new();
        for (i, bg) in self.plane_bind_groups.iter().enumerate() {
            if self.plane_visible[i] {
                planes.push(bg);
            }
        }
        let mut axes: Vec<&GpuBindGroup> = Vec::new();
        for (i, bg) in self.axis_bind_groups.iter().enumerate() {
            if self.axes_visible[i] {
                axes.push(bg);
            }
        }
        let mut lines: Vec<frame::DebugMeshRef> = Vec::new();
        if self.origin_visible {
            let m = &self.origin;
            lines.push((&m.vertex_buffer, &m.index_buffer, m.index_count));
        }

        let overlay = frame::Overlay {
            grid_pipeline: &self.grid_pipeline,
            grid_planes: &planes,
            axis_pipeline: &self.axis_pipeline,
            axes: &axes,
            line_pipeline: &self.line_pipeline,
            line_model_bind_group: &self.identity_model_bind_group,
            lines: &lines,
        };

        let mut basemap_tiles: Vec<frame::BasemapTileRef> = Vec::new();
        if self.globe_enabled {
            for key in &self.globe_visible {
                if let Some(t) = self.globe_tiles.get(key) {
                    basemap_tiles.push((
                        &t.texture,
                        &t.mesh.vertex_buffer,
                        &t.mesh.index_buffer,
                        t.mesh.index_count,
                    ));
                }
            }
        }
        let basemap = frame::Basemap {
            pipeline: &self.basemap_pipeline,
            model_bind_group: &self.identity_model_bind_group,
            tiles: &basemap_tiles,
        };

        frame::draw_scene(
            &self.device,
            &self.context,
            &self.scene,
            &self.assets,
            Some(&self.camera_bind_group),
            &self.depth_texture,
            Some(&basemap),
            Some(&overlay),
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

    // --- API repères : plans de grille, axes, origine ---

    pub fn set_plane_xy_visible(&mut self, visible: bool) { self.plane_visible[0] = visible; }
    pub fn set_plane_xz_visible(&mut self, visible: bool) { self.plane_visible[1] = visible; }
    pub fn set_plane_yz_visible(&mut self, visible: bool) { self.plane_visible[2] = visible; }

    pub fn set_axis_x_visible(&mut self, visible: bool) { self.axes_visible[0] = visible; }
    pub fn set_axis_y_visible(&mut self, visible: bool) { self.axes_visible[1] = visible; }
    pub fn set_axis_z_visible(&mut self, visible: bool) { self.axes_visible[2] = visible; }

    pub fn set_origin_visible(&mut self, visible: bool) { self.origin_visible = visible; }

    // --- API globe / fond de carte dynamique ---

    pub fn set_globe_enabled(&mut self, enabled: bool) {
        self.globe_enabled = enabled;
        if !enabled {
            self.globe_tiles.clear();
            self.globe_requested.clear();
            self.globe_visible.clear();
        }
    }

    // Recentre l'orbite sur le point du globe sous le pixel (NDC ∈ [-1,1]).
    // Garde l'œil immobile ; renvoie true si le rayon touche le globe.
    pub fn set_orbit_from_screen(&mut self, ndc_x: f32, ndc_y: f32) -> bool {
        let (eye, dir) = self.camera.screen_ray(ndc_x, ndc_y);
        let r = self.globe_radius;
        // |eye + t·dir|² = r² ; dir unitaire → a = 1.
        let b = 2.0 * (eye[0] * dir[0] + eye[1] * dir[1] + eye[2] * dir[2]);
        let c = eye[0] * eye[0] + eye[1] * eye[1] + eye[2] * eye[2] - r * r;
        let disc = b * b - 4.0 * c;
        if disc < 0.0 {
            return false;
        }
        let sq = disc.sqrt();
        let t0 = (-b - sq) * 0.5;
        let t1 = (-b + sq) * 0.5;
        let t = if t0 > 0.0 { t0 } else { t1 };
        if t <= 0.0 {
            return false;
        }
        let hit = [eye[0] + t * dir[0], eye[1] + t * dir[1], eye[2] + t * dir[2]];
        self.camera.orbit_to_point(hit);
        true
    }

    // Transforme une coordonnée terrestre (lon/lat deg, alt unités monde) en
    // position monde WebGPU. Base des futurs calques SIG.
    pub fn geo_to_world(&self, lon: f32, lat: f32, alt: f32) -> Vec<f32> {
        let transform = geo::Geographic { radius: self.globe_radius as f64 };
        let p = transform.to_world(lon as f64, lat as f64, alt as f64);
        vec![p[0], p[1], p[2]]
    }

    // Centre la vue sur une coordonnée terrestre (lon/lat deg) à une altitude
    // donnée (unités monde au-dessus de la surface). Le point passe au nadir.
    pub fn focus_on(&mut self, lon: f32, lat: f32, altitude: f32) {
        let p = self.geo_to_world(lon, lat, 0.0);
        self.camera.orbit_to_point([p[0], p[1], p[2]]);
        self.camera.distance = self.globe_radius + altitude.max(self.globe_radius * 5.0e-6);
    }

    // Recalcule l'ensemble de tuiles désirées selon la caméra (zoom + fenêtre),
    // évince le superflu et renvoie les tuiles manquantes à charger : un tableau
    // plat [z, col, row, z, col, row, …] consommé par JS pour le fetch WMTS.
    pub fn update_globe(&mut self) -> Vec<i32> {
        if !self.globe_enabled {
            return Vec::new();
        }

        // Altitude au-dessus de la surface → niveau de zoom.
        let eye = self.camera.position();
        let dist = (eye[0] * eye[0] + eye[1] * eye[1] + eye[2] * eye[2]).sqrt();
        let alt = (dist - self.globe_radius).max(1e-4);
        // Niveau de zoom : taille de tuile ≈ résolution écran. La constante (20)
        // est calibrée pour atteindre z19 (~0,2 m/px à 45° de latitude).
        let z = ((20.0 / alt).log2().round() as i32).clamp(1, 19) as u32;
        let n = 1i64 << z;

        // Point du globe sous la caméra → tuile centrale.
        let inv = 1.0 / dist.max(1e-6);
        let dir = [eye[0] * inv, eye[1] * inv, eye[2] * inv];
        let (lon, lat) = geo::world_dir_to_lonlat(dir);
        let lat = lat.clamp(-85.0, 85.0);
        let (cc, cr) = geo::lonlat_to_tile(lon, lat, z);
        let center_col = cc.floor() as i64;
        let center_row = cr.floor() as i64;

        // Fenêtre de tuiles autour du centre. Longitude qui boucle (mod n),
        // latitude bornée (pas de tuile au-delà des limites Mercator).
        const W: i64 = 3;
        let mut desired: HashSet<(u32, u32, u32)> = HashSet::new();
        for d_row in -W..=W {
            let row = center_row + d_row;
            if row < 0 || row >= n {
                continue;
            }
            for d_col in -W..=W {
                let col = ((center_col + d_col) % n + n) % n; // wrap seamless
                desired.insert((z, col as u32, row as u32));
            }
        }

        // Garde en cache toutes les tuiles du zoom courant (le pan reste
        // instantané quand on revient) ; lâche les autres niveaux de zoom.
        self.globe_tiles.retain(|key, _| key.0 == z);
        self.globe_requested.retain(|key| key.0 == z && desired.contains(key));

        // Borne mémoire : au-delà de la capacité, lâche les tuiles hors fenêtre.
        const CACHE_CAP: usize = 256;
        if self.globe_tiles.len() > CACHE_CAP {
            let extra: Vec<(u32, u32, u32)> = self
                .globe_tiles
                .keys()
                .filter(|k| !desired.contains(*k))
                .cloned()
                .collect();
            for k in extra {
                if self.globe_tiles.len() <= CACHE_CAP {
                    break;
                }
                self.globe_tiles.remove(&k);
            }
        }

        // Fenêtre courante à dessiner.
        self.globe_visible = desired.clone();

        // Tuiles manquantes (ni en cache, ni déjà demandées).
        let mut pending: Vec<i32> = Vec::new();
        for key in &desired {
            if !self.globe_tiles.contains_key(key) && !self.globe_requested.contains(key) {
                self.globe_requested.insert(*key);
                pending.push(key.0 as i32);
                pending.push(key.1 as i32);
                pending.push(key.2 as i32);
            }
        }
        pending
    }

    // Reçoit une tuile chargée (ImageBitmap) et la drape sur le globe.
    pub fn add_globe_tile(
        &mut self,
        z: u32,
        x: u32,
        y: u32,
        bitmap: web_sys::ImageBitmap,
    ) -> Result<(), JsValue> {
        let key = (z, x, y);
        self.globe_requested.remove(&key);

        let (verts, indices) = geo::build_tile_mesh(z, x, y, self.globe_radius as f64);
        let mesh = make_debug_mesh(&self.device, &verts, &indices)?;

        let texture = texture::upload_bitmap(&self.device, &bitmap)?;
        let view = texture.create_view()?;
        let tex_entry = web_sys::GpuBindGroupEntry::new_with_gpu_texture_view(0, &view);
        let samp_entry = web_sys::GpuBindGroupEntry::new(1, &self.sampler);
        let texture_bg = self.device.create_bind_group(
            &web_sys::GpuBindGroupDescriptor::new(
                &[tex_entry, samp_entry],
                &self.texture_bind_group_layout,
            ),
        );

        self.globe_tiles.insert(key, BasemapTile { mesh, texture: texture_bg });
        Ok(())
    }

    // Abandonne une demande de tuile (échec de fetch) pour pouvoir réessayer.
    pub fn cancel_globe_tile(&mut self, z: u32, x: u32, y: u32) {
        self.globe_requested.remove(&(z, x, y));
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
                // Pan désactivé sur le globe : l'orbite reste centrée sur la Terre.
                if !self.globe_enabled && (self.right_down || self.middle_down) {
                    self.camera.pan(dx, dy);
                }
                if scroll.abs() > 0.0 {
                    if self.globe_enabled {
                        // Zoom sur l'ALTITUDE au-dessus de la surface : le pas est
                        // grand en altitude (traversée rapide) et minuscule près du
                        // sol (réglage fin). La caméra reste hors du globe car
                        // distance = rayon + altitude > rayon.
                        let r = self.globe_radius;
                        let alt = (self.camera.distance - r).max(r * 5.0e-6);
                        let step = (scroll * 0.0025).clamp(-0.33, 0.33);
                        let new_alt = (alt * (1.0 + step)).max(r * 5.0e-6);
                        self.camera.distance = r + new_alt;
                    } else {
                        self.camera.zoom(scroll);
                    }
                }
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

    // Adapte les plans near/far à l'altitude : sinon le near fixe (0.01) coupe
    // les tuiles dès qu'on descend près du sol (écran vide → zoom bloqué haut).
    fn update_clip_planes(&mut self) {
        if !self.globe_enabled {
            return;
        }
        let eye = self.camera.position();
        let d = (eye[0] * eye[0] + eye[1] * eye[1] + eye[2] * eye[2]).sqrt();
        let r = self.globe_radius;
        let alt = (d - r).max(r * 1e-7);
        self.camera.near = (alt * 0.25).max(1e-9);
        self.camera.far = d + r * 1.1;
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

// Crée les buffers GPU d'un mesh de debug (lignes) à partir de données CPU.
fn make_debug_mesh(
    device: &GpuDevice,
    vertices: &[f32],
    indices: &[u16],
) -> Result<DebugMesh, JsValue> {
    let vertex_buffer = buffer::create_buffer_with_data(
        device,
        vertices,
        GPU_BUFFER_USAGE_VERTEX | GPU_BUFFER_USAGE_COPY_DST,
    )?;
    let index_buffer = buffer::create_buffer_with_data(
        device,
        indices,
        GPU_BUFFER_USAGE_INDEX | GPU_BUFFER_USAGE_COPY_DST,
    )?;
    Ok(DebugMesh {
        vertex_buffer,
        index_buffer,
        index_count: indices.len() as u32,
    })
}

// Crée un bind group group(1) ne portant qu'un id (plan ou axe) en info.x.
fn make_id_bind_group(
    device: &GpuDevice,
    layout: &GpuBindGroupLayout,
    id: u32,
) -> Result<GpuBindGroup, JsValue> {
    let data: [u32; 4] = [id, 0, 0, 0];
    let buffer = buffer::create_buffer_with_data(
        device,
        &data,
        GPU_BUFFER_USAGE_UNIFORM | GPU_BUFFER_USAGE_COPY_DST,
    )?;
    let entry = web_sys::GpuBindGroupEntry::new_with_gpu_buffer(0, &buffer);
    Ok(device.create_bind_group(
        &web_sys::GpuBindGroupDescriptor::new(&[entry], layout),
    ))
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
