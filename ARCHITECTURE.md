# alengine — État actuel de l'architecture

## Vue d'ensemble

`alengine` est une bibliothèque Rust compilée en WASM qui exploite **WebGPU** pour le rendu 3D temps réel dans le navigateur. L'objectif final est un moteur de type **Potree** : visualisation de nuages de points LiDAR massifs avec des capacités SIG avancées.

```
alengine/
├── src/                    # Bibliothèque Rust → WASM
│   ├── lib.rs              # Point d'entrée wasm-bindgen
│   ├── utils.rs            # Panic hook
│   ├── models/
│   │   └── World.rs        # Structures de données (Scene, Assets, Handle<T>)
│   ├── render/
│   │   ├── mod.rs          # API publique du renderer
│   │   ├── browser.rs      # Accès DOM (canvas)
│   │   ├── gpu.rs          # Init WebGPU (adapter, device, context)
│   │   ├── frame.rs        # Boucle de rendu (draw_scene)
│   │   ├── pipeline.rs     # Création des pipelines WGSL
│   │   ├── mesh_buffers.rs # Création des buffers vertex/index
│   │   ├── buffer.rs       # Utilitaire générique de buffer GPU
│   │   └── constants.rs    # Flags GPU, strides
│   └── 3d_models/
│       ├── ico_sphere.rs   # Génération de sphère ico subdivisée
│       └── triangle.rs     # Placeholder triangle
├── shaders/                # WGSL shaders
│   ├── ico_sphere.wgsl     # Shader diffus principal
│   ├── diffuse.wgsl
│   ├── glass.wgsl          # Fresnel / spéculaire
│   ├── texture.wgsl        # Checkerboard procédural
│   └── glow.wgsl           # Emissive
└── webTest/                # Serveur de debug JS/HTML
    ├── index.html
    ├── index.js
    └── index.css
```

---

## Ce qui est implémenté

### ✅ Initialisation WebGPU
- Requête d'adapter (`navigator.gpu.requestAdapter`)
- Création du device et de la queue
- Configuration du canvas context (format preferred)
- Gestion async via `wasm-bindgen-futures`

### ✅ Rendu de base
- Pipeline de rendu WGSL configurable par entité
- Vertex layout : `position: float32x3`, `normal: float32x3`
- Bind group pour la transform matrix (uniform buffer 4×4)
- Clear pass avec couleur de fond
- Draw indexed (triangles)

### ✅ Génération de maillage
- Icosphère subdivisée (3 niveaux → 4 096 triangles)
- Données entrelacées position + normale

### ✅ Système de transform
- Struct `Transform` exposée à JS via wasm-bindgen
- Calcul TRS : `Translation × Rotation(quaternion) × Scale`
- Uniform buffer pour la model matrix

### ✅ Shaders multiples
- Injection de shader personnalisé depuis JS
- 5 shaders inclus (diffus, verre, glow, texture, checkerboard)

### ✅ Interopérabilité JS
- `Render`, `Transform` accessibles depuis JavaScript
- `add_sphere(transform, shader)` chaînable

---

## Ce qui manque (gaps critiques)

| Fonctionnalité | Priorité | Impact |
|---|---|---|
| **Système ECS** | 🔴 Critique | Architecture de tout le reste |
| **Camera (View + Projection)** | 🔴 Critique | Impossible de naviguer dans la scène |
| **Boucle d'animation (requestAnimationFrame)** | 🔴 Critique | Rendu figé |
| **Depth buffer / Z-test** | 🔴 Critique | Artefacts de profondeur |
| **Rendu de nuage de points** | 🔴 Critique | Objectif principal du moteur |
| **Octree / LOD spatial** | 🔴 Critique | Performance avec millions de points |
| **Frustum culling** | 🟠 Haute | Performance |
| **GPU Instancing** | 🟠 Haute | Performance (points, végétation…) |
| **Chargement LAS/LAZ** | 🟠 Haute | Import de données réelles |
| **Input (souris, clavier)** | 🟠 Haute | Navigation |
| **Projection cartographique** | 🟡 Moyenne | Fonctions SIG |
| **Labels / overlays** | 🟡 Moyenne | SIG |
| **Post-processing** | 🟡 Moyenne | Qualité visuelle |

Voir [ECS_DESIGN.md](./ECS_DESIGN.md) et [PIPELINE_ROADMAP.md](./PIPELINE_ROADMAP.md) pour les détails d'implémentation.
