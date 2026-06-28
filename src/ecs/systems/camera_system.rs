// CameraSystem — calcule la matrice view_proj et met à jour le CameraUniform GPU.
//
// Dépendances : TransformSystem (GlobalTransform à jour).
// Produit : CameraUniform uploadé dans le buffer GPU bind group 0.
//
// Algorithme :
//   1. Trouver l'entité Camera principale (resource ou query)
//   2. Lire son GlobalTransform → extraire position et orientation
//   3. Calculer view  = inverse(GlobalTransform.matrix)
//   4. Calculer proj  = perspective(fov, aspect, near, far)
//   5. view_proj = proj * view
//   6. Uploader dans le CameraUniformBuffer (resource GPU)

use cgmath::{Matrix4, Deg, perspective, SquareMatrix};
use crate::ecs::components::camera::{Camera, CameraUniform};
use crate::ecs::components::transform::GlobalTransform;
use crate::ecs::world::World;

// Ressource contenant l'entité caméra active.
pub struct ActiveCamera(pub crate::ecs::entity::Entity);

pub fn run(world: &mut World) {
    let active_camera_entity = match world.resource::<ActiveCamera>() {
        Some(ac) => ac.0,
        None => return,
    };

    let camera = match world.get_component::<Camera>(active_camera_entity) {
        Some(c) => c,
        None => return,
    };

    let global = match world.get_component::<GlobalTransform>(active_camera_entity) {
        Some(g) => g,
        None => return,
    };

    let view_matrix: Matrix4<f32> = Matrix4::from(global.matrix);

    // Inverse de la world matrix = view matrix.
    let view = view_matrix.invert().unwrap_or(Matrix4::identity());

    // Projection perspective (right-handed, profondeur [0, 1] pour WebGPU).
    let proj: Matrix4<f32> = perspective(
        Deg(camera.fov_y_radians.to_degrees()),
        camera.aspect_ratio,
        camera.near,
        camera.far,
    );

    // WebGPU : Y inversé par rapport à OpenGL.
    // Correction : multiplier la ligne Y de la projection par -1.
    let mut proj_corrected = proj;
    proj_corrected[1][1] *= -1.0;

    let view_proj = proj_corrected * view;

    // Position monde de la caméra (colonne 3 de la world matrix).
    let world_pos = [global.matrix[3][0], global.matrix[3][1], global.matrix[3][2], 1.0];

    let uniform = CameraUniform {
        view_proj: view_proj.into(),
        view_position: world_pos,
    };

    // TODO : uploader `uniform` dans le GpuBuffer dédié via device.queue().write_buffer().
    // Le buffer est une resource dans World : world.resource::<CameraGpuBuffer>().
    let _ = uniform; // placeholder jusqu'à l'implémentation du buffer
}
