# Pipeline Roadmap — Vers un moteur Potree complet

## Pipeline de rendu actuel vs cible

### Actuel (frame unique, pas de caméra)

```
JS: draw_frame()
  └─ frame.rs: draw_scene()
       ├─ Get canvas texture view
       ├─ Create command encoder
       ├─ Begin render pass (clear)
       ├─ For each MeshRenderer:
       │    ├─ set_pipeline
       │    ├─ set_bind_group(0, transform_uniform)
       │    ├─ set_vertex_buffer
       │    ├─ set_index_buffer
       │    └─ draw_indexed
       └─ Submit
```

**Problèmes** :
- Pas de depth buffer → artefacts Z-fighting
- Pas de matrice view/projection → coordonnées monde = NDC (cassé pour vraies scènes)
- Pas de boucle requestAnimationFrame
- Un pipeline par objet → lent avec beaucoup d'entités

---

### Cible (pipeline Potree-like complet)

```
requestAnimationFrame loop
  │
  ├─ [CPU] ECS Systems
  │    ├─ InputSystem       — souris/clavier → CameraController
  │    ├─ CameraSystem      — View + Projection matrices
  │    ├─ TransformSystem   — TRS → GlobalTransform (Mat4)
  │    ├─ FrustumCulling    — tag Visible / Culled
  │    └─ LODSystem         — sélection des noeuds octree visibles
  │
  ├─ [CPU→GPU] Upload
  │    ├─ Camera UBO        — view_proj matrix (16 f32)
  │    ├─ Per-object UBO    — model matrix (instance buffer)
  │    └─ Point streaming   — noeuds LOD uploadés this frame
  │
  └─ [GPU] Render passes
       ├─ Pass 0: Shadow map (optionnel)
       ├─ Pass 1: Geometry
       │    ├─ Meshes (ECS MeshRenderer query)
       │    └─ Point clouds (instanced draw par noeud LOD)
       ├─ Pass 2: Transparents (trié arrière→avant)
       └─ Pass 3: Post-process (FXAA, EDL pour nuages de points)
```

---

## 1. Depth Buffer (priorité immédiate)

**Fichier** : `render/frame.rs` et `render/gpu.rs`

Créer une texture de profondeur lors de l'init et l'attacher à chaque render pass :

```rust
// Dans GpuState, ajouter :
pub depth_texture: GpuTexture,
pub depth_view: GpuTextureView,

// Création :
let depth_desc = GpuTextureDescriptor::new(
    &js_sys::Array::of3(&width.into(), &height.into(), &1u32.into()),
    GpuTextureFormat::Depth24plus,
);
depth_desc.set_usage(TEXTURE_USAGE_RENDER_ATTACHMENT);

// Dans le render pass descriptor :
let depth_attachment = GpuRenderPassDepthStencilAttachment::new(&depth_view);
depth_attachment.set_depth_clear_value(1.0);
depth_attachment.set_depth_load_op(GpuLoadOp::Clear);
depth_attachment.set_depth_store_op(GpuStoreOp::Store);
```

**Dans le pipeline** : activer le depth test :
```rust
let depth_stencil = GpuDepthStencilState::new(GpuTextureFormat::Depth24plus);
depth_stencil.set_depth_write_enabled(true);
depth_stencil.set_depth_compare(GpuCompareFunction::Less);
pipeline_desc.set_depth_stencil(&depth_stencil);
```

---

## 2. Camera System

**Nouveaux fichiers** : `src/ecs/components/camera.rs`, `src/ecs/systems/camera_system.rs`

```rust
pub struct Camera {
    pub fov_y_radians: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

// Uniform buffer partagé (bind group 0, binding 0)
#[repr(C)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub view_pos: [f32; 4],
}
```

**Shader mis à jour** (group 0 = camera, group 1 = model) :
```wgsl
@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> model: ModelUniform;

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = model.matrix * vec4(in.position, 1.0);
    out.clip_pos = camera.view_proj * world_pos;
    out.world_normal = (model.matrix * vec4(in.normal, 0.0)).xyz;
    out.world_pos = world_pos.xyz;
    return out;
}
```

---

## 3. Boucle d'animation

**`webTest/index.js`** (côté JS, simple) :
```javascript
let world; // instance ECS

async function init_engine() {
    world = await World.create();
    // ... spawn entities ...
    requestAnimationFrame(frame);
}

function frame(timestamp) {
    world.tick(timestamp);   // appel unique côté Rust
    requestAnimationFrame(frame);
}
```

