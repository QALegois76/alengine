use wasm_bindgen::prelude::*;
use web_sys::{GpuBindGroup, GpuBuffer, GpuRenderPipeline};

pub struct Handle<T> {
    pub index: u32,
    pub generation: u32,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Handle<T> {
    pub fn new(index: u32) -> Self {
        Self {
            index,
            generation: 0,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> Copy for Handle<T> {}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Handle")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish()
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

#[wasm_bindgen]
#[derive(Copy, Clone)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub z: f32,

    pub rx: f32,
    pub ry: f32,
    pub rz: f32,
    pub rw: f32,

    pub sx: f32,
    pub sy: f32,
    pub sz: f32,
}

#[wasm_bindgen]
impl Transform {
    pub fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            rw: 1.0,
            sx: 1.0,
            sy: 1.0,
            sz: 1.0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct MeshRenderer {
    pub mesh: Handle<Mesh>,
    pub material: Handle<Material>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entity(u32);

pub struct Scene {
    pub entities: Vec<Entity>,

    pub transforms: Vec<Transform>,
    pub mesh_renderers: Vec<Option<MeshRenderer>>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            mesh_renderers: Vec::new(),
            transforms: Vec::new(),
        }
    }

    pub fn add_mesh_renderer(
        &mut self,
        transform: Transform,
        mesh: Handle<Mesh>,
        material: Handle<Material>,
    ) -> Entity {
        let entity = Entity(self.entities.len() as u32);
        self.entities.push(entity);
        self.transforms.push(transform);
        self.mesh_renderers
            .push(Some(MeshRenderer { mesh, material }));
        entity
    }
}

pub struct Assets {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Mesh {
    pub vertex_buffer: GpuBuffer,
    pub index_buffer: GpuBuffer,
    pub index_count: u32,
}

pub struct Material {
    pub pipeline: GpuRenderPipeline,
    pub bind_group: Option<GpuBindGroup>,
}

pub struct RenderItem {
    pub mesh: Handle<Mesh>,
    pub material: Handle<Material>,
    pub transform: [[f32; 4]; 4],
}

pub struct Renderer {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub pipelines: Vec<GpuRenderPipeline>,
}

pub struct BindGroup;
