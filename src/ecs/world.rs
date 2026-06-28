// World — registre central de l'ECS.
//
// Responsabilités :
//   - Gérer le cycle de vie des entités (spawn / despawn)
//   - Router chaque entité vers son archétype (via ArchetypeLocation)
//   - Exposer des queries typées pour les systems
//   - Stocker les ressources globales (Camera, GpuDevice, etc.)

use std::any::{Any, TypeId};
use std::collections::HashMap;
use crate::ecs::entity::{Entity, EntityPool};
use crate::ecs::archetype::{Archetype, ArchetypeSignature};

// Localise une entité dans un archétype.
pub struct ArchetypeLocation {
    pub archetype_id: usize,
    pub row: usize,
}

pub struct World {
    entities: EntityPool,
    archetypes: Vec<Archetype>,
    entity_locations: HashMap<Entity, ArchetypeLocation>,
    // Signature → index dans `archetypes`
    archetype_index: HashMap<ArchetypeSignature, usize>,
    // Ressources globales (singletons) : Camera, GpuDevice, DeltaTime…
    resources: HashMap<TypeId, Box<dyn Any>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: EntityPool::new(),
            archetypes: Vec::new(),
            entity_locations: HashMap::new(),
            archetype_index: HashMap::new(),
            resources: HashMap::new(),
        }
    }

    // --- Ressources ---

    pub fn insert_resource<R: Any>(&mut self, resource: R) {
        self.resources.insert(TypeId::of::<R>(), Box::new(resource));
    }

    pub fn resource<R: Any>(&self) -> Option<&R> {
        self.resources
            .get(&TypeId::of::<R>())
            .and_then(|b| b.downcast_ref::<R>())
    }

    pub fn resource_mut<R: Any>(&mut self) -> Option<&mut R> {
        self.resources
            .get_mut(&TypeId::of::<R>())
            .and_then(|b| b.downcast_mut::<R>())
    }

    // --- Entités ---

    pub fn spawn(&mut self) -> Entity {
        self.entities.spawn()
    }

    pub fn despawn(&mut self, entity: Entity) {
        if let Some(loc) = self.entity_locations.remove(&entity) {
            let swapped = self.archetypes[loc.archetype_id].remove_row(loc.row);
            // Mettre à jour la location de l'entité déplacée (swap-remove).
            if let Some(moved) = swapped {
                if let Some(l) = self.entity_locations.get_mut(&moved) {
                    l.row = loc.row;
                }
            }
        }
        self.entities.despawn(entity);
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }

    // --- Archetypes ---

    fn get_or_create_archetype(
        &mut self,
        sig: ArchetypeSignature,
        strides: Vec<(TypeId, usize)>,
    ) -> usize {
        if let Some(&id) = self.archetype_index.get(&sig) {
            return id;
        }
        let id = self.archetypes.len();
        self.archetypes.push(Archetype::new(sig.clone(), strides));
        self.archetype_index.insert(sig, id);
        id
    }

    // Insère des bytes bruts pour un composant sur une entité déjà dans un archétype.
    // Utilisé par les helpers typés ci-dessous.
    pub fn set_component_bytes(
        &mut self,
        entity: Entity,
        type_id: TypeId,
        bytes: &[u8],
    ) -> bool {
        let loc = match self.entity_locations.get(&entity) {
            Some(l) => (l.archetype_id, l.row),
            None => return false,
        };
        let col_idx = match self.archetypes[loc.0].column_index(type_id) {
            Some(i) => i,
            None => return false,
        };
        self.archetypes[loc.0].columns[col_idx].set_bytes(loc.1, bytes);
        true
    }

    pub fn get_component_bytes(
        &self,
        entity: Entity,
        type_id: TypeId,
    ) -> Option<&[u8]> {
        let loc = self.entity_locations.get(&entity)?;
        let archetype = &self.archetypes[loc.archetype_id];
        let col_idx = archetype.column_index(type_id)?;
        Some(archetype.columns[col_idx].get_bytes(loc.row))
    }

    // --- API typée (helper génériques) ---

    // Insère ou met à jour un composant sur une entité existante.
    // Si l'entité n'est pas encore dans un archétype, crée un archétype singleton.
    pub fn insert_component<C: Any + Copy>(&mut self, entity: Entity, component: C) {
        let type_id = TypeId::of::<C>();
        let stride = std::mem::size_of::<C>();
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(&component as *const C as *const u8, stride)
        };

        // Si l'entité est déjà dans un archétype qui contient ce composant → update.
        if self.set_component_bytes(entity, type_id, bytes) {
            return;
        }

        // Sinon : migration vers un nouvel archétype (ajout de colonne).
        // Cas simple : archétype à un seul composant (point de départ).
        let sig = ArchetypeSignature::new(vec![type_id]);
        let arch_id = self.get_or_create_archetype(sig, vec![(type_id, stride)]);

        let row = self.archetypes[arch_id].entities.len();
        self.archetypes[arch_id].entities.push(entity);
        self.archetypes[arch_id].columns[0].push_bytes(bytes);

        self.entity_locations.insert(entity, ArchetypeLocation { archetype_id: arch_id, row });
    }

    pub fn get_component<C: Any + Copy>(&self, entity: Entity) -> Option<C> {
        let bytes = self.get_component_bytes(entity, TypeId::of::<C>())?;
        if bytes.len() != std::mem::size_of::<C>() {
            return None;
        }
        let value: C = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const C) };
        Some(value)
    }

    // Itère sur toutes les entités d'un archétype contenant un composant donné.
    // Retourne les paires (Entity, bytes_du_composant).
    // Pour des queries multi-composants, voir query.rs.
    pub fn query_raw(&self, type_id: TypeId) -> impl Iterator<Item = (Entity, &[u8])> {
        self.archetypes
            .iter()
            .filter(move |a| a.signature.contains(&type_id))
            .flat_map(move |a| {
                let col_idx = a.column_index(type_id).unwrap();
                let col = &a.columns[col_idx];
                a.entities.iter().enumerate().map(move |(row, &entity)| {
                    (entity, col.get_bytes(row))
                })
            })
    }
}
