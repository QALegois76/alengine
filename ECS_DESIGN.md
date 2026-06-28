# ECS Design — Architecture Unity-like pour alengine

## Concepts fondamentaux

Un **ECS (Entity Component System)** sépare :
- **Entity** : identifiant unique (un entier), sans données propres
- **Component** : donnée pure, pas de logique (`Transform`, `PointCloud`, `Camera`…)
- **System** : logique qui opère sur des ensembles de composants via des requêtes

L'approche Unity DOTS (Data-Oriented Technology Stack) repose sur les **Archetypes** :  
un archétype est un groupe d'entités partageant exactement le même ensemble de types de composants.  
Cela permet un stockage SoA (Structure of Arrays) cache-friendly.

---

## Architecture cible

```
World
├── archetypes: Vec<Archetype>        ← groupes d'entités homogènes
├── entity_index: HashMap<Entity, ArchetypeLocation>
└── resources: HashMap<TypeId, Box<dyn Any>>  ← singletons (GPU, Camera…)

Archetype
├── component_types: Vec<ComponentTypeId>   ← signature
├── columns: Vec<ComponentColumn>           ← une colonne par type
└── entities: Vec<Entity>

ComponentColumn
└── data: Vec<u8>  ← données brutes, interprétées par TypeId + stride
```

### Entity

```rust
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,   // invalide les anciennes références
}
```

### Archetype location

```rust
pub struct ArchetypeLocation {
    pub archetype_id: usize,
    pub row: usize,
}
```

---

## Composants prévus

```rust
// Positionnement dans l'espace monde
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

// Matrice calculée (cache du Transform, mise à jour par TransformSystem)
pub struct GlobalTransform {
    pub matrix: Mat4,
}

// Rendu de mesh classique
pub struct MeshRenderer {
    pub mesh: Handle<Mesh>,
    pub material: Handle<Material>,
}

// Nuage de points LiDAR
pub struct PointCloud {
    pub root_node: Handle<OctreeNode>,
    pub point_budget: u32,
    pub crs: CoordinateSystem,
}

// Caméra
pub struct Camera {
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    pub projection: Mat4,       // calculée par CameraSystem
    pub view: Mat4,             // calculée depuis GlobalTransform
}

// Tag : l'entité est rendue cette frame
pub struct Visible;

// Tag : l'entité n'est PAS dans le frustum
pub struct Culled;

// Tag : l'entité est sélectionnée (UI/SIG)
pub struct Selected;

// Données SIG associées à une entité
pub struct GisAttributes {
    pub properties: HashMap<String, AttributeValue>,
    pub crs: CoordinateSystem,
}
```

---

## Systems

Les systems s'exécutent dans un ordre défini (graph de dépendances).

```
┌─────────────────────────────────────────────┐
│                  FRAME                      │
│                                             │
│  1. InputSystem          (events JS → ECS)  │
│  2. CameraSystem         (view + proj)      │
│  3. TransformSystem      (TRS → GlobalMat4) │
│  4. FrustumCullingSystem (tag Visible/Culled)│
│  5. LODSystem            (octree selection)  │
│  6. PointCloudStreamSystem (upload GPU)     │
│  7. RenderSystem         (draw calls)       │
│  8. UISystem             (overlays, labels) │
└─────────────────────────────────────────────┘
```

### Exemple : TransformSystem

```rust
pub fn transform_system(world: &mut World) {
    // Query : entités avec Transform + GlobalTransform
    for (transform, global) in world.query_mut::<(&Transform, &mut GlobalTransform)>() {
        global.matrix = Mat4::from_scale_rotation_translation(
            transform.scale,
            transform.rotation,
            transform.translation,
        );
    }
}
```

### Exemple : FrustumCullingSystem

```rust
pub fn frustum_culling_system(world: &mut World) {
    let camera = world.resource::<Camera>();
    let frustum = Frustum::from_view_proj(camera.view * camera.projection);

    for (entity, (global, aabb)) in world.query::<(&GlobalTransform, &AABB)>().iter() {
        if frustum.contains_aabb(&aabb.transform(global.matrix)) {
            world.insert_tag(entity, Visible);
            world.remove_tag::<Culled>(entity);
        } else {
            world.insert_tag(entity, Culled);
            world.remove_tag::<Visible>(entity);
        }
    }
}
```

---

## Query API

Inspirée de Bevy/hecs — simple à implémenter en Rust :

```rust
// Lecture seule
for (transform, renderer) in world.query::<(&Transform, &MeshRenderer)>() { ... }

// Écriture
for (mut transform,) in world.query_mut::<(&mut Transform,)>() { ... }

// Avec filtre de tag
for entity in world.query_filtered::<&Transform, With<Visible>>() { ... }

// Entité unique (resource/singleton)
let camera = world.resource::<Camera>();
```

---

## Intégration JS via wasm-bindgen

L'API publique reste simple côté JavaScript :

```javascript
// Créer le monde
const world = await World.create();

// Spawner une entité
const entity = world.spawn()
    .with_transform(Transform.at(0, 0, 0))
    .with_point_cloud("/data/scan.las", { pointBudget: 2_000_000 })
    .build();

// Ajouter une caméra
world.spawn()
    .with_transform(Transform.at(0, 5, 10))
    .with_camera({ fovY: 60, near: 0.1, far: 10000 })
    .build();

// Boucle de rendu
function frame() {
    world.tick();         // exécute tous les systems
    requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
```

---

## Plan d'implémentation (ordre recommandé)

1. **`src/ecs/entity.rs`** — `Entity`, `Generation`
2. **`src/ecs/archetype.rs`** — `Archetype`, `ComponentColumn`
3. **`src/ecs/world.rs`** — `World`, `spawn`, `despawn`, `query`
4. **`src/ecs/query.rs`** — `Query<T>`, `QueryMut<T>`, filtres `With<C>`, `Without<C>`
5. **`src/ecs/system.rs`** — trait `System`, `SystemSchedule`
6. **Composants** — `Transform`, `GlobalTransform`, `Camera`, `MeshRenderer`
7. **Systems** — `TransformSystem`, `CameraSystem`, `RenderSystem`
8. **`PointCloud` + `OctreeNode`** — voir [PIPELINE_ROADMAP.md](./PIPELINE_ROADMAP.md)
