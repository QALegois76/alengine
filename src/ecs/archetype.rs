// Archetype — groupe d'entités partageant exactement le même ensemble de composants.
//
// Stockage SoA (Structure of Arrays) : chaque type de composant a sa propre colonne
// de bytes contiguës. Cache-friendly pour les systems qui itèrent sur de nombreuses entités.
//
// Exemple : un archetype [Transform, MeshRenderer] contient :
//   entities : [E0, E1, E2, ...]
//   columns[0] (Transform)     : [T0_bytes, T1_bytes, T2_bytes, ...]
//   columns[1] (MeshRenderer) : [M0_bytes, M1_bytes, M2_bytes, ...]

use std::any::TypeId;
use crate::ecs::entity::Entity;

pub struct ComponentColumn {
    pub type_id: TypeId,
    pub stride: usize,      // taille d'un élément en bytes
    pub data: Vec<u8>,      // données brutes SoA
}

impl ComponentColumn {
    pub fn new(type_id: TypeId, stride: usize) -> Self {
        Self { type_id, stride, data: Vec::new() }
    }

    // Lit les bytes d'un composant à la ligne `row`.
    pub fn get_bytes(&self, row: usize) -> &[u8] {
        let start = row * self.stride;
        &self.data[start..start + self.stride]
    }

    // Écrit les bytes d'un composant à la ligne `row`.
    pub fn set_bytes(&mut self, row: usize, bytes: &[u8]) {
        let start = row * self.stride;
        self.data[start..start + self.stride].copy_from_slice(bytes);
    }

    // Ajoute un composant à la fin (push).
    pub fn push_bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    // Supprime la ligne `row` en la remplaçant par la dernière (swap-remove).
    pub fn swap_remove(&mut self, row: usize) {
        let last_start = self.data.len() - self.stride;
        let row_start = row * self.stride;
        self.data.copy_within(last_start..last_start + self.stride, row_start);
        self.data.truncate(self.data.len() - self.stride);
    }
}

// Signature d'un archétype : ensemble trié de TypeId de composants.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ArchetypeSignature(pub Vec<TypeId>);

impl ArchetypeSignature {
    pub fn new(mut types: Vec<TypeId>) -> Self {
        types.sort();
        types.dedup();
        Self(types)
    }

    pub fn contains(&self, type_id: &TypeId) -> bool {
        self.0.contains(type_id)
    }
}

pub struct Archetype {
    pub signature: ArchetypeSignature,
    pub entities: Vec<Entity>,
    pub columns: Vec<ComponentColumn>,
}

impl Archetype {
    pub fn new(signature: ArchetypeSignature, strides: Vec<(TypeId, usize)>) -> Self {
        let columns = strides
            .into_iter()
            .map(|(tid, stride)| ComponentColumn::new(tid, stride))
            .collect();
        Self {
            signature,
            entities: Vec::new(),
            columns,
        }
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    // Retourne l'index de la colonne pour un TypeId donné.
    pub fn column_index(&self, type_id: TypeId) -> Option<usize> {
        self.columns.iter().position(|c| c.type_id == type_id)
    }

    // Supprime une ligne (swap-remove) et retourne l'entité déplacée (si swap a eu lieu).
    pub fn remove_row(&mut self, row: usize) -> Option<Entity> {
        let last = self.entities.len() - 1;
        let swapped = if row < last { Some(self.entities[last]) } else { None };

        self.entities.swap_remove(row);
        for col in &mut self.columns {
            col.swap_remove(row);
        }
        swapped
    }
}
