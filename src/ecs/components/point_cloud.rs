// Composant PointCloud — attaché aux entités représentant un nuage de points LiDAR.
//
// L'entité possède aussi un LocalTransform (position monde de l'origine du scan)
// et un AABB (bounding box pour le frustum culling et le LOD).
//
// Rendu : pipeline dédié avec topologie PointList (voir PIPELINE_ROADMAP.md §4).
// Chargement : streaming asynchrone depuis serveur HTTP (format Potree ou LAS brut).

// Mode de coloration des points.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ColorMode {
    Intensity,       // niveaux de gris depuis le retour LiDAR
    Classification,  // couleur par classe LAS (sol, végétation, bâtiment…)
    Height,          // gradient selon Z
    Rgb,             // couleur enregistrée par le scanner
    Normal,          // couleur depuis normale estimée
}

#[derive(Copy, Clone, Debug)]
pub struct PointCloud {
    // Budget de points affichés simultanément (toutes LOD confondues).
    pub point_budget: u32,
    // Taille des points en pixels (influencée par la distance en LOD adaptatif).
    pub point_size: f32,
    pub color_mode: ColorMode,
    // Index du nœud racine de l'octree dans un asset store (à définir).
    pub root_node_index: u32,
    // Système de référence de coordonnées (EPSG code, 0 = non défini).
    pub crs_epsg: u32,
}

impl PointCloud {
    pub fn new(point_budget: u32, root_node_index: u32) -> Self {
        Self {
            point_budget,
            point_size: 1.5,
            color_mode: ColorMode::Intensity,
            root_node_index,
            crs_epsg: 0,
        }
    }
}

// Nœud d'un octree LOD — un nœud = un batch de points à un niveau de détail.
// Correspond à un nœud Potree (fichier .bin / .laz sur le serveur).
#[derive(Copy, Clone, Debug)]
pub enum NodeState {
    Unloaded,   // pas encore demandé
    Loading,    // fetch JS en cours
    Ready,      // buffer GPU prêt à être bindé
    Evicted,    // buffer libéré (hors frustum depuis longtemps)
}

#[derive(Copy, Clone, Debug)]
pub struct OctreeNode {
    // Bounds axis-aligned de ce nœud (en coordonnées monde).
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub level: u8,
    pub point_count: u32,
    // Index dans un Vec<GpuBuffer> dans les assets GPU.
    pub gpu_buffer_index: Option<u32>,
    pub state: NodeState,
    // Indices des 8 enfants (-1 = absent).
    pub children: [i32; 8],
}
