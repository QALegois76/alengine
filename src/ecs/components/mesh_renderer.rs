// Composant MeshRenderer — lie un mesh et un matériau à une entité.
//
// Les handles sont des indices stables dans les Vecs d'Assets.
// Compatible avec le système de Handle<T> existant dans World.rs.

use crate::models::Handle;
use crate::models::{Mesh, Material};

#[derive(Copy, Clone, Debug)]
pub struct MeshRendererComponent {
    pub mesh: Handle<Mesh>,
    pub material: Handle<Material>,
    // Flags de rendu (cast shadow, receive shadow, etc.)
    pub flags: u32,
}

impl MeshRendererComponent {
    pub fn new(mesh: Handle<Mesh>, material: Handle<Material>) -> Self {
        Self { mesh, material, flags: 0 }
    }
}
