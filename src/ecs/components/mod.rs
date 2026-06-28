// Composants ECS — données pures, pas de logique.
//
// Chaque composant doit être :
//   - Copy + Clone (pas de Box, pas de Rc, données inline)
//   - repr(C) si utilisé comme buffer GPU (alignement garanti)
//   - 'static (pas de lifetimes internes)

pub mod transform;
pub mod camera;
pub mod mesh_renderer;
pub mod point_cloud;
pub mod tags;