**`src/render/mod.rs`** (côté Rust) :
```rust
#[wasm_bindgen]
pub fn tick(&mut self, timestamp: f64) {
    // 1. Mise à jour des systems
    self.run_systems(timestamp);
    // 2. Rendu
    self.draw_frame();
}
```

---

## 4. Rendu de nuages de points

### Pipeline dédié

Les nuages de points s'affichent avec une topologie `PointList` (pas de triangles).

```rust
// pipeline.rs — nouveau pipeline point cloud
let primitive = GpuPrimitiveState::new();
primitive.set_topology(GpuPrimitiveTopology::PointList);

// Vertex layout pour un point LAS simplifié
// Offset 0  : position XYZ (f32×3 = 12 bytes)
// Offset 12 : intensity (f32 = 4 bytes)
// Offset 16 : classification (u8, paddé à 4 bytes)
// Stride : 20 bytes
```

**Shader WGSL** (`shaders/point_cloud.wgsl`) :
```wgsl
struct CameraUniform { view_proj: mat4x4<f32>, view_pos: vec4<f32> }
struct NodeUniform   { world_offset: vec4<f32>, point_size: f32 }

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> node:   NodeUniform;

struct PointIn {
    @location(0) position: vec3<f32>,
    @location(1) intensity: f32,
    @location(2) classification: u32,
}

@vertex
fn vs_main(in: PointIn) -> @builtin(position) vec4<f32> {
    let world = in.position + node.world_offset.xyz;
    return camera.view_proj * vec4(world, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Coloration par intensité (mode simplifié)
    return vec4(1.0, 1.0, 1.0, 1.0);
}
```

### Coloration des points

| Mode | Description |
|---|---|
| Intensité | Niveaux de gris depuis le retour LiDAR |
| Classification | Couleur par classe (sol, végétation, bâtiment…) |
| Hauteur (Z) | Gradient colorimétrique |
| RGB | Si le scanner enregistre la couleur |
| Normale | Couleur depuis la normale estimée |

---

## 5. Octree LOD (cœur du système Potree)

### Structure

```
OctreeNode
├── bounds: AABB
├── level: u8                  // 0 = racine, max ~12
├── point_count: u32
├── gpu_buffer: Option<Handle<GpuBuffer>>   // null si pas encore uploadé
├── children: [Option<Handle<OctreeNode>>; 8]
└── state: NodeState           // Unloaded | Loading | Loaded | Evicted
```

### Algorithme de sélection (LOD)

```rust
fn select_nodes(
    root: &OctreeNode,
    camera: &Camera,
    frustum: &Frustum,
    point_budget: u32,
) -> Vec<NodeId> {
    let mut queue = BinaryHeap::new(); // trié par priorité (angle apparent)
    let mut selected = vec![];
    let mut remaining_budget = point_budget;

    queue.push(PriorityNode { node: root, priority: f32::MAX });

    while let Some(pn) = queue.pop() {
        if remaining_budget < pn.node.point_count { break; }
        if !frustum.intersects(&pn.node.bounds) { continue; }

        selected.push(pn.node.id);
        remaining_budget -= pn.node.point_count;

        // Ajouter les enfants avec leur priorité
        for child in pn.node.visible_children(camera) {
            let screen_size = child.bounds.screen_space_error(camera);
            if screen_size > LOD_THRESHOLD {
                queue.push(PriorityNode { node: child, priority: screen_size });
            }
        }
    }

    selected
}
```

### Streaming asynchrone

Les noeuds sont chargés à la demande (depuis URL ou fichier local via fetch JS) :

```rust
// Déclenché par LODSystem quand un noeud est visible mais Unloaded
async fn stream_node(node_id: NodeId, url: &str) -> Result<Vec<Point>, JsValue> {
    let response = JsFuture::from(window().fetch_with_str(url)).await?;
    let buffer = JsFuture::from(response.array_buffer()?).await?;
    // Décodage LAS/binaire propriétaire Potree
    Ok(decode_point_data(&buffer))
}
```

---

## 6. Chargement LAS/LAZ

### Format LAS 1.4

