// Systems — logique opérant sur les composants du World.
//
// Chaque system est une fonction (ou un struct avec état) qui :
//   1. Lit des composants via World::query()
//   2. Écrit des composants via World::insert_component()
//   3. Accède aux ressources via World::resource()
//
// Ordre d'exécution (défini dans le scheduler) :
//   InputSystem → CameraSystem → TransformSystem → FrustumCullingSystem
//   → LODSystem → RenderSystem

pub mod transform_system;
pub mod camera_system;
