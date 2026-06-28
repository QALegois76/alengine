// ECS — Entity Component System (architecture Unity DOTS-like)
//
// Structure :
//   World      → contient tous les archetypes + ressources globales
//   Archetype  → groupe d'entités avec le même ensemble de composants (SoA)
//   Entity     → identifiant unique (index + génération)
//   Component  → donnée pure, sans logique
//   System     → logique opérant sur des queries de composants
//
// Voir ECS_DESIGN.md pour l'architecture complète et les exemples.

pub mod entity;
pub mod archetype;
pub mod world;
pub mod query;
pub mod components;
pub mod systems;
