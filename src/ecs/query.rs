// Query — itération typée sur les composants du World.
//
// L'API cible (voir ECS_DESIGN.md) est :
//   world.query::<(&Transform, &MeshRenderer)>()
//
// Phase 1 : helpers manuels simples (sans macros proc) suffisants pour
// TransformSystem, CameraSystem et RenderSystem.
//
// TODO Phase 2 : macro dérivée Query<T> avec filtres With<C> / Without<C>.

use std::any::TypeId;
use crate::ecs::entity::Entity;
use crate::ecs::world::World;

// Itère sur toutes les entités possédant le composant C, retourne (Entity, C).
pub struct QuerySingle<'w, C> {
    inner: Box<dyn Iterator<Item = (Entity, &'w [u8])> + 'w>,
    _marker: std::marker::PhantomData<C>,
}

impl<'w, C: Copy + 'static> QuerySingle<'w, C> {
    pub fn new(world: &'w World) -> Self {
        Self {
            inner: Box::new(world.query_raw(TypeId::of::<C>())),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'w, C: Copy + 'static> Iterator for QuerySingle<'w, C> {
    type Item = (Entity, C);

    fn next(&mut self) -> Option<Self::Item> {
        let (entity, bytes) = self.inner.next()?;
        let value: C = unsafe {
            std::ptr::read_unaligned(bytes.as_ptr() as *const C)
        };
        Some((entity, value))
    }
}

// Extension de World pour les queries typées.
pub trait WorldQuery {
    fn query<C: Copy + 'static>(&self) -> QuerySingle<C>;
}

impl WorldQuery for World {
    fn query<C: Copy + 'static>(&self) -> QuerySingle<C> {
        QuerySingle::new(self)
    }
}