```
LAS Header (375 bytes)
├── Signature: "LASF"
├── Point Data Format ID (0-10)
├── Number of point records
├── Scale factors (X, Y, Z)
├── Offsets (X, Y, Z)
└── Min/Max bounds

Point Data Record (format 0 : 20 bytes)
├── X: i32 (applique scale + offset pour coordonnées réelles)
├── Y: i32
├── Z: i32
├── Intensity: u16
├── Return number / Classification flags: u8
├── Classification: u8
├── Scan angle: i16
├── User data: u8
└── Point source ID: u16
```

**Parsing en Rust** (`src/io/las.rs`) :
```rust
pub fn parse_las_header(data: &[u8]) -> Result<LasHeader, LasError> {
    let signature = &data[0..4];
    if signature != b"LASF" { return Err(LasError::InvalidSignature); }

    let point_count = u64::from_le_bytes(data[247..255].try_into()?);
    let scale_x = f64::from_le_bytes(data[131..139].try_into()?);
    // ... etc
}

pub fn points_to_gpu_buffer(
    points: &[LasPoint],
    scale: Vec3,
    offset: Vec3,
) -> Vec<f32> {
    points.iter().flat_map(|p| {
        let x = p.x as f64 * scale.x + offset.x;
        let y = p.y as f64 * scale.y + offset.y;
        let z = p.z as f64 * scale.z + offset.z;
        [x as f32, y as f32, z as f32, p.intensity as f32 / 65535.0]
    }).collect()
}
```

---

## 7. Eye-Dome Lighting (EDL)

Potree utilise EDL pour donner de la profondeur aux nuages de points sans calculer de normales.

**Principe** : post-process shader qui compare la profondeur d'un pixel avec ses voisins.

```wgsl
// shaders/edl.wgsl — pass de post-processing
@group(0) @binding(0) var depth_tex: texture_depth_2d;
@group(0) @binding(1) var color_tex: texture_2d<f32>;

fn edl_response(uv: vec2<f32>, radius: f32, depth: f32) -> f32 {
    let neighbors = array<vec2<f32>, 4>(
        vec2(radius, 0.0), vec2(-radius, 0.0),
        vec2(0.0, radius), vec2(0.0, -radius)
    );
    var sum = 0.0;
    for (var i = 0; i < 4; i++) {
        let n_depth = textureSample(depth_tex, s, uv + neighbors[i]);
        sum += max(0.0, log2(depth) - log2(n_depth));
    }
    return sum / 4.0;
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let color = textureSample(color_tex, s, uv);
    let depth = textureSample(depth_tex, s, uv);
    let edl = exp(-edl_response(uv, 1.0 / 512.0, depth) * 300.0);
    return vec4(color.rgb * edl, color.a);
}
```

---

## 8. Fonctions SIG

| Feature | Description | Implémentation |
|---|---|---|
| **Systèmes de coordonnées** | WGS84, Lambert93, UTM… | `src/gis/crs.rs` + proj4rs crate |
| **Reprojection** | Transformer un nuage entre CRS | Appliqué au chargement ou en shader |
| **Mesures 3D** | Distance, surface, volume | Queries ECS + maths géométriques |
| **Clipping box** | Découper le nuage dans un AABB | Uniform GPU + discard dans shader |
| **Classification** | Filtrage par classe LAS | Bit mask dans shader uniform |
| **Annotations** | Points, lignes, polygones 3D | Entités ECS avec composant `Annotation` |
| **Export** | LAS/LAZ, GeoJSON, CSV | `src/io/export.rs` |

---

## Ordre d'implémentation recommandé

```
Phase 1 — Fondations (ECS + Camera)
  ├─ ECS minimal (World, Entity, archetype simple)
  ├─ Camera + depth buffer
  ├─ requestAnimationFrame loop
  └─ Input souris basique (orbite)

Phase 2 — Point cloud basique
  ├─ Parser LAS binaire
  ├─ Pipeline PointList WebGPU
  ├─ Upload buffer + rendu points bruts
  └─ Coloration intensité / hauteur

Phase 3 — LOD + Octree
  ├─ Construction octree offline (outil CLI séparé)
  ├─ Streaming nœuds depuis serveur HTTP
  ├─ Algorithme sélection LOD
  └─ Budget de points dynamique

Phase 4 — Qualité + SIG
  ├─ EDL post-processing
  ├─ Reprojection CRS
  ├─ Mesures et annotations
  └─ Export
```
