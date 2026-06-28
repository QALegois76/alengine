// Tags — composants "marqueurs" sans données (taille zéro).
//
// Utilisés par les queries pour filtrer les entités sans coût mémoire.
// Exemple : With<Visible> sélectionne uniquement les entités passées le frustum culling.

// L'entité est visible cette frame (dans le frustum).
#[derive(Copy, Clone, Debug)]
pub struct Visible;

// L'entité a été rejetée par le frustum culling.
#[derive(Copy, Clone, Debug)]
pub struct Culled;

// L'entité est sélectionnée par l'utilisateur (outil de sélection SIG).
#[derive(Copy, Clone, Debug)]
pub struct Selected;

// L'entité est statique (ne se déplace jamais → GlobalTransform calculé une seule fois).
#[derive(Copy, Clone, Debug)]
pub struct Static;

// L'entité est en cours de chargement (streaming asynchrone).
#[derive(Copy, Clone, Debug)]
pub struct Loading;
