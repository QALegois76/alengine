// TransformSystem — calcule GlobalTransform depuis LocalTransform.
//
// Itère sur toutes les entités avec LocalTransform et met à jour GlobalTransform.
// Dans un ECS complet, seuls les entités avec le flag "dirty" seraient recalculées.
//
// Dépendances : aucune (premier system de la frame).
// Produit : GlobalTransform à jour pour CameraSystem et RenderSystem.

use cgmath::{Matrix4, Quaternion, Vector3, InnerSpace};
use crate::ecs::components::transform::{LocalTransform, GlobalTransform};
use crate::ecs::world::World;

pub fn run(world: &mut World) {
    // Collecte d'abord pour éviter le borrow simultané mut/immut.
    let updates: Vec<_> = world
        .query_raw(std::any::TypeId::of::<LocalTransform>())
        .map(|(entity, bytes)| {
            let local: LocalTransform = unsafe {
                std::ptr::read_unaligned(bytes.as_ptr() as *const LocalTransform)
            };
            (entity, compute_matrix(&local))
        })
        .collect();

    for (entity, matrix) in updates {
        world.insert_component(entity, GlobalTransform { matrix });
    }
}

fn compute_matrix(local: &LocalTransform) -> [[f32; 4]; 4] {
    let [tx, ty, tz] = local.translation;
    let [rx, ry, rz, rw] = local.rotation;
    let [sx, sy, sz] = local.scale;

    let position = Vector3::new(tx, ty, tz);

    // Quaternion dégénéré (tout à zéro) → identité.
    let quat_len_sq = rx * rx + ry * ry + rz * rz + rw * rw;
    let rotation = if quat_len_sq < 1e-6 {
        Quaternion::new(1.0, 0.0, 0.0, 0.0)
    } else {
        Quaternion::new(rw, rx, ry, rz).normalize()
    };

    let t = Matrix4::from_translation(position);
    let r = Matrix4::from(rotation);
    let s = Matrix4::from_nonuniform_scale(sx, sy, sz);

    (t * r * s).into()
}
